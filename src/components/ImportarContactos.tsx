import { useState } from 'react';
import { Button, Group, Modal, ScrollArea, Stack, Table, Text } from '@mantine/core';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '../lib/invoke';
import { readSpreadsheet } from '../lib/spreadsheet';

type Kind = 'clientes' | 'proveedores';
type ImportRow = { fila: number; nombre: string; rfc: string; telefono: string; email: string; nombre_contacto: string };
type RowResult = { fila: number; nombre: string; estado: string; detalle: string };
type ImportResult = { agregados: number; duplicados: number; errores: number; filas: RowResult[] };

function value(row: Record<string, string>, names: string[]): string {
  const normalized = Object.fromEntries(Object.entries(row).map(([key, content]) => [key.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase().replace(/[ _-]/g, ''), content]));
  return names.map((name) => normalized[name]).find((item) => item?.trim()) ?? '';
}

export function ImportarContactos({ kind, onImported }: { kind: Kind; onImported: () => void }) {
  const [rows, setRows] = useState<ImportRow[]>([]);
  const [report, setReport] = useState<ImportResult | null>(null);
  const [finished, setFinished] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const choose = async () => {
    setError('');
    try {
      const path = await open({ title: `Importar ${kind}`, filters: [{ name: 'CSV o Excel', extensions: ['csv', 'xlsx'] }] });
      if (!path || typeof path !== 'string') return;
      const file = new Uint8Array(await invoke<number[]>(kind === 'clientes' ? 'leer_clientes_archivo' : 'leer_proveedores_archivo', { ruta: path }));
      if (file.length > 5 * 1024 * 1024) throw new Error('El archivo supera 5 MB');
      const data = await readSpreadsheet(file, path);
      if (!data.length || data.length > 5000) throw new Error('El archivo debe contener entre 1 y 5000 filas');
      const parsed = data.map((row, index) => ({
        fila: index + 2,
        nombre: value(row, kind === 'clientes' ? ['nombre', 'cliente'] : ['nombreempresa', 'empresa', 'proveedor']),
        rfc: value(row, ['rfc']),
        telefono: value(row, ['telefono', 'tel']),
        email: value(row, ['email', 'correo']),
        nombre_contacto: value(row, ['nombrecontacto', 'contacto']),
      }));
      if (parsed.every((row) => !row.nombre)) throw new Error(`Falta la columna ${kind === 'clientes' ? 'Nombre' : 'Empresa'}`);
      setBusy(true);
      const result = await invoke<ImportResult>(kind === 'clientes' ? 'previsualizar_clientes' : 'previsualizar_proveedores', { filas: parsed });
      setRows(parsed);
      setReport(result);
      setFinished(false);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const confirm = async () => {
    if (!rows.length) return;
    setBusy(true);
    setError('');
    try {
      const result = await invoke<ImportResult>(kind === 'clientes' ? 'confirmar_clientes' : 'confirmar_proveedores', { filas: rows });
      setReport(result);
      setFinished(true);
      onImported();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  return <>
    <Button variant="outline" onClick={choose} loading={busy}>Importar CSV / Excel</Button>
    {error && <Text c="red" size="sm">{error}</Text>}
    <Modal opened={report !== null} onClose={() => { setReport(null); setRows([]); }} title={finished ? 'Resultado de importación' : 'Vista previa de importación'} size="lg">
      {report && <Stack>
        <Text size="sm">{finished ? 'Resultado' : 'Se agregarán'}: {report.agregados} · Duplicados: {report.duplicados} · Errores: {report.errores}</Text>
        <Text size="sm" c="dimmed">Los duplicados y errores se omiten. La confirmación vuelve a comprobar todas las filas.</Text>
        <ScrollArea h={320}>
          <Table striped>
            <Table.Thead><Table.Tr><Table.Th>Fila</Table.Th><Table.Th>Nombre</Table.Th><Table.Th>Estado</Table.Th><Table.Th>Detalle</Table.Th></Table.Tr></Table.Thead>
            <Table.Tbody>{report.filas.map((row) => <Table.Tr key={row.fila}><Table.Td>{row.fila}</Table.Td><Table.Td>{row.nombre}</Table.Td><Table.Td>{row.estado}</Table.Td><Table.Td>{row.detalle}</Table.Td></Table.Tr>)}</Table.Tbody>
          </Table>
        </ScrollArea>
        <Group justify="flex-end">
          {!finished && <Button onClick={confirm} disabled={report.agregados === 0} loading={busy}>Confirmar importación</Button>}
          <Button variant="light" onClick={() => { setReport(null); setRows([]); }}>Cerrar</Button>
        </Group>
      </Stack>}
    </Modal>
  </>;
}
