# Arquitectura y datos locales

La interfaz usa React y TypeScript dentro de Tauri 2. Los comandos Rust gestionan SQLite, autenticación y operaciones del negocio. La base se crea en el directorio de datos de la aplicación de cada usuario; no hay servidor de aplicación ni sincronización entre equipos.

## Acceso

La primera apertura permite crear una cuenta administradora. Las contraseñas se almacenan como hashes Argon2. Cada comando propio de Tauri comprueba la sesión, el estado activo de la cuenta y el permiso actual en SQLite mediante `src-tauri/src/auth.rs`; las rutas de la interfaz también ocultan pantallas sin permiso. El plugin de diálogo selecciona rutas; la lectura de CSV/XLSX y la escritura de reportes se realizan en comandos Rust con permisos y límites de tamaño. Las capacidades de Tauri se definen en `src-tauri/capabilities/default.json`.

## Persistencia

`src-tauri/src/db.rs` crea y actualiza el esquema. `PRAGMA user_version` identifica su versión y las migraciones se ejecutan dentro de una transacción. Ventas y devoluciones guardan importes de efectivo y tarjeta por separado; los kits guardan sus componentes al venderse para poder reponerlos en una devolución.

Los respaldos son copias SQLite locales elegidas por el usuario. El respaldo automático es opcional y se comprueba al iniciar la aplicación y cada hora mientras permanezca abierta; crea como máximo una copia diaria, verifica la integridad y rota solo sus propios archivos. La retención de bitácora se ejecuta después de una copia válida. La restauración comprueba la integridad, guarda una copia de seguridad previa, registra el evento en la base restaurada y cierra la sesión. La importación de clientes y proveedores se analiza en la interfaz y se valida de nuevo en Rust dentro de una transacción. El respaldo y los archivos exportados no están cifrados; quien administre el equipo debe protegerlos con permisos y almacenamiento adecuados.

## Dispositivos e integraciones

El lector de códigos puede comportarse como teclado y enviar un SKU seguido de Enter. El lector móvil opcional sirve una página solo en `127.0.0.1` y usa `adb reverse` para que Chrome en Android la abra como `localhost`; la cámara se procesa en el móvil y solo se envía el SKU. El servicio exige un código de emparejamiento y se cierra al salir del punto de venta o de la sesión. Los tickets usan el diálogo de impresión del sistema. El cobro con tarjeta se realiza fuera de la aplicación; esta registra el importe que declara el operador, sin comunicarse con una terminal ni almacenar números de tarjeta. La aplicación no envía datos del negocio a una nube propia.
