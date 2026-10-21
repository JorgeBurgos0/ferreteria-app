import { Button, Paper, Title, Text, Alert, Table, Modal, TextInput, PasswordInput, Select, Stack, Badge, Group, Tabs } from '@mantine/core';
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
}

export function Configuracion({ username }: ConfiguracionProps) {
    const [mensaje, setMensaje] = useState<string | null>(null);
    const [error, setError] = useState<string | null>(null);

    // Estados para gestión de usuarios
    const [usuarios, setUsuarios] = useState<Usuario[]>([]);
    const [opened, { open, close }] = useDisclosure(false);
    const [nuevoUsername, setNuevoUsername] = useState('');
    const [nuevoPassword, setNuevoPassword] = useState('');
    const [nuevoRol, setNuevoRol] = useState<string | null>('vendedor');

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
        cargarUsuarios();
        cargarDatosEmpresa();
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
            <Table.Td>
                <Button color="red" size="xs" variant="subtle" onClick={() => handleEliminarUsuario(usuario.id, usuario.username)}>
                    <IconTrash size="1rem" />
                </Button>
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

            <Tabs defaultValue="usuarios">
                <Tabs.List mb="md">
                    <Tabs.Tab value="usuarios" leftSection={<IconUsers size="0.8rem" />}>Usuarios</Tabs.Tab>
                    <Tabs.Tab value="empresa" leftSection={<IconBuildingStore size="0.8rem" />}>Datos Empresa</Tabs.Tab>
                    <Tabs.Tab value="respaldos" leftSection={<IconDatabase size="0.8rem" />}>Respaldos</Tabs.Tab>
                    <Tabs.Tab value="auditoria" leftSection={<IconHistory size="0.8rem" />}>Auditoría y Seguridad</Tabs.Tab>
                </Tabs.List>

                <Tabs.Panel value="usuarios">
                    <Title order={4} mb="sm">Gestión de Usuarios</Title>
                    <Text size="sm" c="dimmed" mb="md">Administra quién tiene acceso al sistema.</Text>

                    <Button leftSection={<IconUserPlus size="1rem" />} onClick={open} mb="md">Nuevo Usuario</Button>

                    <Table withTableBorder withColumnBorders>
                        <Table.Thead>
                            <Table.Tr>
                                <Table.Th>Usuario</Table.Th>
                                <Table.Th>Rol</Table.Th>
                                <Table.Th>Acciones</Table.Th>
                            </Table.Tr>
                        </Table.Thead>
                        <Table.Tbody>{rowsUsuarios}</Table.Tbody>
                    </Table>
                </Tabs.Panel>

                <Tabs.Panel value="empresa">
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
                </Tabs.Panel>

                <Tabs.Panel value="respaldos">
                    <Title order={4} mb="sm">Respaldo y Restauración</Title>
                    <Text mb="md">
                        Guarda una copia de seguridad de todos tus datos o restaura una versión anterior.
                    </Text>
                    <Group>
                        <Button onClick={handleCrearRespaldo} variant="outline">
                            Crear Respaldo
                        </Button>
                        <Button onClick={handleRestaurarRespaldo} variant="outline" color="red">
                            Restaurar Respaldo
                        </Button>
                    </Group>
                </Tabs.Panel>

                <Tabs.Panel value="auditoria">
                    <Title order={4} mb="sm">Bitácora de Eventos (Audit Log)</Title>
                    <Text size="sm" c="dimmed" mb="md">Registro de acciones críticas en el sistema.</Text>

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
                </Tabs.Panel>
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
        </Paper>
    );
}
