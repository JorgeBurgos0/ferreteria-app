// src-tauri/src/commands/reportes.rs
use crate::db::DbConnection;
use chrono::Local;
use rusqlite::params;
use tauri::State;

type DbConn<'a> = State<'a, DbConnection>;

#[derive(serde::Serialize)]
pub struct ProductoReporteStock {
    id: i32,
    sku: String,
    nombre: String,
    stock_actual: i32,
    limite: i32, // Minimo o Maximo segun el reporte
}

#[derive(serde::Serialize)]
pub struct VentaReporte {
    id: i32,
    fecha_hora: String,
    total_venta: f64,
    metodo_pago: String,
    detalles: Vec<DetalleVentaReporte>,
}

#[derive(serde::Serialize)]
pub struct DetalleVentaReporte {
    nombre_producto: String,
    cantidad: i32,
    precio_unitario: f64,
    subtotal: f64,
}

#[derive(serde::Serialize)]
pub struct DashboardResumen {
    ventas_hoy: f64,
    items_bajo_stock: i32,
    cantidad_clientes: i32,
}

#[tauri::command]
pub fn obtener_productos_con_bajo_stock(
    db_state: State<DbConnection>,
) -> Result<Vec<ProductoReporteStock>, String> {
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, sku, nombre, stock_actual, COALESCE(stock_minimo, 5) as min_stock FROM productos WHERE stock_actual <= COALESCE(stock_minimo, 5) AND activo = 1")
        .map_err(|e| e.to_string())?;

    let productos_iter = stmt
        .query_map([], |row| {
            Ok(ProductoReporteStock {
                id: row.get(0)?,
                sku: row.get(1)?,
                nombre: row.get(2)?,
                stock_actual: row.get(3)?,
                limite: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut productos = Vec::new();
    for p in productos_iter {
        productos.push(p.map_err(|e| e.to_string())?);
    }
    Ok(productos)
}

#[tauri::command]
pub fn obtener_productos_con_sobre_stock(
    db_state: State<DbConnection>,
) -> Result<Vec<ProductoReporteStock>, String> {
    let conn = db_state.0.lock().map_err(|e| e.to_string())?;
    // Solo consideramos productos que tengan stock_maximo definido
    let mut stmt = conn
        .prepare("SELECT id, sku, nombre, stock_actual, stock_maximo FROM productos WHERE stock_maximo IS NOT NULL AND stock_actual > stock_maximo AND activo = 1")
        .map_err(|e| e.to_string())?;

    let productos_iter = stmt
        .query_map([], |row| {
            Ok(ProductoReporteStock {
                id: row.get(0)?,
                sku: row.get(1)?,
                nombre: row.get(2)?,
                stock_actual: row.get(3)?,
                limite: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut productos = Vec::new();
    for p in productos_iter {
        productos.push(p.map_err(|e| e.to_string())?);
    }
    Ok(productos)
}

#[tauri::command]
pub fn generar_reporte_ventas(
    fecha_inicio: String,
    fecha_fin: String,
    db: DbConn,
) -> Result<Vec<VentaReporte>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // Ajustamos fecha_fin para incluir todo el día (hasta 23:59:59)
    // Asumimos que el frontend envía YYYY-MM-DD
    let fecha_fin_ajustada = format!("{}T23:59:59", fecha_fin);
    let fecha_inicio_ajustada = format!("{}T00:00:00", fecha_inicio);

    let mut stmt = conn.prepare(
        "SELECT id, fecha_hora, total_venta, metodo_pago FROM ventas WHERE fecha_hora BETWEEN ?1 AND ?2 ORDER BY fecha_hora DESC"
    ).map_err(|e| e.to_string())?;

    let ventas_iter = stmt
        .query_map(params![fecha_inicio_ajustada, fecha_fin_ajustada], |row| {
            let id_venta: i32 = row.get(0)?;
            Ok(VentaReporte {
                id: id_venta,
                fecha_hora: row.get(1)?,
                total_venta: row.get(2)?,
                metodo_pago: row.get(3)?,
                detalles: Vec::new(), // Se llenará después
            })
        })
        .map_err(|e| e.to_string())?;

    let mut ventas = Vec::new();
    for v in ventas_iter {
        let mut venta = v.map_err(|e| e.to_string())?;

        // Obtener detalles para esta venta
        let mut stmt_detalles = conn
            .prepare(
                "SELECT p.nombre, dv.cantidad, dv.precio_unitario_en_venta 
             FROM detalle_ventas dv
             JOIN productos p ON dv.id_producto = p.id
             WHERE dv.id_venta = ?1",
            )
            .map_err(|e| e.to_string())?;

        let detalles_iter = stmt_detalles
            .query_map(params![venta.id], |row| {
                let cantidad: i32 = row.get(1)?;
                let precio: f64 = row.get(2)?;
                Ok(DetalleVentaReporte {
                    nombre_producto: row.get(0)?,
                    cantidad,
                    precio_unitario: precio,
                    subtotal: (cantidad as f64) * precio,
                })
            })
            .map_err(|e| e.to_string())?;

        for d in detalles_iter {
            venta.detalles.push(d.map_err(|e| e.to_string())?);
        }
        ventas.push(venta);
    }

    Ok(ventas)
}

#[tauri::command]
pub fn obtener_resumen_dashboard(db: DbConn) -> Result<DashboardResumen, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // 1. Ventas de hoy
    let hoy = Local::now().format("%Y-%m-%d").to_string();
    let inicio_dia = format!("{}T00:00:00", hoy);
    let fin_dia = format!("{}T23:59:59", hoy);

    let ventas_hoy: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total_venta), 0.0) FROM ventas WHERE fecha_hora BETWEEN ?1 AND ?2",
            params![inicio_dia, fin_dia],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // 2. Items bajo stock (<= 5)
    let items_bajo_stock: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM productos WHERE stock_actual <= 5",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // 3. Cantidad de clientes
    let cantidad_clientes: i32 = conn
        .query_row("SELECT COUNT(*) FROM clientes", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(DashboardResumen {
        ventas_hoy,
        items_bajo_stock,
        cantidad_clientes,
    })
}
