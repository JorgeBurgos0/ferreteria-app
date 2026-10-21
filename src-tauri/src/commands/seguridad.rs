// src-tauri/src/commands/seguridad.rs
use crate::{auth::SessionState, commands::auditoria, db::DbConnection};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, OptionalExtension};
use tauri::State;

type DbConn<'a> = State<'a, DbConnection>;

// Struct para devolver los datos del usuario logueado al frontend
#[derive(serde::Serialize, Clone)]
pub struct UsuarioLogueado {
    id: i32,
    username: String,
    rol: String,
    permisos: Vec<String>,
    token: String,
    requiere_cambio_password: bool,
}

// Struct para listar usuarios (sin hash)
#[derive(serde::Serialize)]
pub struct Usuario {
    id: i32,
    username: String,
    rol: String,
    activo: bool,
    permisos: Vec<String>,
}

#[tauri::command]
pub fn login(username: String, password: String, db: DbConn, session: State<SessionState>) -> Result<UsuarioLogueado, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    // 1. Buscamos al usuario por su nombre de usuario
    let account: Option<(i32, String, String, bool, bool)> = conn
        .query_row(
            "SELECT id, password_hash, rol, activo, debe_cambiar_password FROM usuarios WHERE username = ?1",
            params![username],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((id, password_hash, rol, activo, debe_cambiar_password)) = account else {
        if username.len() <= 64 {
            auditoria::registrar_evento_interno(&conn, &username, "INICIO_SESION_FALLIDO", None)
                .map_err(|e| e.to_string())?;
        }
        return Err("Usuario o contraseña incorrectos".into());
    };
    if !activo {
        auditoria::registrar_evento_interno(&conn, &username, "INICIO_SESION_FALLIDO", None)
            .map_err(|e| e.to_string())?;
        return Err("Usuario o contraseña incorrectos".into());
    }

    // 2. Parseamos el hash guardado en la base de datos
    let parsed_hash =
        PasswordHash::new(&password_hash).map_err(|_| "Error interno del servidor".to_string())?;

    // 3. Verificamos que la contraseña proporcionada coincida con el hash
    if Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_err() {
        auditoria::registrar_evento_interno(&conn, &username, "INICIO_SESION_FALLIDO", None)
            .map_err(|e| e.to_string())?;
        return Err("Usuario o contraseña incorrectos".into());
    }

    // 4. Obtener permisos del rol
    let mut stmt_permisos = conn.prepare(
        "SELECT p.clave 
         FROM permisos p
         JOIN rol_permisos rp ON p.id = rp.id_permiso
         JOIN roles r ON rp.id_rol = r.id
         WHERE LOWER(r.nombre) = LOWER(?1)"
    ).map_err(|e| e.to_string())?;

    let permisos_iter = stmt_permisos.query_map(params![rol], |row| {
        row.get(0)
    }).map_err(|e| e.to_string())?;

    let mut permisos = Vec::new();
    for p in permisos_iter {
        permisos.push(p.map_err(|e| e.to_string())?);
    }

    // 5. Si todo es correcto, devolvemos los datos del usuario con sus permisos
    let requiere_cambio_password = debe_cambiar_password || username == "admin" && password == "admin";
    auditoria::registrar_evento_interno(&conn, &username, "INICIO_SESION", None)
        .map_err(|e| e.to_string())?;
    let token = session.start(id, username.clone(), permisos.clone(), requiere_cambio_password)?;
    Ok(UsuarioLogueado { id, username, rol, permisos, token, requiere_cambio_password })
}

#[tauri::command]
pub fn hay_usuarios(db: DbConn) -> Result<bool, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM usuarios", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    Ok(count > 0)
}

