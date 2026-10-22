#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;
use ferreteria_app::{auth, commands, db, db_crypto};

fn main() {
    let commands: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> = Box::new(tauri::generate_handler![
            commands::productos::obtener_productos,
            commands::productos::obtener_productos_paginados,
            commands::productos::agregar_producto,
            commands::productos::actualizar_producto,
            commands::productos::eliminar_producto,
            commands::productos::agregar_stock,
            commands::productos::importar_productos_masivos,
            commands::productos::verificar_skus,
            commands::cotizaciones::crear_cotizacion,
            commands::cotizaciones::obtener_cotizaciones,
            commands::cotizaciones::obtener_detalle_cotizacion,
            commands::cotizaciones::eliminar_cotizacion,
            commands::corte_caja::obtener_resumen_corte,
            commands::corte_caja::guardar_corte,
            commands::corte_caja::eliminar_corte,
            commands::productos::obtener_producto_por_sku,
            commands::productos::guardar_componentes_kit,
            commands::productos::obtener_componentes_kit,
            commands::ventas::realizar_venta,
            commands::reportes::obtener_productos_con_bajo_stock,
            commands::reportes::obtener_productos_con_sobre_stock,
            commands::reportes::generar_reporte_ventas,
            commands::reportes::obtener_resumen_dashboard,
            commands::seguridad::login,
            commands::seguridad::hay_usuarios,
            commands::seguridad::crear_usuario_inicial,
            commands::seguridad::cambiar_password,
            commands::seguridad::logout,
            commands::seguridad::obtener_usuarios,
            commands::seguridad::crear_usuario,
            commands::seguridad::eliminar_usuario,
            commands::seguridad::actualizar_estado_usuario,
            commands::seguridad::restablecer_password_usuario,
            commands::seguridad::obtener_roles,
            commands::seguridad::obtener_permisos_rol,
            commands::seguridad::guardar_permisos_rol,
            commands::devoluciones::buscar_venta_para_devolucion,
            commands::devoluciones::procesar_devolucion,
            commands::clientes::obtener_clientes,
            commands::clientes::agregar_cliente,
            commands::clientes::actualizar_cliente,
            commands::clientes::eliminar_cliente,
            commands::clientes::obtener_historial_compras,
            commands::proveedores::obtener_proveedores,
            commands::proveedores::agregar_proveedor,
            commands::proveedores::actualizar_proveedor,
            commands::proveedores::eliminar_proveedor,
            commands::configuracion::restaurar_backup,
            commands::configuracion::crear_backup,
            commands::configuracion::obtener_configuracion_local,
            commands::configuracion::guardar_configuracion_local,
            commands::importaciones::previsualizar_clientes,
            commands::importaciones::confirmar_clientes,
            commands::importaciones::previsualizar_proveedores,
            commands::importaciones::confirmar_proveedores,
            commands::archivos::leer_catalogo,
            commands::archivos::leer_clientes_archivo,
            commands::archivos::leer_proveedores_archivo,
            commands::archivos::guardar_reporte_archivo,
            commands::archivos::guardar_codigo_barras_svg,
            ferreteria_app::mobile_scanner::iniciar_lector_movil,
            ferreteria_app::mobile_scanner::conectar_android_usb,
            ferreteria_app::mobile_scanner::tomar_lectura_movil,
            ferreteria_app::mobile_scanner::detener_lector_movil,
            ferreteria_app::mobile_scanner::listar_dispositivos_lector,
            ferreteria_app::mobile_scanner::revocar_dispositivo_lector,
            commands::configuracion::obtener_datos_empresa,
            commands::configuracion::guardar_datos_empresa,
            db_crypto::estado_cifrado,
            db_crypto::configurar_base_inicial,
            db_crypto::desbloquear_base,
            db_crypto::activar_cifrado,
            commands::devoluciones::generar_reporte_devoluciones,
            commands::analytics::obtener_productos_mas_vendidos,
            commands::analytics::obtener_ventas_por_categoria,
            commands::analytics::obtener_historial_ventas,
            commands::analytics::obtener_top_entidades,
            commands::analytics::obtener_nuevos_clientes,
            commands::analytics_widgets::guardar_widget_analytics,
            commands::analytics_widgets::obtener_widgets_analytics,
            commands::analytics_widgets::eliminar_widget_analytics,
            commands::auditoria::obtener_logs,
        ]);
    tauri::Builder::default()
        .manage(auth::SessionState::default())
        .manage(ferreteria_app::mobile_scanner::MobileScannerState::default())
        .setup(|app| {
            let handle = app.handle();
            let app_dir = handle
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            let db_path = app_dir.join("ferreteria.db");
            db::preparar_directorio_local(&app_dir).expect("Failed to prepare private data directory");
            db_crypto::recover_interrupted_swap(&db_path).expect("Failed to recover database conversion");
            let first_run = !db_path.exists();
            if !first_run {
                db::preparar_archivo_local(&app_dir, &db_path).expect("Failed to protect database file");
            }
            let encrypted = !first_run && db_crypto::is_encrypted(&db_path).expect("Failed to inspect database header");
            let conn = if first_run || encrypted { Connection::open_in_memory().expect("Failed to open startup placeholder") }
                else { Connection::open(&db_path).expect("Failed to open database") };
            if !first_run && !encrypted { db::inicializar(&conn).expect("Failed to initialize database"); }
            app.manage(db::DbConnection(Mutex::new(conn)));
            app.manage(db_crypto::CryptoState::new(db_path, first_run, encrypted));
            let background = app.handle().clone();
            std::thread::spawn(move || loop {
                if let (Some(crypto), Some(database)) = (
                    background.try_state::<db_crypto::CryptoState>(),
                    background.try_state::<db::DbConnection>(),
                ) {
                    if let Ok(key) = crypto.key() {
                        if let Ok(mut conn) = database.0.lock() {
                            if let Err(error) = commands::backups::run_automatic_recorded_key(&mut conn, key.as_deref()) {
                                eprintln!("Respaldo automático: {error}");
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(3600));
            });
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(move |invoke: tauri::ipc::Invoke| {
            let command = invoke.message.command();
            let webview = invoke.message.webview_ref();
            let crypto = webview.state::<db_crypto::CryptoState>();
            if matches!(command, "estado_cifrado" | "configurar_base_inicial" | "desbloquear_base") {
                return commands(invoke);
            }
            if crypto.status().map_or(true, |status| status.locked) {
                invoke.resolver.reject("Primero abre la base de datos");
                return true;
            }
            if matches!(command, "login" | "hay_usuarios" | "crear_usuario_inicial") {
                return commands(invoke);
            }
            let authorized = auth::required_permission(command).is_ok_and(|permission| {
                let token = invoke
                    .message
                    .headers()
                    .get("x-session-token")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("");
                let session = webview.state::<auth::SessionState>();
                if !session.authorize(token, permission) {
                    return false;
                }
                if permission == Some("logout") {
                    return true;
                }
                let Ok(current) = session.current_user() else {
                    return false;
                };
                let db = webview.state::<db::DbConnection>();
                let Ok(conn) = db.0.lock() else {
                    return false;
                };
                auth::user_still_allowed(&conn, current.user_id, permission)
            });
            if authorized {
                commands(invoke)
            } else {
                invoke.resolver.reject("Sesión inválida o permiso insuficiente");
                true
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
