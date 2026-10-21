import { useState, useEffect } from 'react';
import { invoke } from '../lib/invoke';
import { Table, Button, Modal, TextInput, Group, Title, Paper, Stack } from '@mantine/core';
import { useDisclosure } from '@mantine/hooks';

interface Proveedor {
    id: number;
    nombre_empresa: string;
    nombre_contacto: string;
    telefono: string;
    email: string;
}

export function GestionProveedores() {
    const [proveedores, setProveedores] = useState<Proveedor[]>([]);
    const [opened, { open, close }] = useDisclosure(false);
    const [proveedorAEditar, setProveedorAEditar] = useState<Proveedor | null>(null);

    // Estados para el formulario
    const [nombreEmpresa, setNombreEmpresa] = useState('');
    const [nombreContacto, setNombreContacto] = useState('');
    const [telefono, setTelefono] = useState('');
    const [email, setEmail] = useState('');

    const cargarProveedores = async () => {
        try {
            const res: Proveedor[] = await invoke('obtener_proveedores');
            setProveedores(res);
        } catch (error) {
            console.error("Error al cargar proveedores:", error);
            alert(`Error al cargar proveedores: ${error}`);
        }
    };

    useEffect(() => {
        cargarProveedores();
    }, []);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        
        const datosProveedor = {
            nombreEmpresa,
            nombreContacto,
            telefono,
            email,
        };

        try {
            if (proveedorAEditar) {
                // MODO EDICIÓN
                await invoke('actualizar_proveedor', { id: proveedorAEditar.id, ...datosProveedor });
            } else {
                // MODO CREACIÓN
                await invoke('agregar_proveedor', datosProveedor);
            }

            // --- ESTAS LÍNEAS AHORA SE EJECUTAN EN AMBOS CASOS ---
            limpiarFormularioYcerrar();
            cargarProveedores(); // <-- ESTA LÍNEA ES LA QUE FALTABA DENTRO DEL IF ANTERIORMENTE

        } catch (error) {
            console.error("Error al guardar proveedor:", error);
            if (String(error).includes('UNIQUE constraint failed')) {
                alert(`Error: El nombre de la empresa "${nombreEmpresa}" ya existe.`);
            } else {
                alert(`Error al guardar proveedor: ${error}`);
            }
        }
    };

    const handleAbrirModalCrear = () => {
        limpiarFormularioYcerrar();
        setProveedorAEditar(null);
        open();
    };

    const handleAbrirModalEditar = (proveedor: Proveedor) => {
        setProveedorAEditar(proveedor);
        setNombreEmpresa(proveedor.nombre_empresa);
        setNombreContacto(proveedor.nombre_contacto);
        setTelefono(proveedor.telefono);
        setEmail(proveedor.email);
        open();
    };
        
    const handleEliminarProveedor = async (id: number) => {
        const confirmado = window.confirm("¿Estás seguro de que quieres eliminar a este proveedor?");
        if (confirmado) {
            try {
                await invoke('eliminar_proveedor', { id });
                cargarProveedores();
            } catch (error) {
                console.error("Error al eliminar proveedor:", error);
                alert(`Error al eliminar proveedor: ${error}`);
            }
        }
    };

    const limpiarFormularioYcerrar = () => {
        setNombreEmpresa('');
        setNombreContacto('');
        setTelefono('');
        setEmail('');
        setProveedorAEditar(null);
        close();
    };

    const rows = proveedores.map((proveedor) => (
        <Table.Tr key={proveedor.id}>
            <Table.Td>{proveedor.nombre_empresa}</Table.Td>
            <Table.Td>{proveedor.nombre_contacto}</Table.Td>
            <Table.Td>{proveedor.telefono}</Table.Td>
            <Table.Td>{proveedor.email}</Table.Td>
            <Table.Td>
                <Group>
                    <Button size="xs" variant="outline" onClick={() => handleAbrirModalEditar(proveedor)}>Editar</Button>
                    <Button size="xs" color="red" onClick={() => handleEliminarProveedor(proveedor.id)}>Eliminar</Button>
                </Group>
            </Table.Td>
        </Table.Tr>
    ));

    return (
        <Paper p="md" shadow="xs">
            <Modal opened={opened} onClose={limpiarFormularioYcerrar} title={proveedorAEditar ? 'Editar Proveedor' : 'Añadir Nuevo Proveedor'}>
                <form onSubmit={handleSubmit}>
                    <Stack>
                        <TextInput label="Nombre de la Empresa" value={nombreEmpresa} onChange={(e) => setNombreEmpresa(e.currentTarget.value)} required />
                        <TextInput label="Nombre del Contacto" value={nombreContacto} onChange={(e) => setNombreContacto(e.currentTarget.value)} />
                        <TextInput label="Teléfono" value={telefono} onChange={(e) => setTelefono(e.currentTarget.value)} />
                        <TextInput label="Email" type="email" value={email} onChange={(e) => setEmail(e.currentTarget.value)} />
                        <Button type="submit">Guardar Cambios</Button>
                    </Stack>
                </form>
            </Modal>
            <Group justify="space-between" mb="md">
                <Title order={2}>Gestión de Proveedores</Title>
                <Button onClick={handleAbrirModalCrear}>Añadir Proveedor</Button>
            </Group>
            <Table striped highlightOnHover withTableBorder>
                <Table.Thead>
                    <Table.Tr>
                        <Table.Th>Empresa</Table.Th>
                        <Table.Th>Contacto</Table.Th>
                        <Table.Th>Teléfono</Table.Th>
                        <Table.Th>Email</Table.Th>
                        <Table.Th>Acciones</Table.Th>
                    </Table.Tr>
                </Table.Thead>
                <Table.Tbody>{rows.length > 0 ? rows : <Table.Tr><Table.Td colSpan={5} align="center">No hay proveedores registrados.</Table.Td></Table.Tr>}</Table.Tbody>
            </Table>
        </Paper>
    );
}