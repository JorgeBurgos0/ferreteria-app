export async function readSpreadsheet(data: Uint8Array, filename: string): Promise<Record<string, string>[]> {
  if (filename.toLowerCase().endsWith('.csv')) {
    return parseCsv(new TextDecoder().decode(data));
  }
  const ExcelJS = (await import('exceljs')).default;
  const workbook = new ExcelJS.Workbook();
  await workbook.xlsx.load(data as unknown as Parameters<typeof workbook.xlsx.load>[0]);
  const sheet = workbook.worksheets[0];
  if (!sheet) return [];
  const headers: string[] = [];
  sheet.getRow(1).eachCell((cell, column) => {
    headers[column - 1] = cell.text.trim();
  });
  const rows: Record<string, string>[] = [];
  sheet.eachRow((row, number) => {
    if (number === 1) return;
    const values: Record<string, string> = {};
    headers.forEach((header, index) => {
      if (header) values[header] = row.getCell(index + 1).text.trim();
    });
    if (Object.values(values).some(Boolean)) rows.push(values);
  });
  return rows;
}

export async function writeSpreadsheet(data: Record<string, unknown>[], sheetName: string): Promise<Uint8Array> {
  const ExcelJS = (await import('exceljs')).default;
  const workbook = new ExcelJS.Workbook();
  const sheet = workbook.addWorksheet(sheetName);
  const headers = Object.keys(data[0] ?? {});
  sheet.addRow(headers);
  data.forEach((row) => {
    sheet.addRow(headers.map((header) => {
      const value = row[header];
      return typeof value === 'number' && Number.isFinite(value) ? value : String(value ?? '');
    }));
  });
  const buffer = await workbook.xlsx.writeBuffer();
  return new Uint8Array(buffer);
}

function parseCsv(input: string): Record<string, string>[] {
  const rows: string[][] = [];
  const firstLine = input.split(/\r?\n/, 1)[0] ?? '';
  const separator = firstLine.includes(';') && !firstLine.includes(',') ? ';' : ',';
  let row: string[] = [];
  let cell = '';
  let quoted = false;
  for (let index = 0; index < input.length; index += 1) {
    const char = input[index];
    if (char === '"') {
      if (quoted && input[index + 1] === '"') {
        cell += '"';
        index += 1;
      } else {
        quoted = !quoted;
      }
    } else if (char === separator && !quoted) {
      row.push(cell);
      cell = '';
    } else if ((char === '\n' || char === '\r') && !quoted) {
      if (char === '\r' && input[index + 1] === '\n') index += 1;
      row.push(cell);
      if (row.some((value) => value.trim())) rows.push(row);
      row = [];
      cell = '';
    } else {
      cell += char;
    }
  }
  row.push(cell);
  if (row.some((value) => value.trim())) rows.push(row);
  const headers = (rows.shift() ?? []).map((header) => header.replace(/^\uFEFF/, '').trim());
  return rows.map((values) => Object.fromEntries(headers.map((header, index) => [header, (values[index] ?? '').trim()])));
}
