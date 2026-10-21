// src/components/PuntoDeVenta.tsx
import { useState, useEffect } from 'react';
import { Button, Paper, Title, Table, Group, Text, NumberInput, ActionIcon, Stack, Select, Modal, SegmentedControl, Switch, Alert } from '@mantine/core';
import { IconTrash, IconShoppingCart, IconPrinter, IconFileText, IconAlertCircle } from '@tabler/icons-react';
import { invoke } from '../lib/invoke';
import { TicketImprimible, DatosVentaTicket } from './TicketImprimible';
import { notifications } from '@mantine/notifications';

interface Producto {
    id: number;
    sku: string;
    nombre: string;
    descripcion: string;
    precio_venta: number;
    precio_mayoreo?: number;
    stock_actual: number;
}

interface ItemCarrito extends Producto {
    cantidad: number;
}

interface Cliente {
    id: number;
    nombre: string;
    rfc: string;
}

interface DatosEmpresa {
    nombre: string;
    direccion: string;
    telefono: string;
    logo: string;
}

interface PuntoDeVentaProps {
    userId?: number;
}

export function PuntoDeVenta({ userId }: PuntoDeVentaProps) {
    const [productos, setProductos] = useState<Producto[]>([]);
    const [carrito, setCarrito] = useState<ItemCarrito[]>([]);
    const [total, setTotal] = useState(0);
    const [skuInput, setSkuInput] = useState('');
    const [datosUltimaVenta, setDatosUltimaVenta] = useState<DatosVentaTicket | null>(null);

    // Estados para clientes
    const [clientes, setClientes] = useState<Cliente[]>([]);
    const [clienteSeleccionadoId, setClienteSeleccionadoId] = useState<string | null>(null);

    // Datos Empresa
    const [datosEmpresa, setDatosEmpresa] = useState<DatosEmpresa | undefined>(undefined);

    // Estados para Pagos
    const [pagoModalOpen, setPagoModalOpen] = useState(false);
    const [metodoPago, setMetodoPago] = useState('Efectivo');
    const [pagoCon, setPagoCon] = useState<number | string>('');
    const [montoEfectivo, setMontoEfectivo] = useState<number>(0);
    const [montoTarjeta, setMontoTarjeta] = useState<number>(0);

    // Nuevo estado para Precio Mayoreo
    const [usarMayoreo, setUsarMayoreo] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        cargarProductos();
        cargarClientes();
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

    // Lógica del Escáner de Códigos de Barras
    useEffect(() => {
        let buffer = '';
        let timeout: number | null = null;

        const handleKeyDown = (e: KeyboardEvent) => {
            // Ignorar si el foco está en un input
            if (document.activeElement instanceof HTMLInputElement || document.activeElement instanceof HTMLTextAreaElement) {
                return;
            }

            if (e.key === 'Enter') {
                if (buffer.length > 0) {
                    procesarCodigoBarras(buffer);
                    buffer = '';
                }
            } else if (e.key.length === 1) {
                buffer += e.key;

                // Reiniciar buffer si pasa mucho tiempo entre teclas (tecleo manual vs escáner)
                if (timeout) clearTimeout(timeout);
                timeout = setTimeout(() => {
                    buffer = '';
                }, 100) as unknown as number;
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => {
            window.removeEventListener('keydown', handleKeyDown);
            if (timeout) clearTimeout(timeout);
        };
    }, [productos, carrito]);

    const procesarCodigoBarras = (codigo: string) => {
        const producto = productos.find(p => p.sku === codigo);
        if (producto) {
            agregarAlCarrito(producto);
        }
    };

    const agregarAlCarrito = (producto: Producto) => {
        if (producto.stock_actual <= 0) {
            setError(`Producto "${producto.nombre}" sin stock.`);
            return;
        }

        const itemExistente = carrito.find(item => item.id === producto.id);
        if (itemExistente) {
            if (itemExistente.cantidad < producto.stock_actual) {
                setCarrito(prev => prev.map(item => item.id === producto.id ? { ...item, cantidad: item.cantidad + 1 } : item));
            } else {
                setError(`No hay más stock para "${producto.nombre}".`);
            }
        } else {
            setCarrito(prev => [...prev, { ...producto, cantidad: 1 }]);
        }
        setError(null); // Limpiar errores al agregar con éxito
    };

    useEffect(() => {
        const nuevoTotal = carrito.reduce((acc, item) => {
            const precio = usarMayoreo && item.precio_mayoreo ? item.precio_mayoreo : item.precio_venta;
            return acc + (precio * item.cantidad);
        }, 0);
        setTotal(nuevoTotal);
    }, [carrito, usarMayoreo]);

    const cargarProductos = async () => {
        try {
            const lista: Producto[] = await invoke('obtener_productos');
            setProductos(lista);
        } catch (error) {
            console.error("Error al cargar productos:", error);
        }
    };

    const cargarClientes = async () => {
        try {
            const lista: Cliente[] = await invoke('obtener_clientes');
            setClientes(lista);
        } catch (error) {
            console.error("Error al cargar clientes:", error);
        }
    };

    const handleProductoSelect = (idProductoStr: string | null) => {
        if (!idProductoStr) return;
        const idProducto = parseInt(idProductoStr);
        const producto = productos.find(p => p.id === idProducto);

        if (producto) {
            agregarAlCarrito(producto);
            setSkuInput(''); // Limpiar el select
        }
    };

    const handleEliminarDelCarrito = (id: number) => {
        setCarrito(carrito.filter(item => item.id !== id));
    };

    const handleCambiarCantidad = (id: number, cantidad: number) => {
        if (cantidad <= 0) {
            handleEliminarDelCarrito(id);
            return;
        }

        const productoOriginal = productos.find(p => p.id === id);
        if (productoOriginal && cantidad > productoOriginal.stock_actual) {
            setError(`Solo hay ${productoOriginal.stock_actual} unidades disponibles.`);
            return;
        }

        setCarrito(carrito.map(item => item.id === id ? { ...item, cantidad } : item));
    };

    const handleAbrirModalPago = () => {
        if (carrito.length === 0) {
            setError("El carrito está vacío.");
            return;
        }
        setPagoCon('');
        setMontoEfectivo(0);
        setMontoTarjeta(0);
        setMetodoPago('Efectivo');
        setPagoModalOpen(true);
    };

    const handleConfirmarPago = async () => {
        let metodoPagoString = metodoPago;

        if (metodoPago === 'Efectivo') {
            const pago = Number(pagoCon);
            if (pago < total) {
                notifications.show({ title: 'Error de Pago', message: "El monto recibido es menor al total.", color: 'red' });
                return;
            }
            metodoPagoString = `Efectivo (Recibido: $${pago.toFixed(2)}, Cambio: $${(pago - total).toFixed(2)})`;
        } else if (metodoPago === 'Mixto') {
            if (Math.abs((montoEfectivo + montoTarjeta) - total) > 0.01) {
                notifications.show({ title: 'Error de Pago', message: `La suma de pagos ($${(montoEfectivo + montoTarjeta).toFixed(2)}) no coincide con el total ($${total.toFixed(2)}).`, color: 'red' });
                return;
            }
            metodoPagoString = `Mixto (Efec: $${montoEfectivo.toFixed(2)}, Tarj: $${montoTarjeta.toFixed(2)})`;
        }

        const itemsVenta = carrito.map(item => ({
            id_producto: item.id,
            cantidad: item.cantidad
        }));

        try {
            const idVenta: number = await invoke('realizar_venta', {
                items: itemsVenta,
                id_cliente: clienteSeleccionadoId ? parseInt(clienteSeleccionadoId) : null,
                id_usuario: userId, // Enviar ID de usuario
                metodoPago,
                montoEfectivo: metodoPago === 'Efectivo' ? total : metodoPago === 'Mixto' ? montoEfectivo : 0,
                montoTarjeta: metodoPago === 'Tarjeta' ? total : metodoPago === 'Mixto' ? montoTarjeta : 0,
                aplicarMayoreo: usarMayoreo
            });

            setPagoModalOpen(false);

            // Preparar datos para el ticket
            const clienteObj = clientes.find(c => c.id.toString() === clienteSeleccionadoId);
            setDatosUltimaVenta({
                id: idVenta,
                fecha: new Date().toISOString(),
                items: carrito.map(item => ({
                    cantidad: item.cantidad,
                    nombre: item.nombre,
                    precio: usarMayoreo && item.precio_mayoreo != null ? item.precio_mayoreo : item.precio_venta
                })),
                total: total,
                metodoPago: metodoPagoString,
                nombre_cliente: clienteObj ? clienteObj.nombre : undefined
            });

            setCarrito([]);
            setClienteSeleccionadoId(null);
            cargarProductos(); // Recargar stock
        } catch (error) {
            console.error(error);
            setError(`Error al realizar venta: ${error} `);
        }
    };

    const handleGuardarCotizacion = async () => {
        if (carrito.length === 0) {
            setError("El carrito está vacío.");
            return;
        }

        const itemsCotizacion = carrito.map(item => ({
            id_producto: item.id,
            cantidad: item.cantidad,
            precio_unitario: usarMayoreo && item.precio_mayoreo ? item.precio_mayoreo : item.precio_venta
        }));

        try {
            await invoke('crear_cotizacion', {
                idCliente: clienteSeleccionadoId ? parseInt(clienteSeleccionadoId) : null,
                items: itemsCotizacion,
                total: total
            });
            notifications.show({ title: 'Cotización', message: "Cotización guardada exitosamente.", color: 'green' });
            setCarrito([]);
            setClienteSeleccionadoId(null);
        } catch (error) {
            console.error("Error al guardar cotización:", error);
            setError(`Error al guardar cotización: ${error}`);
        }
    };

    const handleImprimirTicket = () => {
        window.print();
    };

    const rows = carrito.map((item) => {
        const precioAplicado = usarMayoreo && item.precio_mayoreo ? item.precio_mayoreo : item.precio_venta;
        return (
            <Table.Tr key={item.id}>
                <Table.Td>{item.nombre}</Table.Td>
                <Table.Td>
                    ${precioAplicado.toFixed(2)}
                    {usarMayoreo && item.precio_mayoreo && <Text span size="xs" c="dimmed" ml={4}>(Mayoreo)</Text>}
                </Table.Td>
                <Table.Td>
                    <NumberInput
                        value={item.cantidad}
                        onChange={(val) => handleCambiarCantidad(item.id, Number(val))}
                        min={1}
                        max={item.stock_actual}
                        allowNegative={false}
                    />
                </Table.Td>
                <Table.Td>${(precioAplicado * item.cantidad).toFixed(2)}</Table.Td>
                <Table.Td>
                    <ActionIcon color="red" onClick={() => handleEliminarDelCarrito(item.id)}>
                        <IconTrash size="1rem" />
                    </ActionIcon>
                </Table.Td>
            </Table.Tr>
        );
    });

    return (
        <Stack>
            <Group justify="space-between">
                <Title order={2}>Punto de Venta</Title>
                <Switch
                    label="Precio Mayoreo"
                    checked={usarMayoreo}
                    onChange={(event) => setUsarMayoreo(event.currentTarget.checked)}
                    size="md"
                />
            </Group>

            <Paper p="md" shadow="xs" withBorder>
                {error && <Alert icon={<IconAlertCircle size="1rem" />} title="Atención" color="red" withCloseButton onClose={() => setError(null)} mb="md">{error}</Alert>}
                <Group align="flex-end" mb="md">
                    <Select
                        label="Buscar Producto"
                        placeholder="Escribe nombre o SKU"
                        data={productos.map(p => ({ value: p.id.toString(), label: `${p.nombre} - $${p.precio_venta} (Stock: ${p.stock_actual})` }))}
                        searchable
                        nothingFoundMessage="No encontrado"
                        value={skuInput}
                        onChange={handleProductoSelect}
                        style={{ flex: 1 }}
                    />
                    <Select
                        label="Cliente (Opcional)"
                        placeholder="Seleccionar cliente"
                        data={clientes.map(c => ({ value: c.id.toString(), label: c.nombre }))}
                        searchable
                        clearable
                        value={clienteSeleccionadoId}
                        onChange={setClienteSeleccionadoId}
                        style={{ width: 250 }}
                    />
                </Group>

                <Table striped highlightOnHover withTableBorder>
                    <Table.Thead>
                        <Table.Tr>
                            <Table.Th>Producto</Table.Th>
                            <Table.Th>Precio</Table.Th>
                            <Table.Th>Cantidad</Table.Th>
                            <Table.Th>Subtotal</Table.Th>
                            <Table.Th></Table.Th>
                        </Table.Tr>
                    </Table.Thead>
                    <Table.Tbody>{rows.length > 0 ? rows : <Table.Tr><Table.Td colSpan={5} style={{ textAlign: 'center' }}>Carrito vacío</Table.Td></Table.Tr>}</Table.Tbody>
                </Table>

                <Group justify="space-between" mt="xl">
                    <Title order={3}>Total: ${total.toFixed(2)}</Title>
                    <Group>
                        <Button leftSection={<IconFileText size="1rem" />} variant="light" color="blue" onClick={handleGuardarCotizacion}>
                            Guardar Cotización
                        </Button>
                        {datosUltimaVenta && (
                            <Button leftSection={<IconPrinter size="1rem" />} variant="outline" onClick={handleImprimirTicket}>
                                Imprimir Último Ticket
                            </Button>
                        )}
                        <Button leftSection={<IconShoppingCart size="1rem" />} size="md" onClick={handleAbrirModalPago}>
                            Cobrar
                        </Button>
                    </Group>
                </Group>
            </Paper>

            {datosUltimaVenta && (
                <TicketImprimible datos={datosUltimaVenta} datosEmpresa={datosEmpresa} />
            )}

            <Modal opened={pagoModalOpen} onClose={() => setPagoModalOpen(false)} title={<Title order={3}>Procesar Pago</Title>} centered>
                <Stack>
                    <Title order={4} ta="center">Total a Pagar: ${total.toFixed(2)}</Title>

                    <SegmentedControl
                        value={metodoPago}
                        onChange={setMetodoPago}
                        data={[
                            { label: 'Efectivo', value: 'Efectivo' },
                            { label: 'Tarjeta', value: 'Tarjeta' },
                            { label: 'Mixto', value: 'Mixto' },
                        ]}
                    />

                    {metodoPago === 'Efectivo' && (
                        <>
                            <NumberInput
                                label="Monto Recibido"
                                placeholder="0.00"
                                prefix="$"
                                value={pagoCon}
                                onChange={setPagoCon}
                                min={0}
                            />
                            <Group justify="space-between">
                                <Text fw={700}>Cambio:</Text>
                                <Text fw={700} size="xl" c={Number(pagoCon) >= total ? 'teal' : 'red'}>
                                    ${(Number(pagoCon) - total).toFixed(2)}
                                </Text>
                            </Group>
                        </>
                    )}

                    {metodoPago === 'Mixto' && (
                        <>
                            <NumberInput
                                label="Monto en Efectivo"
                                prefix="$"
                                value={montoEfectivo}
                                onChange={(val) => setMontoEfectivo(Number(val))}
                                min={0}
                            />
                            <NumberInput
                                label="Monto en Tarjeta"
                                prefix="$"
                                value={montoTarjeta}
                                onChange={(val) => setMontoTarjeta(Number(val))}
                                min={0}
                            />
                            <Text size="sm" c={Math.abs((montoEfectivo + montoTarjeta) - total) < 0.01 ? 'teal' : 'red'}>
                                Restante: ${(total - (montoEfectivo + montoTarjeta)).toFixed(2)}
                            </Text>
                        </>
                    )}

                    <Button fullWidth size="md" onClick={handleConfirmarPago} mt="md">
                        Confirmar Pago
                    </Button>
                </Stack>
            </Modal>
        </Stack>
    );
}
