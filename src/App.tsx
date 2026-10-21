// src/App.tsx
import { lazy, Suspense, useState } from 'react';
import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { LayoutPrincipal } from './components/LayoutPrincipal';
import { Login } from './components/Login';
import { clearSessionToken, invoke } from './lib/invoke';

const Cotizaciones = lazy(() => import('./components/Cotizaciones'));
const GestionInventario = lazy(() => import('./components/GestionInventario').then((module) => ({ default: module.GestionInventario })));
const PuntoDeVenta = lazy(() => import('./components/PuntoDeVenta').then((module) => ({ default: module.PuntoDeVenta })));
const Reportes = lazy(() => import('./components/Reportes').then((module) => ({ default: module.Reportes })));
const GestionClientes = lazy(() => import('./components/GestionClientes').then((module) => ({ default: module.GestionClientes })));
const GestionProveedores = lazy(() => import('./components/GestionProveedores').then((module) => ({ default: module.GestionProveedores })));
const Configuracion = lazy(() => import('./components/Configuracion').then((module) => ({ default: module.Configuracion })));
const Dashboard = lazy(() => import('./components/Dashboard').then((module) => ({ default: module.Dashboard })));
const GestionDevoluciones = lazy(() => import('./components/GestionDevoluciones').then((module) => ({ default: module.GestionDevoluciones })));
const CorteCaja = lazy(() => import('./components/CorteCaja').then((module) => ({ default: module.CorteCaja })));
const GestionRoles = lazy(() => import('./components/GestionRoles').then((module) => ({ default: module.GestionRoles })));

interface UsuarioLogueado {
  id: number;
  username: string;
  rol: string;
  permisos: string[];
}

function App() {
  const [usuario, setUsuario] = useState<UsuarioLogueado | null>(null);

  if (!usuario) {
    // Si no hay un usuario en el estado, mostramos la pantalla de Login.
    // Le pasamos la función `setUsuario` para que el componente Login
    // pueda actualizar el estado de App cuando el login sea exitoso.
    return <Login onLoginSuccess={setUsuario} />;
  }

  const defaultRoute = usuario.permisos.includes('ver_dashboard') ? '/'
    : usuario.permisos.includes('realizar_venta') ? '/punto-venta'
      : usuario.permisos.includes('ver_reportes') ? '/reportes'
        : usuario.permisos.includes('gestion_inventario') ? '/inventario'
          : usuario.permisos.includes('configuracion') ? '/configuracion'
            : usuario.permisos.includes('gestionar_usuarios') ? '/roles'
              : null;

  // Si hay un usuario, mostramos la aplicación principal con sus rutas.
  return (
    <HashRouter>
      <Suspense fallback={<div style={{ padding: 24 }}>Cargando…</div>}>
      <Routes>
        <Route path="/" element={<LayoutPrincipal rol={usuario.rol} username={usuario.username} permisos={usuario.permisos} onLogout={() => {
          void invoke('logout').catch(() => undefined).finally(() => {
            clearSessionToken();
            setUsuario(null);
          });
        }} />}>
          <Route index element={usuario.permisos.includes('ver_dashboard') ? <Dashboard canSell={usuario.permisos.includes('realizar_venta')} canManageInventory={usuario.permisos.includes('gestion_inventario')} /> : defaultRoute ? <Navigate to={defaultRoute} replace /> : <div>Tu cuenta no tiene permisos asignados.</div>} />
          <Route path="punto-venta" element={usuario.permisos.includes('realizar_venta') ? <PuntoDeVenta userId={usuario.id} /> : <Navigate to="/" replace />} />
          <Route path="cotizaciones" element={usuario.permisos.includes('realizar_venta') ? <Cotizaciones /> : <Navigate to="/" replace />} />
          <Route path="reportes" element={usuario.permisos.includes('ver_reportes') ? <Reportes /> : <Navigate to="/" replace />} />
          <Route path="devoluciones" element={usuario.permisos.includes('realizar_venta') ? <GestionDevoluciones /> : <Navigate to="/" replace />} />
          <Route path="corte-caja" element={usuario.permisos.includes('realizar_venta') ? <CorteCaja username={usuario.username} rol={usuario.rol} /> : <Navigate to={defaultRoute ?? '/'} replace />} />
          <Route path="clientes" element={usuario.permisos.includes('realizar_venta') ? <GestionClientes /> : <Navigate to="/" replace />} />

          {/* Rutas protegidas */}
          <Route path="inventario" element={usuario.permisos.includes('gestion_inventario') ? <GestionInventario username={usuario.username} /> : <Navigate to="/" replace />} />
          <Route path="proveedores" element={usuario.permisos.includes('gestion_inventario') ? <GestionProveedores /> : <Navigate to="/" replace />} />
          <Route path="configuracion" element={usuario.permisos.includes('configuracion') ? <Configuracion username={usuario.username} /> : <Navigate to="/" replace />} />
          <Route path="roles" element={usuario.permisos.includes('gestionar_usuarios') ? <GestionRoles /> : <Navigate to="/" replace />} />
        </Route>
      </Routes>
      </Suspense>
    </HashRouter>
  );
}

export default App;
