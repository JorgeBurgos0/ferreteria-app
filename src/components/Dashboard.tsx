import { useEffect, useState } from 'react';
import { Paper, Text, Title, Group, ThemeIcon, Button, SimpleGrid, Grid } from '@mantine/core';
import { IconCurrencyDollar, IconAlertTriangle, IconUsers, IconShoppingCart, IconBox, IconTrash } from '@tabler/icons-react';
import { invoke } from '../lib/invoke';
import { useNavigate } from 'react-router-dom';
import { BarChart, DonutChart } from '@mantine/charts';
import { notifications } from '@mantine/notifications';

interface DashboardResumen {
    ventas_hoy: number;
    items_bajo_stock: number;
    cantidad_clientes: number;
}

interface ProductoMasVendido {
    nombre_producto: string;
    cantidad_total: number;
    total_ingresos: number;
}

interface VentaCategoria {
    categoria: string;
    total_ventas: number;
    cantidad_items: number;
}

interface AnalyticsWidget {
    id: number;
    nombre: string;
    tipo_reporte: string;
    configuracion: string;
    orden: number;
    created_at: string;
}

export function Dashboard({ canSell, canManageInventory }: { canSell: boolean; canManageInventory: boolean }) {
    const [resumen, setResumen] = useState<DashboardResumen | null>(null);
    const [topProductos, setTopProductos] = useState<ProductoMasVendido[]>([]);
    const [topVendedores, setTopVendedores] = useState<any[]>([]); // New state
    const [ventasCategoria, setVentasCategoria] = useState<VentaCategoria[]>([]);
    const [customWidgets, setCustomWidgets] = useState<AnalyticsWidget[]>([]);
    const [widgetsData, setWidgetsData] = useState<Record<number, any[]>>({});
    const navigate = useNavigate();

    useEffect(() => {
        cargarResumen();
    }, []);

    const cargarResumen = async () => {
        try {
            const data: DashboardResumen = await invoke('obtener_resumen_dashboard');
            setResumen(data);

            const hoy = new Date();
            const hace7dias = new Date();
            hace7dias.setDate(hace7dias.getDate() - 30); // Últimos 30 días para charts

            const inicio = hace7dias.toISOString().split('T')[0];
            const fin = hoy.toISOString().split('T')[0];

            const top: ProductoMasVendido[] = await invoke('obtener_productos_mas_vendidos', { fechaInicio: inicio, fechaFin: fin });
            setTopProductos(top);

            try {
                const vendedores: any[] = await invoke('obtener_top_entidades', { entidad: 'vendedor', fecha_inicio: inicio, fecha_fin: fin });
                setTopVendedores(vendedores);
            } catch (e) { console.error("Error cargando vendedores", e); }

            const cat: VentaCategoria[] = await invoke('obtener_ventas_por_categoria', { fechaInicio: inicio, fechaFin: fin });
            setVentasCategoria(cat);

            await cargarCustomWidgets();

        } catch (error) {
            console.error("Error al cargar dashboard:", error);
        }
    };

    const cargarCustomWidgets = async () => {
        try {
            const widgets: AnalyticsWidget[] = await invoke('obtener_widgets_analytics');
            setCustomWidgets(widgets);

            // Load data for each widget
            const dataMap: Record<number, any[]> = {};
            for (const widget of widgets) {
                const config = JSON.parse(widget.configuracion);
                try {
                    let data: any[] = [];
                    if (widget.tipo_reporte === 'ventas-tiempo') {
                        data = await invoke('obtener_historial_ventas', {
                            fechaInicio: config.fechaInicio,
                            fechaFin: config.fechaFin,
                            agrupacion: config.agrupacion || 'dia'
                        });
                    } else if (widget.tipo_reporte === 'top-entidades') {
                        data = await invoke('obtener_top_entidades', {
                            entidad: config.entidad || 'cliente',
                            fechaInicio: config.fechaInicio,
                            fechaFin: config.fechaFin
                        });
                    } else if (widget.tipo_reporte === 'nuevos-clientes') {
                        data = await invoke('obtener_nuevos_clientes', {
                            fechaInicio: config.fechaInicio,
                            fechaFin: config.fechaFin,
                            agrupacion: config.agrupacion || 'dia'
                        });
                    }
                    dataMap[widget.id] = data;
                } catch (e) {
                    console.error(`Error loading data for widget ${widget.id}:`, e);
                    dataMap[widget.id] = [];
                }
            }
            setWidgetsData(dataMap);
        } catch (error) {
            console.error("Error al cargar custom widgets:", error);
        }
    };

    const handleEliminarWidget = async (id: number) => {
        try {
            await invoke('eliminar_widget_analytics', { id });
            notifications.show({
                title: 'Éxito',
                message: 'Widget eliminado del Dashboard.',
                color: 'green'
            });
            await cargarCustomWidgets();
        } catch (error) {
            console.error("Error al eliminar widget:", error);
            notifications.show({ title: 'Error', message: String(error), color: 'red' });
        }
    };

    return (
        <div>
            <Title order={2} mb="xl">Panel de Control</Title>

            <SimpleGrid cols={{ base: 1, sm: 3 }} spacing="lg" mb="xl">
                <Paper p="md" shadow="xs" radius="md" withBorder>
                    <Group>
                        <ThemeIcon size="xl" radius="md" color="green" variant="light">
                            <IconCurrencyDollar size="1.5rem" />
                        </ThemeIcon>
                        <div>
                            <Text c="dimmed" size="xs" tt="uppercase" fw={700}>
                                Ventas de Hoy
                            </Text>
                            <Text fw={700} size="xl">
                                ${resumen?.ventas_hoy.toFixed(2) ?? '0.00'}
                            </Text>
                        </div>
                    </Group>
                </Paper>

                <Paper p="md" shadow="xs" radius="md" withBorder>
                    <Group>
                        <ThemeIcon size="xl" radius="md" color="orange" variant="light">
                            <IconAlertTriangle size="1.5rem" />
                        </ThemeIcon>
                        <div>
                            <Text c="dimmed" size="xs" tt="uppercase" fw={700}>
                                Alertas de Stock
                            </Text>
                            <Text fw={700} size="xl">
                                {resumen?.items_bajo_stock ?? 0}
                            </Text>
                        </div>
                    </Group>
                </Paper>

                <Paper p="md" shadow="xs" radius="md" withBorder>
                    <Group>
                        <ThemeIcon size="xl" radius="md" color="blue" variant="light">
                            <IconUsers size="1.5rem" />
                        </ThemeIcon>
                        <div>
                            <Text c="dimmed" size="xs" tt="uppercase" fw={700}>
                                Clientes Registrados
                            </Text>
                            <Text fw={700} size="xl">
                                {resumen?.cantidad_clientes ?? 0}
                            </Text>
                        </div>
                    </Group>
                </Paper>
            </SimpleGrid>

            {topProductos.length > 0 && (
                <Grid mb="xl">
                    <Grid.Col span={{ base: 12, md: 8 }}>
                        <Paper p="md" shadow="xs" radius="md" withBorder>
                            <Title order={4} mb="md">Top Productos (Últimos 30 días)</Title>
                            <BarChart
                                h={300}
                                data={topProductos}
                                dataKey="nombre_producto"
                                series={[
                                    { name: 'cantidad_total', color: 'blue', label: 'Unidades Vendidas' },
                                    { name: 'total_ingresos', color: 'teal', label: 'Ingresos ($)' }
                                ]}
                                tickLine="y"
                            />
                        </Paper>
                    </Grid.Col>
                    <Grid.Col span={{ base: 12, md: 4 }}>
                        <Paper p="md" shadow="xs" radius="md" withBorder>
                            <Title order={4} mb="md">Ventas por Categoría</Title>
                            <DonutChart
                                withLabelsLine
                                withLabels
                                data={ventasCategoria.map(c => ({ name: c.categoria, value: c.total_ventas, color: 'indigo' }))}
                                tooltipDataSource="segment"
                            />
                        </Paper>
                    </Grid.Col>
                </Grid>
            )}

            {topVendedores.length > 0 && (
                <Paper p="md" shadow="xs" radius="md" withBorder mb="xl">
                    <Title order={4} mb="md">Top Vendedores (Últimos 30 días)</Title>
                    <BarChart
                        h={300}
                        data={topVendedores}
                        dataKey="nombre"
                        series={[
                            { name: 'total_ventas', color: 'grape', label: 'Ventas ($)' }
                        ]}
                        tickLine="y"
                    />
                </Paper>
            )}

            {/* Custom Analytics Widgets */}
            {customWidgets.length > 0 && (
                <div style={{ marginBottom: '2rem' }}>
                    <Title order={3} mb="md" mt="xl">Mis Gráficas Guardadas</Title>
                    <Grid mb="xl">
                        {customWidgets.map((widget) => {
                            const data = widgetsData[widget.id] || [];
                            const config = JSON.parse(widget.configuracion);

                            let dataKey = '';
                            let label = '';

                            if (widget.tipo_reporte === 'top-entidades') {
                                dataKey = 'nombre';
                                label = 'Total ($)';
                            } else if (widget.tipo_reporte === 'nuevos-clientes') {
                                dataKey = 'fecha';
                                label = 'Nuevos Clientes';
                            } else if (widget.tipo_reporte === 'ventas-tiempo') {
                                dataKey = 'fecha';
                                label = config.metrica === 'total' ? 'Total ($)' : 'Cantidad';
                            }

                            const series = [
                                {
                                    name: widget.tipo_reporte === 'top-entidades' ? 'total_ventas' :
                                        widget.tipo_reporte === 'nuevos-clientes' ? 'cantidad' :
                                            (config.metrica === 'total' ? 'total_ventas' : 'cantidad_items'),
                                    color: 'violet',
                                    label
                                }
                            ];

                            return (
                                <Grid.Col key={widget.id} span={{ base: 12, md: 6 }}>
                                    <Paper p="md" shadow="xs" radius="md" withBorder>
                                        <Group justify="space-between" mb="md">
                                            <Title order={5}>{widget.nombre}</Title>
                                            <Button
                                                size="xs"
                                                color="red"
                                                variant="subtle"
                                                leftSection={<IconTrash size={14} />}
                                                onClick={() => handleEliminarWidget(widget.id)}
                                            >
                                                Eliminar
                                            </Button>
                                        </Group>
                                        {data.length > 0 ? (
                                            <BarChart
                                                h={250}
                                                data={data}
                                                dataKey={dataKey}
                                                series={series}
                                                tickLine="y"
                                            />
                                        ) : (
                                            <Text c="dimmed" ta="center" py="xl">
                                                No hay datos disponibles
                                            </Text>
                                        )}
                                    </Paper>
                                </Grid.Col>
                            );
                        })}
                    </Grid>
                </div>
            )}

            <Title order={4} mb="md">Acciones Rápidas</Title>
            <Group>
                {canSell && <Button
                    size="lg"
                    leftSection={<IconShoppingCart />}
                    onClick={() => navigate('/punto-venta')}
                >
                    Ir al Punto de Venta
                </Button>}
                {canManageInventory && <Button
                    size="lg"
                    variant="light"
                    leftSection={<IconBox />}
                    onClick={() => navigate('/inventario')}
                >
                    Gestionar Inventario
                </Button>}
            </Group>
        </div>
    );
}
