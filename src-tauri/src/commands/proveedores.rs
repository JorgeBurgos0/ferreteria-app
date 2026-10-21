use tauri::State;
use rusqlite::params;
use crate::{db::DbConnection, models::Proveedor};

type DbConn<'a> = State<'a, DbConnection>;

#[tauri::command]
pub fn obtener_proveedores(db: DbConn) -> Result<Vec<Proveedor>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, nombre_empresa, nombre_contacto, telefono, email FROM proveedores")
        .map_err(|e| e.to_string())?;
    let proveedores_iter = stmt
        .query_map([], |row| {
            Ok(Proveedor {
                id: row.get(0)?,
                nombre_empresa: row.get(1)?,
                nombre_contacto: row.get(2)?,
                telefono: row.get(3)?,
                email: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    proveedores_iter.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn agregar_proveedor(nombre_empresa: String, nombre_contacto: String, telefono: String, email: String, db: DbConn) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO proveedores (nombre_empresa, nombre_contacto, telefono, email) VALUES (?1, ?2, ?3, ?4)",
        params![nombre_empresa, nombre_contacto, telefono, email],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn actualizar_proveedor(nombre_empresa:String, nombre_contacto: String, telefono: String, email: String, id: i32, db: DbConn) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE proveedores SET nombre_empresa = ?1, nombre_contacto = ?2, telefono = ?3, email = ?4 WHERE id = ?5",
        params![nombre_empresa, nombre_contacto, telefono, email, id],
        ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn eliminar_proveedor(id: i32, db: DbConn) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM proveedores WHERE id = ?1",
        params![id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}