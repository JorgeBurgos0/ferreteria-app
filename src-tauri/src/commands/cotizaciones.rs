use crate::{auth::SessionState, commands::auditoria, db::DbConnection};
use rusqlite::{params, Connection, OptionalExtension};
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

#[derive(serde::Serialize)]
pub struct LineaCotizacion {
    id_producto: i32,
    sku: String,
    nombre: String,
    cantidad: i32,
    precio_unitario: f64,
}

#[derive(serde::Serialize)]
pub struct CotizacionDetallada {
    id: i32,
    fecha_hora: String,
    vigencia: Option<String>,
    nombre_cliente: Option<String>,
    total_cotizacion: f64,
    items: Vec<LineaCotizacion>,
}

pub fn quote_details(conn: &Connection, id: i32) -> Result<Option<CotizacionDetallada>, String> {
    let quote = conn.query_row(
        "SELECT c.id, c.fecha_hora, c.vigencia, cl.nombre, c.total_cotizacion
         FROM cotizaciones c LEFT JOIN clientes cl ON cl.id = c.id_cliente WHERE c.id = ?1",
        params![id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).optional().map_err(|e| e.to_string())?;
    let Some((id, fecha_hora, vigencia, nombre_cliente, total_cotizacion)) = quote else { return Ok(None); };
    let mut statement = conn.prepare(
        "SELECT d.id_producto, COALESCE(p.sku, ''), COALESCE(p.nombre, 'Producto no disponible'), d.cantidad, d.precio_unitario
         FROM detalle_cotizaciones d LEFT JOIN productos p ON p.id = d.id_producto
         WHERE d.id_cotizacion = ?1 ORDER BY d.id",
    ).map_err(|e| e.to_string())?;
    let items = statement.query_map(params![id], |row| Ok(LineaCotizacion {
        id_producto: row.get(0)?, sku: row.get(1)?, nombre: row.get(2)?, cantidad: row.get(3)?, precio_unitario: row.get(4)?,
    })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(Some(CotizacionDetallada { id, fecha_hora, vigencia, nombre_cliente, total_cotizacion, items }))
}

#[tauri::command]
pub fn crear_cotizacion(
    db_state: State<DbConnection>,
    id_cliente: Option<i32>,
    items: Vec<ItemCotizacion>,
    total: f64,
    session: State<SessionState>,
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

    let actor = session.current_user()?.username;
    auditoria::registrar_evento_interno(&tx, &actor, "CREAR_COTIZACION", Some(&format!("cotizacion_id={id_cotizacion}")))
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(id_cotizacion)
}

#[tauri::command]
pub fn obtener_detalle_cotizacion(id: i32, db_state: State<DbConnection>) -> Result<CotizacionDetallada, String> {
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    quote_details(&conn, id)?.ok_or("Cotización no encontrada".into())
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
pub fn eliminar_cotizacion(id: i32, db_state: State<DbConnection>, session: State<SessionState>) -> Result<(), String> {
    let mut conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "DELETE FROM detalle_cotizaciones WHERE id_cotizacion = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;

    let deleted = tx.execute("DELETE FROM cotizaciones WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    if deleted != 1 { return Err("Cotización no encontrada".into()); }

    let actor = session.current_user()?.username;
    auditoria::registrar_evento_interno(&tx, &actor, "ELIMINAR_COTIZACION", Some(&format!("cotizacion_id={id}")))
        .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
