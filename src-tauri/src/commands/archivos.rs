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

#[tauri::command]
pub fn guardar_codigo_barras_svg(ruta: String, codigo: String, anchos: Vec<u8>) -> Result<(), String> {
    let path = Path::new(&ruta);
    if extension(path) != "svg" || codigo.is_empty() || codigo.len() > 128
        || !codigo.bytes().all(|byte| (32..=126).contains(&byte))
        || anchos.len() != (codigo.len() + 2) * 6 + 7
        || anchos.iter().any(|width| !(1..=4).contains(width))
        || anchos.chunks_exact(6).take(codigo.len() + 2).any(|pattern| pattern.iter().map(|value| *value as u16).sum::<u16>() != 11)
        || anchos[anchos.len() - 7..].iter().map(|value| *value as u16).sum::<u16>() != 13 {
        return Err("La etiqueta SVG no es válida".into());
    }
    let total_width: u32 = anchos.iter().map(|width| *width as u32).sum::<u32>() + 20;
    let mut contenido = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {total_width} 90\" width=\"{}\" height=\"180\"><rect width=\"100%\" height=\"100%\" fill=\"#fff\"/>", total_width * 2);
    let mut x = 10u32;
    for (index, width) in anchos.iter().enumerate() {
        if index % 2 == 0 { contenido.push_str(&format!("<rect x=\"{x}\" y=\"5\" width=\"{width}\" height=\"65\" fill=\"#000\"/>")); }
        x += *width as u32;
    }
    let label = codigo.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;");
    contenido.push_str(&format!("<text x=\"10\" y=\"84\" font-family=\"sans-serif\" font-size=\"12\">{label}</text></svg>"));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| format!("Elige un nombre nuevo para la etiqueta: {e}"))?;
    if let Err(error) = file.write_all(contenido.as_bytes()).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(path);
        return Err(error.to_string());
    }
    Ok(())
}
