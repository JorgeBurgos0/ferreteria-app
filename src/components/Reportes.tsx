
import { useState } from 'react';
import { invoke } from '../lib/invoke';
import { Table, Title, Paper, Alert, Group, Button, Accordion, Text, Tabs, Grid, Select, Modal, TextInput } from '@mantine/core';
import { IconCash, IconChartBar, IconChartPie, IconPrinter, IconArrowBackUp, IconDeviceFloppy } from '@tabler/icons-react';
import { writeSpreadsheet } from '../lib/spreadsheet';
import { save } from '@tauri-apps/plugin-dialog';
import { notifications } from '@mantine/notifications';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';

// --- Interfaces ---
interface Producto {
  id: number;
  sku: string;
  nombre: string;
  stock_actual: number;
}
interface DetalleVenta {
  nombre_producto: string;
  cantidad: number;
  precio_unitario: number;
  subtotal: number;
}
interface VentaCompleta {
  id: number;
  fecha_hora: string;
  total_venta: number;
  metodo_pago: string;
  detalles: DetalleVenta[];
}
interface DetalleDevolucionReporte {
  nombre_producto: string;
  cantidad: number;
  monto_reembolsado: number;
}
interface DevolucionReporte {
  id: number;
  id_venta_original: number;
  fecha_hora: string;
  total_reembolsado: number;
  motivo: string;
  detalles: DetalleDevolucionReporte[];
}
interface DatosHistorial {
  fecha: string;
  total_ventas: number;
  cantidad_items: number;
}
interface ReporteEntidad {
  nombre: string;
  total_ventas: number;
}
interface ReporteNuevosClientes {
  fecha: string;
  cantidad: number;
}

// Función auxiliar para formatear la fecha a YYYY-MM-DD
function formatearFechaParaSQL(fecha: Date): string {
  const anio = fecha.getFullYear();
  const mes = (fecha.getMonth() + 1).toString().padStart(2, '0');
  const dia = fecha.getDate().toString().padStart(2, '0');
  return `${anio}-${mes}-${dia}`;
}

function parsearFechaLocal(valor: string): Date {
  const [anio, mes, dia] = valor.split('-').map(Number);
  return new Date(anio, mes - 1, dia);
}

