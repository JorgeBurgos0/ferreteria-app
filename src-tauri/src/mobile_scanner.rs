use rand_core::{OsRng, RngCore};
use serde::Serialize;
use std::{
    collections::VecDeque,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::Command,
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::State;

const PAGE: &str = include_str!("mobile_scanner.html");
const SCRIPT: &str = include_str!("mobile_scanner.js");

#[derive(Clone, Serialize)]
pub struct ScannerInfo {
    port: u16,
    code: String,
}

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

fn write_response(stream: &mut TcpStream, code: &str, content_type: &str, body: &str) {
    let header = format!(
        "HTTP/1.1 {code}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; media-src 'self' blob:\r\n\r\n",
        body.len(),
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}

fn handle_connection(
    mut stream: TcpStream,
    port: u16,
    code: &str,
    token: &str,
    queue: &Mutex<VecDeque<String>>,
    failures: &Mutex<(u8, Instant)>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let mut bytes = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let header_end = loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(size) => bytes.extend_from_slice(&chunk[..size]),
        }
        if let Some(index) = bytes.windows(4).position(|slice| slice == b"\r\n\r\n") {
            break index + 4;
        }
        if bytes.len() > 4096 {
            write_response(&mut stream, "413 Payload Too Large", "text/plain", "Solicitud demasiado grande");
            return;
        }
    };
    let Ok(header) = std::str::from_utf8(&bytes[..header_end]).map(str::to_owned) else { return; };
    let mut lines = header.split("\r\n");
    let Some(request) = lines.next() else { return; };
    let mut parts = request.split_whitespace();
    let (Some(method), Some(path), Some(version)) = (parts.next(), parts.next(), parts.next()) else { return; };
    if parts.next().is_some() || version != "HTTP/1.1" {
        write_response(&mut stream, "400 Bad Request", "text/plain", "Solicitud inválida");
        return;
    }
    let mut length = None;
    let mut authorization = "";
    let mut origin = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else { continue; };
        match name.to_ascii_lowercase().as_str() {
            "content-length" => length = value.trim().parse::<usize>().ok(),
            "authorization" => authorization = value.trim(),
            "origin" => origin = Some(value.trim()),
            _ => {}
        }
    }
    if method == "GET" && path == "/" {
        write_response(&mut stream, "200 OK", "text/html; charset=utf-8", PAGE);
        return;
    }
    if method == "GET" && path == "/scanner.js" {
        write_response(&mut stream, "200 OK", "text/javascript; charset=utf-8", SCRIPT);
        return;
    }
    if method != "POST" || !matches!(path, "/pair" | "/scan") {
        write_response(&mut stream, "404 Not Found", "text/plain", "No encontrado");
        return;
    }
    let expected_origin = format!("http://localhost:{port}");
    let alternate_origin = format!("http://127.0.0.1:{port}");
    if !matches!(origin, Some(value) if value == expected_origin || value == alternate_origin) {
        write_response(&mut stream, "403 Forbidden", "text/plain", "Origen no autorizado");
        return;
    }
    let Some(size) = length.filter(|size| *size <= 256) else {
        write_response(&mut stream, "413 Payload Too Large", "text/plain", "Datos inválidos");
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
        write_response(&mut stream, "400 Bad Request", "text/plain", "Texto inválido");
        return;
    };
    if path == "/pair" {
        let Ok(mut guard) = failures.lock() else { return; };
        if guard.1.elapsed() >= Duration::from_secs(60) { *guard = (0, Instant::now()); }
        if guard.0 >= 5 {
            write_response(&mut stream, "429 Too Many Requests", "text/plain", "Espera un minuto");
        } else if body.trim() == code {
            *guard = (0, Instant::now());
            write_response(&mut stream, "200 OK", "text/plain", token);
        } else {
            guard.0 += 1;
            write_response(&mut stream, "403 Forbidden", "text/plain", "Código incorrecto");
        }
        return;
    }
    if authorization != format!("Bearer {token}") {
        write_response(&mut stream, "403 Forbidden", "text/plain", "Sin autorización");
        return;
    }
    let sku = body.trim();
    if sku.is_empty() || sku.len() > 128 || sku.chars().any(char::is_control) {
        write_response(&mut stream, "400 Bad Request", "text/plain", "SKU inválido");
        return;
    }
    let Ok(mut pending) = queue.lock() else { return; };
    if pending.len() == 128 {
        write_response(&mut stream, "429 Too Many Requests", "text/plain", "Lector ocupado");
        return;
    }
    pending.push_back(sku.to_owned());
    write_response(&mut stream, "200 OK", "text/plain", "Recibido");
}

