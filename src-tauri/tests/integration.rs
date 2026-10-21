use ferreteria_app::{auth::{required_permission, user_still_allowed, SessionState}, commands::{configuracion::restore_database, corte_caja::totals}, db};
use rusqlite::{params, Connection};
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

fn test_dir() -> PathBuf {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("ferreteria-integration-{}-{nonce}", std::process::id()));
    fs::create_dir(&path).unwrap();
    path
}

#[test]
fn permissions_reject_other_roles_and_invalidate_old_sessions() {
    let state = SessionState::default();
    let seller = state.start(2, "vendedor".into(), vec!["realizar_venta".into()], false).unwrap();
    assert!(state.authorize(&seller, required_permission("realizar_venta").unwrap()));
    for command in ["procesar_devolucion", "guardar_corte", "crear_cotizacion", "agregar_cliente", "crear_backup", "guardar_datos_empresa", "obtener_logs"] {
        assert!(!state.authorize(&seller, required_permission(command).unwrap()), "{command}");
    }
    assert!(!state.authorize(&seller, required_permission("restaurar_backup").unwrap()));
    assert!(!state.authorize(&seller, required_permission("agregar_producto").unwrap()));
    assert!(!state.authorize("", None));
    assert!(required_permission("comando_inexistente").is_err());

    let admin = state.start(1, "admin".into(), vec!["restaurar_respaldos".into()], true).unwrap();
    assert!(!state.authorize(&seller, None));
    assert!(!state.authorize(&admin, required_permission("restaurar_backup").unwrap()));
    assert!(state.authorize(&admin, required_permission("cambiar_password").unwrap()));
    state.password_changed().unwrap();
    assert!(state.authorize(&admin, required_permission("restaurar_backup").unwrap()));
    state.clear().unwrap();
    assert!(!state.authorize(&admin, None));
}

