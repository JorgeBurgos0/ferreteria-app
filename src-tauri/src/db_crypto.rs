use crate::{auth::SessionState, db::{self, DbConnection}, mobile_scanner::MobileScannerState};
use rusqlite::{params, Connection, OpenFlags};
use std::{fs, io::Read, path::{Path, PathBuf}, sync::Mutex};
use tauri::State;

const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";
const STAGING_NAME: &str = ".ferreteria-antes-cifrado.db";

enum Mode { New, Plain, Locked, Open(String) }

pub struct CryptoState { path: PathBuf, mode: Mutex<Mode> }

#[derive(serde::Serialize)]
pub struct CryptoStatus { pub first_run: bool, pub encrypted: bool, pub locked: bool }

impl CryptoState {
    pub fn new(path: PathBuf, first_run: bool, encrypted: bool) -> Self {
        let mode = if first_run { Mode::New } else if encrypted { Mode::Locked } else { Mode::Plain };
        Self { path, mode: Mutex::new(mode) }
    }

    pub fn status(&self) -> Result<CryptoStatus, String> {
        let mode = self.mode.lock().map_err(|_| "Estado de cifrado no disponible".to_string())?;
        Ok(CryptoStatus {
            first_run: matches!(*mode, Mode::New),
            encrypted: matches!(*mode, Mode::Locked | Mode::Open(_)),
            locked: matches!(*mode, Mode::New | Mode::Locked),
        })
    }

    pub fn key(&self) -> Result<Option<String>, String> {
        let mode = self.mode.lock().map_err(|_| "Estado de cifrado no disponible".to_string())?;
        match &*mode {
            Mode::Plain => Ok(None),
            Mode::Open(key) => Ok(Some(key.clone())),
            Mode::New | Mode::Locked => Err("Primero abre la base de datos".into()),
        }
    }
}

pub fn recover_interrupted_swap(path: &Path) -> Result<(), String> {
    let staging = path.with_file_name(STAGING_NAME);
    if !path.exists() && staging.exists() {
        fs::rename(staging, path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn is_encrypted(path: &Path) -> Result<bool, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 16];
    let read = file.read(&mut header).map_err(|e| e.to_string())?;
    if read == 0 { return Ok(false); }
    if read != 16 { return Err("La base de datos tiene una cabecera incompleta".into()); }
    Ok(&header != SQLITE_HEADER)
}

fn key_connection(conn: &Connection, password: &str) -> Result<(), String> {
    if password.is_empty() { return Err("Escribe la contraseña maestra".into()); }
    conn.pragma_update(None, "key", password).map_err(|e| e.to_string())?;
    let _: String = conn.query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .map_err(|_| "Esta compilación no incluye SQLCipher".to_string())?;
    Ok(())
}

pub fn open_encrypted(path: &Path, password: &str, read_only: bool) -> Result<Connection, String> {
    let flags = if read_only { OpenFlags::SQLITE_OPEN_READ_ONLY } else { OpenFlags::SQLITE_OPEN_READ_WRITE };
    let conn = Connection::open_with_flags(path, flags).map_err(|e| e.to_string())?;
    key_connection(&conn, password)?;
    let check: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|_| "Contraseña maestra incorrecta o base de datos dañada".to_string())?;
    if check != "ok" { return Err("La base de datos no superó la comprobación de integridad".into()); }
    Ok(conn)
}

pub fn create_keyed_destination(path: &Path, password: &str) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    key_connection(&conn, password)?;
    Ok(conn)
}

pub fn create_database_file(path: &Path, password: Option<&str>) -> Result<Connection, String> {
    if path.exists() { return Err("Ya existe una base de datos".into()); }
    if password.is_some_and(|key| key.chars().count() < 12) {
        return Err("La contraseña maestra debe tener al menos 12 caracteres".into());
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    drop(file);
    let result = (|| -> Result<Connection, String> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(|e| e.to_string())?;
        if let Some(key) = password { key_connection(&conn, key)?; }
        db::inicializar(&conn).map_err(|e| e.to_string())?;
        if is_encrypted(path)? != password.is_some() { return Err("La base no se creó en el modo elegido".into()); }
        Ok(conn)
    })();
    if result.is_err() { let _ = fs::remove_file(path); }
    result
}

