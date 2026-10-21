#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Producto {
    pub id: i32,
    pub sku: String,
    pub nombre: String,
    pub descripcion: String,
    pub precio_venta: f64,
    pub precio_mayoreo: Option<f64>,
    pub stock_actual: i32,
    pub categoria: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct ItemVenta {
    pub id_producto: i32,
    pub cantidad: i32,
}

#[derive(serde::Serialize, Debug)]
pub struct DetalleVenta {
    pub nombre_producto: String,
    pub cantidad: i32,
    pub precio_unitario: f64,
    pub subtotal: f64,
}

#[derive(serde::Serialize, Debug)]
pub struct VentaCompleta {
    pub id: i32,
    pub fecha_hora: String,
    pub total_venta: f64,
    pub metodo_pago: String,
    pub detalles: Vec<DetalleVenta>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Cliente {
    pub id: i32,
    pub nombre: String,
    pub rfc: String,
    pub telefono: String,
    pub email: String,
    pub puntos: Option<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Proveedor {
    pub id: i32,
    pub nombre_empresa: String,
    pub nombre_contacto: String,
    pub telefono: String,
    pub email: String,
}
