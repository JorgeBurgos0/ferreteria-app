use crate::{auth::SessionState, commands::auditoria, models::ItemVenta, payments::validate_payment};
use rusqlite::{params, Transaction};
use std::collections::HashMap;
use tauri::State;

type DbConn<'a> = State<'a, crate::db::DbConnection>;

fn realizar_venta_en_tx(
    tx: &Transaction,
    items: &[ItemVenta],
    id_cliente: Option<i32>,
    id_usuario: i32,
    metodo_pago: &str,
    monto_efectivo: f64,
    monto_tarjeta: f64,
    aplicar_mayoreo: bool,
) -> Result<i64, String> {
    if items.is_empty() {
        return Err("El carrito está vacío".into());
    }

    let mut required_stock: HashMap<i32, i64> = HashMap::new();
    let mut kit_components: HashMap<(i32, i32), i64> = HashMap::new();
    let mut sale_lines: Vec<(i32, i32, f64)> = Vec::with_capacity(items.len());
    let mut total_venta = 0.0;

    for item in items {
        if item.cantidad <= 0 {
            return Err("Las cantidades deben ser positivas".into());
        }
        let (precio_venta, precio_mayoreo, es_kit): (f64, Option<f64>, i32) = tx
            .query_row(
                "SELECT precio_venta, precio_mayoreo, es_kit FROM productos WHERE id = ?1 AND activo = 1",
                params![item.id_producto],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| format!("Producto {} no disponible", item.id_producto))?;
        let precio = if aplicar_mayoreo {
            precio_mayoreo.unwrap_or(precio_venta)
        } else {
            precio_venta
        };
        if !precio.is_finite() || precio < 0.0 {
            return Err("El producto tiene un precio inválido".into());
        }
        total_venta += precio * f64::from(item.cantidad);
        sale_lines.push((item.id_producto, item.cantidad, precio));

        if es_kit == 0 {
            let current = required_stock.entry(item.id_producto).or_default();
            *current = current.checked_add(i64::from(item.cantidad))
                .ok_or("La cantidad es demasiado grande")?;
        } else {
            let mut statement = tx
                .prepare("SELECT id_componente, cantidad FROM producto_compuesto WHERE id_kit = ?1")
                .map_err(|e| e.to_string())?;
            let components = statement
                .query_map(params![item.id_producto], |row| {
                    Ok((row.get::<_, i32>(0)?, row.get::<_, f64>(1)?))
                })
                .map_err(|e| e.to_string())?;
            let mut has_components = false;
            for component in components {
                let (component_id, quantity) = component.map_err(|e| e.to_string())?;
                if !quantity.is_finite() || quantity <= 0.0 || quantity.fract() != 0.0 || quantity > i64::MAX as f64 {
                    return Err("El kit contiene una cantidad de componentes inválida".into());
                }
                let required = (quantity as i64)
                    .checked_mul(i64::from(item.cantidad))
                    .ok_or("La cantidad del kit es demasiado grande")?;
                let current = required_stock.entry(component_id).or_default();
                *current = current.checked_add(required)
                    .ok_or("La cantidad del kit es demasiado grande")?;
                let kit_total = kit_components.entry((item.id_producto, component_id)).or_default();
                *kit_total = kit_total.checked_add(required)
                    .ok_or("La cantidad del kit es demasiado grande")?;
                has_components = true;
            }
            if !has_components {
                return Err("El kit no tiene componentes".into());
            }
        }
    }
    if !total_venta.is_finite() {
        return Err("El total de la venta es inválido".into());
    }
    let breakdown = validate_payment(metodo_pago, monto_efectivo, monto_tarjeta, total_venta)?;
    for (product_id, required) in &required_stock {
        let (stock, is_kit): (i64, i32) = tx
            .query_row(
                "SELECT stock_actual, es_kit FROM productos WHERE id = ?1 AND activo = 1",
                params![product_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| format!("Componente {} no disponible", product_id))?;
        if is_kit != 0 || stock < *required {
            return Err(format!("Stock insuficiente para el producto {}", product_id));
        }
    }

    let fecha_hora = chrono::Local::now().to_rfc3339();
    tx.execute(
        "INSERT INTO ventas (fecha_hora, id_cliente, total_venta, metodo_pago, id_usuario, pago_efectivo, pago_tarjeta) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![fecha_hora, id_cliente, total_venta, metodo_pago, id_usuario, breakdown.cash, breakdown.card],
    ).map_err(|e| e.to_string())?;
    let sale_id = tx.last_insert_rowid();
    for (product_id, quantity, price) in sale_lines {
        tx.execute(
            "INSERT INTO detalle_ventas (id_venta, id_producto, cantidad, precio_unitario_en_venta) VALUES (?1, ?2, ?3, ?4)",
            params![sale_id, product_id, quantity, price],
        ).map_err(|e| e.to_string())?;
    }
    for ((kit_id, component_id), quantity) in kit_components {
        tx.execute(
            "INSERT INTO detalle_venta_componentes (id_venta, id_producto_kit, id_componente, cantidad_descontada) VALUES (?1, ?2, ?3, ?4)",
            params![sale_id, kit_id, component_id, quantity],
        ).map_err(|e| e.to_string())?;
    }
    for (product_id, required) in required_stock {
        let changed = tx.execute(
            "UPDATE productos SET stock_actual = stock_actual - ?1 WHERE id = ?2 AND stock_actual >= ?1",
            params![required, product_id],
        ).map_err(|e| e.to_string())?;
        if changed != 1 {
            return Err(format!("Stock insuficiente para el producto {}", product_id));
        }
    }
    if let Some(client_id) = id_cliente {
        let points = (total_venta / 10.0).floor() as i32;
        if points > 0 {
            tx.execute(
                "UPDATE clientes SET puntos = COALESCE(puntos, 0) + ?1 WHERE id = ?2",
                params![points, client_id],
            ).map_err(|e| e.to_string())?;
        }
    }
    Ok(sale_id)
}

#[tauri::command]
pub fn realizar_venta(
    items: Vec<ItemVenta>,
    id_cliente: Option<i32>,
    metodo_pago: String,
    monto_efectivo: f64,
    monto_tarjeta: f64,
    aplicar_mayoreo: bool,
    db: DbConn,
    session: State<SessionState>,
) -> Result<i64, String> {
    let user = session.current_user()?;
    let user_id = user.user_id;
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let closed: i64 = tx.query_row(
        "SELECT COUNT(*) FROM cortes_caja WHERE REPLACE(fecha, ' ', '') = ?1",
        params![today], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if closed > 0 {
        return Err("La caja de hoy ya fue cerrada; reabre el corte antes de vender".into());
    }
    let sale_id = realizar_venta_en_tx(
        &tx,
        &items,
        id_cliente,
        user_id,
        &metodo_pago,
        monto_efectivo,
        monto_tarjeta,
        aplicar_mayoreo,
    )?;
    let details = format!("venta_id={sale_id}; efectivo={monto_efectivo:.2}; tarjeta={monto_tarjeta:.2}");
    auditoria::registrar_evento_interno(&tx, &user.username, "VENTA", Some(&details))
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(sale_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn repeated_items_cannot_overdraw_stock() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::inicializar(&conn).unwrap();
        conn.execute(
            "INSERT INTO productos (sku, nombre, precio_venta, stock_actual) VALUES ('P1', 'Pala', 10, 5)",
            [],
        ).unwrap();
        let tx = conn.transaction().unwrap();
        let items = [
            ItemVenta { id_producto: 1, cantidad: 3 },
            ItemVenta { id_producto: 1, cantidad: 3 },
        ];
        assert!(realizar_venta_en_tx(&tx, &items, None, 1, "Efectivo", 60.0, 0.0, false).is_err());
        drop(tx);
        let stock: i32 = conn.query_row("SELECT stock_actual FROM productos WHERE id = 1", [], |row| row.get(0)).unwrap();
        let sales: i64 = conn.query_row("SELECT COUNT(*) FROM ventas", [], |row| row.get(0)).unwrap();
        assert_eq!(stock, 5);
        assert_eq!(sales, 0);
    }

    #[test]
    fn kit_deducts_component_stock() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::inicializar(&conn).unwrap();
        conn.execute(
            "INSERT INTO productos (sku, nombre, precio_venta, stock_actual) VALUES ('P1', 'Tornillo', 10, 5)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO productos (sku, nombre, precio_venta, stock_actual, es_kit) VALUES ('K1', 'Kit', 50, 0, 1)",
            [],
        ).unwrap();
        conn.execute("INSERT INTO producto_compuesto (id_kit, id_componente, cantidad) VALUES (2, 1, 2)", []).unwrap();
        let tx = conn.transaction().unwrap();
        let items = [ItemVenta { id_producto: 2, cantidad: 1 }];
        realizar_venta_en_tx(&tx, &items, None, 1, "Efectivo", 50.0, 0.0, false).unwrap();
        tx.commit().unwrap();
        let stock: i32 = conn.query_row("SELECT stock_actual FROM productos WHERE id = 1", [], |row| row.get(0)).unwrap();
        assert_eq!(stock, 3);
        let consumed: i64 = conn.query_row(
            "SELECT cantidad_descontada FROM detalle_venta_componentes WHERE id_venta = 1 AND id_producto_kit = 2 AND id_componente = 1",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(consumed, 2);
    }
}
