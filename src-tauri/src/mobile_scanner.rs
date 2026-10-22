use crate::{auth::SessionState, db::DbConnection};
use rand_core::{OsRng, RngCore};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::VecDeque, io::{Read, Write}, net::TcpListener, process::Command, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}, thread, time::{Duration, Instant}};
use tauri::{Manager, State};

const PAGE: &str = include_str!("mobile_scanner.html");
const SCRIPT: &str = include_str!("mobile_scanner.js");
const USB_PORT: u16 = 33737;

#[derive(Clone, Serialize)]
pub struct ScannerInfo {
    port: u16,
    code: String,
    url: String,
}

#[derive(Serialize)]
pub struct ScannerDevice { id: i64, nombre: String, fecha_registro: String }

#[derive(Deserialize)]
struct PairRequest { code: String, name: String }

struct Server {
    info: ScannerInfo,
    queue: Arc<Mutex<VecDeque<String>>>,
    stop: Arc<AtomicBool>,
    worker: thread::JoinHandle<()>,
    usb_connected: bool,
}

#[derive(Default)]
pub struct MobileScannerState(Mutex<Option<Server>>);

fn random_hex(bytes: usize) -> String {
    let mut value = vec![0u8; bytes];
    OsRng.fill_bytes(&mut value);
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn token_hash(token: &str) -> String { format!("{:x}", Sha256::digest(token.as_bytes())) }

fn write_response<W: Write>(stream: &mut W, code: &str, content_type: &str, body: &str) {
    let header = format!(
        "HTTP/1.1 {code}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; media-src 'self' blob:\r\n\r\n",
        body.len(),
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

fn authorized(conn: &Connection, user_id: i32, authorization: &str) -> bool {
    if !crate::auth::user_still_allowed(conn, user_id, Some("realizar_venta")) { return false; }
    let Some(token) = authorization.strip_prefix("Bearer ") else { return false; };
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) { return false; }
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM lector_dispositivos WHERE id_usuario = ?1 AND token_hash = ?2)",
        params![user_id, token_hash(token)], |row| row.get::<_, bool>(0),
    ).unwrap_or(false)
}

