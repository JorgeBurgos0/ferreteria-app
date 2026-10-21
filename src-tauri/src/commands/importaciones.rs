use crate::{auth::SessionState, commands::auditoria, db::DbConnection};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;

#[derive(Clone, Deserialize)]
pub struct ImportRow {
    pub fila: usize,
    pub nombre: String,
    #[serde(default)] pub rfc: String,
    #[serde(default)] pub telefono: String,
    #[serde(default)] pub email: String,
    #[serde(default)] pub nombre_contacto: String,
}

#[derive(Serialize)]
pub struct RowResult {
    pub fila: usize,
    pub nombre: String,
    pub estado: String,
    pub detalle: String,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub agregados: usize,
    pub duplicados: usize,
    pub errores: usize,
    pub filas: Vec<RowResult>,
}

fn validate_batch(tipo: &str, rows: &[ImportRow]) -> Result<(), String> {
    if tipo != "clientes" && tipo != "proveedores" { return Err("Tipo de importación inválido".into()); }
    if rows.is_empty() || rows.len() > 5000 { return Err("El archivo debe contener entre 1 y 5000 filas".into()); }
    Ok(())
}

fn normalized(value: &str) -> String { value.trim().to_lowercase() }

fn classify(conn: &Connection, tipo: &str, row: &ImportRow, seen: &mut HashSet<String>) -> Result<RowResult, String> {
    let nombre = row.nombre.trim();
    let mut result = RowResult { fila: row.fila, nombre: nombre.into(), estado: "nuevo".into(), detalle: String::new() };
    if row.fila < 2 || nombre.is_empty() || nombre.chars().count() > 150 {
        result.estado = "error".into(); result.detalle = "Nombre obligatorio (máximo 150 caracteres)".into(); return Ok(result);
    }
    if row.rfc.chars().count() > 20 || row.telefono.chars().count() > 40 || row.email.chars().count() > 254 || row.nombre_contacto.chars().count() > 150 {
        result.estado = "error".into(); result.detalle = "Una columna supera el tamaño permitido".into(); return Ok(result);
    }
    if !row.email.trim().is_empty() && (!row.email.contains('@') || row.email.chars().any(char::is_whitespace)) {
        result.estado = "error".into(); result.detalle = "Correo electrónico inválido".into(); return Ok(result);
    }
    let (keys, exists) = if tipo == "proveedores" {
        let key = format!("empresa:{}", normalized(nombre));
        let exists: i64 = conn.query_row("SELECT EXISTS(SELECT 1 FROM proveedores WHERE lower(trim(nombre_empresa)) = ?1)", params![normalized(nombre)], |r| r.get(0)).map_err(|e| e.to_string())?;
        (vec![key], exists != 0)
    } else {
        let rfc = normalized(&row.rfc);
        let email = normalized(&row.email);
        let phone = row.telefono.trim();
        let name = normalized(nombre);
        let mut keys = Vec::new();
        if !rfc.is_empty() { keys.push(format!("rfc:{rfc}")); }
        if !email.is_empty() { keys.push(format!("correo:{email}")); }
        if !phone.is_empty() { keys.push(format!("nombre:{name}:telefono:{phone}")); }
        if keys.is_empty() { keys.push(format!("nombre:{name}")); }
        let exists: i64 = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM clientes WHERE
             (?1 != '' AND lower(trim(rfc)) = ?1) OR
             (?2 != '' AND lower(trim(email)) = ?2) OR
             (?3 != '' AND lower(trim(nombre)) = ?4 AND trim(telefono) = ?3) OR
             (?1 = '' AND ?2 = '' AND ?3 = '' AND lower(trim(nombre)) = ?4))",
            params![rfc, email, phone, name], |r| r.get(0),
        ).map_err(|e| e.to_string())?;
        (keys, exists != 0)
    };
    if exists || keys.iter().any(|key| seen.contains(key)) {
        result.estado = "duplicado".into(); result.detalle = "Ya existe en la base o en el archivo".into();
    } else {
        seen.extend(keys);
    }
    Ok(result)
}

pub fn process(conn: &mut Connection, tipo: &str, rows: &[ImportRow], actor: Option<&str>) -> Result<ImportResult, String> {
    validate_batch(tipo, rows)?;
    let mut seen = HashSet::new();
    let mut report = ImportResult { agregados: 0, duplicados: 0, errores: 0, filas: Vec::with_capacity(rows.len()) };
    if let Some(actor) = actor {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for row in rows {
            let mut status = classify(&tx, tipo, row, &mut seen)?;
            if status.estado == "nuevo" {
                let inserted = if tipo == "clientes" {
                    tx.execute(
                        "INSERT INTO clientes (nombre, rfc, telefono, email, puntos, fecha_registro) VALUES (?1, NULLIF(?2, ''), ?3, ?4, 0, datetime('now','localtime'))",
                        params![row.nombre.trim(), row.rfc.trim(), row.telefono.trim(), row.email.trim()],
                    )
                } else {
                    tx.execute(
                        "INSERT INTO proveedores (nombre_empresa, nombre_contacto, telefono, email) VALUES (?1, ?2, ?3, ?4)",
                        params![row.nombre.trim(), row.nombre_contacto.trim(), row.telefono.trim(), row.email.trim()],
                    )
                };
                if let Err(error) = inserted {
                    status.estado = "error".into(); status.detalle = format!("No se pudo guardar: {error}");
                }
            }
            match status.estado.as_str() { "nuevo" => report.agregados += 1, "duplicado" => report.duplicados += 1, _ => report.errores += 1 }
            report.filas.push(status);
        }
        auditoria::registrar_evento_interno(&tx, actor, "IMPORTAR_DATOS", Some(&format!("tipo={tipo}; agregados={}; duplicados={}; errores={}", report.agregados, report.duplicados, report.errores)))
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    } else {
        for row in rows {
            let status = classify(conn, tipo, row, &mut seen)?;
            match status.estado.as_str() { "nuevo" => report.agregados += 1, "duplicado" => report.duplicados += 1, _ => report.errores += 1 }
            report.filas.push(status);
        }
    }
    Ok(report)
}

fn preview(tipo: &str, filas: Vec<ImportRow>, db: State<DbConnection>) -> Result<ImportResult, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    process(&mut conn, tipo, &filas, None)
}

fn commit(tipo: &str, filas: Vec<ImportRow>, db: State<DbConnection>, session: State<SessionState>) -> Result<ImportResult, String> {
    let actor = session.current_user()?.username;
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    process(&mut conn, tipo, &filas, Some(&actor))
}

#[tauri::command]
pub fn previsualizar_clientes(filas: Vec<ImportRow>, db: State<DbConnection>) -> Result<ImportResult, String> { preview("clientes", filas, db) }

#[tauri::command]
pub fn confirmar_clientes(filas: Vec<ImportRow>, db: State<DbConnection>, session: State<SessionState>) -> Result<ImportResult, String> { commit("clientes", filas, db, session) }

#[tauri::command]
pub fn previsualizar_proveedores(filas: Vec<ImportRow>, db: State<DbConnection>) -> Result<ImportResult, String> { preview("proveedores", filas, db) }

#[tauri::command]
pub fn confirmar_proveedores(filas: Vec<ImportRow>, db: State<DbConnection>, session: State<SessionState>) -> Result<ImportResult, String> { commit("proveedores", filas, db, session) }
