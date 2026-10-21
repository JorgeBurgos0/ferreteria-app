import { useState, useEffect } from 'react';
import { invoke } from '../lib/invoke';
import { Table, Button, Group, Title, Paper, Modal, Stack, Text } from '@mantine/core';
import { IconTrash, IconPrinter } from '@tabler/icons-react';

interface Cotizacion {
    id: number;
    fecha_hora: string;
    id_cliente: number | null;
    total_cotizacion: number;
    vigencia: string | null;
}

interface DetalleCotizacion {
    id: number;
    fecha_hora: string;
    vigencia: string | null;
    nombre_cliente: string | null;
    total_cotizacion: number;
    items: { id_producto: number; sku: string; nombre: string; cantidad: number; precio_unitario: number }[];
}

interface Empresa { nombre: string; direccion: string; telefono: string; logo: string }

export default function Cotizaciones() {
    const [cotizaciones, setCotizaciones] = useState<Cotizacion[]>([]);
    const [detalle, setDetalle] = useState<DetalleCotizacion | null>(null);
    const [empresa, setEmpresa] = useState<Empresa | null>(null);

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
        invoke<Empresa>('obtener_datos_empresa').then(setEmpresa).catch(console.error);
    }, []);

    const abrirDetalle = async (id: number) => {
        try {
            setDetalle(await invoke<DetalleCotizacion>('obtener_detalle_cotizacion', { id }));
        } catch (error) {
            alert(`No se pudo abrir la cotización: ${error}`);
        }
    };

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
                    <Button size="xs" variant="light" onClick={() => abrirDetalle(cot.id)}>Ver / imprimir</Button>
                    <Button size="xs" color="red" variant="subtle" aria-label={`Eliminar cotización ${cot.id}`} onClick={() => handleEliminar(cot.id)}><IconTrash size={16} /></Button>
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
            <Modal opened={detalle !== null} onClose={() => setDetalle(null)} title={`Cotización #${detalle?.id ?? ''}`} size="lg">
                {detalle && <Stack>
                    <div id="cotizacion-imprimible">
                        <div style={{ textAlign: 'center', marginBottom: 16 }}>
                            {empresa?.logo && <img src={empresa.logo} alt="Logo" style={{ maxWidth: 90, maxHeight: 70, objectFit: 'contain' }} />}
                            <Title order={3}>{empresa?.nombre?.trim() || 'Ferretería'}</Title>
                            {empresa?.direccion && <Text size="sm">{empresa.direccion}</Text>}
                            {empresa?.telefono && <Text size="sm">Tel: {empresa.telefono}</Text>}
                        </div>
                        <Title order={4}>Cotización #{detalle.id}</Title>
                        <Text size="sm">Fecha: {detalle.fecha_hora}</Text>
                        <Text size="sm">Válida hasta: {detalle.vigencia ?? 'Sin vigencia definida'}</Text>
                        <Text size="sm">Cliente: {detalle.nombre_cliente || 'Público general'}</Text>
                        <Table striped withTableBorder mt="md">
                            <Table.Thead><Table.Tr><Table.Th>Producto</Table.Th><Table.Th>Cant.</Table.Th><Table.Th>Precio</Table.Th><Table.Th>Subtotal</Table.Th></Table.Tr></Table.Thead>
                            <Table.Tbody>{detalle.items.map((item, index) => <Table.Tr key={`${item.id_producto}-${index}`}><Table.Td>{item.nombre} ({item.sku})</Table.Td><Table.Td>{item.cantidad}</Table.Td><Table.Td>${item.precio_unitario.toFixed(2)}</Table.Td><Table.Td>${(item.precio_unitario * item.cantidad).toFixed(2)}</Table.Td></Table.Tr>)}</Table.Tbody>
                        </Table>
                        <Text fw={700} ta="right" mt="md">Total: ${detalle.total_cotizacion.toFixed(2)}</Text>
                        <Text size="xs" mt="md">Esta cotización no reserva inventario. Confirma disponibilidad antes de cobrar.</Text>
                    </div>
                    <Group justify="flex-end"><Button leftSection={<IconPrinter size={16} />} onClick={() => window.print()}>Imprimir</Button></Group>
                </Stack>}
            </Modal>
        </Paper>
    );
}
