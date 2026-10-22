# Guía de operación

La aplicación trabaja con una base SQLite local por instalación; no sincroniza equipos ni envía ventas, clientes o proveedores a un servicio en la nube. Si guardas un respaldo o exportación en una carpeta sincronizada por otro programa, esa copia queda bajo las reglas de ese programa.

## Primera apertura y usuarios

En la primera apertura se elige base local cifrada (opción propuesta) o sin cifrar. Si se elige cifrado, se establece una contraseña maestra adicional de al menos 12 caracteres antes de crear `admin`. Después se crea `admin` con una contraseña elegida por el responsable (mínimo 12 caracteres y sin incluir el nombre de usuario). Desde **Configuración → Usuarios**, una cuenta con `gestionar_usuarios` puede crear, desactivar, reactivar, restablecer la contraseña o eliminar cuentas `admin` y `vendedor`. Las cuentas nuevas y las contraseñas restablecidas exigen cambio en el siguiente inicio de sesión. Cada usuario puede cambiar su propia contraseña desde el menú lateral, proporcionando la actual. Las contraseñas se guardan como hashes Argon2. No se puede desactivar ni eliminar el último administrador activo.

Los roles iniciales son **Admin** (todos los permisos) y **Vendedor** (panel, ventas, cotizaciones, devoluciones, cortes y clientes). Desde **Roles y Permisos** se pueden añadir o quitar accesos a inventario, reportes y cada una de esas operaciones, además de empresa, respaldos, restauración y bitácora. Admin debe conservar `gestionar_usuarios` y `restaurar_respaldos`. Los permisos retirados o las cuentas desactivadas dejan de autorizar comandos de inmediato; la interfaz actualiza sus menús al iniciar sesión de nuevo. No hay creación de nuevos roles: se editan los permisos de Admin y Vendedor.

| Permiso | Acceso principal |
| --- | --- |
| `ver_dashboard` | Panel y gráficos |
| `gestion_inventario` | Productos, kits y proveedores |
| `realizar_venta` | Cobrar ventas |
| `gestionar_cotizaciones` | Guardar y consultar cotizaciones |
| `gestionar_devoluciones` | Buscar ventas y procesar devoluciones |
| `cerrar_caja` | Consultar y guardar cortes |
| `reabrir_caja` | Reabrir un corte; requiere también `cerrar_caja` |
| `gestionar_clientes` | Crear, modificar y eliminar clientes; consultar historial |
| `ver_reportes` | Reportes de ventas, devoluciones e inventario |
| `gestionar_usuarios` | Cuentas y permisos de roles |
| `editar_empresa` | Cambiar nombre, contacto y logo del negocio |
| `gestionar_respaldos` | Crear y configurar respaldos locales y la retención de bitácora |
| `restaurar_respaldos` | Restaurar una base desde un respaldo |
| `ver_bitacora` | Consultar eventos registrados |

## Ventas y datos del negocio

Inventario permite crear/editar productos y kits, ajustar stock e importar productos desde **CSV o XLSX**. Cada producto conserva un SKU y puede tener un código de barras distinto. El código se puede introducir manualmente, importar mediante la columna `CodigoBarras` o generar para artículos propios con **Generar propio**. El formulario muestra su representación Code 128 y, después de guardar el producto, permite exportarla como SVG; la aplicación no imprime etiquetas todavía. Si un producto ya tiene código del fabricante, registra ese valor y conserva su SKU interno. En XLSX, configura la columna de códigos como texto para no perder ceros iniciales. El importador muestra una vista previa; no interpreta PDF. En **Clientes** y **Proveedores** también se puede importar CSV/XLSX con vista previa. Clientes acepta la columna `Nombre` (o `Cliente`) y opcionalmente `RFC`, `Telefono` y `Email`; proveedores acepta `Empresa` (o `NombreEmpresa`) y opcionalmente `Contacto`, `Telefono` y `Email`. La primera fila debe contener esos encabezados. Se aceptan hasta 5000 filas y 5 MB por archivo; se omiten filas inválidas y duplicadas, y se muestra el resultado por fila. La importación agrega registros nuevos y no reemplaza los existentes. Para clientes se comparan los identificadores disponibles: RFC, correo y nombre con teléfono; cuando no hay ninguno se compara el nombre. Para proveedores se compara el nombre de empresa.

El vendedor, con los permisos iniciales, puede consultar el panel, vender en efectivo, tarjeta o pago mixto, registrar clientes, hacer cotizaciones y devoluciones y cerrar caja. Cada una de esas acciones se puede retirar por separado. Reabrir un corte tiene un permiso adicional. La opción **Tarjeta** solo registra el importe que el operador indique: no envía el cobro a una terminal ni confirma su resultado. Se cobra en la terminal por separado y luego se registra en la aplicación. El corte compara los importes guardados y los contados.

**Configuración → Datos Empresa** guarda nombre, dirección, teléfono y logo PNG/JPEG (hasta 1 MB) en SQLite. El nombre aparece en la cabecera de la aplicación; los datos y el logo aparecen en el ticket de venta. Cualquier usuario con sesión puede leer esos datos para mostrar el ticket, aunque solo quien tenga `editar_empresa` puede modificarlos. Antes de entregar tickets reales, cambia el nombre inicial «Mi Ferretería» por el del negocio. La dirección y el teléfono son opcionales y se omiten del ticket mientras estén vacíos.

