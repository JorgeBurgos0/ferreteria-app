# Instalación

El entorno probado para la versión 0.1.0 es Fedora x86_64. No hay sincronización entre equipos: cada usuario del sistema operativo conserva sus datos en su propio directorio local.

## Instalar un RPM publicado

Cuando haya un paquete en [GitHub Releases](https://github.com/JorgeBurgos0/ferreteria-app/releases), descarga el RPM firmado, su archivo `.sha256` y la [clave pública](../packaging/RPM-GPG-KEY-ferreteria-app.asc). La huella esperada es `6312 F1D3 0C64 1693 54FF 6302 BEC1 76AD BD45 19E1`. Desde la carpeta de descargas, sustituye el nombre de ejemplo por el del paquete descargado:

```bash
gpg --show-keys --with-fingerprint RPM-GPG-KEY-ferreteria-app.asc
sudo rpmkeys --import RPM-GPG-KEY-ferreteria-app.asc
rpmkeys --checksig ferreteria-app-0.1.0-5.x86_64.rpm
sha256sum -c ferreteria-app-0.1.0-5.x86_64.rpm.sha256
sudo dnf install ./ferreteria-app-0.1.0-5.x86_64.rpm
```

Compara la huella antes de importar la clave; la verificación debe mostrar la firma y los resúmenes como correctos. DNF instalará las bibliotecas necesarias disponibles en Fedora. Una actualización del RPM conserva la base y las cuentas del usuario. Si el mismo número de paquete ya está instalado y necesitas sustituirlo, usa `sudo dnf reinstall` con su ruta.

Para usar Android como lector por USB también se necesita ADB:

```bash
sudo dnf install android-tools
```

Después de instalar, continúa con [primeros pasos](primeros-pasos.md).

## Compilar desde el código fuente

Se necesitan Node.js 22 con npm, Rust/Cargo y las bibliotecas de desarrollo de Tauri. En Fedora:

```bash
sudo dnf install webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel libxdo-devel dbus-devel openssl-devel
```

Consulta los [requisitos de Tauri para Linux](https://v2.tauri.app/start/prerequisites/) para otras distribuciones. Desde la raíz del repositorio:

```bash
npm ci
XDG_DATA_HOME="$PWD/.local/dev-data" npm run tauri dev
```

El directorio `.local/` está excluido de Git y separa la base de desarrollo de la instalación normal. Para comprobar cambios:

```bash
npm test
npm run build
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

Para crear un RPM local:

```bash
npm run tauri build -- --bundles rpm
```

El paquete sin firmar se crea en `src-tauri/target/release/bundle/rpm/`. Los RPM publicados se firman por separado. Las bases, los respaldos y los archivos generados se excluyen del repositorio.
