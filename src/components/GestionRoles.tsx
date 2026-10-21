import { useState, useEffect } from 'react';
import { Title, Table, Button, Group, Modal, Stack, Text, Alert, MultiSelect } from '@mantine/core';
import { IconEdit, IconAlertCircle } from '@tabler/icons-react';
import { invoke } from '../lib/invoke';
import { notifications } from '@mantine/notifications';

interface Rol {
    id: number;
    nombre: string;
    descripcion: string;
}

const PERMISOS_DISPONIBLES = [
    { value: 'ver_dashboard', label: 'Ver Dashboard' },
    { value: 'gestion_inventario', label: 'Gestión de Inventario (Productos/Proveedores)' },
    { value: 'realizar_venta', label: 'Cobrar ventas' },
    { value: 'gestionar_cotizaciones', label: 'Crear y consultar cotizaciones' },
    { value: 'gestionar_devoluciones', label: 'Procesar devoluciones' },
    { value: 'cerrar_caja', label: 'Cerrar caja' },
    { value: 'reabrir_caja', label: 'Reabrir caja' },
    { value: 'gestionar_clientes', label: 'Gestionar clientes' },
    { value: 'ver_reportes', label: 'Ver Reportes' },
    { value: 'editar_empresa', label: 'Editar datos de empresa' },
    { value: 'gestionar_respaldos', label: 'Crear respaldos' },
    { value: 'restaurar_respaldos', label: 'Restaurar respaldos' },
    { value: 'ver_bitacora', label: 'Consultar bitácora' },
    { value: 'gestionar_usuarios', label: 'Gestionar Usuarios y Roles' },
];

export function GestionRoles() {
    const [roles, setRoles] = useState<Rol[]>([]);
    const [modalAbierto, setModalAbierto] = useState(false);

    // State form
    const [rolSeleccionado, setRolSeleccionado] = useState<Rol | null>(null);
    const [permisosSeleccionados, setPermisosSeleccionados] = useState<string[]>([]);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        cargarRoles();
    }, []);

    const cargarRoles = async () => {
        try {
            const resp: Rol[] = await invoke('obtener_roles');
            setRoles(resp);
        } catch (e) {
            notifications.show({ title: 'Error', message: 'No se pudieron cargar los roles', color: 'red' });
        }
    };

    const handleAbrirModal = async (rol: Rol) => {
        setError(null);
        setRolSeleccionado(rol);
        try {
            const perms: string[] = await invoke('obtener_permisos_rol', { idRol: rol.id });
            setPermisosSeleccionados(perms);
        } catch (error) {
            console.error("Error al cargar permisos del rol:", error);
            notifications.show({ title: 'Error', message: 'No se pudieron cargar los permisos actuales.', color: 'red' });
            setPermisosSeleccionados([]);
        }
        setModalAbierto(true);
    };

    const handleGuardar = async () => {
        if (!rolSeleccionado) {
            return;
        }

        try {
            await invoke('guardar_permisos_rol', { idRol: rolSeleccionado.id, permisos: permisosSeleccionados });
            notifications.show({ title: 'Éxito', message: 'Permisos actualizados correctamente', color: 'green' });
            setModalAbierto(false);
        } catch (e) {
            setError(String(e));
        }
    };

    return (
        <div style={{ padding: 20 }}>
            <Group justify="space-between" mb="md">
                <Title order={2}>Gestión de Roles</Title>
            </Group>

            <Table striped highlightOnHover withTableBorder>
                <Table.Thead>
                    <Table.Tr>
                        <Table.Th>Nombre</Table.Th>
                        <Table.Th>Descripción</Table.Th>
                        <Table.Th>Acciones</Table.Th>
                    </Table.Tr>
                </Table.Thead>
                <Table.Tbody>
                    {roles.map(rol => (
                        <Table.Tr key={rol.id}>
                            <Table.Td>{rol.nombre}</Table.Td>
                            <Table.Td>{rol.descripcion}</Table.Td>
                            <Table.Td>
                                <Button variant="subtle" size="xs" onClick={() => handleAbrirModal(rol)}><IconEdit size={14} /> Permisos</Button>
                            </Table.Td>
                        </Table.Tr>
                    ))}
                </Table.Tbody>
            </Table>

            <Modal opened={modalAbierto} onClose={() => setModalAbierto(false)} title={`Permisos: ${rolSeleccionado?.nombre || ''}`}>
                <Stack>
                    {error && <Alert color="red" icon={<IconAlertCircle size={16} />}>{error}</Alert>}
                    <Text size="sm" c="dimmed">Selecciona los permisos para este rol. Nota: Esto sobrescribirá la configuración anterior.</Text>

                    <MultiSelect
                        label="Permisos Asignados"
                        data={PERMISOS_DISPONIBLES}
                        value={permisosSeleccionados}
                        onChange={setPermisosSeleccionados}
                        placeholder="Seleccionar permisos"
                        searchable
                        nothingFoundMessage="No hay más permisos"
                        hidePickedOptions
                    />

                    <Button onClick={handleGuardar}>Guardar Permisos</Button>
                </Stack>
            </Modal>
        </div>
    );
}
