use crate::db::DbConnection;
use rusqlite::params;
use tauri::State;

type DbConn<'a> = State<'a, DbConnection>;

#[derive(serde::Serialize)]
pub struct LogAuditoria {
    pub id: i32,
    pub fecha_hora: String,
    pub usuario: String,
    pub accion: String,
    pub detalles: Option<String>,
}

// Función interna para registrar eventos desde otros comandos
pub fn registrar_evento_interno(
    conn: &rusqlite::Connection,
    usuario: &str,
    accion: &str,
    detalles: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let fecha_hora = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO auditoria (fecha_hora, usuario, accion, detalles) VALUES (?1, ?2, ?3, ?4)",
        params![fecha_hora, usuario, accion, detalles],
    )?;
    Ok(())
}

#[tauri::command]
pub fn obtener_logs(
    fecha_inicio: String,
    fecha_fin: String,
    db: DbConn,
) -> Result<Vec<LogAuditoria>, String> {
    let start = chrono::NaiveDate::parse_from_str(&fecha_inicio, "%Y-%m-%d")
        .map_err(|_| "Fecha inicial inválida".to_string())?;
    let end = chrono::NaiveDate::parse_from_str(&fecha_fin, "%Y-%m-%d")
        .map_err(|_| "Fecha final inválida".to_string())?;
    if end < start { return Err("El rango de fechas es inválido".into()); }
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, fecha_hora, usuario, accion, detalles 
         FROM auditoria 
         WHERE substr(fecha_hora, 1, 10) BETWEEN ?1 AND ?2
         ORDER BY id DESC LIMIT 1000",
        )
        .map_err(|e| e.to_string())?;

    let logs_iter = stmt
        .query_map(params![fecha_inicio, fecha_fin], |row| {
            Ok(LogAuditoria {
                id: row.get(0)?,
                fecha_hora: row.get(1)?,
                usuario: row.get(2)?,
                accion: row.get(3)?,
                detalles: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    logs_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
