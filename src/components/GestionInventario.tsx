import { useState, useEffect } from 'react';
import { Table, Button, Group, Modal, TextInput, NumberInput, Title, ActionIcon, Text, Badge, Switch, Select, Stack, Alert, Pagination, Checkbox, ScrollArea } from '@mantine/core';
import { open } from '@tauri-apps/plugin-dialog';
import { readSpreadsheet } from '../lib/spreadsheet';
import { notifications } from '@mantine/notifications';
import { IconEdit, IconTrash, IconPlus, IconAlertCircle, IconSearch, IconFileSpreadsheet } from '@tabler/icons-react';
import { invoke } from '../lib/invoke';

interface Producto {
  id: number;
  sku: string;
  nombre: string;
  descripcion: string;
  precio_venta: number;
  precio_mayoreo?: number;
  stock_actual: number;
  stock_minimo?: number;
  stock_maximo?: number;
  categoria?: string;
  ubicacion?: string;
  es_kit: boolean;
}

interface ComponenteKit {
  id_componente: number;
  cantidad: number;
  nombre?: string;
}

interface ProductosPaginados {
  total: number;
  productos: Producto[];
}

interface ProductoPrevisualizacion {
  sku: string;
  nombre_actual?: string;
  precio_actual?: number;
  stock_actual?: number;
  existe: boolean;
  // Datos nuevos del Excel
  nombre_nuevo: string;
  precio_nuevo: number;
  stock_nuevo: number; // Esto es stock a sumar o stock total? En la logica IMPORT: se SUMA stock.
  // Para simplificar la UI, mostraremos "Stock Actual + Nuevo = Total" si es posible, o solo "Stock a sumar".
  // El comando importar SUMA stock.
}