pub fn encrypt_copy(source: &Connection, destination: &Path, password: &str) -> Result<(), String> {
    if password.chars().count() < 12 { return Err("La contraseña maestra debe tener al menos 12 caracteres".into()); }
    if destination.exists() { return Err("El archivo temporal ya existe".into()); }
    let result = (|| -> Result<(), String> {
        source.execute("ATTACH DATABASE ?1 AS encrypted KEY ?2", params![destination.to_string_lossy(), password])
            .map_err(|e| e.to_string())?;
        let export_result = (|| -> Result<(), String> {
            source.query_row("SELECT sqlcipher_export('encrypted')", [], |_| Ok(())).map_err(|e| e.to_string())?;
            let version: i64 = source.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(|e| e.to_string())?;
            source.execute_batch(&format!("PRAGMA encrypted.user_version = {version}")).map_err(|e| e.to_string())?;
            Ok(())
        })();
        let detach_result = source.execute_batch("DETACH DATABASE encrypted").map_err(|e| e.to_string());
        export_result?;
        detach_result?;
        let encrypted = open_encrypted(destination, password, true)?;
        let actual_version: i64 = encrypted.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        let source_version: i64 = source.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        if actual_version != source_version { return Err("La copia cifrada perdió la versión de la base".into()); }
        let tables: i64 = encrypted.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        let original_tables: i64 = source.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'", [], |row| row.get(0)).map_err(|e| e.to_string())?;
        if tables != original_tables { return Err("La copia cifrada no contiene todas las tablas".into()); }
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(destination); }
    result
}

fn reopen_plain(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(|e| e.to_string())
}

/// Returns whether the temporary plaintext file was removed after a verified swap.
pub fn convert_plain_database(current: &mut Connection, path: &Path, password: &str) -> Result<bool, String> {
    let staging = path.with_file_name(STAGING_NAME);
    if staging.exists() { return Err("Hay una conversión anterior pendiente de revisar".into()); }
    let temp = path.with_file_name(format!(".ferreteria-cifrada-{}-{}.db", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos()));
    encrypt_copy(current, &temp, password)?;
    crate::commands::backups::protect_file(&temp)?;
    let placeholder = Connection::open_in_memory().map_err(|e| e.to_string())?;
    let previous = std::mem::replace(current, placeholder);
    drop(previous);
    if let Err(error) = fs::rename(path, &staging) {
        *current = reopen_plain(path)?;
        let _ = fs::remove_file(temp);
        return Err(error.to_string());
    }
    if let Err(error) = fs::rename(&temp, path) {
        fs::rename(&staging, path).map_err(|rollback| format!(
            "Falló la conversión ({error}) y la recuperación ({rollback}); conserva {}", staging.display()
        ))?;
        *current = reopen_plain(path)?;
        let _ = fs::remove_file(temp);
        return Err(error.to_string());
    }
    let encrypted = (|| -> Result<Connection, String> {
        let conn = open_encrypted(path, password, false)?;
        db::inicializar(&conn).map_err(|e| e.to_string())?;
        Ok(conn)
    })();
    let encrypted = match encrypted {
        Ok(conn) => conn,
        Err(error) => {
            fs::remove_file(path).map_err(|cleanup| format!(
                "La copia cifrada falló ({error}) y no se pudo retirarla ({cleanup}); conserva {}", staging.display()
            ))?;
            fs::rename(&staging, path).map_err(|rollback| format!(
                "La copia cifrada falló ({error}) y la recuperación también ({rollback}); conserva {}", staging.display()
            ))?;
            *current = reopen_plain(path)?;
            return Err(error);
        }
    };
    *current = encrypted;
    Ok(fs::remove_file(staging).is_ok())
}

#[tauri::command]
pub fn estado_cifrado(state: State<CryptoState>) -> Result<CryptoStatus, String> { state.status() }

#[tauri::command]
pub fn configurar_base_inicial(encrypted: bool, password: Option<String>, state: State<CryptoState>, db: State<DbConnection>) -> Result<(), String> {
    let mut mode = state.mode.lock().map_err(|_| "Estado de cifrado no disponible".to_string())?;
    if !matches!(*mode, Mode::New) { return Err("La base ya fue configurada".into()); }
    let password = if encrypted {
        let value = password.ok_or("Escribe la contraseña maestra")?;
        if value.chars().count() < 12 { return Err("La contraseña maestra debe tener al menos 12 caracteres".into()); }
        Some(value)
    } else { None };
    let conn = create_database_file(&state.path, password.as_deref())?;
    *db.0.lock().map_err(|_| "Base no disponible".to_string())? = conn;
    *mode = match password { Some(key) => Mode::Open(key), None => Mode::Plain };
    Ok(())
}

#[tauri::command]
pub fn desbloquear_base(password: String, state: State<CryptoState>, db: State<DbConnection>) -> Result<(), String> {
    let mut mode = state.mode.lock().map_err(|_| "Estado de cifrado no disponible".to_string())?;
    if !matches!(*mode, Mode::Locked) { return Err("La base ya está abierta".into()); }
    let conn = open_encrypted(&state.path, &password, false)?;
    db::inicializar(&conn).map_err(|e| e.to_string())?;
    *db.0.lock().map_err(|_| "Base no disponible".to_string())? = conn;
    *mode = Mode::Open(password);
    let staging = state.path.with_file_name(STAGING_NAME);
    if staging.exists() {
        if let Err(error) = fs::remove_file(&staging) {
            eprintln!("No se pudo retirar la copia temporal sin cifrar {}: {error}", staging.display());
        }
    }
    Ok(())
}

#[tauri::command]
pub fn activar_cifrado(password: String, state: State<CryptoState>, db: State<DbConnection>,
    session: State<SessionState>, scanner: State<MobileScannerState>) -> Result<(), String> {
    let user = session.current_user()?;
    if !user.permissions.iter().any(|p| p == "gestionar_usuarios") {
        return Err("Solo una cuenta con gestión de usuarios puede activar el cifrado".into());
    }
    if password.chars().count() < 12 { return Err("La contraseña maestra debe tener al menos 12 caracteres".into()); }
    let mut mode = state.mode.lock().map_err(|_| "Estado de cifrado no disponible".to_string())?;
    if !matches!(*mode, Mode::Plain) { return Err("La base ya está cifrada".into()); }
    scanner.stop()?;
    let mut current = db.0.lock().map_err(|_| "Base no disponible".to_string())?;
    let removed_plaintext = convert_plain_database(&mut current, &state.path, &password)?;
    *mode = Mode::Open(password);
    crate::commands::auditoria::registrar_evento_interno(&current, &user.username, "ACTIVAR_CIFRADO", None)
        .map_err(|e| e.to_string())?;
    if !removed_plaintext {
        return Err("La base está cifrada, pero no se pudo retirar una copia temporal anterior sin cifrar; revisa la carpeta de datos de la aplicación".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::backups;

    fn test_dir() -> PathBuf {
        let root = std::env::temp_dir().join(format!("ferreteria-crypto-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn encrypt_copy_preserves_schema_and_rejects_wrong_password() {
        let root = test_dir();
        let plain = root.join("plain.db");
        let encrypted = root.join("encrypted.db");
        let conn = Connection::open(&plain).unwrap();
        db::inicializar(&conn).unwrap();
        conn.execute("INSERT INTO productos (sku, nombre, precio_venta, stock_actual) VALUES ('TEST-1', 'Martillo', 100, 2)", []).unwrap();
        encrypt_copy(&conn, &encrypted, "contraseña maestra segura").unwrap();
        assert!(!is_encrypted(&plain).unwrap());
        assert!(is_encrypted(&encrypted).unwrap());
        assert!(open_encrypted(&encrypted, "incorrecta", false).is_err());
        let opened = open_encrypted(&encrypted, "contraseña maestra segura", false).unwrap();
        assert_eq!(opened.query_row::<String, _, _>("SELECT nombre FROM productos WHERE sku = 'TEST-1'", [], |row| row.get(0)).unwrap(), "Martillo");
        assert_eq!(opened.query_row::<i64, _, _>("PRAGMA user_version", [], |row| row.get(0)).unwrap(), 7);
        drop(opened);
        drop(conn);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn encrypted_backup_remains_encrypted() {
        let root = test_dir();
        let plain = root.join("plain.db");
        let encrypted = root.join("encrypted.db");
        let backup = root.join("backup.db");
        let restored_path = root.join("restored.db");
        let safety_path = root.join("safety.db");
        let conn = Connection::open(&plain).unwrap();
        db::inicializar(&conn).unwrap();
        conn.execute("INSERT INTO usuarios (username, password_hash, rol) VALUES ('admin', 'hash', 'admin')", []).unwrap();
        conn.execute("INSERT INTO productos (sku, nombre, precio_venta, stock_actual) VALUES ('TEST-2', 'Clavo', 5, 10)", []).unwrap();
        encrypt_copy(&conn, &encrypted, "contraseña maestra segura").unwrap();
        let opened = open_encrypted(&encrypted, "contraseña maestra segura", false).unwrap();
        backups::create_verified_key(&opened, &backup, Some("contraseña maestra segura")).unwrap();
        assert!(is_encrypted(&backup).unwrap());
        assert!(open_encrypted(&backup, "incorrecta", true).is_err());
        open_encrypted(&backup, "contraseña maestra segura", true).unwrap();
        let mut restored = create_keyed_destination(&restored_path, "contraseña maestra segura").unwrap();
        db::inicializar(&restored).unwrap();
        restored.execute("INSERT INTO usuarios (username, password_hash, rol) VALUES ('admin', 'hash', 'admin')", []).unwrap();
        crate::commands::configuracion::restore_database_key(&backup, &mut restored, &safety_path, Some("contraseña maestra segura")).unwrap();
        assert!(is_encrypted(&safety_path).unwrap());
        let name: String = restored.query_row("SELECT nombre FROM productos WHERE sku = 'TEST-2'", [], |row| row.get(0)).unwrap();
        assert_eq!(name, "Clavo");
        drop(restored);
        drop(opened);
        drop(conn);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn conversion_replaces_plain_database_without_losing_rows() {
        let root = test_dir();
        let path = root.join("ferreteria.db");
        let mut conn = Connection::open(&path).unwrap();
        db::inicializar(&conn).unwrap();
        conn.execute("INSERT INTO productos (sku, nombre, precio_venta, stock_actual) VALUES ('ANTES', 'Tornillo', 1, 12)", []).unwrap();
        assert!(convert_plain_database(&mut conn, &path, "contraseña maestra segura").unwrap());
        assert!(is_encrypted(&path).unwrap());
        assert!(!path.with_file_name(STAGING_NAME).exists());
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM productos WHERE sku = 'ANTES'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 1);
        drop(conn);
        let reopened = open_encrypted(&path, "contraseña maestra segura", false).unwrap();
        let count: i64 = reopened.query_row("SELECT COUNT(*) FROM productos WHERE sku = 'ANTES'", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 1);
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn new_database_can_be_encrypted_or_plain() {
        let root = test_dir();
        let encrypted_path = root.join("encrypted.db");
        let plain_path = root.join("plain.db");
        let encrypted = create_database_file(&encrypted_path, Some("contraseña maestra segura")).unwrap();
        let plain = create_database_file(&plain_path, None).unwrap();
        assert!(is_encrypted(&encrypted_path).unwrap());
        assert!(!is_encrypted(&plain_path).unwrap());
        assert!(create_database_file(&encrypted_path, None).is_err());
        drop(encrypted);
        drop(plain);
        fs::remove_dir_all(root).unwrap();
    }
}