fn handle_connection<S: Read + Write>(
    stream: &mut S, origin_expected: &str, code: &str,
    queue: &Mutex<VecDeque<String>>, failures: &Mutex<(u8, Instant)>,
    db: &Mutex<Connection>, user_id: i32,
) {
    let mut bytes = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let header_end = loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(size) => bytes.extend_from_slice(&chunk[..size]),
        }
        if let Some(index) = bytes.windows(4).position(|slice| slice == b"\r\n\r\n") { break index + 4; }
        if bytes.len() > 4096 {
            write_response(stream, "413 Payload Too Large", "text/plain", "Solicitud demasiado grande");
            return;
        }
    };
    let Ok(header) = std::str::from_utf8(&bytes[..header_end]).map(str::to_owned) else { return; };
    let mut lines = header.split("\r\n");
    let Some(request) = lines.next() else { return; };
    let mut parts = request.split_whitespace();
    let (Some(method), Some(path), Some(version)) = (parts.next(), parts.next(), parts.next()) else { return; };
    if parts.next().is_some() || version != "HTTP/1.1" {
        write_response(stream, "400 Bad Request", "text/plain", "Solicitud inválida");
        return;
    }
    let mut length = None;
    let mut authorization = "";
    let mut origin = None;
    let mut host = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else { continue; };
        match name.to_ascii_lowercase().as_str() {
            "content-length" => length = value.trim().parse::<usize>().ok(),
            "authorization" => authorization = value.trim(),
            "origin" => origin = Some(value.trim()),
            "host" => host = Some(value.trim()),
            _ => {}
        }
    }
    let expected_host = origin_expected.split_once("://").map(|(_, host)| host).unwrap_or("");
    let alternate_host = expected_host.replace("localhost", "127.0.0.1");
    if host != Some(expected_host) && host != Some(alternate_host.as_str()) {
        write_response(stream, "403 Forbidden", "text/plain", "Equipo no autorizado");
        return;
    }
    if method == "GET" && path == "/" {
        write_response(stream, "200 OK", "text/html; charset=utf-8", PAGE);
        return;
    }
    if method == "GET" && path == "/scanner.js" {
        write_response(stream, "200 OK", "text/javascript; charset=utf-8", SCRIPT);
        return;
    }
    if method != "POST" || !matches!(path, "/pair" | "/resume" | "/scan") {
        write_response(stream, "404 Not Found", "text/plain", "No encontrado");
        return;
    }
    let alternate_origin = origin_expected.replace("localhost", "127.0.0.1");
    if !matches!(origin, Some(value) if value == origin_expected || value == alternate_origin) {
        write_response(stream, "403 Forbidden", "text/plain", "Origen no autorizado");
        return;
    }
    let Some(size) = length.or_else(|| (path == "/resume").then_some(0)).filter(|size| *size <= 256) else {
        write_response(stream, "413 Payload Too Large", "text/plain", "Datos inválidos");
        return;
    };
    while bytes.len() - header_end < size {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(count) => bytes.extend_from_slice(&chunk[..count]),
        }
        if bytes.len() - header_end > 256 { return; }
    }
    let Ok(body) = std::str::from_utf8(&bytes[header_end..header_end + size]) else {
        write_response(stream, "400 Bad Request", "text/plain", "Texto inválido");
        return;
    };
    if path == "/pair" {
        let Ok(mut guard) = failures.lock() else { return; };
        if guard.1.elapsed() >= Duration::from_secs(60) { *guard = (0, Instant::now()); }
        if guard.0 >= 5 {
            write_response(stream, "429 Too Many Requests", "text/plain", "Espera un minuto");
            return;
        }
        let request: PairRequest = match serde_json::from_str(body) {
            Ok(value) => value,
            Err(_) => { write_response(stream, "400 Bad Request", "text/plain", "Datos inválidos"); return; }
        };
        if request.code != code {
            guard.0 += 1;
            write_response(stream, "403 Forbidden", "text/plain", "Código incorrecto");
            return;
        }
        let name = request.name.trim();
        if name.is_empty() || name.len() > 48 || name.chars().any(char::is_control) {
            write_response(stream, "400 Bad Request", "text/plain", "Nombre de móvil inválido");
            return;
        }
        let token = random_hex(32);
        let Ok(mut conn) = db.lock() else { return; };
        if !crate::auth::user_still_allowed(&conn, user_id, Some("realizar_venta")) {
            write_response(stream, "403 Forbidden", "text/plain", "Usuario sin permiso de venta");
            return;
        }
        let registered = (|| -> Result<(), rusqlite::Error> {
            let username: String = conn.query_row("SELECT username FROM usuarios WHERE id = ?1", [user_id], |row| row.get(0))?;
            let tx = conn.transaction()?;
            tx.execute("INSERT INTO lector_dispositivos (id_usuario, nombre, token_hash, fecha_registro) VALUES (?1, ?2, ?3, datetime('now'))", params![user_id, name, token_hash(&token)])?;
            crate::commands::auditoria::registrar_evento_interno(&tx, &username, "REGISTRAR_LECTOR_MOVIL", Some(name))?;
            tx.commit()
        })();
        if registered.is_err() {
            write_response(stream, "500 Internal Server Error", "text/plain", "No se pudo registrar el móvil");
            return;
        }
        *guard = (0, Instant::now());
        write_response(stream, "200 OK", "text/plain", &token);
        return;
    }
    let Ok(conn) = db.lock() else { return; };
    if !authorized(&conn, user_id, authorization) {
        write_response(stream, "403 Forbidden", "text/plain", "Sin autorización");
        return;
    }
    if path == "/resume" {
        write_response(stream, "200 OK", "text/plain", "Conectado");
        return;
    }
    let codigo = body.trim();
    if codigo.is_empty() || codigo.len() > 128 || codigo.chars().any(char::is_control) {
        write_response(stream, "400 Bad Request", "text/plain", "Código inválido");
        return;
    }
    let Ok(mut pending) = queue.lock() else { return; };
    if pending.len() == 128 {
        write_response(stream, "429 Too Many Requests", "text/plain", "Lector ocupado");
        return;
    }
    pending.push_back(codigo.to_owned());
    write_response(stream, "200 OK", "text/plain", "Recibido");
}

