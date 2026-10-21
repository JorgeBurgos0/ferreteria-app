use crate::commands::auditoria;
use crate::{auth::SessionState, db::DbConnection};
use chrono::Local;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{backup, Connection, OpenFlags, params};
use std::path::Path;
use tauri::{AppHandle, Manager, State};

type DbConn<'a> = State<'a, DbConnection>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DatosEmpresa {
    nombre: String,
    direccion: String,
    telefono: String,
    logo: String,
}

fn logo_as_data_uri(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    let bytes = if let Some(encoded) = value.strip_prefix("data:image/png;base64,")
        .or_else(|| value.strip_prefix("data:image/jpeg;base64,")) {
        STANDARD.decode(encoded).map_err(|_| "Logo inválido".to_string())?
    } else {
        std::fs::read(value).map_err(|_| "No se pudo leer el logo".to_string())?
    };
    if bytes.len() > 1024 * 1024 {
        return Err("El logo debe medir menos de 1 MB".into());
    }
    let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg"
    } else {
        return Err("El logo debe ser PNG o JPEG".into());
    };
    Ok(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}

#[tauri::command]
pub fn obtener_datos_empresa(db: DbConn) -> Result<DatosEmpresa, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    
    let mut datos = conn.query_row(
        "SELECT nombre, direccion, telefono, logo FROM datos_empresa WHERE id = 1",
        [],
        |row| {
            Ok(DatosEmpresa {
                nombre: row.get(0)?,
                direccion: row.get(1)?,
                telefono: row.get(2)?,
                logo: row.get(3)?,
            })
        }
    ).map_err(|e| e.to_string())?;

    if !datos.logo.is_empty() && !datos.logo.starts_with("data:") {
        if let Ok(encoded) = logo_as_data_uri(&datos.logo) {
            conn.execute("UPDATE datos_empresa SET logo = ?1 WHERE id = 1", params![encoded])
                .map_err(|e| e.to_string())?;
            datos.logo = encoded;
        } else {
            datos.logo.clear();
        }
    }

    Ok(datos)
}

#[tauri::command]
pub fn guardar_datos_empresa(
    db: DbConn,
    nombre: String,
    direccion: String,
    telefono: String,
    logo: String,
    session: State<SessionState>,
) -> Result<(), String> {
    let logo = logo_as_data_uri(&logo)?;
    let actor = session.current_user()?.username;
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE datos_empresa SET nombre = ?1, direccion = ?2, telefono = ?3, logo = ?4 WHERE id = 1",
        params![nombre, direccion, telefono, logo],
    ).map_err(|e| e.to_string())?;
    auditoria::registrar_evento_interno(&tx, &actor, "DATOS_EMPRESA", None)
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}


pub fn restore_database(backup_path: &Path, conn_dest: &mut Connection, safety_path: &Path) -> Result<(), String> {
    if !backup_path.exists() {
        return Err("El archivo de respaldo no existe".to_string());
    }
    if conn_dest.path().is_some_and(|path| {
        Path::new(path).canonicalize().ok() == backup_path.canonicalize().ok()
    }) {
        return Err("El respaldo y la base actual son el mismo archivo".into());
    }
    let conn_backup = Connection::open_with_flags(backup_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let integrity: String = conn_backup.query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return Err("El respaldo está dañado".into());
    }
    let required_tables: i64 = conn_backup.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('usuarios', 'productos', 'ventas', 'detalle_ventas', 'roles', 'datos_empresa')",
        [], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if required_tables != 6 {
        return Err("El archivo no es un respaldo de esta aplicación".into());
    }
    let users: i64 = conn_backup.query_row("SELECT COUNT(*) FROM usuarios", [], |row| row.get(0))
        .map_err(|_| "El archivo no es un respaldo válido de esta aplicación".to_string())?;
    if users == 0 {
        return Err("El respaldo no contiene usuarios".into());
    }
    let mut safety = Connection::open(safety_path).map_err(|e| e.to_string())?;
    {
        let backup = backup::Backup::new(&*conn_dest, &mut safety).map_err(|e| e.to_string())?;
        backup.step(-1).map_err(|e| e.to_string())?;
    }
    let restore_result = (|| -> Result<(), String> {
        {
            let backup = backup::Backup::new(&conn_backup, conn_dest).map_err(|e| e.to_string())?;
            backup.step(-1).map_err(|e| e.to_string())?;
        }
        crate::db::inicializar(conn_dest).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(error) = restore_result {
        let recovery = backup::Backup::new(&safety, conn_dest).map_err(|e| e.to_string())?;
        recovery.step(-1).map_err(|e| e.to_string())?;
        return Err(format!("Restauración cancelada: {error}. Se conservó una copia en {}", safety_path.display()));
    }
    Ok(())
}

#[tauri::command]
pub fn restaurar_backup(ruta_backup: String, db: DbConn, app: AppHandle, session: State<SessionState>) -> Result<(), String> {
    let mut conn_dest = db.0.lock().map_err(|e| e.to_string())?;
    let safety_path = app.path().app_data_dir().map_err(|e| e.to_string())?
        .join(format!("respaldo-previo-restauracion-{}.db", Local::now().timestamp_millis()));
    restore_database(Path::new(&ruta_backup), &mut conn_dest, &safety_path)?;
    session.clear()?;

    Ok(())
}

#[tauri::command]
pub fn crear_backup(ruta_destino: String, db: DbConn, session: State<SessionState>) -> Result<(), String> {
    let usuario = session.current_user()?.username;
    // 1. Lock the current database connection (Source)
    let conn_src = db.0.lock().map_err(|e| e.to_string())?;

    // 2. Open connection to the destination file (will be created if not exists)
    let dest_path = Path::new(&ruta_destino);
    // Ensure parent dir exists? Usually handled by save dialog but good to check?
    // Rusqlite open will create the file, but not parent dirs.
    // We assume the path comes from a save dialog which ensures valid path.

    let mut conn_dest = Connection::open(dest_path).map_err(|e| e.to_string())?;

    // 3. Perform the backup (Backup from 'conn_src' TO 'conn_dest')
    // backup::Backup::new(destination, destination_name, source, source_name)
    // Destination is &mut conn_dest
    // Source is &conn_src (DbConnection mutex guard behaves like &Connection)
    let backup = backup::Backup::new(&conn_src, &mut conn_dest).map_err(|e| e.to_string())?;

    // 4. Run step(-1) to copy everything
    backup.step(-1).map_err(|e| e.to_string())?;

    // 5. Audit Log
    let detalles = format!("Creó respaldo en: {}", ruta_destino);
    auditoria::registrar_evento_interno(&conn_src, &usuario, "CREAR_BACKUP", Some(&detalles))
        .map_err(|e| e.to_string())?;

    Ok(())
}
