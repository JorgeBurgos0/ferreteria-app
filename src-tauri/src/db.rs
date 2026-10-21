use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;

pub struct DbConnection(pub Mutex<Connection>);

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), rusqlite::Error> {
    let exists: Option<i32> = conn
        .query_row(
            "SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2",
            params![table, column],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"))?;
    }
    Ok(())
}

pub fn inicializar(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > 3 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    conn.execute_batch("SAVEPOINT app_schema")?;
    let result = (|| -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS productos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sku TEXT UNIQUE NOT NULL,
            nombre TEXT NOT NULL,
            descripcion TEXT,
            precio_venta REAL NOT NULL,
            precio_compra REAL,
            stock_actual INTEGER NOT NULL DEFAULT 0,
            stock_minimo INTEGER DEFAULT 5,
            id_proveedor INTEGER,
            ubicacion TEXT,
            activo INTEGER DEFAULT 1,
            precio_mayoreo REAL,
            FOREIGN KEY (id_proveedor) REFERENCES proveedores(id)
        );
        ",
    )?;

    // Migraciones manuales (intentar agregar columnas si no existen)
    ensure_column(conn, "productos", "activo", "INTEGER DEFAULT 1")?;
    ensure_column(conn, "productos", "precio_mayoreo", "REAL")?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS proveedores (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre_empresa TEXT NOT NULL UNIQUE,
            nombre_contacto TEXT,
            telefono TEXT,
            email TEXT
        );
        CREATE TABLE IF NOT EXISTS ventas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fecha_hora TEXT NOT NULL,
            id_cliente INTEGER,
            total_venta REAL NOT NULL,
            metodo_pago TEXT,
            FOREIGN KEY (id_cliente) REFERENCES clientes(id)
        );
        CREATE TABLE IF NOT EXISTS detalle_ventas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            id_venta INTEGER NOT NULL,
            id_producto INTEGER NOT NULL,
            cantidad INTEGER NOT NULL,
            precio_unitario_en_venta REAL NOT NULL,
            FOREIGN KEY (id_venta) REFERENCES ventas(id),
            FOREIGN KEY (id_producto) REFERENCES productos(id)
        );
        CREATE TABLE IF NOT EXISTS detalle_venta_componentes (
            id_venta INTEGER NOT NULL,
            id_producto_kit INTEGER NOT NULL,
            id_componente INTEGER NOT NULL,
            cantidad_descontada INTEGER NOT NULL CHECK (cantidad_descontada > 0),
            PRIMARY KEY (id_venta, id_producto_kit, id_componente),
            FOREIGN KEY (id_venta) REFERENCES ventas(id),
            FOREIGN KEY (id_producto_kit) REFERENCES productos(id),
            FOREIGN KEY (id_componente) REFERENCES productos(id)
        );
        CREATE TABLE IF NOT EXISTS cotizaciones (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fecha_hora TEXT NOT NULL,
            id_cliente INTEGER,
            total_cotizacion REAL NOT NULL,
            vigencia TEXT,
            FOREIGN KEY (id_cliente) REFERENCES clientes(id)
        );
        CREATE TABLE IF NOT EXISTS detalle_cotizaciones (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            id_cotizacion INTEGER NOT NULL,
            id_producto INTEGER NOT NULL,
            cantidad INTEGER NOT NULL,
            precio_unitario REAL NOT NULL,
            FOREIGN KEY (id_cotizacion) REFERENCES cotizaciones(id),
            FOREIGN KEY (id_producto) REFERENCES productos(id)
        );
        CREATE TABLE IF NOT EXISTS producto_compuesto (
            id_kit INTEGER NOT NULL,
            id_componente INTEGER NOT NULL,
            cantidad REAL NOT NULL,
            PRIMARY KEY (id_kit, id_componente),
            FOREIGN KEY (id_kit) REFERENCES productos(id),
            FOREIGN KEY (id_componente) REFERENCES productos(id)
        );
        CREATE TABLE IF NOT EXISTS clientes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL,
            rfc TEXT UNIQUE,
            telefono TEXT,
            email TEXT
        );
        CREATE TABLE IF NOT EXISTS devoluciones (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            id_venta_original INTEGER NOT NULL,
            fecha_hora TEXT NOT NULL,
            total_reembolsado REAL NOT NULL,
            motivo TEXT,
            FOREIGN KEY (id_venta_original) REFERENCES ventas(id)
        );
        CREATE TABLE IF NOT EXISTS detalle_devoluciones (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            id_devolucion INTEGER NOT NULL,
            id_producto INTEGER NOT NULL,
            cantidad INTEGER NOT NULL,
            monto_reembolsado REAL NOT NULL,
            FOREIGN KEY (id_devolucion) REFERENCES devoluciones(id),
            FOREIGN KEY (id_producto) REFERENCES productos(id)
        );
        CREATE TABLE IF NOT EXISTS usuarios (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            rol TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS auditoria (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fecha_hora TEXT NOT NULL,
            usuario TEXT NOT NULL,
            accion TEXT NOT NULL,
            detalles TEXT
        );
        CREATE TABLE IF NOT EXISTS cortes_caja (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fecha TEXT NOT NULL UNIQUE,
            total_ventas REAL NOT NULL,
            esperado_efectivo REAL NOT NULL,
            esperado_tarjeta REAL NOT NULL,
            contado_efectivo REAL NOT NULL,
            contado_tarjeta REAL NOT NULL,
            diferencia REAL NOT NULL,
            usuario TEXT NOT NULL,
            fecha_hora_cierre TEXT NOT NULL
        );
        ",
    )?;

    // Migraciones para Phase 3
    // Intentamos agregar las columnas. Si fallan porque ya existen, ignoramos el error.
    // SQLite no soporta IF NOT EXISTS en ADD COLUMN en versiones viejas, pero en las nuevas sí.
    // Sin embargo, para asegurar compatibilidad, lo hacemos en bloques separados.

    ensure_column(conn, "clientes", "puntos", "INTEGER DEFAULT 0")?;
    ensure_column(conn, "productos", "categoria", "TEXT DEFAULT 'General'")?;
    ensure_column(conn, "productos", "es_kit", "INTEGER DEFAULT 0")?;
    ensure_column(conn, "productos", "stock_maximo", "INTEGER")?;
    
    // Migraciones Phase 8: Advanced Analytics
    ensure_column(conn, "clientes", "fecha_registro", "TEXT")?;
    ensure_column(conn, "ventas", "id_usuario", "INTEGER")?;
    ensure_column(conn, "ventas", "pago_efectivo", "REAL")?;
    ensure_column(conn, "ventas", "pago_tarjeta", "REAL")?;
    ensure_column(conn, "devoluciones", "reembolso_efectivo", "REAL")?;
    ensure_column(conn, "devoluciones", "reembolso_tarjeta", "REAL")?;
    conn.execute_batch("UPDATE clientes SET fecha_registro = COALESCE((SELECT MIN(substr(fecha_hora, 1, 10)) FROM ventas WHERE ventas.id_cliente = clientes.id), datetime('now','localtime')) WHERE fecha_registro IS NULL OR fecha_registro = ''")?;
    
    // Migraciones Phase 4: RBAC
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS roles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL UNIQUE,
            descripcion TEXT
        );
        CREATE TABLE IF NOT EXISTS permisos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            clave TEXT NOT NULL UNIQUE, -- ej: 'ver_reportes', 'eliminar_productos'
            descripcion TEXT
        );
        CREATE TABLE IF NOT EXISTS rol_permisos (
            id_rol INTEGER NOT NULL,
            id_permiso INTEGER NOT NULL,
            PRIMARY KEY (id_rol, id_permiso),
            FOREIGN KEY (id_rol) REFERENCES roles(id),
            FOREIGN KEY (id_permiso) REFERENCES permisos(id)
        );
        
        -- Insertar roles por defecto si no existen
        INSERT OR IGNORE INTO roles (id, nombre, descripcion) VALUES (1, 'Admin', 'Acceso total');
        INSERT OR IGNORE INTO roles (id, nombre, descripcion) VALUES (2, 'Vendedor', 'Acceso limitado a ventas y clientes');

        -- Insertar algunos permisos básicos
        INSERT OR IGNORE INTO permisos (clave, descripcion) VALUES ('ver_dashboard', 'Ver el dashboard principal');
        INSERT OR IGNORE INTO permisos (clave, descripcion) VALUES ('gestion_inventario', 'Crear, editar y eliminar productos');
        INSERT OR IGNORE INTO permisos (clave, descripcion) VALUES ('realizar_venta', 'Acceso al punto de venta');
        INSERT OR IGNORE INTO permisos (clave, descripcion) VALUES ('ver_reportes', 'Ver reportes de ventas');
        INSERT OR IGNORE INTO permisos (clave, descripcion) VALUES ('configuracion', 'Acceso a configuración del sistema');
        INSERT OR IGNORE INTO permisos (clave, descripcion) VALUES ('gestionar_usuarios', 'Crear y eliminar usuarios');
        
        -- Asignar todos los permisos al Admin (id 1)
        INSERT OR IGNORE INTO rol_permisos (id_rol, id_permiso) SELECT 1, id FROM permisos;

        -- Asignar permisos básicos al Vendedor (id 2)
        INSERT OR IGNORE INTO rol_permisos (id_rol, id_permiso) 
        SELECT 2, id FROM permisos WHERE clave IN ('ver_dashboard', 'realizar_venta');
        "
    )?;

    // Migrar usuarios existentes para que 'rol' coincida con la nueva tabla (opcional o futuro refactor)
    // Por ahora el sistema usaba strings 'admin'/'vendedor'. La tabla 'usuarios' tiene columna 'rol' (TEXT).
    // Podríamos dejarlo así y mapear ese TEXT al nombre del Rol en la nueva tabla, o migrar 'usuarios' para usar 'id_rol'.
    // Para simplificar y mantener backward compatibility, seguiremos usando el string 'rol' en usuarios,
    // pero al loguearse buscaremos los permisos asociados a ese nombre de rol.

    // Tabla de Datos de la Empresa (Personalización)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS datos_empresa (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            nombre TEXT DEFAULT 'Mi Ferretería',
            direccion TEXT DEFAULT '',
            telefono TEXT DEFAULT '',
            logo TEXT DEFAULT ''
        )",
        [],
    )?;

    // Insertar registro por defecto si no existe
    conn.execute(
        "INSERT OR IGNORE INTO datos_empresa (id, nombre) VALUES (1, 'Mi Ferretería')",
        [],
    )?;

    // Tabla de Analytics Widgets (Gráficas Personalizadas)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS analytics_widgets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL,
            tipo_reporte TEXT NOT NULL,
            configuracion TEXT NOT NULL,
            orden INTEGER NOT NULL,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    if version < 3 {
        conn.execute_batch(
            "UPDATE datos_empresa SET direccion = '' WHERE direccion = 'Calle Principal #123';
             UPDATE datos_empresa SET telefono = '' WHERE telefono = '555-0000';"
        )?;
    }
    conn.execute_batch("PRAGMA user_version = 3;")?;
    Ok(())
    })();
    match result {
        Ok(()) => conn.execute_batch("RELEASE app_schema"),
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK TO app_schema; RELEASE app_schema");
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_schema_is_reentrant_and_enforces_foreign_keys() {
        let conn = Connection::open_in_memory().unwrap();
        inicializar(&conn).unwrap();
        inicializar(&conn).unwrap();
        let enabled: i32 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0)).unwrap();
        let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(enabled, 1);
        assert_eq!(version, 3);
        let (direccion, telefono): (String, String) = conn.query_row(
            "SELECT direccion, telefono FROM datos_empresa WHERE id = 1", [],
            |row| Ok((row.get(0)?, row.get(1)?))
        ).unwrap();
        assert!(direccion.is_empty() && telefono.is_empty());
        assert!(conn.execute(
            "INSERT INTO detalle_ventas (id_venta, id_producto, cantidad, precio_unitario_en_venta) VALUES (999, 999, 1, 1)",
            [],
        ).is_err());
    }

    #[test]
    fn existing_clients_receive_a_registration_date() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE clientes (id INTEGER PRIMARY KEY, nombre TEXT NOT NULL, rfc TEXT UNIQUE, telefono TEXT, email TEXT); INSERT INTO clientes (id, nombre) VALUES (1, 'A');").unwrap();
        inicializar(&conn).unwrap();
        let date: String = conn.query_row("SELECT fecha_registro FROM clientes WHERE id = 1", [], |row| row.get(0)).unwrap();
        assert!(!date.is_empty());
    }

    #[test]
    fn removes_only_old_company_placeholders() {
        let conn = Connection::open_in_memory().unwrap();
        inicializar(&conn).unwrap();
        conn.execute("UPDATE datos_empresa SET direccion = 'Calle Principal #123', telefono = '555-0000'", []).unwrap();
        conn.execute_batch("PRAGMA user_version = 2").unwrap();
        inicializar(&conn).unwrap();
        let (direccion, telefono): (String, String) = conn.query_row(
            "SELECT direccion, telefono FROM datos_empresa WHERE id = 1", [],
            |row| Ok((row.get(0)?, row.get(1)?))
        ).unwrap();
        assert!(direccion.is_empty() && telefono.is_empty());
        conn.execute("UPDATE datos_empresa SET direccion = 'Sucursal Centro', telefono = '5551234567'", []).unwrap();
        inicializar(&conn).unwrap();
        let (direccion, telefono): (String, String) = conn.query_row(
            "SELECT direccion, telefono FROM datos_empresa WHERE id = 1", [],
            |row| Ok((row.get(0)?, row.get(1)?))
        ).unwrap();
        assert_eq!(direccion, "Sucursal Centro");
        assert_eq!(telefono, "5551234567");
    }
}
