// Anchuras de barras y espacios de Code 128, símbolos 0..106.
const PATTERNS = [
  '212222','222122','222221','121223','121322','131222','122213','122312','132212','221213',
  '221312','231212','112232','122132','122231','113222','123122','123221','223211','221132',
  '221231','213212','223112','312131','311222','321122','321221','312212','322112','322211',
  '212123','212321','232121','111323','131123','131321','112313','132113','132311','211313',
  '231113','231311','112133','112331','132131','113123','113321','133121','313121','211331',
  '231131','213113','213311','213131','311123','311321','331121','312113','312311','332111',
  '314111','221411','431111','111224','111422','121124','121421','141122','141221','112214',
  '112412','122114','122411','142112','142211','241211','221114','413111','241112','134111',
  '111242','121142','121241','114212','124112','124211','411212','421112','421211','212141',
  '214121','412121','111143','111341','131141','114113','114311','411113','411311','113141',
  '114131','311141','411131','211412','211214','211232','2331112',
];

export function code128Symbols(value: string): number[] {
  if (!value || value.length > 128 || [...value].some(char => char.charCodeAt(0) < 32 || char.charCodeAt(0) > 126)) {
    throw new Error('Código 128 admite de 1 a 128 caracteres ASCII imprimibles');
  }
  const symbols = [...value].map(char => char.charCodeAt(0) - 32);
  const checksum = (104 + symbols.reduce((sum, symbol, index) => sum + symbol * (index + 1), 0)) % 103;
  return [104, ...symbols, checksum, 106]; // Inicio B, datos, comprobación, parada.
}

export function code128Bars(value: string): { dark: boolean; width: number }[] {
  return code128Symbols(value).flatMap(symbol => [...PATTERNS[symbol]].map((width, index) => ({ dark: index % 2 === 0, width: Number(width) })));
}

export function code128Svg(value: string): string {
  const bars = code128Bars(value);
  const width = bars.reduce((sum, bar) => sum + bar.width, 20);
  let x = 10;
  const rectangles = bars.map(bar => {
    const start = x;
    x += bar.width;
    return bar.dark ? `<rect x="${start}" y="5" width="${bar.width}" height="65" fill="#000"/>` : '';
  }).join('');
  const label = value.replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;' })[char] || char);
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} 90" width="${width * 2}" height="180"><rect width="100%" height="100%" fill="#fff"/>${rectangles}<text x="10" y="84" font-family="sans-serif" font-size="12">${label}</text></svg>`;
}
