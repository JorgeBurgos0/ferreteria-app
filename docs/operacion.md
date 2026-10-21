# Guía de operación

La aplicación trabaja con una base SQLite local por instalación; no sincroniza equipos ni envía ventas, clientes o proveedores a un servicio en la nube. Si guardas un respaldo o exportación en una carpeta sincronizada por otro programa, esa copia queda bajo las reglas de ese programa.

## Primera apertura y usuarios

En la primera apertura se crea `admin` con una contraseña elegida por el responsable (mínimo 12 caracteres y sin incluir el nombre de usuario). Después, desde **Configuración → Usuarios**, una cuenta con `configuracion` y `gestionar_usuarios` puede crear o eliminar cuentas `admin` o `vendedor`. No hay edición de usuarios, desactivación temporal ni restablecimiento de contraseñas desde esa pantalla. Las contraseñas se guardan como hashes Argon2.

Los roles iniciales son **Admin** (todos los permisos) y **Vendedor** (`ver_dashboard` y `realizar_venta`). Desde **Roles y Permisos** se pueden añadir o quitar los seis permisos existentes a esos dos roles. Admin debe conservar `gestionar_usuarios` y `configuracion`. Los cambios se cargan al iniciar sesión; pide al usuario que salga y vuelva a entrar. No hay creación de nuevos roles ni permisos separados para devolución, caja o clientes: todos forman parte de `realizar_venta`. Configuración reúne usuarios, datos de empresa, respaldos y bitácora bajo un mismo acceso.

## Ventas y datos del negocio

Inventario permite crear/editar productos y kits, ajustar stock e importar productos desde **CSV o XLSX**. El importador muestra una vista previa; no interpreta PDF. Los clientes y proveedores se crean o editan **uno a uno** en sus pantallas; aún no importan ni exportan hojas de cálculo.

El vendedor puede consultar el panel, vender en efectivo, tarjeta o pago mixto, registrar clientes, hacer cotizaciones y devoluciones y cerrar caja. La opción **Tarjeta** solo registra el importe que el operador indique: no envía el cobro a una terminal ni confirma su resultado. Se cobra en la terminal por separado y luego se registra en la aplicación. El corte compara los importes guardados y los contados.

**Configuración → Datos Empresa** guarda nombre, dirección, teléfono y logo PNG/JPEG (hasta 1 MB) en SQLite. El nombre aparece en la cabecera de la aplicación; los datos y el logo aparecen en el ticket de venta. Cualquier usuario con sesión puede leer esos datos para mostrar el ticket, aunque solo quien tenga `configuracion` puede modificarlos. La dirección y el teléfono se omiten del ticket mientras estén vacíos.

## Archivos, impresoras y periféricos

- El ticket de venta, el corte y ciertos reportes usan el diálogo de impresión del sistema (`window.print()`). No se ha validado una impresora física ni hay control directo de impresoras térmicas.
- La pantalla de cotizaciones permite consultar y eliminar cotizaciones guardadas. La impresión y conversión directa a venta aún no están implementadas.
- En punto de venta se reconoce un lector de códigos de barras que actúe como teclado: escribe un SKU completo y envía Enter rápidamente. No hay conexión específica a USB/serial, lectura de imágenes, generación de etiquetas ni prueba física del lector.
- Los reportes se pueden exportar a XLSX mediante el selector de archivos. La exportación no es PDF.

## Datos, bitácora y respaldos

En Fedora, la base se crea en el directorio de datos de la aplicación, normalmente `~/.local/share/io.github.jorgeburgos0.ferreteria/ferreteria.db`. La aplicación no crea respaldos automáticos: **Configuración → Respaldos** permite guardar una copia SQLite donde el operador elija y restaurarla. La restauración valida el archivo, conserva una copia previa en el directorio de datos y obliga a iniciar sesión otra vez. Los respaldos y archivos XLSX no están cifrados; protégelos como datos del negocio y de clientes.

La bitácora actual guarda en SQLite acciones de productos (alta, edición, baja, stock e importación), creación de respaldos, ventas, devoluciones, cortes, inicios de sesión, cambios de usuarios/permisos y datos de empresa. La pantalla deja consultar por fechas. Todavía no registra restauraciones ni intentos fallidos de acceso. No hay borrado automático ni plazo de conservación: los registros permanecen hasta que se elimine/restaure la base o se añada una política de retención.

El repositorio Git contiene código, documentación, licencia y CSV de **ensayo**. Excluye `artifacts/`, `node_modules/`, compilaciones y bases de datos locales. Nunca publiques `ferreteria.db`, respaldos, exportaciones con datos reales ni la clave GPG privada.