impl MobileScannerState {
    fn run(&self, listener: TcpListener, info: ScannerInfo, db: Connection, user_id: i32) -> Result<ScannerInfo, String> {
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let mut guard = self.0.lock().map_err(|e| e.to_string())?;
        if let Some(server) = guard.as_ref() { return Ok(server.info.clone()); }
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_queue = Arc::clone(&queue);
        let worker_stop = Arc::clone(&stop);
        let worker_code = info.code.clone();
        let worker_url = info.url.clone();
        let worker = thread::spawn(move || {
            let failures = Mutex::new((0, Instant::now()));
            let db = Mutex::new(db);
            while !worker_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                        let mut stream = stream;
                        handle_connection(&mut stream, &worker_url, &worker_code, &worker_queue, &failures, &db, user_id);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(30)),
                    Err(_) => break,
                }
            }
        });
        *guard = Some(Server { info: info.clone(), queue, stop, worker, usb_connected: false });
        Ok(info)
    }

    fn start_usb(&self, db: Connection, user_id: i32) -> Result<ScannerInfo, String> {
        if let Some(server) = self.0.lock().map_err(|e| e.to_string())?.as_ref() { return Ok(server.info.clone()); }
        let listener = TcpListener::bind(("127.0.0.1", USB_PORT))
            .map_err(|e| format!("No se pudo abrir el puerto local {USB_PORT}: {e}"))?;
        let port = USB_PORT;
        let mut number = [0u8; 4]; OsRng.fill_bytes(&mut number);
        let info = ScannerInfo { port, code: format!("{:08}", u32::from_le_bytes(number) % 100_000_000),
            url: format!("http://localhost:{port}") };
        self.run(listener, info, db, user_id)
    }

    pub fn next(&self) -> Result<Option<String>, String> {
        let guard = self.0.lock().map_err(|e| e.to_string())?;
        let Some(server) = guard.as_ref() else { return Ok(None); };
        let mut queue = server.queue.lock().map_err(|e| e.to_string())?;
        Ok(queue.pop_front())
    }

    pub fn usb(&self) -> Result<(), String> {
        let mut guard = self.0.lock().map_err(|e| e.to_string())?;
        let server = guard.as_mut().ok_or("Activa primero el lector móvil")?;
        let port = format!("tcp:{}", server.info.port);
        let output = Command::new("adb").args(["reverse", &port, &port]).output()
            .map_err(|_| "Falta adb. Instala android-tools y conecta el móvil con depuración USB".to_string())?;
        if output.status.success() { server.usb_connected = true; Ok(()) }
        else { Err(format!("ADB: {}", String::from_utf8_lossy(&output.stderr).trim())) }
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut guard = self.0.lock().map_err(|e| e.to_string())?;
        if let Some(server) = guard.take() {
            server.stop.store(true, Ordering::Relaxed);
            let _ = server.worker.join();
            if server.usb_connected {
                let port = format!("tcp:{}", server.info.port);
                let _ = Command::new("adb").args(["reverse", "--remove", &port]).output();
            }
        }
        Ok(())
    }
}

fn scanner_db(app: &tauri::AppHandle) -> Result<Connection, String> {
    let path = app.path().app_data_dir().map_err(|e| e.to_string())?.join("ferreteria.db");
    let key = app.state::<crate::db_crypto::CryptoState>().key()?;
    let conn = if let Some(password) = key {
        crate::db_crypto::open_encrypted(&path, &password, false)?
    } else {
        Connection::open(path).map_err(|e| e.to_string())?
    };
    conn.execute_batch("PRAGMA foreign_keys = ON").map_err(|e| e.to_string())?;
    conn.busy_timeout(Duration::from_secs(2)).map_err(|e| e.to_string())?;
    Ok(conn)
}

#[tauri::command]
pub fn iniciar_lector_movil(scanner: State<MobileScannerState>, session: State<SessionState>, app: tauri::AppHandle) -> Result<ScannerInfo, String> {
    scanner.start_usb(scanner_db(&app)?, session.current_user()?.user_id)
}

#[tauri::command]
pub fn conectar_android_usb(scanner: State<MobileScannerState>) -> Result<(), String> { scanner.usb() }

#[tauri::command]
pub fn tomar_lectura_movil(scanner: State<MobileScannerState>) -> Result<Option<String>, String> { scanner.next() }

#[tauri::command]
pub fn detener_lector_movil(scanner: State<MobileScannerState>) -> Result<(), String> { scanner.stop() }

#[tauri::command]
pub fn listar_dispositivos_lector(db: State<DbConnection>, session: State<SessionState>) -> Result<Vec<ScannerDevice>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let user_id = session.current_user()?.user_id;
    let mut stmt = conn.prepare("SELECT id, nombre, fecha_registro FROM lector_dispositivos WHERE id_usuario = ?1 ORDER BY id DESC").map_err(|e| e.to_string())?;
    let devices = stmt.query_map([user_id], |row| Ok(ScannerDevice { id: row.get(0)?, nombre: row.get(1)?, fecha_registro: row.get(2)? }))
        .map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(devices)
}

