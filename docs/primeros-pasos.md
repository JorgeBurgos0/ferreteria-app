# Primeros pasos

Esta guía recorre una instalación nueva. Cada usuario del sistema operativo tiene su propia base local; la aplicación no comparte datos entre equipos.

## 1. Crear la base y la cuenta administradora

1. Abre **Ferretería App** y elige **Cifrar la base** o **Guardar sin cifrar**. El cifrado aparece seleccionado inicialmente. Si lo eliges, crea una contraseña maestra de al menos 12 caracteres y guárdala en un lugar seguro: sin ella no podrás recuperar la base ni sus respaldos cifrados.
2. Pulsa **Crear base local**. Crea la cuenta `admin` con otra contraseña de al menos 12 caracteres.
3. Entra con `admin` y la contraseña de esa cuenta. Si la base está cifrada, en aperturas posteriores se solicita primero la contraseña maestra y después la de usuario.

El RPM puede actualizarse sin borrar esta base. Reinstalar la aplicación tampoco restablece las contraseñas de las cuentas existentes.

## 2. Preparar el negocio y los respaldos

1. En **Configuración → Datos Empresa**, cambia el nombre inicial y añade los datos que quieras mostrar en los tickets. Dirección y teléfono son opcionales.
2. En **Configuración → Respaldos**, crea un respaldo manual con **Crear Respaldo**.
3. Para activar copias automáticas, elige una carpeta existente, marca **Activar respaldos automáticos**, selecciona cuántas copias conservar y pulsa **Guardar configuración**. Comprueba la fecha del último respaldo y cualquier error mostrado. Las copias automáticas se realizan mientras la aplicación está abierta, como máximo una por día.

Guarda otra copia en una unidad distinta del equipo cuando los datos sean importantes. Los archivos exportados y las copias creadas antes de activar el cifrado conservan su estado original.

## 3. Registrar productos y hacer una venta de prueba

1. En **Inventario**, pulsa **Agregar Producto** y escribe SKU, nombre, precio de venta y stock inicial. El código de barras es opcional y puede ser distinto del SKU. Guarda el producto.
2. En **Punto de venta**, busca el producto por nombre o introduce su SKU o código de barras y pulsa Enter. Comprueba la cantidad y el total.
3. Pulsa el botón de venta, selecciona efectivo, tarjeta o pago mixto y confirma. Para tarjeta, cobra primero en la terminal externa: la aplicación solo registra el importe indicado.
4. Revisa el ticket en la vista previa del sistema antes de imprimirlo. La compatibilidad con el papel y la impresora depende de la configuración de impresión del equipo.

También puedes importar productos, clientes y proveedores desde CSV/XLSX. La [guía de operación](operacion.md) describe las columnas y los límites; los archivos de `tests/fixtures/` son exclusivamente datos ficticios de ensayo.

## Recuperación y dudas frecuentes

- **Olvidé la contraseña de `admin`:** otra cuenta con permiso `gestionar_usuarios` puede restablecerla. Reinstalar el RPM no cambia las cuentas guardadas en la base.
- **Olvidé la contraseña maestra:** una base cifrada no se puede abrir ni recuperar sin ella. Un respaldo cifrado con esa misma clave tampoco resuelve su pérdida.
- **Necesito restaurar:** en **Configuración → Respaldos**, selecciona **Restaurar Respaldo**. La aplicación muestra el archivo y pide confirmación, conserva una copia de la base anterior y cierra la sesión tras restaurar.
- **¿Dónde están mis datos?** En Fedora, normalmente en `~/.local/share/io.github.jorgeburgos0.ferreteria/ferreteria.db`. No adjuntes ese archivo a reportes públicos.