## Archivos, impresoras y periféricos

- Tickets de venta, cortes, cotizaciones y reportes se imprimen mediante el diálogo del sistema. Revisa la vista previa y la configuración de papel de tu impresora; la aplicación no controla directamente impresoras térmicas.
- Las cotizaciones guardadas muestran artículos, precios, cliente y vigencia; pueden imprimirse y eliminarse. No reservan inventario. Para cobrar sus artículos se realiza una venta en el punto de venta con los precios vigentes.
- En el punto de venta, un lector que funciona como teclado puede enviar el SKU o código de barras seguido de Enter. También se puede escribir en el campo **Escanear o escribir SKU/código** o buscar el producto por nombre. No hay conexión directa a lectores USB/serial.
- **Lector con Android por USB:** instala ADB (`android-tools` en Fedora), activa la depuración USB, conecta el cable y autoriza el equipo; pulsa **Activar por USB** y abre en un navegador Android compatible `http://localhost:33737`. En el primer uso, registra el móvil con el código de ocho dígitos mostrado en el punto de venta. El registro se conserva entre sesiones mientras el navegador mantenga los datos del sitio; puedes revocarlo desde el punto de venta. Concede permiso a la cámara y apunta al código. Se necesita un navegador con `BarcodeDetector`, como Chrome o Brave en Android; los formatos disponibles dependen de su versión. Cada lectura envía un identificador y detiene la cámara para evitar duplicados; pulsa **Activar cámara** para leer otro producto. Se transmite solo el texto leído, sin imagen ni nube. Puedes detener el lector desde el punto de venta. La conexión inalámbrica queda para una futura app acompañante; esta versión no solicita instalar certificados en Android.
- Los reportes se exportan como XLSX. El archivo se guarda mediante un comando autorizado por `ver_reportes`; la aplicación solicita un nombre nuevo y no sobrescribe archivos existentes.

## Datos, bitácora y respaldos

En Fedora, la base se crea en el directorio de datos de la aplicación, normalmente `~/.local/share/io.github.jorgeburgos0.ferreteria/ferreteria.db`. La aplicación restringe esa carpeta al usuario del sistema (`0700`) y el archivo de la base (`0600`), también al abrir una instalación existente. Si se elige cifrado, la base usa SQLCipher y pide la contraseña maestra al abrir la aplicación. Sin cifrado, el contenido sigue legible para quien tenga acceso a esa cuenta o al disco sin cifrar. Una cuenta con `gestionar_usuarios` puede activar el cifrado más adelante desde **Configuración → Cifrado**. **Si se pierde la contraseña maestra, no hay forma de recuperar la base ni los respaldos cifrados.** Guárdala separada del equipo. El cifrado no se puede desactivar desde la interfaz.

**Configuración → Respaldos** permite guardar una copia SQLite manual donde el operador elija con `gestionar_respaldos` y restaurarla con `restaurar_respaldos`. Antes de restaurar se confirma el archivo elegido; al terminar se muestra el resultado y se vuelve al inicio de sesión. La restauración valida el archivo, conserva una copia previa en el directorio de datos y registra la acción. En modo cifrado, la copia usa la misma contraseña maestra y la restauración acepta respaldos cifrados con esa clave; en modo sin cifrar, acepta respaldos sin cifrar.

Con `gestionar_respaldos` se puede activar el **respaldo automático local**, elegir una carpeta existente y conservar entre 1 y 30 copias. Se crea una copia al activar la opción y después, como máximo, una por día mientras la aplicación esté abierta. La aplicación comprueba la integridad antes de conservar la copia y rota solo los archivos automáticos que creó con el prefijo `ferreteria-auto-`; los respaldos manuales no entran en esa rotación. La pantalla muestra la fecha de la última copia y el último error. Si la carpeta deja de estar disponible, el respaldo falla y se reintentará durante la siguiente comprobación; conviene revisar periódicamente la carpeta. Los respaldos nuevos siguen el modo de cifrado de la base. Los respaldos anteriores y los archivos XLSX siguen sin cifrar; protege o retira esas copias. Tras restaurar una base creada en otro equipo, revisa la ruta de la carpeta de respaldos.

La bitácora guarda en SQLite acciones de productos (alta, edición, baja, stock e importación), importación de clientes/proveedores, cotizaciones, respaldos, restauraciones, ventas, devoluciones, cortes y reaperturas, inicios de sesión correctos/fallidos, cambios de contraseñas, usuarios/permisos, registro y revocación de lectores móviles y datos de empresa. Cada registro tiene actor y fecha; la pantalla consulta hasta 1000 eventos del rango con `ver_bitacora`. La conservación es indefinida por defecto. Se puede elegir 90, 180, 365 o 730 días desde la configuración de respaldos, pero solo si el respaldo automático está activo. La depuración ocurre después de crear una copia verificada que conserva los registros antiguos. No existe un botón para borrar la bitácora.
