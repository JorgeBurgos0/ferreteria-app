use crate::db::DbConnection;
use rusqlite::params;
use tauri::State;

type DbConn<'a> = State<'a, DbConnection>;

#[derive(serde::Serialize)]
pub struct ProductoMasVendido {
    pub nombre_producto: String,
    pub cantidad_total: i32,
    pub total_ingresos: f64,
}

#[derive(serde::Serialize)]
pub struct VentaPorCategoria {
    pub categoria: String,
    pub total_ventas: f64,
    pub cantidad_items: i32,
}

#[derive(serde::Serialize)]
pub struct DatosHistorial {
    pub fecha: String,
    pub total_ventas: f64,
    pub cantidad_items: i32,
}

#[derive(serde::Serialize)]
pub struct ReporteEntidad {
    pub nombre: String,
    pub total_ventas: f64,
}

#[derive(serde::Serialize)]
pub struct ReporteNuevosClientes {
    pub fecha: String,
    pub cantidad: i32,
}

#[tauri::command]
pub fn obtener_productos_mas_vendidos(
    fecha_inicio: String,
    fecha_fin: String,
    db: DbConn,
) -> Result<Vec<ProductoMasVendido>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // Agregamos hora inicio y fin para cubrir todo el día
    let inicio = format!("{}T00:00:00", fecha_inicio);
    let fin = format!("{}T23:59:59", fecha_fin);

    let mut stmt = conn
        .prepare(
            "SELECT 
            p.nombre, 
            SUM(dv.cantidad) as cantidad_total, 
            SUM(dv.cantidad * dv.precio_unitario_en_venta) as total_ingresos
        FROM detalle_ventas dv
        JOIN ventas v ON dv.id_venta = v.id
        JOIN productos p ON dv.id_producto = p.id
        WHERE v.fecha_hora BETWEEN ?1 AND ?2
        GROUP BY p.id
        ORDER BY cantidad_total DESC
        LIMIT 10",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![inicio, fin], |row| {
            Ok(ProductoMasVendido {
                nombre_producto: row.get(0)?,
                cantidad_total: row.get(1)?,
                total_ingresos: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn obtener_ventas_por_categoria(
    fecha_inicio: String,
    fecha_fin: String,
    db: DbConn,
) -> Result<Vec<VentaPorCategoria>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let inicio = format!("{}T00:00:00", fecha_inicio);
    let fin = format!("{}T23:59:59", fecha_fin);

    let mut stmt = conn
        .prepare(
            "SELECT 
            COALESCE(p.categoria, 'Sin Categoría') as categoria, 
            SUM(dv.cantidad * dv.precio_unitario_en_venta) as total_ventas,
            SUM(dv.cantidad) as cantidad_items
        FROM detalle_ventas dv
        JOIN ventas v ON dv.id_venta = v.id
        JOIN productos p ON dv.id_producto = p.id
        WHERE v.fecha_hora BETWEEN ?1 AND ?2
        GROUP BY p.categoria
        ORDER BY total_ventas DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![inicio, fin], |row| {
            Ok(VentaPorCategoria {
                categoria: row.get(0)?,
                total_ventas: row.get(1)?,
                cantidad_items: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}


#[tauri::command]
pub fn obtener_historial_ventas(
    fecha_inicio: String,
    fecha_fin: String,
    agrupacion: String, // "dia" | "mes"
    db: DbConn,
) -> Result<Vec<DatosHistorial>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let inicio = format!("{}T00:00:00", fecha_inicio);
    let fin = format!("{}T23:59:59", fecha_fin);

    // Definir formato de fecha según agrupación
    let formato_fecha = match agrupacion.as_str() {
        "mes" => "%Y-%m",
        _ => "%Y-%m-%d", // "dia" por default
    };

    let sql = format!(
        "SELECT 
            strftime('{}', fecha_hora) as fecha_grupo,
            SUM(total_venta) as total,
            COUNT(*) as cantidad
         FROM ventas 
         WHERE fecha_hora BETWEEN ?1 AND ?2 
         GROUP BY fecha_grupo 
         ORDER BY fecha_grupo ASC",
        formato_fecha
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![inicio, fin], |row| {
            Ok(DatosHistorial {
                fecha: row.get(0)?,
                total_ventas: row.get(1)?,
                cantidad_items: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn obtener_top_entidades(
    entidad: String, // "cliente", "vendedor", "proveedor"
    fecha_inicio: String,
    fecha_fin: String,
    db: DbConn,
) -> Result<Vec<ReporteEntidad>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    
    let inicio = format!("{}T00:00:00", fecha_inicio);
    let fin = format!("{}T23:59:59", fecha_fin);

    let sql = match entidad.as_str() {
        "cliente" => "
            SELECT c.nombre, SUM(v.total_venta) as total
            FROM ventas v
            JOIN clientes c ON v.id_cliente = c.id
            WHERE v.fecha_hora BETWEEN ?1 AND ?2
            GROUP BY c.id
            ORDER BY total DESC
            LIMIT 10",
        "vendedor" => "
            SELECT u.username, SUM(v.total_venta) as total
            FROM ventas v
            JOIN usuarios u ON v.id_usuario = u.id
            WHERE v.fecha_hora BETWEEN ?1 AND ?2
            GROUP BY u.id
            ORDER BY total DESC
            LIMIT 10",
        "proveedor" => "
            SELECT pr.nombre_empresa, SUM(dv.cantidad * dv.precio_unitario_en_venta) as total
            FROM detalle_ventas dv
            JOIN ventas v ON dv.id_venta = v.id
            JOIN productos p ON dv.id_producto = p.id
            JOIN proveedores pr ON p.id_proveedor = pr.id
            WHERE v.fecha_hora BETWEEN ?1 AND ?2
            GROUP BY pr.id
            ORDER BY total DESC
            LIMIT 10",
        _ => return Err("Entidad no válida".to_string()),
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![inicio, fin], |row| {
        Ok(ReporteEntidad {
            nombre: row.get(0)?,
            total_ventas: row.get(1)?,
        })
    }).map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn obtener_nuevos_clientes(
    fecha_inicio: String,
    fecha_fin: String,
    agrupacion: String, // "dia" | "mes"
    db: DbConn,
) -> Result<Vec<ReporteNuevosClientes>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    
    let formato_grupo = match agrupacion.as_str() {
        "mes" => "%Y-%m",
        _ => "%Y-%m-%d",
    };

    let sql = format!(
        "SELECT strftime('{}', fecha_registro) AS fecha, COUNT(*) AS cantidad
         FROM clientes
         WHERE DATE(fecha_registro) BETWEEN ?1 AND ?2
         GROUP BY strftime('{}', fecha_registro)
         ORDER BY fecha ASC",
        formato_grupo, formato_grupo
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![fecha_inicio, fecha_fin], |row| {
        Ok(ReporteNuevosClientes {
            fecha: row.get(0)?,
            cantidad: row.get(1)?,
        })
    }).map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
