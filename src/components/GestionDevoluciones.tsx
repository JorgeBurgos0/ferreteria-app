import { useState } from 'react';
import { TextInput, Button, Paper, Title, Table, NumberInput, Group, Text, Stack } from '@mantine/core';
import { IconSearch, IconArrowBackUp } from '@tabler/icons-react';
import { invoke } from '../lib/invoke';

interface ItemVentaDevolucion {
    id_producto: number;
    nombre_producto: string;
    cantidad_vendida: number;
    precio_unitario: number;
    cantidad_devuelta_previamente: number;
}

interface VentaParaDevolucion {
    id: number;
    fecha_hora: string;
    total_venta: number;
    items: ItemVentaDevolucion[];
}

interface ItemDevolucionInput {
    id_producto: number;
    cantidad: number;
    precio_reembolso: number;
}

export function GestionDevoluciones() {
    const [idVentaBusqueda, setIdVentaBusqueda] = useState('');
    const [venta, setVenta] = useState<VentaParaDevolucion | null>(null);
    const [cantidadesADevolver, setCantidadesADevolver] = useState<Record<number, number>>({});
    const [motivo, setMotivo] = useState('');

    const handleBuscarVenta = async () => {
        if (!idVentaBusqueda) return;
        try {
            const resultado: VentaParaDevolucion = await invoke('buscar_venta_para_devolucion', {
                idVenta: parseInt(idVentaBusqueda)
            });

            // Validar fecha (Límite de 30 días)
            const fechaVenta = new Date(resultado.fecha_hora);
            const fechaActual = new Date();
            const diferenciaTiempo = fechaActual.getTime() - fechaVenta.getTime();
            const diasDiferencia = diferenciaTiempo / (1000 * 3600 * 24);

            if (diasDiferencia > 30) {
                alert(`Esta venta fue realizada hace ${Math.floor(diasDiferencia)} días. El límite para devoluciones es de 30 días.`);
                setVenta(null);
                return;
            }

            setVenta(resultado);
            setCantidadesADevolver({}); // Reset inputs
        } catch (error) {
            console.error(error);
            alert(`No se encontró la venta o hubo un error: ${error}`);
            setVenta(null);
        }
    };

    const handleCantidadChange = (idProducto: number, cantidad: number) => {
        setCantidadesADevolver(prev => ({
            ...prev,
            [idProducto]: cantidad
        }));
    };

    const handleProcesarDevolucion = async () => {
        if (!venta) return;

        const items: ItemDevolucionInput[] = Object.entries(cantidadesADevolver)
            .filter(([_, cantidad]) => cantidad > 0)
            .map(([idProductoStr, cantidad]) => {
                const idProducto = parseInt(idProductoStr);
                const itemOriginal = venta.items.find(i => i.id_producto === idProducto);
                return {
                    id_producto: idProducto,
                    cantidad,
                    precio_reembolso: itemOriginal ? itemOriginal.precio_unitario : 0
                };
            });

        if (items.length === 0) {
            alert('Seleccione al menos un producto para devolver.');
            return;
        }

        try {
            await invoke('procesar_devolucion', {
                idVenta: venta.id,
                items,
                motivo: motivo || 'Sin motivo especificado'
            });

            alert('Devolución procesada correctamente.');
            setVenta(null);
            setIdVentaBusqueda('');
            setMotivo('');
            setCantidadesADevolver({});
        } catch (error) {
            console.error(error);
            alert(`Error al procesar devolución: ${error}`);
        }
    };

    const rows = venta?.items.map((item) => {
        const disponibleParaDevolver = item.cantidad_vendida - item.cantidad_devuelta_previamente;
        const cantidadSeleccionada = cantidadesADevolver[item.id_producto] || 0;

        return (
            <Table.Tr key={item.id_producto}>
                <Table.Td>{item.nombre_producto}</Table.Td>
                <Table.Td>{item.cantidad_vendida}</Table.Td>
                <Table.Td>{item.cantidad_devuelta_previamente}</Table.Td>
                <Table.Td>${item.precio_unitario.toFixed(2)}</Table.Td>
                <Table.Td>
                    <NumberInput
                        min={0}
                        max={disponibleParaDevolver}
                        value={cantidadSeleccionada}
                        onChange={(val) => handleCantidadChange(item.id_producto, Number(val))}
                        disabled={disponibleParaDevolver <= 0}
                    />
                </Table.Td>
                <Table.Td>${(cantidadSeleccionada * item.precio_unitario).toFixed(2)}</Table.Td>
            </Table.Tr>
        );
    });

    const totalReembolso = venta ? Object.entries(cantidadesADevolver).reduce((sum, [id, cant]) => {
        const item = venta.items.find(i => i.id_producto === parseInt(id));
        return sum + (cant * (item?.precio_unitario || 0));
    }, 0) : 0;

    return (
        <Stack>
            <Title order={2}>Gestión de Devoluciones</Title>

            <Paper p="md" shadow="xs" withBorder>
                <Group align="flex-end">
                    <TextInput
                        label="ID de Venta"
                        placeholder="Ej. 123"
                        value={idVentaBusqueda}
                        onChange={(e) => setIdVentaBusqueda(e.currentTarget.value)}
                        onKeyDown={(e) => e.key === 'Enter' && handleBuscarVenta()}
                    />
                    <Button onClick={handleBuscarVenta} leftSection={<IconSearch size="1rem" />}>
                        Buscar Venta
                    </Button>
                </Group>
            </Paper>

            {venta && (
                <Paper p="md" shadow="xs" withBorder>
                    <Group mb="md" justify="space-between">
                        <Text fw={700} size="lg">Venta #{venta.id} - {new Date(venta.fecha_hora).toLocaleString()}</Text>
                        <Text>Total Venta: ${venta.total_venta.toFixed(2)}</Text>
                    </Group>

                    <Table striped highlightOnHover withTableBorder>
                        <Table.Thead>
                            <Table.Tr>
                                <Table.Th>Producto</Table.Th>
                                <Table.Th>Cant. Vendida</Table.Th>
                                <Table.Th>Prev. Devuelto</Table.Th>
                                <Table.Th>Precio Unit.</Table.Th>
                                <Table.Th>Cant. a Devolver</Table.Th>
                                <Table.Th>Subtotal Reembolso</Table.Th>
                            </Table.Tr>
                        </Table.Thead>
                        <Table.Tbody>{rows}</Table.Tbody>
                    </Table>

                    <Group mt="xl" justify="flex-end" align="flex-end">
                        <TextInput
                            label="Motivo de Devolución"
                            placeholder="Ej. Producto defectuoso"
                            value={motivo}
                            onChange={(e) => setMotivo(e.currentTarget.value)}
                            style={{ flex: 1 }}
                        />
                        <Stack gap="xs" align="flex-end">
                            <Text size="sm" c="dimmed">Total a Reembolsar</Text>
                            <Text size="xl" fw={700} c="red">${totalReembolso.toFixed(2)}</Text>
                        </Stack>
                        <Button
                            color="red"
                            size="md"
                            leftSection={<IconArrowBackUp size="1rem" />}
                            onClick={handleProcesarDevolucion}
                            disabled={totalReembolso <= 0}
                        >
                            Confirmar Devolución
                        </Button>
                    </Group>
                </Paper>
            )}
        </Stack>
    );
}