export function GestionInventario({ username }: { username: string }) {
  const [productos, setProductos] = useState<Producto[]>([]);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [modoEdicion, setModoEdicion] = useState(false);
  const [productoId, setProductoId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Preview Import State
  const [previewModalOpen, setPreviewModalOpen] = useState(false);
  const [previewData, setPreviewData] = useState<any[]>([]); // Data raw del excel + info de existencia
  const [selectedSkus, setSelectedSkus] = useState<string[]>([]); // SKUs seleccionados para importar

  // Pagination State
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [total, setTotal] = useState(0);
  const LIMIT = 10;

  // Form States
  const [sku, setSku] = useState('');
  const [nombre, setNombre] = useState('');
  const [descripcion, setDescripcion] = useState('');
  const [precio, setPrecio] = useState<number | ''>('');
  const [precioMayoreo, setPrecioMayoreo] = useState<number | ''>('');
  const [stock, setStock] = useState<number | ''>('');
  const [stockMinimo, setStockMinimo] = useState<number | ''>(5);
  const [stockMaximo, setStockMaximo] = useState<number | ''>('');
  const [categoria, setCategoria] = useState('');
  const [ubicacion, setUbicacion] = useState('');

  // Kit States
  const [esKit, setEsKit] = useState(false);
  const [componentesKit, setComponentesKit] = useState<ComponenteKit[]>([]);
  const [componenteSeleccionado, setComponenteSeleccionado] = useState<string | null>(null);
  const [cantidadComponente, setCantidadComponente] = useState<number>(1);

  useEffect(() => {
    cargarProductos();
  }, [page, search]); // Recargar al cambiar página o búsqueda

  const cargarProductos = async () => {
    try {
      const resp: ProductosPaginados = await invoke('obtener_productos_paginados', {
        page: page,
        limit: LIMIT,
        search: search
      });
      setProductos(resp.productos);
      setTotal(resp.total);
    } catch (error) {
      console.error("Error al cargar productos:", error);
    }
  };

  const handleAbrirModalAgregar = () => {
    setModoEdicion(false);
    setProductoId(null);
    limpiarFormulario();
    setError(null);
    setModalAbierto(true);
  };

  const handleAbrirModalEditar = async (producto: Producto) => {
    setModoEdicion(true);
    setProductoId(producto.id);
    setSku(producto.sku);
    setNombre(producto.nombre);
    setDescripcion(producto.descripcion || '');
    setPrecio(producto.precio_venta);
    setPrecioMayoreo(producto.precio_mayoreo || '');
    setStock(producto.stock_actual);
    setStockMinimo(producto.stock_minimo || 5);
    setStockMaximo(producto.stock_maximo || '');
    setCategoria(producto.categoria || '');
    setUbicacion(producto.ubicacion || '');
    setEsKit(producto.es_kit);

    if (producto.es_kit) {
      try {
        const comps: ComponenteKit[] = await invoke('obtener_componentes_kit', { idKit: producto.id });
        // Mapear nombres para mostrar en UI
        const compsConNombre = comps.map(c => {
          const p = productos.find(prod => prod.id === c.id_componente);
          return { ...c, nombre: p ? p.nombre : 'Desconocido' };
        });
        setComponentesKit(compsConNombre);
      } catch (e) {
        console.error("Error cargando componentes", e);
        setComponentesKit([]);
      }
    } else {
      setComponentesKit([]);
    }

    setModalAbierto(true);
  };

  const limpiarFormulario = () => {
    setSku('');
    setNombre('');
    setDescripcion('');
    setPrecio('');
    setPrecioMayoreo('');
    setStock('');
    setStockMinimo(5);
    setStockMaximo('');
    setCategoria('');
    setUbicacion('');
    setEsKit(false);
    setComponentesKit([]);
  };

  const handleAgregarComponente = () => {
    if (!componenteSeleccionado) return;
    const idComp = parseInt(componenteSeleccionado);
    const prod = productos.find(p => p.id === idComp);
    if (!prod) return;

    setComponentesKit([...componentesKit, {
      id_componente: idComp,
      cantidad: cantidadComponente,
      nombre: prod.nombre
    }]);
    setComponenteSeleccionado(null);
    setCantidadComponente(1);
  };

  const handleEliminarComponente = (index: number) => {
    const nuevos = [...componentesKit];
    nuevos.splice(index, 1);
    setComponentesKit(nuevos);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const datosProducto = {
      sku,
      nombre,
      descripcion,
      precioVenta: Number(precio),
      precioMayoreo: precioMayoreo === '' ? null : Number(precioMayoreo),
      stockActual: esKit ? 0 : Number(stock), // Stock de kit es calculado
      stockMinimo: stockMinimo === '' ? null : Number(stockMinimo),
      stockMaximo: stockMaximo === '' ? null : Number(stockMaximo),
      categoria,
      ubicacion,
      esKit,
      usuario: username
    };

    try {
      let idGenerado = productoId;
      if (modoEdicion && productoId) {
        await invoke('actualizar_producto', { id: productoId, ...datosProducto });
      } else {
        idGenerado = await invoke('agregar_producto', datosProducto);
      }

      // Guardar componentes si es kit
      if (esKit && idGenerado) {
        await invoke('guardar_componentes_kit', {
          idKit: idGenerado,
          componentes: componentesKit.map(c => ({ id_componente: c.id_componente, cantidad: c.cantidad }))
        });
      }

      setModalAbierto(false);
      cargarProductos();
    } catch (error) {
      console.error("Error al guardar producto:", error);
      setError(String(error));
    }
  };


  const handleImportarExcel = async () => {
    try {
      const selected = await open({
        title: 'Seleccionar archivo Excel',
        filters: [{ name: 'Excel / CSV', extensions: ['xlsx', 'csv'] }]
      });

      if (selected && typeof selected === 'string') {
        const fileData = new Uint8Array(await invoke<number[]>('leer_catalogo', { ruta: selected }));
        const jsonData = await readSpreadsheet(fileData, selected);

        // Map data from Excel
        const productosRaw = jsonData.map((row: any) => ({
          sku: String(row.SKU || row.sku || ''),
          nombre: String(row.Nombre || row.nombre || ''),
          descripcion: row.Descripcion || row.descripcion || null,
          precio_venta: Number(row.Precio || row.PrecioVenta || row.precio_venta || 0),
          precio_mayoreo: row.PrecioMayoreo ? Number(row.PrecioMayoreo) : null,
          stock_actual: Number(row.Stock || row.stock_actual || 0),
          stock_minimo: row.StockMinimo ? Number(row.StockMinimo) : null,
          stock_maximo: row.StockMaximo ? Number(row.StockMaximo) : null,
          categoria: row.Categoria || row.categoria || null,
          ubicacion: row.Ubicacion || row.ubicacion || null,
        })).filter(p => p.sku !== '' && p.nombre !== '');

        if (productosRaw.length === 0) {
          notifications.show({ title: 'Error', message: 'No se encontraron productos válidos.', color: 'red' });
          return;
        }

        // Verificar Existencia Backend
        const skus = productosRaw.map(p => p.sku);
        const verificacion: ProductoPrevisualizacion[] = await invoke('verificar_skus', { skus });

        // Merge verification info with excel data
        const combinedData = productosRaw.map(p => {
          const info = verificacion.find(v => v.sku === p.sku);
          return {
            ...p,
            existe: info?.existe || false,
            nombre_actual: info?.nombre_actual,
            precio_actual: info?.precio_actual,
            stock_actual_db: info?.stock_actual
          };
        });

        setPreviewData(combinedData);
        setSelectedSkus(combinedData.map(p => p.sku)); // All selected by default
        setPreviewModalOpen(true);
      }
    } catch (error) {
      console.error("Error importando Excel:", error);
      notifications.show({ title: 'Error de Lectura', message: String(error), color: 'red' });
    }
  };

  const confirmarImportacion = async () => {
    const productosFinales = previewData.filter(p => selectedSkus.includes(p.sku));

    try {
      const resultado: any = await invoke('importar_productos_masivos', { productos: productosFinales, usuario: username });

      if (resultado.errores.length > 0) {
        notifications.show({
          title: 'Importación Parcial',
          message: `Se importaron ${resultado.exitosos} productos. Hubo ${resultado.errores.length} errores. Revisa la consola.`,
          color: 'yellow',
          autoClose: 10000
        });
        console.warn("Errores:", resultado.errores);
      } else {
        notifications.show({ title: 'Éxito', message: `Se importaron ${resultado.exitosos} productos.`, color: 'green' });
      }
      setPreviewModalOpen(false);
      cargarProductos();
    } catch (error) {
      notifications.show({ title: 'Error backend', message: String(error), color: 'red' });
    }
  };

  const toggleAllSelection = (checked: boolean) => {
    if (checked) {
      setSelectedSkus(previewData.map(p => p.sku));
    } else {
      setSelectedSkus([]);
    }
  };

  const toggleSkuSelection = (sku: string) => {
    setSelectedSkus(prev => prev.includes(sku) ? prev.filter(s => s !== sku) : [...prev, sku]);
  };

  const handleEliminar = async (id: number) => {
    if (confirm('¿Estás seguro de eliminar este producto?')) {
      try {
        await invoke('eliminar_producto', { id, usuario: username });
        cargarProductos();
      } catch (error) {
        console.error("Error al eliminar:", error);
      }
    }
  };

  const rows = productos.map((producto) => (
    <Table.Tr key={producto.id}>
      <Table.Td>{producto.sku}</Table.Td>
      <Table.Td>
        {producto.nombre}
        {producto.es_kit && <Badge color="violet" ml="xs">KIT</Badge>}
      </Table.Td>
      <Table.Td>{producto.categoria}</Table.Td>
      <Table.Td>${producto.precio_venta.toFixed(2)}</Table.Td>
      <Table.Td>{producto.stock_actual}</Table.Td>
      <Table.Td>{producto.ubicacion}</Table.Td>
      <Table.Td>
        <Group gap={4}>
          <ActionIcon variant="subtle" color="blue" onClick={() => handleAbrirModalEditar(producto)}>
            <IconEdit size={16} />
          </ActionIcon>
          <ActionIcon variant="subtle" color="red" onClick={() => handleEliminar(producto.id)}>
            <IconTrash size={16} />
          </ActionIcon>
        </Group>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <div style={{ padding: '20px' }}>
      <Group justify="space-between" mb="md">
        <Title order={2}>Inventario</Title>
        <Group>
          <TextInput
            placeholder="Buscar por nombre o SKU"
            leftSection={<IconSearch size={14} />}
            value={search}
            onChange={(e) => { setSearch(e.currentTarget.value); setPage(1); }}
          />
          <Button leftSection={<IconPlus size={14} />} onClick={handleAbrirModalAgregar}>
            Agregar Producto
          </Button>
          <Button leftSection={<IconFileSpreadsheet size={14} />} color="green" onClick={handleImportarExcel}>
            Importar Excel
          </Button>
        </Group>
      </Group>

      <Table striped highlightOnHover withTableBorder>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>SKU</Table.Th>
            <Table.Th>Nombre</Table.Th>
            <Table.Th>Categoría</Table.Th>
            <Table.Th>Precio</Table.Th>
            <Table.Th>Stock</Table.Th>
            <Table.Th>Ubicación</Table.Th>
            <Table.Th>Acciones</Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{rows}</Table.Tbody>
      </Table>

      <Group justify="center" mt="md">
        <Pagination total={Math.ceil(total / LIMIT)} value={page} onChange={setPage} />
      </Group>

      <Modal opened={modalAbierto} onClose={() => setModalAbierto(false)} title={modoEdicion ? "Editar Producto" : "Nuevo Producto"} size="lg">
        <form onSubmit={handleSubmit}>
          <Stack>
            {error && <Alert icon={<IconAlertCircle size="1rem" />} title="Error" color="red" withCloseButton onClose={() => setError(null)}>{error}</Alert>}
            <Group grow>
              <TextInput label="SKU" value={sku} onChange={(e) => setSku(e.currentTarget.value)} required />
              <TextInput label="Nombre" value={nombre} onChange={(e) => setNombre(e.currentTarget.value)} required />
            </Group>

            <TextInput label="Descripción" value={descripcion} onChange={(e) => setDescripcion(e.currentTarget.value)} />

            <Group grow>
              <TextInput label="Categoría" value={categoria} onChange={(e) => setCategoria(e.currentTarget.value)} placeholder="General" />
              <TextInput label="Ubicación" value={ubicacion} onChange={(e) => setUbicacion(e.currentTarget.value)} placeholder="Pasillo 1" />
            </Group>

            <Group grow>
              <NumberInput
                label="Precio de Venta"
                value={precio}
                onChange={(value) => { if (typeof value === 'number' || value === '') { setPrecio(value); } }}
                decimalScale={2}
                fixedDecimalScale
                min={0}
                prefix="$"
                required
              />
              <NumberInput
                label="Precio Mayoreo"
                value={precioMayoreo}
                onChange={(value) => { if (typeof value === 'number' || value === '') { setPrecioMayoreo(value); } }}
                decimalScale={2}
                fixedDecimalScale
                min={0}
                prefix="$"
              />
            </Group>

            <Switch
              label="Es Kit / Combo"
              checked={esKit}
              onChange={(e) => setEsKit(e.currentTarget.checked)}
              mb="sm"
            />

            {!esKit ? (
              <Group grow>
                <NumberInput
                  label="Stock Inicial"
                  value={stock}
                  onChange={(value) => { if (typeof value === 'number' || value === '') { setStock(value); } }}
                  allowDecimal={false}
                  min={0}
                  required={!modoEdicion}
                  disabled={modoEdicion}
                />
                <NumberInput
                  label="Stock Mínimo"
                  value={stockMinimo}
                  onChange={(value) => { if (typeof value === 'number' || value === '') { setStockMinimo(value); } }}
                  min={0}
                />
                <NumberInput
                  label="Stock Máximo"
                  value={stockMaximo}
                  onChange={(value) => { if (typeof value === 'number' || value === '') { setStockMaximo(value); } }}
                  min={0}
                />
              </Group>
            ) : (
              <Stack gap="xs" p="xs" bg="gray.1" style={{ borderRadius: 8 }}>
                <Text fw={500} size="sm">Componentes del Kit</Text>
                <Group align="flex-end">
                  <Select
                    label="Agregar Componente"
                    placeholder="Buscar producto"
                    data={productos.filter(p => !p.es_kit && p.id !== productoId).map(p => ({ value: p.id.toString(), label: p.nombre }))}
                    searchable
                    value={componenteSeleccionado}
                    onChange={setComponenteSeleccionado}
                    style={{ flex: 1 }}
                  />
                  <NumberInput
                    label="Cant."
                    value={cantidadComponente}
                    onChange={(val) => setCantidadComponente(Number(val))}
                    min={1}
                    style={{ width: 80 }}
                  />
                  <Button onClick={handleAgregarComponente} variant="light"><IconPlus size={16} /></Button>
                </Group>

                {componentesKit.length > 0 && (
                  <Table withTableBorder>
                    <Table.Thead><Table.Tr><Table.Th>Componente</Table.Th><Table.Th>Cant.</Table.Th><Table.Th></Table.Th></Table.Tr></Table.Thead>
                    <Table.Tbody>
                      {componentesKit.map((c, idx) => (
                        <Table.Tr key={idx}>
                          <Table.Td>{c.nombre}</Table.Td>
                          <Table.Td>{c.cantidad}</Table.Td>
                          <Table.Td>
                            <ActionIcon color="red" size="sm" onClick={() => handleEliminarComponente(idx)}><IconTrash size={14} /></ActionIcon>
                          </Table.Td>
                        </Table.Tr>
                      ))}
                    </Table.Tbody>
                  </Table>
                )}
              </Stack>
            )}

            <Button type="submit" mt="md">Guardar Cambios</Button>
          </Stack>
        </form>
      </Modal>
      <Modal opened={previewModalOpen} onClose={() => setPreviewModalOpen(false)} title="Previsualización de Importación" size="80%">
        <Stack>
          <Text size="sm">
            Verifica los datos antes de importar.
            <Text span c="green" fw={700}> Verde</Text>: Productos nuevos.
            <Text span c="yellow" fw={700}> Amarillo</Text>: Productos existentes (se actualizarán precios y sumará stock).
          </Text>
          <Group>
            <Checkbox
              label="Seleccionar Todos"
              checked={selectedSkus.length === previewData.length && previewData.length > 0}
              indeterminate={selectedSkus.length > 0 && selectedSkus.length < previewData.length}
              onChange={(e) => toggleAllSelection(e.currentTarget.checked)}
            />
            <Text size="sm" c="dimmed">Seleccionados: {selectedSkus.length} de {previewData.length}</Text>
          </Group>

          <ScrollArea h={400}>
            <Table stickyHeader>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th w={40}></Table.Th>
                  <Table.Th>SKU</Table.Th>
                  <Table.Th>Producto</Table.Th>
                  <Table.Th>Precio Actual {"->"} Nuevo</Table.Th>
                  <Table.Th>Stock Actual + Importar</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {previewData.map((p) => {
                  const isSelected = selectedSkus.includes(p.sku);
                  return (
                    <Table.Tr key={p.sku} bg={p.existe ? 'yellow.1' : 'green.1'}>
                      <Table.Td>
                        <Checkbox checked={isSelected} onChange={() => toggleSkuSelection(p.sku)} />
                      </Table.Td>
                      <Table.Td>{p.sku}</Table.Td>
                      <Table.Td>
                        <Text size="sm" fw={500}>{p.nombre}</Text>
                        {p.existe && <Text size="xs" c="dimmed">Actual: {p.nombre_actual}</Text>}
                      </Table.Td>
                      <Table.Td>
                        <Text size="sm">${p.precio_venta}</Text>
                        {p.existe && p.precio_venta !== p.precio_actual && (
                          <Text size="xs" td="line-through" c="dimmed">${p.precio_actual}</Text>
                        )}
                      </Table.Td>
                      <Table.Td>
                        {p.existe ? (
                          <Text size="sm">{p.stock_actual_db} + {p.stock_actual} = {p.stock_actual_db + p.stock_actual}</Text>
                        ) : (
                          <Text size="sm">{p.stock_actual}</Text>
                        )}
                      </Table.Td>
                    </Table.Tr>
                  );
                })}
              </Table.Tbody>
            </Table>
          </ScrollArea>

          <Group justify="flex-end">
            <Button variant="default" onClick={() => setPreviewModalOpen(false)}>Cancelar</Button>
            <Button onClick={confirmarImportacion} color="blue" disabled={selectedSkus.length === 0}>
              Confirmar Importación ({selectedSkus.length})
            </Button>
          </Group>
        </Stack>
      </Modal>
    </div>
  );
}
