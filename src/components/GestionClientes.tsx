import { useState, useEffect } from 'react';
import { invoke } from '../lib/invoke';
import { Table, Button, Modal, TextInput, Group, Title, Paper, Stack, Badge } from '@mantine/core';
import { useDisclosure } from '@mantine/hooks';
import { ImportarContactos } from './ImportarContactos';

interface Cliente {
    id: number;
    nombre: string;
    rfc: string;
    telefono: string;
    email: string;
    puntos: number;
}

interface CompraCliente {
    id_venta: number;
    fecha: string;
    total: number;
    productos: string;
}

export function GestionClientes() {
    const [clientes, setClientes] = useState<Cliente[]>([]);
    const [opened, { open, close }] = useDisclosure(false);
    const [openedHistorial, { open: openHistorial, close: closeHistorial }] = useDisclosure(false);

    const [clienteAEditar, setClienteAEditar] = useState<Cliente | null>(null);
    const [historialCompras, setHistorialCompras] = useState<CompraCliente[]>([]);
    const [clienteSeleccionado, setClienteSeleccionado] = useState<Cliente | null>(null);

    const [nombre, setNombre] = useState('');
    const [rfc, setRfc] = useState('');
    const [telefono, setTelefono] = useState('');
    const [email, setEmail] = useState('');

    const cargarClientes = async () => {
        try {
            const clientesCargados: Cliente[] = await invoke('obtener_clientes');
            setClientes(clientesCargados);
        } catch (error) {
            console.error("Error al cargar clientes:", error);
            alert(`Error al cargar clientes: ${error}`);
        }
    };

    useEffect(() => {
        cargarClientes();
    }, []);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();

        const datosCliente = {
            nombre,
            rfc,
            telefono,
            email,
        };

        try {
            if (clienteAEditar) {
                // MODO EDICIÓN
                await invoke('actualizar_cliente', { id: clienteAEditar.id, ...datosCliente });
            } else {
                // MODO CREACIÓN
                await invoke('agregar_cliente', datosCliente);
            }

            limpiarFormularioYcerrar();
            cargarClientes();

        } catch (error) {
            console.error("Error al guardar cliente:", error);
            alert(`Error al guardar cliente: ${error}`);
        }
    };

    const handleAbrirModalCrear = () => {
        limpiarFormularioYcerrar(); // Limpiamos por si había algo de una edición anterior
        setClienteAEditar(null);
        open();
    }

    const handleAbrirModalEditar = (cliente: Cliente) => {
        setClienteAEditar(cliente);
        setNombre(cliente.nombre);
        setRfc(cliente.rfc);
        setTelefono(cliente.telefono);
        setEmail(cliente.email);
        open();
    }

    const handleEliminarCliente = async (id: number) => {
        // En un futuro, aquí podrías poner un Modal de confirmación.
        const confirmado = window.confirm("¿Estás seguro de que quieres eliminar a este cliente?");
        if (confirmado) {
            try {
                await invoke('eliminar_cliente', { id });
                cargarClientes();
            } catch (error) {
                console.error("Error al eliminar cliente:", error);
                alert(`Error al eliminar cliente: ${error}`);
            }
        }
    }

    const handleVerHistorial = async (cliente: Cliente) => {
        setClienteSeleccionado(cliente);
        try {
            const historial: CompraCliente[] = await invoke('obtener_historial_compras', { idCliente: cliente.id });
            setHistorialCompras(historial);
            openHistorial();
        } catch (error) {
            console.error("Error al cargar historial:", error);
            alert(`Error al cargar historial: ${error}`);
        }
    }

    const limpiarFormularioYcerrar = () => {
        setNombre('');
        setRfc('');
        setTelefono('');
        setEmail('');
        setClienteAEditar(null);
        close();
    }

    const rows = clientes.map((cliente) => (
        <Table.Tr key={cliente.id}>
            <Table.Td>{cliente.nombre}</Table.Td>
            <Table.Td>{cliente.rfc}</Table.Td>
            <Table.Td>{cliente.telefono}</Table.Td>
            <Table.Td>{cliente.email}</Table.Td>
            <Table.Td><Badge color="blue">{cliente.puntos || 0} pts</Badge></Table.Td>
            <Table.Td>
                <Group>
                    <Button size="xs" variant="light" onClick={() => handleVerHistorial(cliente)}>Historial</Button>
                    <Button size="xs" variant="outline" onClick={() => handleAbrirModalEditar(cliente)}>Editar</Button>
                    <Button size="xs" color="red" onClick={() => handleEliminarCliente(cliente.id)}>Eliminar</Button>
                </Group>
            </Table.Td>
        </Table.Tr>
    ));

    return (
        <Paper p="md" shadow="xs">
            {/* Modal de Edición/Creación */}
            <Modal opened={opened} onClose={limpiarFormularioYcerrar} title={clienteAEditar ? 'Editar Cliente' : 'Añadir Nuevo Cliente'}>
                <form onSubmit={handleSubmit}>
                    <Stack>
                        <TextInput label="Nombre Completo" value={nombre} onChange={(e) => setNombre(e.currentTarget.value)} required />
                        <TextInput label="RFC" value={rfc} onChange={(e) => setRfc(e.currentTarget.value)} />
                        <TextInput label="Teléfono" value={telefono} onChange={(e) => setTelefono(e.currentTarget.value)} />
                        <TextInput label="Email" type="email" value={email} onChange={(e) => setEmail(e.currentTarget.value)} />
                        <Button type="submit">Guardar Cambios</Button>
                    </Stack>
                </form>
            </Modal>

            {/* Modal de Historial */}
            <Modal opened={openedHistorial} onClose={closeHistorial} title={`Historial de Compras - ${clienteSeleccionado?.nombre}`} size="lg">
                <Table striped highlightOnHover>
                    <Table.Thead>
                        <Table.Tr>
                            <Table.Th>Fecha</Table.Th>
                            <Table.Th>Productos</Table.Th>
                            <Table.Th>Total</Table.Th>
                        </Table.Tr>
                    </Table.Thead>
                    <Table.Tbody>
                        {historialCompras.length > 0 ? (
                            historialCompras.map((compra) => (
                                <Table.Tr key={compra.id_venta}>
                                    <Table.Td>{new Date(compra.fecha).toLocaleDateString()} {new Date(compra.fecha).toLocaleTimeString()}</Table.Td>
                                    <Table.Td>{compra.productos}</Table.Td>
                                    <Table.Td>${compra.total.toFixed(2)}</Table.Td>
                                </Table.Tr>
                            ))
                        ) : (
                            <Table.Tr><Table.Td colSpan={3} align="center">Este cliente no ha realizado compras.</Table.Td></Table.Tr>
                        )}
                    </Table.Tbody>
                </Table>
            </Modal>

            <Group justify="space-between" mb="md">
                <Title order={2}>Gestión de Clientes</Title>
                <Group><ImportarContactos kind="clientes" onImported={cargarClientes} /><Button onClick={handleAbrirModalCrear}>Añadir Cliente</Button></Group>
            </Group>
            <Table striped highlightOnHover withTableBorder>
                <Table.Thead>
                    <Table.Tr>
                        <Table.Th>Nombre</Table.Th>
                        <Table.Th>RFC</Table.Th>
                        <Table.Th>Teléfono</Table.Th>
                        <Table.Th>Email</Table.Th>
                        <Table.Th>Puntos</Table.Th>
                        <Table.Th>Acciones</Table.Th>
                    </Table.Tr>
                </Table.Thead>
                <Table.Tbody>{rows.length > 0 ? rows : <Table.Tr><Table.Td colSpan={6} align="center">No hay clientes registrados.</Table.Td></Table.Tr>}</Table.Tbody>
            </Table>
        </Paper>
    );
}
