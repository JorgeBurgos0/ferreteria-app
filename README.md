# Ferretería App

Aplicación de escritorio para inventario y ventas de una ferretería. Usa Tauri 2, React, TypeScript y SQLite.

Incluye punto de venta, productos y kits, clientes, proveedores, cotizaciones, devoluciones, cortes de caja, reportes, usuarios, roles y respaldos.

## Empezar

Consulta [instalación y desarrollo](docs/instalacion.md) para preparar Fedora y ejecutar la aplicación. Desde la raíz del repositorio, con Node.js, Rust y las bibliotecas de Tauri instaladas:

```bash
npm ci
npm run tauri dev
```

`npm run build` compila la interfaz; `npm test` prueba la lectura y escritura de hojas de cálculo. La base de datos SQLite se crea en el directorio de datos de la aplicación. Al abrirla por primera vez, el responsable crea la cuenta administradora y elige una contraseña de al menos 12 caracteres.

## Documentación

La [documentación](docs/README.md) contiene las guías de instalación y operación, así como la arquitectura y el tratamiento de datos locales. Las actualizaciones del esquema de SQLite se hacen en `src-tauri/src/db.rs` y se registran con `PRAGMA user_version`.

## Licencia

Copyright © 2026 Jorge Burgos. El código de este proyecto se distribuye bajo [GPL-3.0-only](LICENSE).
