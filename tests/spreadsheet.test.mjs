import test from 'node:test';
import assert from 'node:assert/strict';
import { readSpreadsheet, writeSpreadsheet } from '../src/lib/spreadsheet.ts';

test('reads CSV with quoted commas and line breaks', async () => {
  const csv = 'SKU,Nombre,Stock\nABC,"Martillo, grande",2\nDEF,"Línea\nnueva",3\n';
  const rows = await readSpreadsheet(new TextEncoder().encode(csv), 'productos.csv');
  assert.deepEqual(rows, [
    { SKU: 'ABC', Nombre: 'Martillo, grande', Stock: '2' },
    { SKU: 'DEF', Nombre: 'Línea\nnueva', Stock: '3' },
  ]);
});

test('exports and reads XLSX product rows', async () => {
  const data = [{ SKU: 'ABC', Nombre: 'Martillo', Stock: 2 }];
  const bytes = await writeSpreadsheet(data, 'Productos');
  const rows = await readSpreadsheet(bytes, 'productos.xlsx');
  assert.deepEqual(rows, [{ SKU: 'ABC', Nombre: 'Martillo', Stock: '2' }]);
});
