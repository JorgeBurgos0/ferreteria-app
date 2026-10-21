use rusqlite::{params, Result};
use serde::{Deserialize, Serialize};
use tauri::State;

type DbConn<'a> = State<'a, crate::db::DbConnection>;

#[derive(Serialize, Deserialize, Debug)]
pub struct AnalyticsWidget {
    pub id: i32,
    pub nombre: String,
    pub tipo_reporte: String,
    pub configuracion: String, // JSON string
    pub orden: i32,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct NuevoWidget {
    pub nombre: String,
    pub tipo_reporte: String,
    pub configuracion: String,
}

#[tauri::command]
pub fn guardar_widget_analytics(
    widget: NuevoWidget,
    db: DbConn,
) -> Result<i64, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // Get the next order number
    let max_orden: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(orden), 0) FROM analytics_widgets",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO analytics_widgets (nombre, tipo_reporte, configuracion, orden, created_at) 
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![
            widget.nombre,
            widget.tipo_reporte,
            widget.configuracion,
            max_orden + 1
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn obtener_widgets_analytics(db: DbConn) -> Result<Vec<AnalyticsWidget>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, nombre, tipo_reporte, configuracion, orden, created_at FROM analytics_widgets ORDER BY orden ASC")
        .map_err(|e| e.to_string())?;

    let widgets = stmt
        .query_map([], |row| {
            Ok(AnalyticsWidget {
                id: row.get(0)?,
                nombre: row.get(1)?,
                tipo_reporte: row.get(2)?,
                configuracion: row.get(3)?,
                orden: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(widgets)
}

#[tauri::command]
pub fn eliminar_widget_analytics(id: i32, db: DbConn) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM analytics_widgets WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}