impl MobileScannerState {
    pub fn start(&self) -> Result<ScannerInfo, String> {
        let mut guard = self.0.lock().map_err(|e| e.to_string())?;
        if let Some(server) = guard.as_ref() { return Ok(server.info.clone()); }
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let mut number = [0u8; 4];
        OsRng.fill_bytes(&mut number);
        let info = ScannerInfo { port, code: format!("{:08}", u32::from_le_bytes(number) % 100_000_000) };
        let token = random_hex(32);
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_queue = Arc::clone(&queue);
        let worker_stop = Arc::clone(&stop);
        let worker_code = info.code.clone();
        let worker = thread::spawn(move || {
            let failures = Mutex::new((0, Instant::now()));
            while !worker_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => handle_connection(stream, port, &worker_code, &token, &worker_queue, &failures),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(30)),
                    Err(_) => break,
                }
            }
        });
        *guard = Some(Server { info: info.clone(), queue, stop, worker, usb_connected: false });
        Ok(info)
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
            .map_err(|_| "Falta adb. Instala el paquete android-tools y conecta el móvil con depuración USB".to_string())?;
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

#[tauri::command]
pub fn iniciar_lector_movil(scanner: State<MobileScannerState>) -> Result<ScannerInfo, String> { scanner.start() }

#[tauri::command]
pub fn conectar_android_usb(scanner: State<MobileScannerState>) -> Result<(), String> { scanner.usb() }

#[tauri::command]
pub fn tomar_lectura_movil(scanner: State<MobileScannerState>) -> Result<Option<String>, String> { scanner.next() }

#[tauri::command]
pub fn detener_lector_movil(scanner: State<MobileScannerState>) -> Result<(), String> { scanner.stop() }

#[cfg(test)]
mod tests {
    use super::*;

    fn request(port: u16, method: &str, path: &str, body: &str, auth: Option<&str>, origin: Option<&str>) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let authorization = auth.map(|value| format!("Authorization: Bearer {value}\r\n")).unwrap_or_default();
        let origin = origin.map(|value| format!("Origin: {value}\r\n")).unwrap_or_default();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost:{port}\r\n{origin}{authorization}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len(),
        );
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
        let content = response.split_once("\r\n\r\n").unwrap().1.to_owned();
        (status, content)
    }

    #[test]
    fn pair_scan_and_stop_stay_on_loopback() {
        let scanner = MobileScannerState::default();
        let info = scanner.start().unwrap();
        assert_eq!(scanner.start().unwrap().port, info.port);
        let origin = format!("http://localhost:{}", info.port);
        assert_eq!(request(info.port, "POST", "/scan", "SKU-1", None, Some(&origin)).0, 403);
        assert_eq!(request(info.port, "POST", "/pair", &info.code, None, Some("http://evil.invalid")).0, 403);
        assert_eq!(request(info.port, "POST", "/pair", "000", None, Some(&origin)).0, 403);
        let (status, token) = request(info.port, "POST", "/pair", &info.code, None, Some(&origin));
        assert_eq!(status, 200);
        assert_eq!(token.len(), 64);
        assert_eq!(request(info.port, "POST", "/scan", "SKU-1", Some(&token), Some(&origin)).0, 200);
        assert_eq!(scanner.next().unwrap().as_deref(), Some("SKU-1"));
        assert_eq!(scanner.next().unwrap(), None);
        scanner.stop().unwrap();
        assert!(TcpStream::connect(("127.0.0.1", info.port)).is_err());
    }
}