#[test]
fn migrated_roles_keep_access_and_revocations_survive_restart() {
    let conn = Connection::open_in_memory().unwrap();
    db::inicializar(&conn).unwrap();
    conn.execute("INSERT INTO permisos (clave) VALUES ('configuracion')", []).unwrap();
    conn.execute(
        "INSERT INTO rol_permisos (id_rol, id_permiso) SELECT 2, id FROM permisos WHERE clave = 'configuracion'",
        [],
    ).unwrap();
    conn.execute(
        "DELETE FROM rol_permisos WHERE id_rol = 2 AND id_permiso IN
         (SELECT id FROM permisos WHERE clave IN ('gestionar_devoluciones', 'cerrar_caja', 'gestionar_cotizaciones', 'gestionar_clientes', 'gestionar_respaldos', 'restaurar_respaldos', 'editar_empresa', 'ver_bitacora', 'reabrir_caja'))",
        [],
    ).unwrap();
    conn.execute_batch("PRAGMA user_version = 3").unwrap();
    db::inicializar(&conn).unwrap();
    let restored: i64 = conn.query_row(
        "SELECT COUNT(*) FROM rol_permisos rp JOIN permisos p ON p.id = rp.id_permiso
         WHERE rp.id_rol = 2 AND p.clave IN ('gestionar_devoluciones', 'cerrar_caja', 'restaurar_respaldos')",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(restored, 3);
    let obsolete: i64 = conn.query_row("SELECT COUNT(*) FROM permisos WHERE clave = 'configuracion'", [], |row| row.get(0)).unwrap();
    assert_eq!(obsolete, 0);

    conn.execute(
        "DELETE FROM rol_permisos WHERE id_rol = 2 AND id_permiso = (SELECT id FROM permisos WHERE clave = 'cerrar_caja')",
        [],
    ).unwrap();
    db::inicializar(&conn).unwrap();
    let revived: i64 = conn.query_row(
        "SELECT COUNT(*) FROM rol_permisos rp JOIN permisos p ON p.id = rp.id_permiso
         WHERE rp.id_rol = 2 AND p.clave = 'cerrar_caja'",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(revived, 0);
}

#[test]
fn database_revocation_and_account_status_take_effect_immediately() {
    let conn = Connection::open_in_memory().unwrap();
    db::inicializar(&conn).unwrap();
    conn.execute("INSERT INTO usuarios (id, username, password_hash, rol) VALUES (2, 'seller', 'test', 'vendedor')", []).unwrap();
    assert!(user_still_allowed(&conn, 2, Some("realizar_venta")));
    conn.execute(
        "DELETE FROM rol_permisos WHERE id_rol = 2 AND id_permiso = (SELECT id FROM permisos WHERE clave = 'realizar_venta')",
        [],
    ).unwrap();
    assert!(!user_still_allowed(&conn, 2, Some("realizar_venta")));
    conn.execute("UPDATE usuarios SET debe_cambiar_password = 1 WHERE id = 2", []).unwrap();
    assert!(!user_still_allowed(&conn, 2, None));
    assert!(user_still_allowed(&conn, 2, Some("cambiar_password")));
    conn.execute("UPDATE usuarios SET activo = 0 WHERE id = 2", []).unwrap();
    assert!(!user_still_allowed(&conn, 2, Some("cambiar_password")));
}

#[test]
fn restore_replaces_data_and_rejects_unrelated_database() {
    let dir = test_dir();
    let source = dir.join("source.db");
    let destination = dir.join("destination.db");
    let safety = dir.join("safety.db");
    let safety_after_failure = dir.join("safety-after-failure.db");
    let unrelated = dir.join("unrelated.db");
    let future_schema = dir.join("future.db");

    let original = Connection::open(&source).unwrap();
    db::inicializar(&original).unwrap();
    original.execute("INSERT INTO usuarios (username, password_hash, rol) VALUES ('admin', 'test-hash', 'Admin')", []).unwrap();
    original.execute("INSERT INTO productos (sku, nombre, precio_venta, stock_actual) VALUES ('SKU-1', 'Martillo', 50, 10)", []).unwrap();
    drop(original);

    let mut active = Connection::open(&destination).unwrap();
    db::inicializar(&active).unwrap();
    active.execute("INSERT INTO usuarios (username, password_hash, rol) VALUES ('admin2', 'test-hash', 'Admin')", []).unwrap();
    active.execute("INSERT INTO productos (sku, nombre, precio_venta, stock_actual) VALUES ('SKU-2', 'Clavo', 2, 100)", []).unwrap();
    let fake = Connection::open(&unrelated).unwrap();
    fake.execute("CREATE TABLE usuarios (id INTEGER)", []).unwrap();
    drop(fake);

    assert!(restore_database(&unrelated, &mut active, &safety).is_err());
    assert_eq!(active.query_row("SELECT sku FROM productos", [], |row| row.get::<_, String>(0)).unwrap(), "SKU-2");
    assert!(restore_database(&destination, &mut active, &safety).is_err());
    restore_database(&source, &mut active, &safety).unwrap();
    assert_eq!(active.query_row("SELECT sku FROM productos", [], |row| row.get::<_, String>(0)).unwrap(), "SKU-1");
    assert_eq!(active.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0)).unwrap(), "ok");
    let prior = Connection::open(&safety).unwrap();
    assert_eq!(prior.query_row("SELECT sku FROM productos", [], |row| row.get::<_, String>(0)).unwrap(), "SKU-2");
    drop(prior);

    fs::copy(&source, &future_schema).unwrap();
    let future = Connection::open(&future_schema).unwrap();
    future.execute_batch("PRAGMA user_version = 99").unwrap();
    drop(future);
    active.execute("INSERT INTO productos (sku, nombre, precio_venta, stock_actual) VALUES ('SKU-3', 'Conservar', 1, 1)", []).unwrap();
    assert!(restore_database(&future_schema, &mut active, &safety_after_failure).is_err());
    assert_eq!(active.query_row("SELECT COUNT(*) FROM productos WHERE sku = 'SKU-3'", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
    assert_eq!(active.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0)).unwrap(), 5);
    drop(active);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cash_and_card_reconcile_refunds_on_their_own_date() {
    let conn = Connection::open_in_memory().unwrap();
    db::inicializar(&conn).unwrap();
    let sales = [
        ("2026-10-19T10:00:00-06:00", 100.0, "Mixto", Some(60.0), Some(40.0)),
        ("2026-10-19T11:00:00-06:00", 50.0, "Tarjeta", Some(0.0), Some(50.0)),
        ("2026-10-19T12:00:00-06:00", 20.0, "Efectivo", Some(20.0), Some(0.0)),
    ];
    for (date, total, method, cash, card) in sales {
        conn.execute("INSERT INTO ventas (fecha_hora, total_venta, metodo_pago, pago_efectivo, pago_tarjeta) VALUES (?1, ?2, ?3, ?4, ?5)", params![date, total, method, cash, card]).unwrap();
    }
    conn.execute("INSERT INTO devoluciones (id_venta_original, fecha_hora, total_reembolsado, reembolso_efectivo, reembolso_tarjeta) VALUES (1, '2026-10-19T14:00:00-06:00', 50, 30, 20)", []).unwrap();
    conn.execute("INSERT INTO devoluciones (id_venta_original, fecha_hora, total_reembolsado, reembolso_efectivo, reembolso_tarjeta) VALUES (2, '2026-10-20T09:00:00-06:00', 10, 0, 10)", []).unwrap();
    assert_eq!(totals(&conn, "2026-10-19").unwrap(), (170.0, 50.0, 70.0));
    assert_eq!(totals(&conn, "2026-10-20").unwrap(), (0.0, 0.0, -10.0));
}