fn validar_password(password: &str, username: &str) -> Result<(), String> {
    if password.chars().count() < 12 || password.to_lowercase().contains(&username.to_lowercase()) {
        return Err("La contraseña debe tener al menos 12 caracteres y no incluir el usuario".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn crear_usuario_inicial(password: String, db: DbConn) -> Result<(), String> {
    validar_password(&password, "admin")?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM usuarios", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if count != 0 {
        return Err("La cuenta inicial ya fue creada".to_string());
    }
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| "No se pudo hashear la contraseña".to_string())?
        .to_string();

    // Creamos el usuario "admin" si no existe
    conn.execute(
        "INSERT INTO usuarios (username, password_hash, rol) VALUES (?1, ?2, ?3)",
        params!["admin", password_hash, "admin"],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn cambiar_password(password_actual: String, password_nueva: String, db: DbConn, session: State<SessionState>) -> Result<(), String> {
    let current = session.current_user()?;
    validar_password(&password_nueva, &current.username)?;
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let previous_hash: String = conn.query_row(
        "SELECT password_hash FROM usuarios WHERE id = ?1", params![current.user_id], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    let parsed = PasswordHash::new(&previous_hash).map_err(|_| "Contraseña guardada inválida")?;
    Argon2::default().verify_password(password_actual.as_bytes(), &parsed)
        .map_err(|_| "La contraseña actual es incorrecta".to_string())?;
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password_nueva.as_bytes(), &salt)
        .map_err(|_| "No se pudo proteger la contraseña".to_string())?
        .to_string();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("UPDATE usuarios SET password_hash = ?1, debe_cambiar_password = 0 WHERE id = ?2", params![hash, current.user_id])
        .map_err(|e| e.to_string())?;
    auditoria::registrar_evento_interno(&tx, &current.username, "CAMBIAR_PASSWORD", None)
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    session.password_changed()?;
    Ok(())
}

#[tauri::command]
pub fn logout(session: State<SessionState>) -> Result<(), String> {
    session.clear()
}

#[tauri::command]
pub fn obtener_usuarios(db: DbConn) -> Result<Vec<Usuario>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, username, rol, activo FROM usuarios")
        .map_err(|e| e.to_string())?;

    let usuarios_iter = stmt
        .query_map([], |row| {
            Ok(Usuario {
                id: row.get(0)?,
                username: row.get(1)?,
                rol: row.get(2)?,
                activo: row.get(3)?,
                permisos: Vec::new(), // Por defecto vacío en lista simple
            })
        })
        .map_err(|e| e.to_string())?;

    let mut usuarios = Vec::new();
    for usuario in usuarios_iter {
        usuarios.push(usuario.map_err(|e| e.to_string())?);
    }
    Ok(usuarios)
}

#[tauri::command]
pub fn crear_usuario(
    username: String,
    password: String,
    rol: String,
    db: DbConn,
    session: State<SessionState>,
) -> Result<(), String> {
    validar_password(&password, &username)?;
    let actor = session.current_user()?.username;
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let role_exists: Option<i32> = conn.query_row(
        "SELECT 1 FROM roles WHERE LOWER(nombre) = LOWER(?1)",
        params![rol], |row| row.get(0),
    ).optional().map_err(|e| e.to_string())?;
    if role_exists.is_none() {
        return Err("El rol no existe".into());
    }

    // Hasheamos la contraseña
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| "No se pudo hashear la contraseña".to_string())?
        .to_string();

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO usuarios (username, password_hash, rol, debe_cambiar_password) VALUES (?1, ?2, ?3, 1)",
        params![username, password_hash, rol],
    )
    .map_err(|e| e.to_string())?;
    let details = format!("usuario={username}; rol={rol}");
    auditoria::registrar_evento_interno(&tx, &actor, "CREAR_USUARIO", Some(&details))
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn eliminar_usuario(id: i32, db: DbConn, session: State<SessionState>) -> Result<(), String> {
    let actor = session.current_user()?;
    if id == actor.user_id {
        return Err("No puedes eliminar tu cuenta activa".into());
    }
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let (is_admin, is_active): (bool, bool) = conn.query_row(
        "SELECT LOWER(rol) = 'admin', activo FROM usuarios WHERE id = ?1",
        params![id], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(|e| e.to_string())?.ok_or("Usuario no encontrado")?;
    if is_admin && is_active {
        let admins: i64 = conn.query_row(
            "SELECT COUNT(*) FROM usuarios WHERE LOWER(rol) = 'admin' AND activo = 1", [], |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        if admins <= 1 {
            return Err("No se puede eliminar el último administrador".to_string());
        }
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM usuarios WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    let details = format!("usuario_id={id}");
    auditoria::registrar_evento_interno(&tx, &actor.username, "ELIMINAR_USUARIO", Some(&details))
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn actualizar_estado_usuario(id: i32, activo: bool, db: DbConn, session: State<SessionState>) -> Result<(), String> {
    let actor = session.current_user()?;
    if id == actor.user_id && !activo {
        return Err("No puedes desactivar tu cuenta activa".into());
    }
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let role: String = tx.query_row("SELECT rol FROM usuarios WHERE id = ?1", params![id], |row| row.get(0))
        .map_err(|_| "Usuario no encontrado".to_string())?;
    if !activo && role.eq_ignore_ascii_case("admin") {
        let active_admins: i64 = tx.query_row(
            "SELECT COUNT(*) FROM usuarios WHERE LOWER(rol) = 'admin' AND activo = 1",
            [], |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        if active_admins <= 1 {
            return Err("No se puede desactivar el último administrador activo".into());
        }
    }
    tx.execute("UPDATE usuarios SET activo = ?1 WHERE id = ?2", params![activo, id])
        .map_err(|e| e.to_string())?;
    let details = format!("usuario_id={id}; activo={activo}");
    auditoria::registrar_evento_interno(&tx, &actor.username, "ESTADO_USUARIO", Some(&details))
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn restablecer_password_usuario(id: i32, password_nueva: String, db: DbConn, session: State<SessionState>) -> Result<(), String> {
    let actor = session.current_user()?;
    if id == actor.user_id {
        return Err("Usa el cambio de contraseña de tu propia cuenta".into());
    }
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let username: String = conn.query_row("SELECT username FROM usuarios WHERE id = ?1", params![id], |row| row.get(0))
        .map_err(|_| "Usuario no encontrado".to_string())?;
    validar_password(&password_nueva, &username)?;
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password_nueva.as_bytes(), &salt)
        .map_err(|_| "No se pudo proteger la contraseña".to_string())?
        .to_string();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE usuarios SET password_hash = ?1, debe_cambiar_password = 1 WHERE id = ?2",
        params![hash, id],
    ).map_err(|e| e.to_string())?;
    let details = format!("usuario_id={id}");
    auditoria::registrar_evento_interno(&tx, &actor.username, "RESTABLECER_PASSWORD", Some(&details))
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize)]
pub struct Rol {
    id: i32,
    nombre: String,
    descripcion: Option<String>,
}

#[derive(serde::Serialize)]
pub struct Permiso {
    id: i32,
    clave: String,
    descripcion: Option<String>,
}

#[tauri::command]
pub fn obtener_roles(db: DbConn) -> Result<Vec<Rol>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    // Corregimos la query para usar la tabla roles
    let mut stmt = conn.prepare("SELECT id, nombre, descripcion FROM roles").map_err(|e| e.to_string())?;
    
    let roles = stmt.query_map([], |row| {
        Ok(Rol {
            id: row.get(0)?,
            nombre: row.get(1)?,
            descripcion: row.get(2)?,
        })
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

    Ok(roles)
}

#[tauri::command]
pub fn obtener_permisos_rol(db: DbConn, id_rol: i32) -> Result<Vec<String>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare(
        "SELECT p.clave 
         FROM permisos p
         JOIN rol_permisos rp ON p.id = rp.id_permiso
         WHERE rp.id_rol = ?1"
    ).map_err(|e| e.to_string())?;

    let permisos_iter = stmt.query_map(params![id_rol], |row| {
        row.get::<_, String>(0)
    }).map_err(|e| e.to_string())?;

    let mut permisos = Vec::new();
    for p in permisos_iter {
        permisos.push(p.map_err(|e| e.to_string())?);
    }

    Ok(permisos)
}

#[tauri::command]
pub fn guardar_permisos_rol(db: DbConn, id_rol: i32, permisos: Vec<String>, session: State<SessionState>) -> Result<(), String> {
    if permisos.iter().any(|p| p == "reabrir_caja") && !permisos.iter().any(|p| p == "cerrar_caja") {
        return Err("Para reabrir caja también se necesita acceso al corte".into());
    }
    if id_rol == 1 && (!permisos.iter().any(|p| p == "gestionar_usuarios")
        || !permisos.iter().any(|p| p == "restaurar_respaldos")) {
        return Err("El rol Admin debe conservar gestión de usuarios y restauración".into());
    }
    let actor = session.current_user()?.username;
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let role_exists: i64 = tx.query_row("SELECT COUNT(*) FROM roles WHERE id = ?1", params![id_rol], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if role_exists != 1 {
        return Err("El rol no existe".into());
    }

    let mut validated = Vec::with_capacity(permisos.len());
    for clave in &permisos {
        let id: i32 = tx.query_row("SELECT id FROM permisos WHERE clave = ?1", params![clave], |row| row.get(0))
            .map_err(|_| format!("Permiso desconocido: {clave}"))?;
        if !validated.contains(&id) {
            validated.push(id);
        }
    }

    // Limpiar permisos actuales
    tx.execute("DELETE FROM rol_permisos WHERE id_rol = ?1", params![id_rol]).map_err(|e| e.to_string())?;

    for id in validated {
        tx.execute("INSERT INTO rol_permisos (id_rol, id_permiso) VALUES (?1, ?2)", params![id_rol, id])
            .map_err(|e| e.to_string())?;
    }

    let details = format!("rol_id={id_rol}; permisos={}", permisos.join(","));
    auditoria::registrar_evento_interno(&tx, &actor, "CAMBIAR_PERMISOS", Some(&details))
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
