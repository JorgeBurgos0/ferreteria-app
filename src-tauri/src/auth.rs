use rand_core::{OsRng, RngCore};
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

pub fn required_permission(command: &str) -> Result<Option<&'static str>, ()> {
    let permission = match command {
        "logout" => Some("logout"),
        "cambiar_password" => Some("cambiar_password"),
        "obtener_productos" | "obtener_productos_paginados" | "obtener_producto_por_sku"
        | "obtener_clientes" | "obtener_proveedores" | "obtener_datos_empresa"
        | "obtener_cotizaciones" | "obtener_roles" | "obtener_permisos_rol" => None,
        "obtener_productos_con_bajo_stock" => None,
        "agregar_producto" | "actualizar_producto" | "eliminar_producto" | "agregar_stock"
        | "importar_productos_masivos" | "verificar_skus" | "guardar_componentes_kit"
        | "obtener_componentes_kit" | "agregar_proveedor" | "actualizar_proveedor"
        | "eliminar_proveedor" => Some("gestion_inventario"),
        "realizar_venta" | "crear_cotizacion" | "eliminar_cotizacion"
        | "buscar_venta_para_devolucion" | "procesar_devolucion"
        | "agregar_cliente" | "actualizar_cliente" | "eliminar_cliente"
        | "obtener_historial_compras" | "obtener_resumen_corte" | "guardar_corte"
        => Some("realizar_venta"),
        "obtener_usuarios" | "crear_usuario" | "eliminar_usuario" | "guardar_permisos_rol" => {
            Some("gestionar_usuarios")
        }
        "obtener_productos_con_sobre_stock" | "generar_reporte_ventas"
        | "generar_reporte_devoluciones" => Some("ver_reportes"),
        "obtener_resumen_dashboard" | "obtener_productos_mas_vendidos"
        | "obtener_ventas_por_categoria" | "obtener_historial_ventas"
        | "obtener_top_entidades" | "obtener_nuevos_clientes"
        | "guardar_widget_analytics" | "obtener_widgets_analytics"
        | "eliminar_widget_analytics" => Some("ver_dashboard"),
        "crear_backup" | "restaurar_backup" | "guardar_datos_empresa" | "obtener_logs"
        | "eliminar_corte" => {
            Some("configuracion")
        }
        _ => return Err(()),
    };
    Ok(permission)
}
