use rand_core::{OsRng, RngCore};
use rusqlite::{params, Connection};
use std::sync::Mutex;

#[derive(Clone)]
pub struct Session {
    pub user_id: i32,
    pub username: String,
    pub permissions: Vec<String>,
    pub token: String,
    pub must_change_password: bool,
}

#[derive(Default)]
pub struct SessionState(pub Mutex<Option<Session>>);

impl SessionState {
    pub fn start(
        &self,
        user_id: i32,
        username: String,
        permissions: Vec<String>,
        must_change_password: bool,
    ) -> Result<String, String> {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let token: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        *self.0.lock().map_err(|_| "Sesión no disponible")? = Some(Session {
            user_id,
            username,
            permissions,
            token: token.clone(),
            must_change_password,
        });
        Ok(token)
    }

    pub fn authorize(&self, token: &str, permission: Option<&str>) -> bool {
        let Ok(session) = self.0.lock() else {
            return false;
        };
        let Some(session) = session.as_ref() else {
            return false;
        };
        if token.is_empty() || token != session.token {
            return false;
        }
        if matches!(permission, Some("logout" | "cambiar_password")) {
            return true;
        }
        if session.must_change_password {
            return false;
        }
        permission.is_none_or(|required| session.permissions.iter().any(|p| p == required))
    }

    pub fn current_user(&self) -> Result<Session, String> {
        self.0
            .lock()
            .map_err(|_| "Sesión no disponible".to_string())?
            .clone()
            .ok_or_else(|| "Inicia sesión".to_string())
    }

    pub fn password_changed(&self) -> Result<(), String> {
        let mut session = self.0.lock().map_err(|_| "Sesión no disponible")?;
        if let Some(session) = session.as_mut() {
            session.must_change_password = false;
        }
        Ok(())
    }

    pub fn clear(&self) -> Result<(), String> {
        *self.0.lock().map_err(|_| "Sesión no disponible")? = None;
        Ok(())
    }
}

pub fn user_still_allowed(conn: &Connection, user_id: i32, permission: Option<&str>) -> bool {
    let (active, must_change): (bool, bool) = conn.query_row(
        "SELECT activo, debe_cambiar_password FROM usuarios WHERE id = ?1",
        params![user_id], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap_or((false, false));
    if !active {
        return false;
    }
    if permission == Some("cambiar_password") {
        return true;
    }
    if must_change {
        return false;
    }
    let Some(required) = permission else {
        return true;
    };
    conn.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM usuarios u
           JOIN roles r ON LOWER(r.nombre) = LOWER(u.rol)
           JOIN rol_permisos rp ON rp.id_rol = r.id
           JOIN permisos p ON p.id = rp.id_permiso
           WHERE u.id = ?1 AND p.clave = ?2
         )",
        params![user_id, required], |row| row.get::<_, bool>(0),
    ).unwrap_or(false)
}

pub fn required_permission(command: &str) -> Result<Option<&'static str>, ()> {
    let permission = match command {
        "logout" => Some("logout"),
        "cambiar_password" => Some("cambiar_password"),
        "obtener_productos" | "obtener_productos_paginados" | "obtener_producto_por_sku"
        | "obtener_clientes" | "obtener_datos_empresa" => None,
        "obtener_productos_con_bajo_stock" => None,
        "agregar_producto" | "actualizar_producto" | "eliminar_producto" | "agregar_stock"
        | "importar_productos_masivos" | "verificar_skus" | "guardar_componentes_kit"
        | "obtener_componentes_kit" | "obtener_proveedores" | "agregar_proveedor"
        | "actualizar_proveedor" | "eliminar_proveedor" => Some("gestion_inventario"),
        "realizar_venta" => Some("realizar_venta"),
        "crear_cotizacion" | "obtener_cotizaciones" | "eliminar_cotizacion" => Some("gestionar_cotizaciones"),
        "buscar_venta_para_devolucion" | "procesar_devolucion" => Some("gestionar_devoluciones"),
        "agregar_cliente" | "actualizar_cliente" | "eliminar_cliente"
        | "obtener_historial_compras" => Some("gestionar_clientes"),
        "obtener_resumen_corte" | "guardar_corte" => Some("cerrar_caja"),
        "eliminar_corte" => Some("reabrir_caja"),
        "obtener_usuarios" | "crear_usuario" | "eliminar_usuario" | "guardar_permisos_rol"
        | "actualizar_estado_usuario" | "restablecer_password_usuario" => {
            Some("gestionar_usuarios")
        }
        "obtener_roles" | "obtener_permisos_rol" => Some("gestionar_usuarios"),
        "obtener_productos_con_sobre_stock" | "generar_reporte_ventas"
        | "generar_reporte_devoluciones" => Some("ver_reportes"),
        "obtener_resumen_dashboard" | "obtener_productos_mas_vendidos"
        | "obtener_ventas_por_categoria" | "obtener_historial_ventas"
        | "obtener_top_entidades" | "obtener_nuevos_clientes"
        | "guardar_widget_analytics" | "obtener_widgets_analytics"
        | "eliminar_widget_analytics" => Some("ver_dashboard"),
        "crear_backup" => Some("gestionar_respaldos"),
        "restaurar_backup" => Some("restaurar_respaldos"),
        "guardar_datos_empresa" => Some("editar_empresa"),
        "obtener_logs" => Some("ver_bitacora"),
        _ => return Err(()),
    };
    Ok(permission)
}
