use crate::{auth::SessionState, commands::auditoria, db::DbConnection, payments::{legacy_payment, Breakdown}};
use chrono::{DateTime, Local, Utc};
use rusqlite::params;
use std::collections::HashSet;
use tauri::State;

type DbConn<'a> = State<'a, DbConnection>;

#[derive(serde::Serialize)]
pub struct ItemVentaDevolucion {
    id_producto: i32,
    nombre_producto: String,
    cantidad_vendida: i32,
    precio_unitario: f64,
    cantidad_devuelta_previamente: i32,
}

#[derive(serde::Serialize)]
pub struct VentaParaDevolucion {
    id: i32,
    fecha_hora: String,
    total_venta: f64,
    items: Vec<ItemVentaDevolucion>,
}

#[derive(serde::Deserialize)]
pub struct ItemDevolucionInput {
    id_producto: i32,
    cantidad: i32,
    precio_reembolso: f64, // Usually same as sold price, but allows flexibility
}

#[tauri::command]
pub fn buscar_venta_para_devolucion(
    id_venta: i32,
    db: DbConn,
) -> Result<VentaParaDevolucion, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // 1. Get basic sale info
    let (fecha_hora, total_venta): (String, f64) = conn
        .query_row(
            "SELECT fecha_hora, total_venta FROM ventas WHERE id = ?1",
            params![id_venta],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Venta no encontrada".to_string())?;

    // 2. Get items and check previous returns
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.nombre, SUM(dv.cantidad), MAX(dv.precio_unitario_en_venta),
         COALESCE((SELECT SUM(dd.cantidad) 
                   FROM detalle_devoluciones dd 
                   JOIN devoluciones d ON dd.id_devolucion = d.id 
                   WHERE d.id_venta_original = ?1 AND dd.id_producto = p.id), 0) as devueltos
         FROM detalle_ventas dv
         JOIN productos p ON dv.id_producto = p.id
         WHERE dv.id_venta = ?1
         GROUP BY p.id, p.nombre",
        )
        .map_err(|e| e.to_string())?;

    let items_iter = stmt
        .query_map(params![id_venta], |row| {
            Ok(ItemVentaDevolucion {
                id_producto: row.get(0)?,
                nombre_producto: row.get(1)?,
                cantidad_vendida: row.get(2)?,
                precio_unitario: row.get(3)?,
                cantidad_devuelta_previamente: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    for item in items_iter {
        items.push(item.map_err(|e| e.to_string())?);
    }

    Ok(VentaParaDevolucion {
        id: id_venta,
        fecha_hora,
        total_venta,
        items,
    })
}

#[tauri::command]
pub fn procesar_devolucion(
    id_venta: i32,
    items: Vec<ItemDevolucionInput>,
    motivo: String,
    db: DbConn,
    session: State<SessionState>,
) -> Result<(), String> {
    if items.is_empty() {
        return Err("Selecciona al menos un producto".into());
    }
    let username = session.current_user()?.username;
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let today = Local::now().format("%Y-%m-%d").to_string();
    let closed: i64 = tx.query_row(
        "SELECT COUNT(*) FROM cortes_caja WHERE REPLACE(fecha, ' ', '') = ?1",
        params![today], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if closed > 0 {
        return Err("La caja de hoy ya fue cerrada; reabre el corte antes de devolver".into());
    }
    let (fecha_venta, total_venta, metodo_pago, pago_efectivo, pago_tarjeta, id_cliente):
        (String, f64, String, Option<f64>, Option<f64>, Option<i32>) = tx.query_row(
        "SELECT fecha_hora, total_venta, metodo_pago, pago_efectivo, pago_tarjeta, id_cliente FROM ventas WHERE id = ?1",
        params![id_venta],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    ).map_err(|_| "Venta no encontrada".to_string())?;
    let parsed_date = DateTime::parse_from_rfc3339(&fecha_venta)
        .map_err(|_| "Fecha de venta inválida".to_string())?;
    if Utc::now().signed_duration_since(parsed_date.with_timezone(&Utc)).num_days() > 30 {
        return Err("El plazo de devolución de 30 días terminó".into());
    }
    let mut seen = HashSet::new();
    let mut validated_items = Vec::with_capacity(items.len());
    let mut total_reembolsado = 0.0;
    for item in items {
        if item.cantidad <= 0 || !seen.insert(item.id_producto) {
            return Err("Cantidad o producto repetido inválido".into());
        }
        let (sold_quantity, original_price): (i64, f64) = tx.query_row(
            "SELECT SUM(cantidad), MAX(precio_unitario_en_venta) FROM detalle_ventas WHERE id_venta = ?1 AND id_producto = ?2",
            params![id_venta, item.id_producto],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|_| "Producto no encontrado en la venta".to_string())?;
        let previous: i64 = tx.query_row(
            "SELECT COALESCE(SUM(dd.cantidad), 0) FROM detalle_devoluciones dd JOIN devoluciones d ON d.id = dd.id_devolucion WHERE d.id_venta_original = ?1 AND dd.id_producto = ?2",
            params![id_venta, item.id_producto],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        if i64::from(item.cantidad) > sold_quantity - previous {
            return Err("La cantidad excede las unidades disponibles para devolución".into());
        }
        if !item.precio_reembolso.is_finite() || (item.precio_reembolso - original_price).abs() > 0.001 {
            return Err("El reembolso debe usar el precio original de venta".into());
        }
        total_reembolsado += f64::from(item.cantidad) * original_price;
        validated_items.push((item.id_producto, item.cantidad, original_price, sold_quantity));
    }
    if !total_reembolsado.is_finite() {
        return Err("Importe de devolución inválido".into());
    }
    if total_venta <= 0.0 {
        return Err("La venta original no tiene un importe válido".into());
    }
    let payment = match (pago_efectivo, pago_tarjeta) {
        (Some(cash), Some(card)) => Breakdown { cash, card },
        _ => legacy_payment(&metodo_pago, total_venta)?,
    };
    let reembolso_efectivo = (total_reembolsado * payment.cash / total_venta * 100.0).round() / 100.0;
    let reembolso_tarjeta = total_reembolsado - reembolso_efectivo;
    let fecha = Local::now().to_rfc3339();

    // 1. Create Refund Record
    tx.execute(
        "INSERT INTO devoluciones (id_venta_original, fecha_hora, total_reembolsado, motivo, reembolso_efectivo, reembolso_tarjeta) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id_venta, fecha, total_reembolsado, motivo, reembolso_efectivo, reembolso_tarjeta],
    ).map_err(|e| e.to_string())?;

    let id_devolucion = tx.last_insert_rowid();

    // 2. Process each item
    for (product_id, quantity, price, sold_quantity) in validated_items {
        tx.execute(
            "INSERT INTO detalle_devoluciones (id_devolucion, id_producto, cantidad, monto_reembolsado) VALUES (?1, ?2, ?3, ?4)",
            params![id_devolucion, product_id, quantity, f64::from(quantity) * price],
        ).map_err(|e| e.to_string())?;
        let is_kit: i32 = tx.query_row(
            "SELECT es_kit FROM productos WHERE id = ?1", params![product_id], |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        if is_kit == 0 {
            tx.execute(
                "UPDATE productos SET stock_actual = stock_actual + ?1 WHERE id = ?2",
                params![quantity, product_id],
            ).map_err(|e| e.to_string())?;
        } else {
            let mut statement = tx.prepare(
                "SELECT id_componente, cantidad_descontada FROM detalle_venta_componentes WHERE id_venta = ?1 AND id_producto_kit = ?2",
            ).map_err(|e| e.to_string())?;
            let components = statement.query_map(params![id_venta, product_id], |row| {
                Ok((row.get::<_, i32>(0)?, row.get::<_, i64>(1)?))
            }).map_err(|e| e.to_string())?;
            let mut found = false;
            for component in components {
                let (component_id, count) = component.map_err(|e| e.to_string())?;
                if count <= 0 || count % sold_quantity != 0 {
                    return Err("Historial de componentes del kit inválido".into());
                }
                let units = (count / sold_quantity).checked_mul(i64::from(quantity))
                    .ok_or("Cantidad de componentes demasiado grande")?;
                tx.execute(
                    "UPDATE productos SET stock_actual = stock_actual + ?1 WHERE id = ?2",
                    params![units, component_id],
                ).map_err(|e| e.to_string())?;
                found = true;
            }
            if !found {
                return Err("Esta venta de kit es anterior al registro de componentes; ajusta el inventario manualmente".into());
            }
        }
    }

    if let Some(client_id) = id_cliente {
        let points = (total_reembolsado / 10.0).floor() as i32;
        if points > 0 {
            tx.execute(
                "UPDATE clientes SET puntos = MAX(0, COALESCE(puntos, 0) - ?1) WHERE id = ?2",
                params![points, client_id],
            ).map_err(|e| e.to_string())?;
        }
    }

    let details = format!("devolucion_id={id_devolucion}; venta_id={id_venta}; efectivo={reembolso_efectivo:.2}; tarjeta={reembolso_tarjeta:.2}");
    auditoria::registrar_evento_interno(&tx, &username, "DEVOLUCION", Some(&details))
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct DetalleDevolucionReporte {
    nombre_producto: String,
    cantidad: i32,
    monto_reembolsado: f64,
}

#[derive(serde::Serialize)]
pub struct DevolucionReporte {
    id: i32,
    id_venta_original: i32,
    fecha_hora: String,
    total_reembolsado: f64,
    motivo: String,
    detalles: Vec<DetalleDevolucionReporte>,
}

#[tauri::command]
pub fn generar_reporte_devoluciones(
    fecha_inicio: String,
    fecha_fin: String,
    db: DbConn,
) -> Result<Vec<DevolucionReporte>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, id_venta_original, fecha_hora, total_reembolsado, motivo 
             FROM devoluciones 
             WHERE date(fecha_hora) BETWEEN ?1 AND ?2
             ORDER BY fecha_hora DESC",
        )
        .map_err(|e| e.to_string())?;

    let devoluciones_iter = stmt
        .query_map(params![fecha_inicio, fecha_fin], |row| {
            Ok(DevolucionReporte {
                id: row.get(0)?,
                id_venta_original: row.get(1)?,
                fecha_hora: row.get(2)?,
                total_reembolsado: row.get(3)?,
                motivo: row.get(4)?,
                detalles: Vec::new(), // Se llenará después
            })
        })
        .map_err(|e| e.to_string())?;

    let mut devoluciones = Vec::new();
    for dev in devoluciones_iter {
        let mut d = dev.map_err(|e| e.to_string())?;

        // Obtener detalles para esta devolución
        let mut stmt_detalles = conn
            .prepare(
                "SELECT p.nombre, dd.cantidad, dd.monto_reembolsado
             FROM detalle_devoluciones dd
             JOIN productos p ON dd.id_producto = p.id
             WHERE dd.id_devolucion = ?1",
            )
            .map_err(|e| e.to_string())?;

        let detalles_iter = stmt_detalles
            .query_map(params![d.id], |row| {
                Ok(DetalleDevolucionReporte {
                    nombre_producto: row.get(0)?,
                    cantidad: row.get(1)?,
                    monto_reembolsado: row.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;

        for det in detalles_iter {
            d.detalles.push(det.map_err(|e| e.to_string())?);
        }

        devoluciones.push(d);
    }

    Ok(devoluciones)
}
