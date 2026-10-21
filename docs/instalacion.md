# Instalación y desarrollo

## Fedora 43

Instala Node.js/npm y Rust/Cargo para tu usuario. Si aún no tienes Rust, el instalador oficial no requiere `sudo`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Las bibliotecas de sistema que necesita Tauri se instalan con privilegios de administrador:

```bash
sudo dnf install webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel libxdo-devel dbus-devel
```

`wget` solo se necesita si se usa para descargar el instalador de Rust; no es requisito para compilar esta aplicación. Consulta también los [requisitos oficiales de Tauri para Linux](https://v2.tauri.app/start/prerequisites/).

## Desarrollo

Desde la raíz del repositorio:

```bash
npm ci
npm run tauri dev
```

La primera ejecución crea SQLite en el directorio de datos de la aplicación. Crea una cuenta administradora con una contraseña propia de al menos 12 caracteres. Para comprobar cambios:

```bash
npm test
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Para generar un paquete de escritorio:

```bash
npm run tauri build
```

Para generar un RPM en Fedora:

```bash
npm run tauri build -- --bundles rpm
```

El resultado se crea en `src-tauri/target/release/bundle/rpm/`. Para instalar un paquete generado en tu equipo, usa `sudo dnf install` seguido de su ruta. Verifica la firma del editor y la suma SHA-256 antes de instalar paquetes descargados.

Prueba los flujos de venta, devolución y respaldo con datos de ensayo antes de usar una base real. Conserva tus respaldos fuera del directorio de trabajo.