export function Reportes() {
  const [productosBajoStock, setProductosBajoStock] = useState<Producto[]>([]);
  const [productosSobreStock, setProductosSobreStock] = useState<Producto[]>([]); // Nuevo estado
  const [reporteVentas, setReporteVentas] = useState<VentaCompleta[]>([]);
  const [reporteDevoluciones, setReporteDevoluciones] = useState<DevolucionReporte[]>([]);

  // Analytics State
  const [datosGrafica, setDatosGrafica] = useState<DatosHistorial[]>([]);
  const [datosEntidad, setDatosEntidad] = useState<ReporteEntidad[]>([]);
  const [datosNuevosClientes, setDatosNuevosClientes] = useState<ReporteNuevosClientes[]>([]);

  const [tipoReporte, setTipoReporte] = useState<string | null>('ventas-tiempo'); // 'ventas-tiempo', 'top-entidades', 'nuevos-clientes'
  const [entidad, setEntidad] = useState<string | null>('cliente'); // 'cliente', 'vendedor', 'proveedor'
  const [agrupacion, setAgrupacion] = useState<string | null>('dia');
  const [metrica, setMetrica] = useState<string | null>('total'); // 'total' | 'cantidad'

  const [fechas, setFechas] = useState<[Date | null, Date | null]>(() => {
    const hoy = new Date();
    return [hoy, hoy];
  });
  const [error, setError] = useState<string | null>(null);

  // Save Widget State
  const [modalGuardarAbierto, setModalGuardarAbierto] = useState(false);
  const [nombreWidget, setNombreWidget] = useState('');

  const cargarProductosBajoStock = async () => {
    try {
      const productos: Producto[] = await invoke('obtener_productos_con_bajo_stock');
      setProductosBajoStock(productos);
    } catch (err) {
      console.error(err);
      setError(err as string);
    }
  };

  const cargarProductosSobreStock = async () => {
    try {
      const productos: Producto[] = await invoke('obtener_productos_con_sobre_stock');
      setProductosSobreStock(productos);
    } catch (err) {
      console.error(err);
      setError(err as string);
    }
  };

  const handleGenerarReporteVentas = async () => {
    if (!fechas[0] || !fechas[1]) {
      notifications.show({
        title: 'Atención',
        message: "Por favor, selecciona un rango de fechas.",
        color: 'yellow'
      });
      return;
    }
    try {
      const fechaInicio = formatearFechaParaSQL(fechas[0]);
      const fechaFin = formatearFechaParaSQL(fechas[1]);
      const reporte: VentaCompleta[] = await invoke('generar_reporte_ventas', { fechaInicio, fechaFin });
      setReporteVentas(reporte);
    } catch (err) {
      console.error(err);
      setError(err as string);
    }
  };

  const handleGenerarReporteDevoluciones = async () => {
    if (!fechas[0] || !fechas[1]) {
      notifications.show({ title: 'Atención', message: "Selecciona un rango de fechas.", color: 'yellow' });
      return;
    }
    try {
      const fechaInicio = formatearFechaParaSQL(fechas[0]);
      const fechaFin = formatearFechaParaSQL(fechas[1]);
      const reporte: DevolucionReporte[] = await invoke('generar_reporte_devoluciones', { fechaInicio, fechaFin });
      setReporteDevoluciones(reporte);
    } catch (err) {
      console.error(err);
      setError(String(err));
    }
  };



  const handleGenerarGrafica = async () => {
    if (!fechas[0] || !fechas[1]) {
      notifications.show({ title: 'Atención', message: "Selecciona un rango de fechas.", color: 'yellow' });
      return;
    }
    const fechaInicio = formatearFechaParaSQL(fechas[0]);
    const fechaFin = formatearFechaParaSQL(fechas[1]);

    try {
      if (tipoReporte === 'ventas-tiempo') {
        const datos: DatosHistorial[] = await invoke('obtener_historial_ventas', {
          fechaInicio, fechaFin, agrupacion: agrupacion || 'dia'
        });
        setDatosGrafica(datos);
        setDatosEntidad([]); setDatosNuevosClientes([]);
      } else if (tipoReporte === 'top-entidades') {
        const datos: ReporteEntidad[] = await invoke('obtener_top_entidades', {
          entidad: entidad || 'cliente', fechaInicio, fechaFin
        });
        setDatosEntidad(datos);
        setDatosGrafica([]); setDatosNuevosClientes([]);
      } else if (tipoReporte === 'nuevos-clientes') {
        const datos: ReporteNuevosClientes[] = await invoke('obtener_nuevos_clientes', {
          fechaInicio, fechaFin, agrupacion: agrupacion || 'dia'
        });
        setDatosNuevosClientes(datos);
        setDatosGrafica([]); setDatosEntidad([]);
      }
    } catch (err) {
      setError(String(err));
    }
  };

  const handleAbrirModalGuardar = () => {
    if (!fechas[0] || !fechas[1]) {
      notifications.show({ title: 'Atención', message: "Primero genera una gráfica.", color: 'yellow' });
      return;
    }
    if (datosGrafica.length === 0 && datosEntidad.length === 0 && datosNuevosClientes.length === 0) {
      notifications.show({ title: 'Atención', message: "No hay datos para guardar.", color: 'yellow' });
      return;
    }
    setModalGuardarAbierto(true);
  };

  const handleGuardarWidget = async () => {
    if (!nombreWidget.trim()) {
      notifications.show({ title: 'Error', message: "Ingresa un nombre para el widget.", color: 'red' });
      return;
    }

    try {
      const configuracion = JSON.stringify({
        tipoReporte,
        fechaInicio: fechas[0] ? formatearFechaParaSQL(fechas[0]) : '',
        fechaFin: fechas[1] ? formatearFechaParaSQL(fechas[1]) : '',
        agrupacion,
        metrica,
        entidad
      });

      await invoke('guardar_widget_analytics', {
        widget: {
          nombre: nombreWidget,
          tipoReporte: tipoReporte || 'ventas-tiempo',
          configuracion
        }
      });

      notifications.show({
        title: 'Éxito',
        message: `Widget "${nombreWidget}" guardado en Dashboard.`,
        color: 'green'
      });

      setModalGuardarAbierto(false);
      setNombreWidget('');
    } catch (err) {
      console.error(err);
      notifications.show({ title: 'Error', message: String(err), color: 'red' });
    }
  };

  // Función para exportar datos a Excel
  const exportarExcel = async (data: any[], nombreArchivo: string, hoja: string) => {
    try {
      const wbout = await writeSpreadsheet(data, hoja);

      // 3. Guardar archivo
      const ruta = await save({
        title: 'Guardar Reporte Excel',
        defaultPath: `${nombreArchivo}-${formatearFechaParaSQL(new Date())}.xlsx`,
        filters: [{
          name: 'Excel Workbook',
          extensions: ['xlsx']
        }]
      });

      if (ruta) {
        await invoke('guardar_reporte_archivo', { ruta, contenido: Array.from(wbout) });
        notifications.show({
          title: 'Éxito',
          message: `Archivo guardado en: ${ruta}`,
          color: 'green'
        });
      }

    } catch (e) {
      console.error("Error al exportar:", e);
      setError(`Error al exportar: ${e}`);
    }
  };

  const handleExportarVentas = () => {
    // Aplanar los datos para que sean útiles en Excel: Una fila por detalle de venta
    const dataExportar = reporteVentas.flatMap(v =>
      v.detalles.map(d => ({
        ID_Venta: v.id,
        Fecha: v.fecha_hora,
        Metodo_Pago: v.metodo_pago,
        Producto: d.nombre_producto,
        Cantidad: d.cantidad,
        Precio_Unitario: d.precio_unitario,
        Subtotal: d.subtotal,
        Total_Venta: v.total_venta // Repetimos el total para referencia
      }))
    );
    exportarExcel(dataExportar, 'reporte_ventas', 'Ventas');
  };

  const handleExportarDevoluciones = () => {
    const dataExportar = reporteDevoluciones.flatMap(d =>
      d.detalles.map(det => ({
        ID_Devolucion: d.id,
        ID_Venta_Orig: d.id_venta_original,
        Fecha: d.fecha_hora,
        Motivo: d.motivo,
        Total_Reembolsado: d.total_reembolsado,
        Producto: det.nombre_producto,
        Cantidad: det.cantidad,
        Monto_Item: det.monto_reembolsado
      }))
    );
    exportarExcel(dataExportar, 'reporte_devoluciones', 'Devoluciones');
  };

  const handleImprimirReporte = () => {
    window.print();
  };

  const handleExportarBajoStock = () => {
    exportarExcel(productosBajoStock, 'reporte_bajo_stock', 'Bajo Stock');
  };

  const handleExportarSobreStock = () => {
    exportarExcel(productosSobreStock, 'reporte_sobre_stock', 'Sobre Stock');
  };

  return (
    <div style={{ padding: '20px' }}>
      <Title order={2} mb="md">Reportes y Estadísticas</Title>

      {error && <Alert color="red" mb="md" onClose={() => setError(null)} withCloseButton>{error}</Alert>}

      <Tabs defaultValue="ventas">
        <Tabs.List>
          <Tabs.Tab value="ventas" leftSection={<IconCash size={16} />}>Ventas</Tabs.Tab>
          <Tabs.Tab value="devoluciones" leftSection={<IconArrowBackUp size={16} />}>Devoluciones</Tabs.Tab>
          <Tabs.Tab value="inventario" leftSection={<IconChartBar size={16} />}>Inventario</Tabs.Tab>
          <Tabs.Tab value="analytics" leftSection={<IconChartPie size={16} />}>Analytics</Tabs.Tab>
        </Tabs.List>

        <Tabs.Panel value="ventas" pt="xs">
          {/* ... (contenido existente de ventas) ... */}
          <Paper p="md" shadow="xs" withBorder mb="md">
            <Group align="flex-end">
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <label style={{ fontSize: '14px', fontWeight: 500 }}>Desde</label>
                <input
                  type="date"
                  value={fechas[0] ? formatearFechaParaSQL(fechas[0]) : ''}
                  onChange={(e) => {
                    setFechas([e.target.value ? parsearFechaLocal(e.target.value) : null, fechas[1]]);
                    e.target.blur();
                  }}
                  style={{
                    padding: '8px 12px',
                    borderRadius: '4px',
                    border: '1px solid var(--mantine-color-default-border)',
                    width: '150px',
                    fontSize: '14px',
                    backgroundColor: 'var(--mantine-color-body)',
                    color: 'var(--mantine-color-text)'
                  }}
                />
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <label style={{ fontSize: '14px', fontWeight: 500 }}>Hasta</label>
                <input
                  type="date"
                  value={fechas[1] ? formatearFechaParaSQL(fechas[1]) : ''}
                  onChange={(e) => {
                    setFechas([fechas[0], e.target.value ? parsearFechaLocal(e.target.value) : null]);
                    e.target.blur();
                  }}
                  style={{
                    padding: '8px 12px',
                    borderRadius: '4px',
                    border: '1px solid var(--mantine-color-default-border)',
                    width: '150px',
                    fontSize: '14px',
                    backgroundColor: 'var(--mantine-color-body)',
                    color: 'var(--mantine-color-text)'
                  }}
                />
              </div>
              <Button onClick={handleGenerarReporteVentas}>Generar Reporte</Button>
              <Button onClick={handleExportarVentas} disabled={reporteVentas.length === 0} color="green">Exportar Excel</Button>
              <Button leftSection={<IconPrinter size={16} />} onClick={handleImprimirReporte} disabled={reporteVentas.length === 0} variant="outline">Imprimir</Button>
            </Group>
          </Paper>

          {/* Componente Imprimible (Oculto en pantalla) */}
          <div id="reporte-imprimible" style={{ display: 'none' }}>
            <style>{`
              @media print {
                #reporte-imprimible { display: block !important; }
              }
            `}</style>
            <Title order={3} ta="center" mb="md">Reporte de Ventas</Title>
            <Text ta="center" size="sm" mb="xl">
              Desde: {fechas[0]?.toLocaleDateString()} - Hasta: {fechas[1]?.toLocaleDateString()}
            </Text>

            <Text fw={700} mb="sm">Resumen:</Text>
            <Text mb="lg">Total Ingresos: ${reporteVentas.reduce((acc, v) => acc + v.total_venta, 0).toFixed(2)}</Text>

            <Table withTableBorder withColumnBorders>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>ID</Table.Th>
                  <Table.Th>Fecha</Table.Th>
                  <Table.Th>Método</Table.Th>
                  <Table.Th>Total</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {reporteVentas.map((v) => (
                  <Table.Tr key={v.id}>
                    <Table.Td>{v.id}</Table.Td>
                    <Table.Td>{new Date(v.fecha_hora).toLocaleString()}</Table.Td>
                    <Table.Td>{v.metodo_pago}</Table.Td>
                    <Table.Td>${v.total_venta.toFixed(2)}</Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          </div>

          {reporteVentas.length > 0 && (
            <Paper p="md" shadow="xs" withBorder>
              <Title order={4} mb="sm">Resultados ({reporteVentas.length} ventas)</Title>
              <Text fw={700} mb="md">Total Ingresos: ${reporteVentas.reduce((acc, v) => acc + v.total_venta, 0).toFixed(2)}</Text>

              <Accordion variant="separated">
                {reporteVentas.map((venta) => (
                  <Accordion.Item key={venta.id} value={venta.id.toString()}>
                    <Accordion.Control>
                      <Group justify="space-between">
                        <Text>Venta #{venta.id} - {new Date(venta.fecha_hora).toLocaleString()}</Text>
                        <Text fw={700}>${venta.total_venta.toFixed(2)} ({venta.metodo_pago})</Text>
                      </Group>
                    </Accordion.Control>
                    <Accordion.Panel>
                      <Table>
                        <Table.Thead><Table.Tr><Table.Th>Producto</Table.Th><Table.Th>Cant</Table.Th><Table.Th>Precio</Table.Th><Table.Th>Subtotal</Table.Th></Table.Tr></Table.Thead>
                        <Table.Tbody>
                          {venta.detalles.map((d, idx) => (
                            <Table.Tr key={idx}>
                              <Table.Td>{d.nombre_producto}</Table.Td>
                              <Table.Td>{d.cantidad}</Table.Td>
                              <Table.Td>${d.precio_unitario.toFixed(2)}</Table.Td>
                              <Table.Td>${d.subtotal.toFixed(2)}</Table.Td>
                            </Table.Tr>
                          ))}
                        </Table.Tbody>
                      </Table>
                    </Accordion.Panel>
                  </Accordion.Item>
                ))}
              </Accordion>
            </Paper>
          )}
        </Tabs.Panel>

        <Tabs.Panel value="devoluciones" pt="xs">
          <Paper p="md" shadow="xs" withBorder mb="md">
            <Group align="flex-end">
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <label style={{ fontSize: '14px', fontWeight: 500 }}>Desde</label>
                <input
                  type="date"
                  value={fechas[0] ? formatearFechaParaSQL(fechas[0]) : ''}
                  onChange={(e) => {
                    setFechas([e.target.value ? parsearFechaLocal(e.target.value) : null, fechas[1]]);
                    e.target.blur();
                  }}
                  style={{
                    padding: '8px 12px',
                    borderRadius: '4px',
                    border: '1px solid var(--mantine-color-default-border)',
                    width: '150px',
                    fontSize: '14px',
                    backgroundColor: 'var(--mantine-color-body)',
                    color: 'var(--mantine-color-text)'
                  }}
                />
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <label style={{ fontSize: '14px', fontWeight: 500 }}>Hasta</label>
                <input
                  type="date"
                  value={fechas[1] ? formatearFechaParaSQL(fechas[1]) : ''}
                  onChange={(e) => {
                    setFechas([fechas[0], e.target.value ? parsearFechaLocal(e.target.value) : null]);
                    e.target.blur();
                  }}
                  style={{
                    padding: '8px 12px',
                    borderRadius: '4px',
                    border: '1px solid var(--mantine-color-default-border)',
                    width: '150px',
                    fontSize: '14px',
                    backgroundColor: 'var(--mantine-color-body)',
                    color: 'var(--mantine-color-text)'
                  }}
                />
              </div>
              <Button onClick={handleGenerarReporteDevoluciones}>Generar Reporte</Button>
              <Button onClick={handleExportarDevoluciones} disabled={reporteDevoluciones.length === 0} color="green">Exportar Excel</Button>
            </Group>
          </Paper>

          {reporteDevoluciones.length > 0 ? (
            <Paper p="md" shadow="xs" withBorder>
              <Title order={4} mb="sm">Resultados ({reporteDevoluciones.length} devoluciones)</Title>
              <Text fw={700} mb="md" c="red">Total Reembolsado: ${reporteDevoluciones.reduce((acc, d) => acc + d.total_reembolsado, 0).toFixed(2)}</Text>

              <Accordion variant="separated">
                {reporteDevoluciones.map((dev) => (
                  <Accordion.Item key={dev.id} value={dev.id.toString()}>
                    <Accordion.Control>
                      <Group justify="space-between">
                        <Text>Devolución #{dev.id} (Venta #{dev.id_venta_original}) - {new Date(dev.fecha_hora).toLocaleString()}</Text>
                        <Text fw={700} c="red">-${dev.total_reembolsado.toFixed(2)}</Text>
                      </Group>
                    </Accordion.Control>
                    <Accordion.Panel>
                      <Text size="sm" mb="xs"><strong>Motivo:</strong> {dev.motivo}</Text>
                      <Table>
                        <Table.Thead><Table.Tr><Table.Th>Producto</Table.Th><Table.Th>Cant</Table.Th><Table.Th>Reembolso</Table.Th></Table.Tr></Table.Thead>
                        <Table.Tbody>
                          {dev.detalles.map((det, idx) => (
                            <Table.Tr key={idx}>
                              <Table.Td>{det.nombre_producto}</Table.Td>
                              <Table.Td>{det.cantidad}</Table.Td>
                              <Table.Td>${det.monto_reembolsado.toFixed(2)}</Table.Td>
                            </Table.Tr>
                          ))}
                        </Table.Tbody>
                      </Table>
                    </Accordion.Panel>
                  </Accordion.Item>
                ))}
              </Accordion>
            </Paper>
          ) : (
            <Text c="dimmed" mt="md">No hay devoluciones para el rango seleccionado (o no has generado el reporte).</Text>
          )}
        </Tabs.Panel>

        <Tabs.Panel value="inventario" pt="xs">
          <Grid>
            <Grid.Col span={6}>
              <Paper p="md" shadow="xs" withBorder>
                <Group justify="space-between" mb="md">
                  <Title order={4} c="red">Productos Bajo Stock</Title>
                  <Group>
                    <Button size="xs" color="green" variant="light" onClick={handleExportarBajoStock} disabled={productosBajoStock.length === 0}>Exportar</Button>
                    <Button size="xs" color="red" variant="light" onClick={cargarProductosBajoStock}>Actualizar</Button>
                  </Group>
                </Group>
                {productosBajoStock.length === 0 ? <Text c="dimmed">No hay alertas.</Text> : (
                  <Table>
                    <Table.Thead><Table.Tr><Table.Th>Producto</Table.Th><Table.Th>Stock</Table.Th><Table.Th>Límite</Table.Th></Table.Tr></Table.Thead>
                    <Table.Tbody>
                      {productosBajoStock.map(p => (
                        <Table.Tr key={p.id}>
                          <Table.Td>{p.nombre}</Table.Td>
                          <Table.Td fw={700} c="red">{p.stock_actual}</Table.Td>
                          <Table.Td>{(p as any).limite || 5}</Table.Td>
                        </Table.Tr>
                      ))}
                    </Table.Tbody>
                  </Table>
                )}
              </Paper>
            </Grid.Col>
            <Grid.Col span={6}>
              <Paper p="md" shadow="xs" withBorder>
                <Group justify="space-between" mb="md">
                  <Title order={4} c="orange">Productos Sobre Stock</Title>
                  <Group>
                    <Button size="xs" color="green" variant="light" onClick={handleExportarSobreStock} disabled={productosSobreStock.length === 0}>Exportar</Button>
                    <Button size="xs" color="orange" variant="light" onClick={cargarProductosSobreStock}>Actualizar</Button>
                  </Group>
                </Group>
                {productosSobreStock.length === 0 ? <Text c="dimmed">No hay alertas.</Text> : (
                  <Table>
                    <Table.Thead><Table.Tr><Table.Th>Producto</Table.Th><Table.Th>Stock</Table.Th><Table.Th>Max</Table.Th></Table.Tr></Table.Thead>
                    <Table.Tbody>
                      {productosSobreStock.map(p => (
                        <Table.Tr key={p.id}>
                          <Table.Td>{p.nombre}</Table.Td>
                          <Table.Td fw={700} c="orange">{p.stock_actual}</Table.Td>
                          <Table.Td>{(p as any).limite}</Table.Td>
                        </Table.Tr>
                      ))}
                    </Table.Tbody>
                  </Table>
                )}
              </Paper>
            </Grid.Col>
          </Grid>
        </Tabs.Panel>

        <Tabs.Panel value="analytics" pt="xs">
          <Paper p="md" shadow="xs" withBorder mb="md">
            <Group align="flex-end">
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <label style={{ fontSize: '14px', fontWeight: 500 }}>Desde</label>
                <input
                  type="date"
                  value={fechas[0] ? formatearFechaParaSQL(fechas[0]) : ''}
                  onChange={(e) => {
                    setFechas([e.target.value ? parsearFechaLocal(e.target.value) : null, fechas[1]]);
                    e.target.blur();
                  }}
                  style={{
                    padding: '8px 12px',
                    borderRadius: '4px',
                    border: '1px solid var(--mantine-color-default-border)',
                    width: '140px',
                    fontSize: '14px',
                    backgroundColor: 'var(--mantine-color-body)',
                    color: 'var(--mantine-color-text)'
                  }}
                />
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <label style={{ fontSize: '14px', fontWeight: 500 }}>Hasta</label>
                <input
                  type="date"
                  value={fechas[1] ? formatearFechaParaSQL(fechas[1]) : ''}
                  onChange={(e) => {
                    setFechas([fechas[0], e.target.value ? parsearFechaLocal(e.target.value) : null]);
                    e.target.blur();
                  }}
                  style={{
                    padding: '8px 12px',
                    borderRadius: '4px',
                    border: '1px solid var(--mantine-color-default-border)',
                    width: '140px',
                    fontSize: '14px',
                    backgroundColor: 'var(--mantine-color-body)',
                    color: 'var(--mantine-color-text)'
                  }}
                />
              </div>
              <Select
                label="Tipo de Reporte"
                data={[
                  { value: 'ventas-tiempo', label: 'Historial de Ventas' },
                  { value: 'top-entidades', label: 'Top 10 (Ranking)' },
                  { value: 'nuevos-clientes', label: 'Nuevos Clientes' }
                ]}
                value={tipoReporte}
                onChange={setTipoReporte}
                style={{ width: 180 }}
              />

              {tipoReporte === 'top-entidades' && (
                <Select
                  label="Entidad"
                  data={[
                    { value: 'cliente', label: 'Mejores Clientes' },
                    { value: 'vendedor', label: 'Mejores Vendedores' },
                    { value: 'proveedor', label: 'Mejores Proveedores' }
                  ]}
                  value={entidad}
                  onChange={setEntidad}
                  style={{ width: 180 }}
                />
              )}

              {tipoReporte !== 'top-entidades' && (
                <Select
                  label="Agrupar por"
                  data={[{ value: 'dia', label: 'Día' }, { value: 'mes', label: 'Mes' }]}
                  value={agrupacion}
                  onChange={setAgrupacion}
                  style={{ width: 100 }}
                />
              )}

              {tipoReporte === 'ventas-tiempo' && (
                <Select
                  label="Métrica"
                  data={[{ value: 'total', label: 'Total ($)' }, { value: 'cantidad', label: 'Cantidad (#)' }]}
                  value={metrica}
                  onChange={setMetrica}
                  style={{ width: 120 }}
                />
              )}

              <Button onClick={handleGenerarGrafica} leftSection={<IconChartBar size={16} />}>
                Generar
              </Button>
              <Button
                onClick={handleAbrirModalGuardar}
                leftSection={<IconDeviceFloppy size={16} />}
                color="green"
                variant="light"
              >
                Guardar en Dashboard
              </Button>
            </Group>
          </Paper>

          {(datosGrafica.length > 0 || datosEntidad.length > 0 || datosNuevosClientes.length > 0) ? (
            <Paper p="md" shadow="xs" withBorder style={{ height: 400 }}>
              <Title order={4} mb="md" ta="center">Resultados</Title>
              <ResponsiveContainer width="100%" height="100%">
                <BarChart
                  data={
                    tipoReporte === 'top-entidades' ? datosEntidad :
                      tipoReporte === 'nuevos-clientes' ? datosNuevosClientes :
                        datosGrafica
                  }
                  margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
                >
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis
                    dataKey={tipoReporte === 'top-entidades' ? 'nombre' : 'fecha'}
                    angle={tipoReporte === 'top-entidades' ? -45 : 0}
                    textAnchor={tipoReporte === 'top-entidades' ? 'end' : 'middle'}
                    height={tipoReporte === 'top-entidades' ? 80 : 30}
                  />
                  <YAxis />
                  <Tooltip />
                  <Legend />
                  <Bar
                    dataKey={
                      tipoReporte === 'top-entidades' ? 'total_ventas' :
                        tipoReporte === 'nuevos-clientes' ? 'cantidad' :
                          (metrica === 'total' ? 'total_ventas' : 'cantidad_items')
                    }
                    name={
                      tipoReporte === 'top-entidades' ? 'Total ($)' :
                        tipoReporte === 'nuevos-clientes' ? 'Clientes Nuevos' :
                          (metrica === 'total' ? 'Total Ventas ($)' : 'Items Vendidos')
                    }
                    fill="#228be6"
                  />
                </BarChart>
              </ResponsiveContainer>
            </Paper>
          ) : (
            <Text c="dimmed" mt="md" ta="center">Genera una gráfica para ver los resultados.</Text>
          )}
        </Tabs.Panel>
      </Tabs>

      {/* Modal para guardar widget */}
      <Modal
        opened={modalGuardarAbierto}
        onClose={() => setModalGuardarAbierto(false)}
        title="Guardar Gráfica en Dashboard"
        centered
      >
        <TextInput
          label="Nombre de la Gráfica"
          placeholder="Ej: Ventas del Mes"
          value={nombreWidget}
          onChange={(e) => setNombreWidget(e.target.value)}
          mb="md"
        />
        <Group justify="flex-end">
          <Button variant="outline" onClick={() => setModalGuardarAbierto(false)}>
            Cancelar
          </Button>
          <Button onClick={handleGuardarWidget} color="green">
            Guardar
          </Button>
        </Group>
      </Modal>
    </div>
  );
}
