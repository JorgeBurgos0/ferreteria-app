use crate::db::DbConnection;
use rusqlite::params;
use tauri::State;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Cotizacion {
    id: i32,
    fecha_hora: String,
    id_cliente: Option<i32>,
    total_cotizacion: f64,
    vigencia: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct ItemCotizacion {
    id_producto: i32,
    cantidad: i32,
    precio_unitario: f64,
}

#[tauri::command]
pub fn crear_cotizacion(
    db_state: State<DbConnection>,
    id_cliente: Option<i32>,
    items: Vec<ItemCotizacion>,
    total: f64,
) -> Result<i64, String> {
    if items.is_empty() {
        return Err("La cotización no tiene productos".into());
    }
    let mut conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut calculated_total = 0.0;
    for item in &items {
        if item.cantidad <= 0 || !item.precio_unitario.is_finite() || item.precio_unitario < 0.0 {
            return Err("Producto o precio inválido en la cotización".into());
        }
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM productos WHERE id = ?1 AND activo = 1",
            params![item.id_producto], |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        if exists != 1 {
            return Err("Producto no disponible".into());
        }
        calculated_total += item.precio_unitario * f64::from(item.cantidad);
    }
    if !total.is_finite() || (total - calculated_total).abs() > 0.01 {
        return Err("El total de la cotización no coincide con sus productos".into());
    }

    // 1. Insertar Cotización
    tx.execute(
        "INSERT INTO cotizaciones (fecha_hora, id_cliente, total_cotizacion, vigencia) VALUES (datetime('now', 'localtime'), ?1, ?2, date('now', '+15 days'))",
        params![id_cliente, calculated_total],
    ).map_err(|e| e.to_string())?;

    let id_cotizacion = tx.last_insert_rowid();

    // 2. Insertar Detalles
    for item in items {
        tx.execute(
            "INSERT INTO detalle_cotizaciones (id_cotizacion, id_producto, cantidad, precio_unitario) VALUES (?1, ?2, ?3, ?4)",
            params![id_cotizacion, item.id_producto, item.cantidad, item.precio_unitario],
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(id_cotizacion)
}

#[tauri::command]
pub fn obtener_cotizaciones(db_state: State<DbConnection>) -> Result<Vec<Cotizacion>, String> {
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, fecha_hora, id_cliente, total_cotizacion, vigencia FROM cotizaciones ORDER BY id DESC")
        .map_err(|e| e.to_string())?;

    let cotizaciones = stmt
        .query_map([], |row| {
            Ok(Cotizacion {
                id: row.get(0)?,
                fecha_hora: row.get(1)?,
                id_cliente: row.get(2)?,
                total_cotizacion: row.get(3)?,
                vigencia: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(cotizaciones)
}

#[tauri::command]
pub fn eliminar_cotizacion(id: i32, db_state: State<DbConnection>) -> Result<(), String> {
    let mut conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "DELETE FROM detalle_cotizaciones WHERE id_cotizacion = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM cotizaciones WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
