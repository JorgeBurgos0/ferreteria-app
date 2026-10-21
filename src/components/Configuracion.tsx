import { Button, Paper, Title, Text, Alert, Table, Modal, TextInput, PasswordInput, Select, Stack, Badge, Group, Tabs, Switch, NumberInput } from '@mantine/core';
import { useDisclosure } from '@mantine/hooks';
import { save, open as openDialog } from '@tauri-apps/plugin-dialog';
import { invoke } from '../lib/invoke';
import { notifications } from '@mantine/notifications';
import { useState, useEffect } from 'react';
import { IconAlertCircle, IconUserPlus, IconTrash, IconUsers, IconDatabase, IconHistory, IconBuildingStore } from '@tabler/icons-react';
import { DatePickerInput } from '@mantine/dates';
import 'dayjs/locale/es';

interface Usuario {
    id: number;
    username: string;
    rol: string;
    activo: boolean;
}

interface LogAuditoria {
    id: number;
    fecha_hora: string;
    usuario: string;
    accion: string;
    detalles: string | null;
}

interface DatosEmpresa {
    nombre: string;
    direccion: string;
    telefono: string;
    logo: string;
}

interface ConfiguracionProps {
    username: string;
    permisos: string[];
}

interface BackupSettings {
    enabled: boolean;
    folder: string;
    keep: number;
    audit_days: number;
    last_backup: string | null;
    last_error: string | null;
}

