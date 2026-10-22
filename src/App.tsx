// src/App.tsx
import { lazy, Suspense, useEffect, useState } from 'react';
import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { LayoutPrincipal } from './components/LayoutPrincipal';
import { Login } from './components/Login';
import { AccesoBase, type CryptoStatus } from './components/AccesoBase';
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
  const [cryptoStatus, setCryptoStatus] = useState<CryptoStatus | null>(null);
  const [cryptoError, setCryptoError] = useState<string | null>(null);

  useEffect(() => {
    invoke<CryptoStatus>('estado_cifrado').then(setCryptoStatus).catch((reason) => setCryptoError(String(reason)));
  }, []);

  if (cryptoError) return <div role="alert" style={{ padding: 24 }}>No se pudo abrir la base de datos: {cryptoError}</div>;
  if (!cryptoStatus) return <div style={{ padding: 24 }}>Abriendo base de datos…</div>;
  if (cryptoStatus.locked) return <AccesoBase status={cryptoStatus} onReady={() => {
    invoke<CryptoStatus>('estado_cifrado').then(setCryptoStatus).catch((reason) => setCryptoError(String(reason)));
  }} />;

  if (!usuario) {
    // Si no hay un usuario en el estado, mostramos la pantalla de Login.
    // Le pasamos la función `setUsuario` para que el componente Login
    // pueda actualizar el estado de App cuando el login sea exitoso.
    return <Login onLoginSuccess={setUsuario} />;
  }

  const has = (permission: string) => usuario.permisos.includes(permission);
  const canConfigure = ['gestionar_usuarios', 'editar_empresa', 'gestionar_respaldos', 'restaurar_respaldos', 'ver_bitacora'].some(has);

  const defaultRoute = has('ver_dashboard') ? '/'
    : has('realizar_venta') || has('gestionar_cotizaciones') ? '/punto-venta'
      : has('gestionar_devoluciones') ? '/devoluciones'
        : has('cerrar_caja') ? '/corte-caja'
          : has('gestionar_clientes') ? '/clientes'
            : has('ver_reportes') ? '/reportes'
              : has('gestion_inventario') ? '/inventario'
                : canConfigure ? '/configuracion'
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
          <Route index element={has('ver_dashboard') ? <Dashboard canSell={has('realizar_venta')} canManageInventory={has('gestion_inventario')} /> : defaultRoute ? <Navigate to={defaultRoute} replace /> : <div>Tu cuenta no tiene permisos asignados.</div>} />
          <Route path="punto-venta" element={has('realizar_venta') || has('gestionar_cotizaciones') ? <PuntoDeVenta userId={usuario.id} canSell={has('realizar_venta')} canQuote={has('gestionar_cotizaciones')} /> : <Navigate to="/" replace />} />
          <Route path="cotizaciones" element={has('gestionar_cotizaciones') ? <Cotizaciones /> : <Navigate to="/" replace />} />
          <Route path="reportes" element={has('ver_reportes') ? <Reportes /> : <Navigate to="/" replace />} />
          <Route path="devoluciones" element={has('gestionar_devoluciones') ? <GestionDevoluciones /> : <Navigate to="/" replace />} />
          <Route path="corte-caja" element={has('cerrar_caja') ? <CorteCaja username={usuario.username} canReopen={has('reabrir_caja')} /> : <Navigate to={defaultRoute ?? '/'} replace />} />
          <Route path="clientes" element={has('gestionar_clientes') ? <GestionClientes /> : <Navigate to="/" replace />} />

          {/* Rutas protegidas */}
          <Route path="inventario" element={has('gestion_inventario') ? <GestionInventario username={usuario.username} /> : <Navigate to="/" replace />} />
          <Route path="proveedores" element={has('gestion_inventario') ? <GestionProveedores /> : <Navigate to="/" replace />} />
          <Route path="configuracion" element={canConfigure ? <Configuracion username={usuario.username} permisos={usuario.permisos} /> : <Navigate to="/" replace />} />
          <Route path="roles" element={has('gestionar_usuarios') ? <GestionRoles /> : <Navigate to="/" replace />} />
        </Route>
      </Routes>
      </Suspense>
    </HashRouter>
  );
}

export default App;
