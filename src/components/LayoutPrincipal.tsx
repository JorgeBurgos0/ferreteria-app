// src/components/LayoutPrincipal.tsx
import { AppShell, NavLink, Title, Group, Text, UnstyledButton, Avatar, rem, Dialog, ActionIcon, Button, Modal, PasswordInput, Stack, useMantineColorScheme } from '@mantine/core';
import { Outlet, Link } from 'react-router-dom';
import { IconShoppingCart, IconArchive, IconReportAnalytics, IconUsers, IconSettings, IconHome, IconArrowBackUp, IconLogout, IconCash, IconAlertCircle, IconFileText, IconSun, IconMoon } from '@tabler/icons-react';
import { useEffect, useState } from 'react';
import { invoke } from '../lib/invoke';
import { notifications } from '@mantine/notifications';

interface LayoutPrincipalProps {
  rol: string;
  username: string;
  permisos: string[];
  onLogout: () => void;
}

export function LayoutPrincipal({ rol, username, permisos, onLogout }: LayoutPrincipalProps) {
  // const isAdmin = rol === 'admin'; // Ya no se usa basado en rol estático
  const canViewDashboard = permisos.includes('ver_dashboard');
  const canSell = permisos.includes('realizar_venta');
  const canQuote = permisos.includes('gestionar_cotizaciones');
  const canManageInventory = permisos.includes('gestion_inventario');
  const canViewReports = permisos.includes('ver_reportes');
  const canConfigure = ['gestionar_usuarios', 'editar_empresa', 'gestionar_respaldos', 'restaurar_respaldos', 'ver_bitacora'].some(permission => permisos.includes(permission));
  const canManageUsers = permisos.includes('gestionar_usuarios');
  const [stockAlertOpen, setStockAlertOpen] = useState(false);
  const [lowStockCount, setLowStockCount] = useState(0);
  const [nombreEmpresa, setNombreEmpresa] = useState('Mi Ferretería');
  const [passwordModalOpen, setPasswordModalOpen] = useState(false);
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const { colorScheme, toggleColorScheme } = useMantineColorScheme();

  useEffect(() => {
    const checkStock = async () => {
      try {
        const productos: any[] = await invoke('obtener_productos_con_bajo_stock');
        if (productos.length > 0) {
          setLowStockCount(productos.length);
          setStockAlertOpen(true);
        }
      } catch (e) {
        console.error(e);
      }
    };

    const cargarDatosEmpresa = async () => {
      try {
        const data: any = await invoke('obtener_datos_empresa');
        if (data && data.nombre) {
          setNombreEmpresa(data.nombre);
        }
      } catch (e) {
        console.error("Error al cargar nombre empresa para layout", e);
      }
    };

    checkStock();
    cargarDatosEmpresa();
  }, []);

  const closePasswordModal = () => {
    setPasswordModalOpen(false);
    setCurrentPassword('');
    setNewPassword('');
    setConfirmPassword('');
    setPasswordError(null);
  };

  const changePassword = async () => {
    if (newPassword !== confirmPassword) {
      setPasswordError('Las contraseñas nuevas no coinciden');
      return;
    }
    try {
      await invoke('cambiar_password', { passwordActual: currentPassword, passwordNueva: newPassword });
      closePasswordModal();
      notifications.show({ title: 'Contraseña actualizada', message: 'El cambio se guardó en este equipo.', color: 'green' });
    } catch (error) {
      setPasswordError(String(error));
    }
  };

  return (
    <AppShell
      header={{ height: 60 }}
      navbar={{ width: 250, breakpoint: 'sm' }}
      padding="md"
    >
      <AppShell.Header p="md">
        <Group justify="space-between">
          <Title order={3}>{nombreEmpresa}</Title>
          <ActionIcon onClick={() => toggleColorScheme()} variant="default" size="lg" aria-label="Toggle color scheme">
            {colorScheme === 'dark' ? <IconSun size="1rem" stroke={1.5} /> : <IconMoon size="1rem" stroke={1.5} />}
          </ActionIcon>
        </Group>
      </AppShell.Header>

      <AppShell.Navbar p="md">
        <div style={{ flex: 1 }}>
          {canViewDashboard && (
            <NavLink
              label="Inicio"
              component={Link}
              to="/"
              leftSection={<IconHome size="1rem" />}
            />
          )}
          {(canSell || canQuote) && (
            <NavLink
              label="Punto de Venta"
              component={Link}
              to="/punto-venta"
              leftSection={<IconShoppingCart size="1rem" />}
            />
          )}
          {canQuote && (
            <NavLink
              label="Cotizaciones"
              component={Link}
              to="/cotizaciones"
              leftSection={<IconFileText size="1rem" />}
            />
          )}

          {canManageInventory && (
            <NavLink
              label="Inventario"
              component={Link}
              to="/inventario"
              leftSection={<IconArchive size="1rem" />}
            />
          )}

          {canViewReports && (
            <NavLink
              label="Reportes"
              component={Link}
              to="/reportes"
              leftSection={<IconReportAnalytics size="1rem" />}
            />
          )}
          {permisos.includes('cerrar_caja') && (
            <NavLink
              label="Corte de Caja"
              component={Link}
              to="/corte-caja"
              leftSection={<IconCash size="1rem" />}
            />
          )}
          {permisos.includes('gestionar_devoluciones') && (
            <NavLink
              label="Devoluciones"
              component={Link}
              to="/devoluciones"
              leftSection={<IconArrowBackUp size="1rem" />}
            />
          )}
          {permisos.includes('gestionar_clientes') && (
            <NavLink
              label="Clientes"
              component={Link}
              to="/clientes"
              leftSection={<IconUsers size="1rem" />}
            />
          )}

          {canManageInventory && (
            <NavLink
              label="Proveedores"
              component={Link}
              to="/proveedores"
              leftSection={<IconUsers size="1rem" />}
            />
          )}

          {canConfigure && (
            <NavLink
              label="Configuración"
              component={Link}
              to="/configuracion"
              leftSection={<IconSettings size="1rem" />}
            />
          )}

          {canManageUsers && (
            <NavLink
              label="Roles y Permisos"
              component={Link}
              to="/roles"
              leftSection={<IconUsers size="1rem" />}
            />
          )}
        </div>

        <div style={{ borderTop: `1px solid var(--mantine-color-gray-3)`, paddingTop: 'var(--mantine-spacing-sm)' }}>
          <Group>
            <Avatar radius="xl" />
            <div style={{ flex: 1 }}>
              <Text size="sm" fw={500}>
                {username}
              </Text>
              <Text c="dimmed" size="xs">
                {rol}
              </Text>
            </div>
            <UnstyledButton onClick={onLogout} title="Cerrar Sesión">
              <IconLogout style={{ width: rem(20), height: rem(20) }} stroke={1.5} />
            </UnstyledButton>
          </Group>
          <Button mt="xs" size="compact-xs" variant="subtle" onClick={() => setPasswordModalOpen(true)}>Cambiar contraseña</Button>
        </div>
      </AppShell.Navbar>

      <AppShell.Main>
        {/* El componente Outlet renderiza la ruta activa (la pantalla actual) */}
        <Outlet />

        <Dialog
          opened={stockAlertOpen}
          withCloseButton
          onClose={() => setStockAlertOpen(false)}
          size="lg"
          radius="md"
          position={{ bottom: 20, right: 20 }}
          bg="red.1"
        >
          <Group align="center">
            <IconAlertCircle size="1.5rem" color="red" />
            <div style={{ flex: 1 }}>
              <Text size="sm" fw={500} c="red.9">
                Alerta de Stock
              </Text>
              <Text size="xs" c="red.8">
                Hay {lowStockCount} productos con stock bajo.
              </Text>
            </div>
          </Group>
        </Dialog>
      </AppShell.Main>
      <Modal opened={passwordModalOpen} onClose={closePasswordModal} title="Cambiar contraseña" centered>
        <Stack>
          <PasswordInput label="Contraseña actual" value={currentPassword} onChange={(event) => setCurrentPassword(event.currentTarget.value)} required />
          <PasswordInput label="Contraseña nueva" description="Al menos 12 caracteres; no debe incluir tu usuario." value={newPassword} onChange={(event) => setNewPassword(event.currentTarget.value)} required minLength={12} />
          <PasswordInput label="Confirmar contraseña nueva" value={confirmPassword} onChange={(event) => setConfirmPassword(event.currentTarget.value)} required minLength={12} />
          {passwordError && <Text c="red" size="sm">{passwordError}</Text>}
          <Button onClick={changePassword}>Guardar contraseña</Button>
        </Stack>
      </Modal>
    </AppShell>
  );
}