#[tauri::command]
pub fn revocar_dispositivo_lector(id: i64, db: State<DbConnection>, session: State<SessionState>) -> Result<(), String> {
    let user = session.current_user()?;
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let changed = tx.execute("DELETE FROM lector_dispositivos WHERE id = ?1 AND id_usuario = ?2", params![id, user.user_id]).map_err(|e| e.to_string())?;
    if changed > 0 {
        crate::commands::auditoria::registrar_evento_interno(&tx, &user.username, "REVOCAR_LECTOR_MOVIL", Some(&format!("Dispositivo {id}"))).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, net::TcpStream};

    fn request(port: u16, method: &str, path: &str, body: &str, auth: Option<&str>, origin: Option<&str>) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let authorization = auth.map(|value| format!("Authorization: Bearer {value}\r\n")).unwrap_or_default();
        let origin = origin.map(|value| format!("Origin: {value}\r\n")).unwrap_or_default();
        let request = format!("{method} {path} HTTP/1.1\r\nHost: localhost:{port}\r\n{origin}{authorization}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new(); stream.read_to_string(&mut response).unwrap();
        let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
        let content = response.split_once("\r\n\r\n").unwrap().1.to_owned();
        (status, content)
    }

    #[test]
    fn pair_scan_resume_and_revoke_stay_on_loopback() {
        let dir = std::env::temp_dir().join(format!("ferreteria-lector-test-{}", random_hex(8)));
        fs::create_dir(&dir).unwrap();
        let db_path = dir.join("test.db");
        let db = Connection::open(&db_path).unwrap();
        crate::db::inicializar(&db).unwrap();
        db.execute("INSERT INTO usuarios (id, username, password_hash, rol) VALUES (1, 'test', 'hash', 'Admin')", []).unwrap();
        let scanner = MobileScannerState::default();
        let info = scanner.start_usb(db, 1).unwrap();
        let origin = info.url.clone();
        assert_eq!(request(info.port, "POST", "/scan", "SKU-1", None, Some(&origin)).0, 403);
        let pair = format!("{{\"code\":\"{}\",\"name\":\"Android\"}}", info.code);
        assert_eq!(request(info.port, "POST", "/pair", &pair, None, Some("http://evil.invalid")).0, 403);
        let wrong = if info.code == "00000000" { "00000001" } else { "00000000" };
        assert_eq!(request(info.port, "POST", "/pair", &format!("{{\"code\":\"{wrong}\",\"name\":\"Android\"}}"), None, Some(&origin)).0, 403);
        let (status, token) = request(info.port, "POST", "/pair", &pair, None, Some(&origin));
        assert_eq!(status, 200);
        assert_eq!(token.len(), 64);
        assert_eq!(request(info.port, "POST", "/resume", "", Some(&token), Some(&origin)).0, 200);
        assert_eq!(request(info.port, "POST", "/scan", "SKU-1", Some(&token), Some(&origin)).0, 200);
        assert_eq!(scanner.next().unwrap().as_deref(), Some("SKU-1"));
        assert_eq!(scanner.next().unwrap(), None);
        scanner.stop().unwrap();
        assert!(TcpStream::connect(("127.0.0.1", info.port)).is_err());
        let scanner = MobileScannerState::default();
        let resumed = scanner.start_usb(Connection::open(&db_path).unwrap(), 1).unwrap();
        assert_eq!(resumed.port, info.port);
        assert_eq!(request(info.port, "POST", "/resume", "", Some(&token), Some(&origin)).0, 200);
        let control = Connection::open(&db_path).unwrap();
        control.execute("DELETE FROM rol_permisos WHERE id_rol = 1 AND id_permiso = (SELECT id FROM permisos WHERE clave = 'realizar_venta')", []).unwrap();
        assert_eq!(request(info.port, "POST", "/resume", "", Some(&token), Some(&origin)).0, 403);
        control.execute("INSERT INTO rol_permisos (id_rol, id_permiso) SELECT 1, id FROM permisos WHERE clave = 'realizar_venta'", []).unwrap();
        control.execute("DELETE FROM lector_dispositivos", []).unwrap();
        assert_eq!(request(info.port, "POST", "/resume", "", Some(&token), Some(&origin)).0, 403);
        scanner.stop().unwrap();
        drop(control);
        fs::remove_dir_all(&dir).unwrap();
    }
}
