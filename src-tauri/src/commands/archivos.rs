use std::{fs, io::Write, path::Path};

const MAX_IMPORT_BYTES: u64 = 10 * 1024 * 1024;
const MAX_EXPORT_BYTES: usize = 20 * 1024 * 1024;

fn extension(path: &Path) -> String {
    path.extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase()
}

pub fn read_spreadsheet(path: &Path) -> Result<Vec<u8>, String> {
    let kind = extension(path);
    if kind != "csv" && kind != "xlsx" {
        return Err("Selecciona un archivo CSV o XLSX".into());
    }
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_IMPORT_BYTES {
        return Err("El archivo debe ser regular y medir como máximo 10 MB".into());
    }
    let data = fs::read(path).map_err(|e| e.to_string())?;
    if data.len() as u64 > MAX_IMPORT_BYTES {
        return Err("El archivo supera 10 MB".into());
    }
    if kind == "csv" && std::str::from_utf8(&data).is_err() {
        return Err("El CSV debe estar codificado en UTF-8".into());
    }
    if kind == "xlsx" && !data.starts_with(b"PK\x03\x04") {
        return Err("El XLSX no es válido".into());
    }
    Ok(data)
}

pub fn write_spreadsheet(path: &Path, data: &[u8]) -> Result<(), String> {
    if extension(path) != "xlsx" || !data.starts_with(b"PK\x03\x04") || data.len() > MAX_EXPORT_BYTES {
        return Err("El reporte debe ser un XLSX válido de hasta 20 MB".into());
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)
        .map_err(|e| format!("No se pudo crear el archivo; elige un nombre nuevo: {e}"))?;
    let result = (|| -> Result<(), String> {
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
        }
        file.write_all(data).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(path); }
    result
}

#[tauri::command]
pub fn leer_catalogo(ruta: String) -> Result<Vec<u8>, String> { read_spreadsheet(Path::new(&ruta)) }

#[tauri::command]
pub fn leer_clientes_archivo(ruta: String) -> Result<Vec<u8>, String> { read_spreadsheet(Path::new(&ruta)) }

#[tauri::command]
pub fn leer_proveedores_archivo(ruta: String) -> Result<Vec<u8>, String> { read_spreadsheet(Path::new(&ruta)) }

#[tauri::command]
pub fn guardar_reporte_archivo(ruta: String, contenido: Vec<u8>) -> Result<(), String> {
    write_spreadsheet(Path::new(&ruta), &contenido)
}
