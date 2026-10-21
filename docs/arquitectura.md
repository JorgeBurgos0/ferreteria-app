# Arquitectura y datos locales

La interfaz usa React y TypeScript dentro de Tauri 2. Los comandos Rust gestionan SQLite, autenticación y operaciones del negocio. La base se crea en el directorio de datos de la aplicación de cada usuario; no hay servidor de aplicación ni sincronización entre equipos.

## Acceso

La primera apertura permite crear una cuenta administradora. Las contraseñas se almacenan como hashes Argon2. Cada comando propio de Tauri comprueba la sesión y el permiso requerido en `src-tauri/src/auth.rs`; las rutas de la interfaz también ocultan pantallas sin permiso. Los permisos de los plugins de archivos y diálogo se definen en `src-tauri/capabilities/default.json`.

## Persistencia

`src-tauri/src/db.rs` crea y actualiza el esquema. `PRAGMA user_version` identifica su versión y las migraciones se ejecutan dentro de una transacción. Ventas y devoluciones guardan importes de efectivo y tarjeta por separado; los kits guardan sus componentes al venderse para poder reponerlos en una devolución.

Los respaldos son copias SQLite elegidas por el usuario. La restauración comprueba la integridad, guarda una copia de seguridad previa y cierra la sesión. El respaldo y los archivos exportados no están cifrados; quien administre el equipo debe protegerlos con permisos y almacenamiento adecuados.

## Dispositivos e integraciones

El lector de códigos debe comportarse como teclado y enviar un SKU seguido de Enter. Los tickets usan el diálogo de impresión del sistema. El cobro con tarjeta se realiza fuera de la aplicación; esta registra el importe que declara el operador, sin comunicarse con una terminal ni almacenar números de tarjeta. La aplicación no envía datos del negocio a una nube propia.
