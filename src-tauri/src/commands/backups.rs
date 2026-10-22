use crate::commands::auditoria;
use chrono::Local;
use rusqlite::{backup, params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};
#[cfg(unix)] use std::os::unix::fs::PermissionsExt;

const PREFIX: &str = "ferreteria-auto-";

pub fn protect_file(path: &Path) -> Result<(), String> {
    #[cfg(unix)] fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    #[cfg(not(unix))] let _ = path;
    Ok(())
}

#[derive(Clone, Deserialize, Serialize)]
pub struct BackupSettings {
    pub enabled: bool,
    pub folder: String,
    pub keep: i64,
    pub audit_days: i64,
    #[serde(default)] pub last_backup: Option<String>,
    #[serde(default)] pub last_error: Option<String>,
}

pub fn settings(conn: &Connection) -> Result<BackupSettings, String> {
    conn.query_row(
        "SELECT respaldo_automatico, carpeta_respaldos, conservar_respaldos, retencion_bitacora_dias, ultimo_respaldo, ultimo_error FROM configuracion_local WHERE id = 1",
        [],
        |row| Ok(BackupSettings { enabled: row.get(0)?, folder: row.get(1)?, keep: row.get(2)?, audit_days: row.get(3)?, last_backup: row.get(4)?, last_error: row.get(5)? }),
    ).map_err(|e| e.to_string())
}

pub fn validate_settings(value: &BackupSettings) -> Result<(), String> {
    if !(1..=30).contains(&value.keep) {
        return Err("Conserva entre 1 y 30 respaldos automáticos".into());
    }
    if value.audit_days != 0 && !(90..=3650).contains(&value.audit_days) {
        return Err("La retención de bitácora debe ser indefinida o de 90 a 3650 días".into());
    }
    if value.audit_days > 0 && !value.enabled {
        return Err("Activa los respaldos automáticos antes de limitar la bitácora".into());
    }
    if value.enabled {
        let folder = Path::new(&value.folder);
        if !folder.is_dir() {
            return Err("Selecciona una carpeta local existente para respaldos".into());
        }
        let probe = folder.join(format!(".ferreteria-escritura-{}-{}", std::process::id(), Local::now().timestamp_nanos_opt().unwrap_or(0)));
        fs::OpenOptions::new().write(true).create_new(true).open(&probe)
            .map_err(|_| "No se puede escribir en la carpeta seleccionada".to_string())?;
        fs::remove_file(probe).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn verify(path: &Path, key: Option<&str>) -> Result<(), String> {
    let conn = if let Some(password) = key {
        crate::db_crypto::open_encrypted(path, password, true)?
    } else {
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| e.to_string())?
    };
    let check: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if check != "ok" {
        return Err("El respaldo no superó la verificación de integridad".into());
    }
    let tables: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('usuarios', 'productos', 'ventas', 'auditoria')",
        [], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if tables != 4 {
        return Err("El respaldo no contiene las tablas esperadas".into());
    }
    Ok(())
}

pub fn create_verified(conn: &Connection, destination: &Path) -> Result<(), String> {
    create_verified_key(conn, destination, None)
}

pub fn create_verified_key(conn: &Connection, destination: &Path, key: Option<&str>) -> Result<(), String> {
    if destination.exists() {
        return Err("Ya existe un archivo con ese nombre".into());
    }
    let parent = destination.parent().ok_or("Destino de respaldo inválido")?;
    if !parent.is_dir() {
        return Err("La carpeta de respaldo no existe".into());
    }
    let temp = parent.join(format!(".ferreteria-{}-{}.tmp", std::process::id(), Local::now().timestamp_nanos_opt().unwrap_or(0)));
    let result = (|| -> Result<(), String> {
        let mut target = if let Some(password) = key {
            crate::db_crypto::create_keyed_destination(&temp, password)?
        } else {
            Connection::open(&temp).map_err(|e| e.to_string())?
        };
        protect_file(&temp)?;
        {
            let backup = backup::Backup::new(conn, &mut target).map_err(|e| e.to_string())?;
            backup.step(-1).map_err(|e| e.to_string())?;
        }
        drop(target);
        verify(&temp, key)?;
        fs::rename(&temp, destination).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(&temp); }
    result
}

fn existing_backups(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = fs::read_dir(folder).map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.file_name().and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix(PREFIX).and_then(|rest| rest.strip_suffix(".db")))
            .is_some_and(|stamp| stamp.len() == 21 && stamp.bytes().enumerate().all(|(index, byte)| {
                if [4, 7, 10, 17].contains(&index) { byte == b'-' } else { byte.is_ascii_digit() }
            })))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

pub fn run_automatic(conn: &mut Connection) -> Result<Option<PathBuf>, String> {
    run_automatic_key(conn, None)
}

pub fn run_automatic_key(conn: &mut Connection, key: Option<&str>) -> Result<Option<PathBuf>, String> {
    let config = settings(conn)?;
    if !config.enabled { return Ok(None); }
    validate_settings(&config)?;
    let folder = Path::new(&config.folder);
    let today = Local::now().format("%Y-%m-%d").to_string();
    if existing_backups(folder)?.iter().filter(|path| path.file_name().and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with(&format!("{PREFIX}{today}-"))))
        .any(|path| verify(path, key).is_ok()) {
        conn.execute("UPDATE configuracion_local SET ultimo_error = NULL WHERE id = 1", [])
            .map_err(|e| e.to_string())?;
        return Ok(None);
    }
    let filename = format!("{PREFIX}{}.db", Local::now().format("%Y-%m-%d-%H%M%S-%3f"));
    let destination = folder.join(filename);
    create_verified_key(conn, &destination, key)?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let details = format!("archivo={}", destination.file_name().and_then(|name| name.to_str()).unwrap_or("respaldo"));
    auditoria::registrar_evento_interno(&tx, "sistema", "RESPALDO_AUTOMATICO", Some(&details))
        .map_err(|e| e.to_string())?;
    tx.execute("UPDATE configuracion_local SET ultimo_respaldo = ?1, ultimo_error = NULL WHERE id = 1", params![Local::now().to_rfc3339()])
        .map_err(|e| e.to_string())?;
    if config.audit_days > 0 {
        tx.execute(
            "DELETE FROM auditoria WHERE substr(fecha_hora, 1, 10) < date('now', 'localtime', ?1)",
            params![format!("-{} days", config.audit_days)],
        ).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;

    let files = existing_backups(folder)?;
    for obsolete in files.iter().take(files.len().saturating_sub(config.keep as usize)) {
        fs::remove_file(obsolete).map_err(|e| e.to_string())?;
    }
    Ok(Some(destination))
}

pub fn run_automatic_recorded(conn: &mut Connection) -> Result<Option<PathBuf>, String> {
    run_automatic_recorded_key(conn, None)
}

pub fn run_automatic_recorded_key(conn: &mut Connection, key: Option<&str>) -> Result<Option<PathBuf>, String> {
    let result = run_automatic_key(conn, key);
    if let Err(error) = &result {
        let _ = conn.execute("UPDATE configuracion_local SET ultimo_error = ?1 WHERE id = 1", params![error]);
    }
    result
}
