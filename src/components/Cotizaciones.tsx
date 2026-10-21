import { useState, useEffect } from 'react';
import { invoke } from '../lib/invoke';
import { Table, Button, Group, Title, Paper } from '@mantine/core';
import { IconTrash } from '@tabler/icons-react';

interface Cotizacion {
    id: number;
    fecha_hora: string;
    id_cliente: number | null;
    total_cotizacion: number;
    vigencia: string | null;
}

export default function Cotizaciones() {
    const [cotizaciones, setCotizaciones] = useState<Cotizacion[]>([]);

    const cargarCotizaciones = async () => {
        try {
            const datos: Cotizacion[] = await invoke('obtener_cotizaciones');
            setCotizaciones(datos);
        } catch (error) {
            console.error("Error al cargar cotizaciones:", error);
        }
    };

    useEffect(() => {
        cargarCotizaciones();
    }, []);

    const handleEliminar = async (id: number) => {
        if (confirm('¿Estás seguro de eliminar esta cotización?')) {
            try {
                await invoke('eliminar_cotizacion', { id });
                cargarCotizaciones();
            } catch (error) {
                console.error("Error al eliminar cotización:", error);
                alert("Error al eliminar cotización");
            }
        }
    };

    const rows = cotizaciones.map((cot) => (
        <Table.Tr key={cot.id}>
            <Table.Td>{cot.id}</Table.Td>
            <Table.Td>{new Date(cot.fecha_hora).toLocaleString()}</Table.Td>
            <Table.Td>{cot.id_cliente ? `Cliente #${cot.id_cliente}` : 'Público General'}</Table.Td>
            <Table.Td>${cot.total_cotizacion.toFixed(2)}</Table.Td>
            <Table.Td>
                <Group gap="xs">
                    <Button size="xs" color="red" variant="subtle" onClick={() => handleEliminar(cot.id)}><IconTrash size={16} /></Button>
                </Group>
            </Table.Td>
        </Table.Tr>
    ));

    return (
        <Paper p="md" shadow="xs">
            <Title order={2} mb="md">Historial de Cotizaciones</Title>
            <Table striped highlightOnHover withTableBorder>
                <Table.Thead>
                    <Table.Tr>
                        <Table.Th>ID</Table.Th>
                        <Table.Th>Fecha</Table.Th>
                        <Table.Th>Cliente</Table.Th>
                        <Table.Th>Total</Table.Th>
                        <Table.Th>Acciones</Table.Th>
                    </Table.Tr>
                </Table.Thead>
                <Table.Tbody>{rows}</Table.Tbody>
            </Table>
        </Paper>
    );
}
