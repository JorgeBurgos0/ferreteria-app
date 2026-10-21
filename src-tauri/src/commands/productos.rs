// src/commands/productos.rs

use crate::commands::auditoria; // Importar auditoria
use crate::db::DbConnection;
use crate::auth::SessionState;
use rusqlite::{params, OptionalExtension};
use tauri::State;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Producto {
    id: i32,
    sku: String,
    nombre: String,
    descripcion: Option<String>,
    precio_venta: f64,
    precio_mayoreo: Option<f64>,
    stock_actual: i32,
    stock_minimo: Option<i32>,
    stock_maximo: Option<i32>,
    categoria: Option<String>,
    ubicacion: Option<String>,
    es_kit: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ComponenteKit {
    id_componente: i32,
    cantidad: f64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProductoImportacion {
    sku: String,
    nombre: String,
    descripcion: Option<String>,
    precio_venta: f64,
    precio_mayoreo: Option<f64>,
    stock_actual: i32,
    stock_minimo: Option<i32>,
    stock_maximo: Option<i32>,
    categoria: Option<String>,
    ubicacion: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ResultadoImportacion {
    exitosos: i32,
    errores: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct ProductoPrevisualizacion {
    sku: String,
    nombre_actual: Option<String>,
    precio_actual: Option<f64>,
    stock_actual: Option<i32>,
    existe: bool,
}

#[derive(serde::Serialize)]
pub struct ProductosPaginados {
    total: i64,
    productos: Vec<Producto>,
}

fn validar_producto(
    sku: &str,
    nombre: &str,
    precio_venta: f64,
    precio_mayoreo: Option<f64>,
    stock_actual: i32,
    stock_minimo: Option<i32>,
    stock_maximo: Option<i32>,
) -> Result<(), String> {
    if sku.trim().is_empty() || nombre.trim().is_empty() {
        return Err("SKU y nombre son obligatorios".into());
    }
    if !precio_venta.is_finite() || precio_venta < 0.0
        || precio_mayoreo.is_some_and(|price| !price.is_finite() || price < 0.0) {
        return Err("Precio inválido".into());
    }
    if stock_actual < 0 || stock_minimo.is_some_and(|stock| stock < 0)
        || stock_maximo.is_some_and(|stock| stock < 0)
        || matches!((stock_minimo, stock_maximo), (Some(min), Some(max)) if max < min) {
        return Err("Límites de stock inválidos".into());
    }
    Ok(())
}

#[tauri::command]
pub fn obtener_productos(db_state: State<DbConnection>) -> Result<Vec<Producto>, String> {
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, sku, nombre, descripcion, precio_venta, precio_mayoreo, stock_actual, stock_minimo, stock_maximo, categoria, ubicacion, es_kit FROM productos WHERE activo = 1")
        .map_err(|e| e.to_string())?;

    let productos_iter = stmt
        .query_map([], |row| {
            let id: i32 = row.get(0)?;
            let es_kit_int: i32 = row.get(11)?;
            let es_kit = es_kit_int != 0;
            let mut stock_actual: i32 = row.get(6)?;

            if es_kit {
                stock_actual = -1;
            }

            Ok(Producto {
                id,
                sku: row.get(1)?,
                nombre: row.get(2)?,
                descripcion: row.get(3)?,
                precio_venta: row.get(4)?,
                precio_mayoreo: row.get(5)?,
                stock_actual,
                stock_minimo: row.get(7)?,
                stock_maximo: row.get(8)?,
                categoria: row.get(9)?,
                ubicacion: row.get(10)?,
                es_kit,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut productos = Vec::new();
    for p in productos_iter {
        productos.push(p.map_err(|e| e.to_string())?);
    }

    // Calcular stock real para kits
    // Primero creamos un mapa de stocks para evitar problemas de borrowing
    let stock_map: std::collections::HashMap<i32, i32> =
        productos.iter().map(|p| (p.id, p.stock_actual)).collect();

    for p in &mut productos {
        if p.es_kit && p.stock_actual == -1 {
            let mut stmt_comp = conn
                .prepare("SELECT id_componente, cantidad FROM producto_compuesto WHERE id_kit = ?1")
                .map_err(|e| e.to_string())?;
            let componentes = stmt_comp
                .query_map(params![p.id], |row| {
                    Ok((row.get::<_, i32>(0)?, row.get::<_, f64>(1)?))
                })
                .map_err(|e| e.to_string())?;

            let mut max_kits = i32::MAX;
            let mut tiene_componentes = false;

            for comp in componentes {
                let (id_comp, cant_requerida) = comp.map_err(|e| e.to_string())?;
                tiene_componentes = true;

                if let Some(&stock_comp) = stock_map.get(&id_comp) {
                    let kits_posibles = (stock_comp as f64 / cant_requerida).floor() as i32;
                    if kits_posibles < max_kits {
                        max_kits = kits_posibles;
                    }
                } else {
                    max_kits = 0;
                }
            }

            p.stock_actual = if tiene_componentes { max_kits } else { 0 };
        }
    }

    Ok(productos)
}

#[tauri::command]
pub fn obtener_productos_paginados(
    db_state: State<DbConnection>,
    page: i64,
    limit: i64,
    search: Option<String>,
) -> Result<ProductosPaginados, String> {
    if page < 1 || !(1..=500).contains(&limit) {
        return Err("Paginación inválida".into());
    }
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let search_term = search.unwrap_or_default();
    let search_pattern = format!("%{}%", search_term);
    let offset = (page - 1).checked_mul(limit).ok_or("Paginación inválida")?;

    // 1. Obtener Total
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM productos WHERE activo = 1 AND (nombre LIKE ?1 OR sku LIKE ?1)",
        params![search_pattern],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    // 2. Obtener Productos Paginados
    let mut stmt = conn
        .prepare("SELECT id, sku, nombre, descripcion, precio_venta, precio_mayoreo, stock_actual, stock_minimo, stock_maximo, categoria, ubicacion, es_kit FROM productos WHERE activo = 1 AND (nombre LIKE ?1 OR sku LIKE ?1) LIMIT ?2 OFFSET ?3")
        .map_err(|e| e.to_string())?;

    let productos_iter = stmt
        .query_map(params![search_pattern, limit, offset], |row| {
             let id: i32 = row.get(0)?;
            let es_kit_int: i32 = row.get(11)?;
            let es_kit = es_kit_int != 0;
            let mut stock_actual: i32 = row.get(6)?;

            if es_kit {
                stock_actual = -1; // Marcador para calcular después
            }

            Ok(Producto {
                id,
                sku: row.get(1)?,
                nombre: row.get(2)?,
                descripcion: row.get(3)?,
                precio_venta: row.get(4)?,
                precio_mayoreo: row.get(5)?,
                stock_actual,
                stock_minimo: row.get(7)?,
                stock_maximo: row.get(8)?,
                categoria: row.get(9)?,
                ubicacion: row.get(10)?,
                es_kit,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut productos = Vec::new();
    for p in productos_iter {
        productos.push(p.map_err(|e| e.to_string())?);
    }

    // Calcular stock de kits (solo para la página actual)
    let stock_map: std::collections::HashMap<i32, i32> =
        productos.iter().map(|p| (p.id, p.stock_actual)).collect();

    for p in &mut productos {
        if p.es_kit && p.stock_actual == -1 {
            let mut stmt_comp = conn
                .prepare("SELECT id_componente, cantidad FROM producto_compuesto WHERE id_kit = ?1")
                .map_err(|e| e.to_string())?;
            let componentes = stmt_comp
                .query_map(params![p.id], |row| {
                    Ok((row.get::<_, i32>(0)?, row.get::<_, f64>(1)?))
                })
                .map_err(|e| e.to_string())?;

            let mut max_kits = i32::MAX;
            let mut tiene_componentes = false;

            for comp in componentes {
                let (id_comp, cant_requerida) = comp.map_err(|e| e.to_string())?;
                tiene_componentes = true;

                // Necesitamos el stock real del componente. 
                // Si no está en 'productos' (página actual), hay que consultarlo.
                let stock_comp = if let Some(&s) = stock_map.get(&id_comp) {
                    s
                } else {
                     conn.query_row("SELECT stock_actual FROM productos WHERE id = ?", params![id_comp], |r| r.get(0)).unwrap_or(0)
                };

                let kits_posibles = (stock_comp as f64 / cant_requerida).floor() as i32;
                if kits_posibles < max_kits {
                    max_kits = kits_posibles;
                }
            }
            p.stock_actual = if tiene_componentes { max_kits } else { 0 };
        }
    }

    Ok(ProductosPaginados { total, productos })
}

#[tauri::command]
pub fn obtener_producto_por_sku(
    db_state: State<DbConnection>,
    sku: String,
) -> Result<Option<Producto>, String> {
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, sku, nombre, descripcion, precio_venta, precio_mayoreo, stock_actual, stock_minimo, stock_maximo, categoria, ubicacion, es_kit FROM productos WHERE sku = ?1 AND activo = 1")
        .map_err(|e| e.to_string())?;

    let producto_result = stmt
        .query_row(params![sku], |row| {
            let id: i32 = row.get(0)?;
            let es_kit_int: i32 = row.get(11)?;
            let es_kit = es_kit_int != 0;

            Ok(Producto {
                id,
                sku: row.get(1)?,
                nombre: row.get(2)?,
                descripcion: row.get(3)?,
                precio_venta: row.get(4)?,
                precio_mayoreo: row.get(5)?,
                stock_actual: row.get(6)?,
                stock_minimo: row.get(7)?,
                stock_maximo: row.get(8)?,
                categoria: row.get(9)?,
                ubicacion: row.get(10)?,
                es_kit,
            })
        })
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some(mut prod) = producto_result {
        if prod.es_kit {
            let mut stmt_comp = conn
                .prepare("SELECT id_componente, cantidad FROM producto_compuesto WHERE id_kit = ?1")
                .map_err(|e| e.to_string())?;
            let componentes_iter = stmt_comp
                .query_map(params![prod.id], |row| {
                    Ok((row.get::<_, i32>(0)?, row.get::<_, f64>(1)?))
                })
                .map_err(|e| e.to_string())?;

            let mut max_kits = i32::MAX;
            let mut tiene_componentes = false;

            for comp in componentes_iter {
                let (id_comp, cant_requerida) = comp.map_err(|e| e.to_string())?;
                tiene_componentes = true;

                let stock_comp: i32 = conn
                    .query_row(
                        "SELECT stock_actual FROM productos WHERE id = ?",
                        params![id_comp],
                        |r| r.get(0),
                    )
                    .unwrap_or(0);

                let kits_posibles = (stock_comp as f64 / cant_requerida).floor() as i32;
                if kits_posibles < max_kits {
                    max_kits = kits_posibles;
                }
            }
            prod.stock_actual = if tiene_componentes { max_kits } else { 0 };
        }
        Ok(Some(prod))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn agregar_producto(
    db_state: State<DbConnection>,
    sku: String,
    nombre: String,
    descripcion: Option<String>,
    precio_venta: f64,
    precio_mayoreo: Option<f64>,
    stock_actual: i32,
    stock_minimo: Option<i32>,
    stock_maximo: Option<i32>,
    categoria: Option<String>,
    ubicacion: Option<String>,
    es_kit: bool,
    session: State<SessionState>,
) -> Result<i64, String> {
    let usuario = session.current_user()?.username;
    validar_producto(&sku, &nombre, precio_venta, precio_mayoreo, stock_actual, stock_minimo, stock_maximo)?;
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let cat = categoria.unwrap_or("General".to_string());
    conn.execute(
        "INSERT INTO productos (sku, nombre, descripcion, precio_venta, precio_mayoreo, stock_actual, stock_minimo, stock_maximo, categoria, ubicacion, es_kit, activo) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1)",
        params![sku, nombre, descripcion, precio_venta, precio_mayoreo, stock_actual, stock_minimo, stock_maximo, cat, ubicacion, es_kit as i32],
    )
    .map_err(|e| e.to_string())?;

    let last_id = conn.last_insert_rowid();
    // Log de auditoría
    let detalles = format!("Creó producto: {} (SKU: {})", nombre, sku);
    auditoria::registrar_evento_interno(&conn, &usuario, "CREAR_PRODUCTO", Some(&detalles))
        .map_err(|e| e.to_string())?;

    Ok(last_id)
}

#[tauri::command]
pub fn actualizar_producto(
    db_state: State<DbConnection>,
    id: i32,
    sku: String,
    nombre: String,
    descripcion: Option<String>,
    precio_venta: f64,
    precio_mayoreo: Option<f64>,
    stock_actual: i32,
    stock_minimo: Option<i32>,
    stock_maximo: Option<i32>,
    categoria: Option<String>,
    ubicacion: Option<String>,
    es_kit: bool,
    session: State<SessionState>,
) -> Result<(), String> {
    let usuario = session.current_user()?.username;
    validar_producto(&sku, &nombre, precio_venta, precio_mayoreo, stock_actual, stock_minimo, stock_maximo)?;
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let cat = categoria.unwrap_or("General".to_string());
    conn.execute(
        "UPDATE productos SET sku = ?1, nombre = ?2, descripcion = ?3, precio_venta = ?4, precio_mayoreo = ?5, stock_actual = ?6, stock_minimo = ?7, stock_maximo = ?8, categoria = ?9, ubicacion = ?10, es_kit = ?11 WHERE id = ?12",
        params![sku, nombre, descripcion, precio_venta, precio_mayoreo, stock_actual, stock_minimo, stock_maximo, cat, ubicacion, es_kit as i32, id],
    )
    .map_err(|e| e.to_string())?;

    // Log de auditoría
    let detalles = format!("Actualizó producto ID: {}. Stock: {}", id, stock_actual);
    auditoria::registrar_evento_interno(&conn, &usuario, "ACTUALIZAR_PRODUCTO", Some(&detalles))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn eliminar_producto(
    id: i32,
    session: State<SessionState>,
    db_state: State<DbConnection>,
) -> Result<(), String> {
    let usuario = session.current_user()?.username;
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    // Soft Delete: Actualizamos activo = 0 en lugar de borrar
    conn.execute("UPDATE productos SET activo = 0 WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    // Log de auditoría
    let detalles = format!("Eliminó producto ID: {}", id);
    auditoria::registrar_evento_interno(&conn, &usuario, "ELIMINAR_PRODUCTO", Some(&detalles))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn agregar_stock(
    db_state: State<DbConnection>,
    id: i32,
    cantidad: i32,
    session: State<SessionState>,
) -> Result<(), String> {
    let usuario = session.current_user()?.username;
    if cantidad <= 0 {
        return Err("La cantidad debe ser positiva".into());
    }
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let changed = conn.execute(
        "UPDATE productos SET stock_actual = stock_actual + ?1 WHERE id = ?2 AND activo = 1 AND es_kit = 0",
        params![cantidad, id],
    )
    .map_err(|e| e.to_string())?;
    if changed != 1 {
        return Err("Producto no encontrado o es un kit".into());
    }

    // Log de auditoría
    let detalles = format!("Agregó {} unidades al producto ID: {}", cantidad, id);
    auditoria::registrar_evento_interno(&conn, &usuario, "AGREGAR_STOCK", Some(&detalles))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn guardar_componentes_kit(
    db_state: State<DbConnection>,
    id_kit: i32,
    componentes: Vec<ComponenteKit>,
) -> Result<(), String> {
    if componentes.is_empty() {
        return Err("El kit debe tener componentes".into());
    }
    let mut conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let kit: i32 = tx.query_row(
        "SELECT es_kit FROM productos WHERE id = ?1 AND activo = 1",
        params![id_kit], |row| row.get(0),
    ).map_err(|_| "Kit no encontrado".to_string())?;
    if kit == 0 {
        return Err("El producto no es un kit".into());
    }
    for component in &componentes {
        if component.id_componente == id_kit
            || !component.cantidad.is_finite()
            || component.cantidad <= 0.0
            || component.cantidad.fract() != 0.0
        {
            return Err("Componente o cantidad inválida".into());
        }
        let is_kit: i32 = tx.query_row(
            "SELECT es_kit FROM productos WHERE id = ?1 AND activo = 1",
            params![component.id_componente], |row| row.get(0),
        ).map_err(|_| "Componente no encontrado".to_string())?;
        if is_kit != 0 {
            return Err("Los kits no pueden contener otros kits".into());
        }
    }

    tx.execute(
        "DELETE FROM producto_compuesto WHERE id_kit = ?1",
        params![id_kit],
    )
    .map_err(|e| e.to_string())?;

    for comp in componentes {
        tx.execute(
            "INSERT INTO producto_compuesto (id_kit, id_componente, cantidad) VALUES (?1, ?2, ?3)",
            params![id_kit, comp.id_componente, comp.cantidad],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn obtener_componentes_kit(
    db_state: State<DbConnection>,
    id_kit: i32,
) -> Result<Vec<ComponenteKit>, String> {
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id_componente, cantidad FROM producto_compuesto WHERE id_kit = ?1")
        .map_err(|e| e.to_string())?;

    let componentes = stmt
        .query_map(params![id_kit], |row| {
            Ok(ComponenteKit {
                id_componente: row.get(0)?,
                cantidad: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(componentes)
}

#[tauri::command]
pub fn importar_productos_masivos(
    db_state: State<DbConnection>,
    productos: Vec<ProductoImportacion>,
    session: State<SessionState>,
) -> Result<ResultadoImportacion, String> {
    let usuario = session.current_user()?.username;
    let mut conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut exitosos = 0;
    let mut errores = Vec::new(); // Lista de SKUs o razones de error

    for p in productos {
        if let Err(error) = validar_producto(
            &p.sku, &p.nombre, p.precio_venta, p.precio_mayoreo,
            p.stock_actual, p.stock_minimo, p.stock_maximo,
        ) {
            errores.push(format!("SKU {}: {}", p.sku, error));
            continue;
        }
        let cat = p.categoria.unwrap_or("General".to_string());
        
        // Verificar si existe
        let existe: i32 = tx.query_row(
            "SELECT COUNT(*) FROM productos WHERE sku = ?1",
            params![p.sku],
            |row| row.get(0),
        ).unwrap_or(0);

        if existe > 0 {
            // Actualizar existente (solo precios y stock para no sobreescribir nombres si no se desea, pero asumiremos update completo)
            let res = tx.execute(
                "UPDATE productos SET nombre=?2, descripcion=?3, precio_venta=?4, precio_mayoreo=?5, stock_actual = stock_actual + ?6, stock_minimo=?7, stock_maximo=?8, categoria=?9, ubicacion=?10, activo=1 WHERE sku=?1",
                params![p.sku, p.nombre, p.descripcion, p.precio_venta, p.precio_mayoreo, p.stock_actual, p.stock_minimo, p.stock_maximo, cat, p.ubicacion],
            );
            match res {
                Ok(_) => exitosos += 1,
                Err(e) => errores.push(format!("Error actualizando SKU {}: {}", p.sku, e)),
            }
        } else {
            // Insertar nuevo
            let res = tx.execute(
                "INSERT INTO productos (sku, nombre, descripcion, precio_venta, precio_mayoreo, stock_actual, stock_minimo, stock_maximo, categoria, ubicacion, es_kit, activo) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 1)",
                params![p.sku, p.nombre, p.descripcion, p.precio_venta, p.precio_mayoreo, p.stock_actual, p.stock_minimo, p.stock_maximo, cat, p.ubicacion],
            );
            match res {
                Ok(_) => exitosos += 1,
                Err(e) => errores.push(format!("Error insertando SKU {}: {}", p.sku, e)),
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;

    // Log de auditoría
    let detalles = format!("Importación masiva: {} procesados.", exitosos);
    auditoria::registrar_evento_interno(&conn, &usuario, "IMPORTACION_MASIVA", Some(&detalles))
        .map_err(|e| e.to_string())?;

    Ok(ResultadoImportacion { exitosos, errores })
}


#[tauri::command]
pub fn verificar_skus(
    db_state: State<DbConnection>,
    skus: Vec<String>,
) -> Result<Vec<ProductoPrevisualizacion>, String> {
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let mut resultados = Vec::new();

    // Optimización posible: usar IN clause si son pocos, o loop si son muchos.
    // Para simplificar y mantener seguridad, usaremos loop preparado.
    let mut stmt = conn.prepare("SELECT nombre, precio_venta, stock_actual FROM productos WHERE sku = ?1").map_err(|e| e.to_string())?;

    for sku in skus {
        let existe_info = stmt.query_row(params![sku], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, i32>(2)?))
        }).optional().map_err(|e| e.to_string())?;

        match existe_info {
            Some((nombre, precio, stock)) => {
                resultados.push(ProductoPrevisualizacion {
                    sku: sku,
                    nombre_actual: Some(nombre),
                    precio_actual: Some(precio),
                    stock_actual: Some(stock),
                    existe: true,
                });
            },
            None => {
                resultados.push(ProductoPrevisualizacion {
                    sku: sku,
                    nombre_actual: None,
                    precio_actual: None,
                    stock_actual: None,
                    existe: false,
                });
            }
        }
    }

    Ok(resultados)
}
