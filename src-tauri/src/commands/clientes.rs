use crate::{db::DbConnection, models::Cliente};
use rusqlite::params;
use tauri::State;

type DbConn<'a> = State<'a, DbConnection>;

#[tauri::command]
pub fn obtener_clientes(db: DbConn) -> Result<Vec<Cliente>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, nombre, rfc, telefono, email, puntos FROM clientes")
        .map_err(|e| e.to_string())?;
    let clientes_iter = stmt
        .query_map([], |row| {
            Ok(Cliente {
                id: row.get(0)?,
                nombre: row.get(1)?,
                rfc: row.get(2)?,
                telefono: row.get(3)?,
                email: row.get(4)?,
                puntos: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    clientes_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn agregar_cliente(
    nombre: String,
    rfc: String,
    telefono: String,
    email: String,
    db: DbConn,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO clientes (nombre, rfc, telefono, email, puntos, fecha_registro) VALUES (?1, ?2, ?3, ?4, 0, datetime('now','localtime'))",
        params![nombre, rfc, telefono, email],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

//Agrega el command de actualizar
#[tauri::command]
pub fn actualizar_cliente(
    id: i32,
    nombre: String,
    rfc: String,
    telefono: String,
    email: String,
    db: DbConn,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE clientes SET nombre = ?1, rfc = ?2, telefono = ?3, email = ?4 WHERE id = ?5",
        params![nombre, rfc, telefono, email, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

//Agrega el command de eliminar
#[tauri::command]
pub fn eliminar_cliente(id: i32, db: DbConn) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM clientes WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct CompraCliente {
    pub id_venta: i32,
    pub fecha: String,
    pub total: f64,
    pub productos: Option<String>, // Concatenated product names, can be NULL
}

#[tauri::command]
pub fn obtener_historial_compras(
    id_cliente: i32,
    db: DbConn,
) -> Result<Vec<CompraCliente>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT v.id, v.fecha_hora, v.total_venta, GROUP_CONCAT(p.nombre, ', ') as productos
         FROM ventas v
         JOIN detalle_ventas dv ON v.id = dv.id_venta
         JOIN productos p ON dv.id_producto = p.id
         WHERE v.id_cliente = ?1
         GROUP BY v.id
         ORDER BY v.fecha_hora DESC",
        )
        .map_err(|e| e.to_string())?;

    let compras_iter = stmt
        .query_map(params![id_cliente], |row| {
            Ok(CompraCliente {
                id_venta: row.get(0)?,
                fecha: row.get(1)?,
                total: row.get(2)?,
                productos: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    compras_iter
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}