export function Configuracion({ username, permisos }: ConfiguracionProps) {
    const canManageUsers = permisos.includes('gestionar_usuarios');
    const canEditCompany = permisos.includes('editar_empresa');
    const canBackup = permisos.includes('gestionar_respaldos');
    const canRestore = permisos.includes('restaurar_respaldos');
    const canAudit = permisos.includes('ver_bitacora');
    const initialTab = canManageUsers ? 'usuarios' : canEditCompany ? 'empresa' : canBackup || canRestore ? 'respaldos' : 'auditoria';
    const [mensaje, setMensaje] = useState<string | null>(null);
    const [error, setError] = useState<string | null>(null);

    // Estados para gestión de usuarios
    const [usuarios, setUsuarios] = useState<Usuario[]>([]);
    const [opened, { open, close }] = useDisclosure(false);
    const [nuevoUsername, setNuevoUsername] = useState('');
    const [nuevoPassword, setNuevoPassword] = useState('');
    const [nuevoRol, setNuevoRol] = useState<string | null>('vendedor');
    const [resetTarget, setResetTarget] = useState<Usuario | null>(null);
    const [resetPassword, setResetPassword] = useState('');
    const [backupSettings, setBackupSettings] = useState<BackupSettings>({ enabled: false, folder: '', keep: 7, audit_days: 0, last_backup: null, last_error: null });

    // Estados para Auditoría
    const [logs, setLogs] = useState<LogAuditoria[]>([]);
    const [fechasLogs, setFechasLogs] = useState<[Date | null, Date | null]>([
        new Date(new Date().setDate(new Date().getDate() - 7)),
        new Date()
    ]);

    // Estado Empresa
    const [datosEmpresa, setDatosEmpresa] = useState<DatosEmpresa>({
        nombre: '',
        direccion: '',
        telefono: '',
        logo: ''
    });

    useEffect(() => {
        if (canManageUsers) cargarUsuarios();
        if (canEditCompany) cargarDatosEmpresa();
        if (canBackup) invoke<BackupSettings>('obtener_configuracion_local').then(setBackupSettings).catch((reason) => setError(String(reason)));
    }, []);

    const cargarDatosEmpresa = async () => {
        try {
            const data: DatosEmpresa = await invoke('obtener_datos_empresa');
            setDatosEmpresa(data);
        } catch (e) {
            console.error("Error al cargar datos empresa", e);
        }
    };

    const handleGuardarEmpresa = async () => {
        try {
            await invoke('guardar_datos_empresa', { ...datosEmpresa });
            notifications.show({ title: 'Guardado', message: 'Datos de la empresa actualizados correctamente', color: 'green' });
        } catch (e) {
            console.error(e);
            notifications.show({ title: 'Error', message: 'No se pudieron guardar los datos', color: 'red' });
        }
    };

    const cargarUsuarios = async () => {
        try {
            const usuariosCargados: Usuario[] = await invoke('obtener_usuarios');
            setUsuarios(usuariosCargados);
        } catch (err) {
            console.error("Error al cargar usuarios:", err);
        }
    };

    const cargarLogs = async () => {
        if (!fechasLogs[0] || !fechasLogs[1]) return;
        try {
            const fechaInicio = fechasLogs[0].toISOString().split('T')[0];
            const fechaFin = fechasLogs[1].toISOString().split('T')[0];
            const logsCargados: LogAuditoria[] = await invoke('obtener_logs', { fechaInicio, fechaFin });
            setLogs(logsCargados);
        } catch (err) {
            console.error("Error al cargar logs:", err);
            setError(String(err));
        }
    };

    const handleCrearUsuario = async () => {
        if (!nuevoUsername || !nuevoPassword || !nuevoRol) {
            notifications.show({ title: 'Error', message: 'Todos los campos son obligatorios', color: 'red' });
            return;
        }
        try {
            await invoke('crear_usuario', { username: nuevoUsername, password: nuevoPassword, rol: nuevoRol, creadoPor: username });
            notifications.show({ title: 'Éxito', message: 'Usuario creado correctamente', color: 'green' });
            close();
            setNuevoUsername('');
            setNuevoPassword('');
            setNuevoRol('vendedor');
            cargarUsuarios();
        } catch (err) {
            console.error(err);
            notifications.show({ title: 'Error', message: String(err), color: 'red' });
        }
    };

    const handleEliminarUsuario = async (id: number, userToDelete: string) => {
        if (!confirm(`¿Estás seguro de eliminar al usuario "${userToDelete}"?`)) return;

        try {
            await invoke('eliminar_usuario', { id, adminUser: username });
            notifications.show({ title: 'Usuario eliminado', message: 'El usuario ha sido eliminado', color: 'blue' });
            cargarUsuarios();
        } catch (err) {
            console.error(err);
            notifications.show({ title: 'Error', message: String(err), color: 'red' });
        }
    };

    const handleActualizarEstado = async (usuario: Usuario) => {
        const accion = usuario.activo ? 'desactivar' : 'activar';
        if (!confirm(`¿Quieres ${accion} la cuenta de ${usuario.username}?`)) return;
        try {
            await invoke('actualizar_estado_usuario', { id: usuario.id, activo: !usuario.activo });
            await cargarUsuarios();
            notifications.show({ title: 'Cuenta actualizada', message: `Cuenta ${usuario.activo ? 'desactivada' : 'activada'}.`, color: 'green' });
        } catch (err) {
            notifications.show({ title: 'Error', message: String(err), color: 'red' });
        }
    };

    const handleRestablecerPassword = async () => {
        if (!resetTarget) return;
        try {
            await invoke('restablecer_password_usuario', { id: resetTarget.id, passwordNueva: resetPassword });
            notifications.show({ title: 'Contraseña restablecida', message: 'La persona deberá cambiarla al iniciar sesión.', color: 'green' });
            setResetTarget(null);
            setResetPassword('');
        } catch (err) {
            notifications.show({ title: 'Error', message: String(err), color: 'red' });
        }
    };

    const handleCrearRespaldo = async () => {
        setMensaje(null);
        setError(null);

        try {
            // 2. Abrimos el diálogo "Guardar como..." del sistema operativo
            const rutaDestino = await save({
                title: 'Guardar respaldo de la base de datos',
                defaultPath: `ferreteria-respaldo-${new Date().toISOString().split('T')[0]}.db`,
                filters: [{
                    name: 'Base de datos SQLite',
                    extensions: ['db']
                }]
            });

            // 3. Si el usuario seleccionó una ruta y no canceló...
            if (rutaDestino) {
                // 4. Invocamos el comando del backend
                await invoke('crear_backup', { rutaDestino, usuario: username });
                notifications.show({
                    title: 'Respaldo Creado',
                    message: `Backup guardado exitosamente en: ${rutaDestino}`,
                    color: 'green',
                    icon: <IconDatabase size="1rem" />
                });
            }

        } catch (err) {
            console.error("Error al crear el respaldo:", err);
            notifications.show({ title: 'Error', message: `Error al crear respaldo: ${err}`, color: 'red' });
        }
    };

    const handleGuardarRespaldoAutomatico = async () => {
        try {
            await invoke('guardar_configuracion_local', { value: backupSettings });
            setBackupSettings(await invoke<BackupSettings>('obtener_configuracion_local'));
            notifications.show({ title: 'Configuración guardada', message: backupSettings.enabled ? 'Revisa la fecha del último respaldo en esta pantalla.' : 'Respaldos automáticos desactivados.', color: 'green' });
        } catch (reason) {
            invoke<BackupSettings>('obtener_configuracion_local').then(setBackupSettings).catch(() => {});
            notifications.show({ title: 'No se pudo configurar el respaldo', message: String(reason), color: 'red' });
        }
    };

    const handleRestaurarRespaldo = async () => {
        if (!confirm("ADVERTENCIA: Al restaurar un respaldo, se PERDERÁN todos los datos actuales y serán reemplazados por los del respaldo. ¿Estás seguro de continuar?")) return;

        setMensaje(null);
        setError(null);

        try {
            const selected = await openDialog({
                title: 'Seleccionar archivo de respaldo',
                filters: [{
                    name: 'Base de datos SQLite',
                    extensions: ['db']
                }]
            });

            if (selected && typeof selected === 'string') {
                await invoke('restaurar_backup', { rutaBackup: selected });
                window.location.reload();
            }
        } catch (err) {
            console.error("Error al restaurar:", err);
            notifications.show({ title: 'Error', message: `Error al restaurar: ${err}`, color: 'red' });
        }
    };

    const rowsUsuarios = usuarios.map((usuario) => (
        <Table.Tr key={usuario.id}>
            <Table.Td>{usuario.username}</Table.Td>
            <Table.Td><Badge color={usuario.rol === 'admin' ? 'blue' : 'gray'}>{usuario.rol}</Badge></Table.Td>
            <Table.Td><Badge color={usuario.activo ? 'green' : 'gray'}>{usuario.activo ? 'Activa' : 'Inactiva'}</Badge></Table.Td>
            <Table.Td>
                <Group gap="xs">
                    <Button size="xs" variant="light" disabled={usuario.username === username} onClick={() => { setResetTarget(usuario); setResetPassword(''); }}>Restablecer contraseña</Button>
                    <Button size="xs" variant="light" color={usuario.activo ? 'orange' : 'green'} disabled={usuario.username === username} onClick={() => handleActualizarEstado(usuario)}>{usuario.activo ? 'Desactivar' : 'Activar'}</Button>
                    <Button color="red" size="xs" variant="subtle" disabled={usuario.username === username} onClick={() => handleEliminarUsuario(usuario.id, usuario.username)} aria-label={`Eliminar ${usuario.username}`}>
                        <IconTrash size="1rem" />
                    </Button>
                </Group>
            </Table.Td>
        </Table.Tr>
    ));

    const rowsLogs = logs.map((log) => (
        <Table.Tr key={log.id}>
            <Table.Td style={{ whiteSpace: 'nowrap' }}>{new Date(log.fecha_hora).toLocaleString('es-MX')}</Table.Td>
            <Table.Td fw={700}>{log.usuario}</Table.Td>
            <Table.Td><Badge color="orange">{log.accion}</Badge></Table.Td>
            <Table.Td>{log.detalles}</Table.Td>
        </Table.Tr>
    ));

    return (
        <Paper p="md" shadow="xs">
            <Title order={2} mb="xl">Configuración y Herramientas</Title>

            {mensaje && <Alert color="green" title="Éxito" mb="md" withCloseButton onClose={() => setMensaje(null)}>{mensaje}</Alert>}
            {error && <Alert color="red" icon={<IconAlertCircle />} title="Error" mb="md" withCloseButton onClose={() => setError(null)}>{error}</Alert>}

            <Tabs defaultValue={initialTab}>
                <Tabs.List mb="md">
                    {canManageUsers && <Tabs.Tab value="usuarios" leftSection={<IconUsers size="0.8rem" />}>Usuarios</Tabs.Tab>}
                    {canEditCompany && <Tabs.Tab value="empresa" leftSection={<IconBuildingStore size="0.8rem" />}>Datos Empresa</Tabs.Tab>}
                    {(canBackup || canRestore) && <Tabs.Tab value="respaldos" leftSection={<IconDatabase size="0.8rem" />}>Respaldos</Tabs.Tab>}
                    {canAudit && <Tabs.Tab value="auditoria" leftSection={<IconHistory size="0.8rem" />}>Auditoría y Seguridad</Tabs.Tab>}
                </Tabs.List>

                {canManageUsers && <Tabs.Panel value="usuarios">
                    <Title order={4} mb="sm">Gestión de Usuarios</Title>
                    <Text size="sm" c="dimmed" mb="md">Administra quién tiene acceso al sistema.</Text>
                    <Text size="sm" c="dimmed" mb="md">Las cuentas nuevas y las contraseñas restablecidas requieren un cambio al primer acceso.</Text>

                    <Button leftSection={<IconUserPlus size="1rem" />} onClick={open} mb="md">Nuevo Usuario</Button>

                    <Table withTableBorder withColumnBorders>
                        <Table.Thead>
                            <Table.Tr>
                                <Table.Th>Usuario</Table.Th>
                                <Table.Th>Rol</Table.Th>
                                <Table.Th>Estado</Table.Th>
                                <Table.Th>Acciones</Table.Th>
                            </Table.Tr>
                        </Table.Thead>
                        <Table.Tbody>{rowsUsuarios}</Table.Tbody>
                    </Table>
                </Tabs.Panel>}

                {canEditCompany && <Tabs.Panel value="empresa">
                    <Title order={4} mb="sm">Personalización del Negocio</Title>
                    <Text size="sm" c="dimmed" mb="md">Estos datos aparecerán en los tickets impresos.</Text>
                    <Stack maw={500}>
                        <TextInput
                            label="Nombre del Negocio"
                            placeholder="Ej. Ferretería El Tornillo"
                            value={datosEmpresa.nombre}
                            onChange={(e) => setDatosEmpresa({ ...datosEmpresa, nombre: e.currentTarget.value })}
                        />
                        <TextInput
                            label="Dirección"
                            placeholder="Dirección de la sucursal"
                            value={datosEmpresa.direccion}
                            onChange={(e) => setDatosEmpresa({ ...datosEmpresa, direccion: e.currentTarget.value })}
                        />
                        <TextInput
                            label="Teléfono"
                            placeholder="Ej. 555-1234"
                            value={datosEmpresa.telefono}
                            onChange={(e) => setDatosEmpresa({ ...datosEmpresa, telefono: e.currentTarget.value })}
                        />
                        <Group align="flex-end">
                            <TextInput
                                label="Logo"
                                placeholder="Selecciona una imagen PNG o JPEG"
                                value={datosEmpresa.logo.startsWith('data:') ? 'Imagen guardada' : datosEmpresa.logo}
                                readOnly
                                style={{ flex: 1 }}
                            />
                            <Button onClick={async () => {
                                const selected = await openDialog({
                                    multiple: false,
                                    filters: [{
                                        name: 'Imágenes',
                                        extensions: ['png', 'jpg', 'jpeg']
                                    }]
                                });
                                if (selected && typeof selected === 'string') {
                                    setDatosEmpresa({ ...datosEmpresa, logo: selected });
                                }
                            }}>Buscar...</Button>
                            <Button variant="light" color="red" onClick={() => setDatosEmpresa({ ...datosEmpresa, logo: '' })}>Quitar</Button>
                        </Group>

                        <Button mt="md" onClick={handleGuardarEmpresa}>Guardar Información</Button>
                    </Stack>
                </Tabs.Panel>}

                {(canBackup || canRestore) && <Tabs.Panel value="respaldos">
                    <Title order={4} mb="sm">Respaldo y Restauración</Title>
                    <Text mb="md">
                        Guarda una copia de seguridad de todos tus datos o restaura una versión anterior.
                    </Text>
                    <Group>
                        {canBackup && <Button onClick={handleCrearRespaldo} variant="outline">
                            Crear Respaldo
                        </Button>}
                        {canRestore && <Button onClick={handleRestaurarRespaldo} variant="outline" color="red">
                            Restaurar Respaldo
                        </Button>}
                    </Group>
                    {canBackup && <Stack mt="lg" maw={600}>
                        <Title order={5}>Respaldo automático local</Title>
                        <Text size="sm" c="dimmed">Al activarlo se crea una copia verificada; después se crea una al día mientras la aplicación esté abierta. Se conservan únicamente las copias automáticas más recientes.</Text>
                        <Switch label="Activar respaldos automáticos" checked={backupSettings.enabled} onChange={(event) => setBackupSettings({ ...backupSettings, enabled: event.currentTarget.checked })} />
                        <Group align="flex-end">
                            <TextInput label="Carpeta local" value={backupSettings.folder} readOnly style={{ flex: 1 }} />
                            <Button variant="light" onClick={async () => {
                                const path = await openDialog({ directory: true, title: 'Seleccionar carpeta para respaldos' });
                                if (typeof path === 'string') setBackupSettings({ ...backupSettings, folder: path });
                            }}>Elegir carpeta</Button>
                        </Group>
                        <NumberInput label="Copias automáticas a conservar" min={1} max={30} value={backupSettings.keep} onChange={(value) => setBackupSettings({ ...backupSettings, keep: Number(value) || 1 })} />
                        <Select label="Retención de bitácora" description="Antes de eliminar registros antiguos se guarda una copia verificada. Solo se aplica con respaldo automático activo." value={String(backupSettings.audit_days)} onChange={(value) => setBackupSettings({ ...backupSettings, audit_days: Number(value ?? 0) })} data={[{ value: '0', label: 'Conservar indefinidamente' }, { value: '90', label: '90 días' }, { value: '180', label: '180 días' }, { value: '365', label: '1 año' }, { value: '730', label: '2 años' }]} />
                        <Button onClick={handleGuardarRespaldoAutomatico}>Guardar configuración</Button>
                        <Text size="sm">Último respaldo automático: {backupSettings.last_backup ? new Date(backupSettings.last_backup).toLocaleString('es-MX') : 'ninguno'}</Text>
                        {backupSettings.last_error && <Alert color="red" title="Último error de respaldo">{backupSettings.last_error}</Alert>}
                        <Button variant="subtle" size="xs" onClick={() => invoke<BackupSettings>('obtener_configuracion_local').then(setBackupSettings).catch((reason) => setError(String(reason)))}>Actualizar estado</Button>
                    </Stack>}
                </Tabs.Panel>}

                {canAudit && <Tabs.Panel value="auditoria">
                    <Title order={4} mb="sm">Bitácora de Eventos (Audit Log)</Title>
                    <Text size="sm" c="dimmed" mb="md">Registro de acciones críticas en el sistema.</Text>
                    <Text size="sm" c="dimmed" mb="md">Se muestran hasta 1000 eventos del rango. La retención se configura en Respaldos.</Text>

                    <Group mb="md" align="flex-end">
                        <DatePickerInput
                            type="range"
                            label="Rango de Fechas"
                            placeholder="Inicio - Fin"
                            value={fechasLogs}
                            onChange={(val) => {
                                const [start, end] = val;
                                setFechasLogs([
                                    start ? new Date(start) : null,
                                    end ? new Date(end) : null
                                ]);
                            }}
                            locale="es"
                        />
                        <Button onClick={cargarLogs} leftSection={<IconHistory size="1rem" />}>Consultar Logs</Button>
                    </Group>

                    <Table striped highlightOnHover withTableBorder>
                        <Table.Thead>
                            <Table.Tr>
                                <Table.Th>Fecha/Hora</Table.Th>
                                <Table.Th>Usuario</Table.Th>
                                <Table.Th>Acción</Table.Th>
                                <Table.Th>Detalles</Table.Th>
                            </Table.Tr>
                        </Table.Thead>
                        <Table.Tbody>
                            {rowsLogs.length > 0 ? rowsLogs : <Table.Tr><Table.Td colSpan={4} align="center">No hay registros para este periodo.</Table.Td></Table.Tr>}
                        </Table.Tbody>
                    </Table>
                </Tabs.Panel>}
            </Tabs>

            <Modal opened={opened} onClose={close} title="Crear Nuevo Usuario" centered>
                <Stack>
                    <TextInput label="Nombre de Usuario" placeholder="ej. vendedor1" value={nuevoUsername} onChange={(e) => setNuevoUsername(e.currentTarget.value)} />
                    <PasswordInput label="Contraseña" placeholder="******" value={nuevoPassword} onChange={(e) => setNuevoPassword(e.currentTarget.value)} />
                    <Select
                        label="Rol"
                        data={[{ value: 'admin', label: 'Administrador' }, { value: 'vendedor', label: 'Vendedor' }]}
                        value={nuevoRol}
                        onChange={setNuevoRol}
                    />
                    <Button onClick={handleCrearUsuario}>Crear Usuario</Button>
                </Stack>
            </Modal>

            <Modal opened={resetTarget !== null} onClose={() => { setResetTarget(null); setResetPassword(''); }} title={`Restablecer contraseña: ${resetTarget?.username ?? ''}`} centered>
                <Stack>
                    <PasswordInput label="Contraseña temporal" description="Al menos 12 caracteres; no debe contener el nombre de usuario." value={resetPassword} onChange={(event) => setResetPassword(event.currentTarget.value)} />
                    <Button onClick={handleRestablecerPassword}>Restablecer</Button>
                </Stack>
            </Modal>
        </Paper>
    );
}
