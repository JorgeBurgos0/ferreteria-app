const pairForm = document.querySelector('#pair-form');
const scanner = document.querySelector('#scanner');
const manualForm = document.querySelector('#manual-form');
const cameraButton = document.querySelector('#camera-button');
const video = document.querySelector('#camera');
const status = document.querySelector('#status');
let token = '';
let stream = null;
let detector = null;
let scanning = false;

function message(value) { status.textContent = value; }

pairForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  try {
    const response = await fetch('/pair', { method: 'POST', body: document.querySelector('#pair-code').value.trim() });
    if (!response.ok) throw new Error(await response.text());
    token = await response.text();
    pairForm.hidden = true;
    scanner.hidden = false;
    message('Conectado. Activa la cámara o escribe un SKU.');
  } catch (error) { message(`No se pudo conectar: ${error.message}`); }
});

async function sendSku(value) {
  const sku = value.trim();
  if (!sku || !token) return false;
  try {
    const response = await fetch('/scan', { method: 'POST', headers: { Authorization: `Bearer ${token}` }, body: sku });
    if (!response.ok) throw new Error(await response.text());
    message(`Enviado: ${sku}. Cámara detenida; actívala para leer otro código.`);
    if (navigator.vibrate) navigator.vibrate(70);
    return true;
  } catch (error) { message(`No se pudo enviar: ${error.message}`); return false; }
}

manualForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const input = document.querySelector('#sku');
  await sendSku(input.value);
  input.value = '';
});

function stopCamera() {
  scanning = false;
  stream?.getTracks().forEach((track) => track.stop());
  stream = null;
  video.srcObject = null;
  cameraButton.textContent = 'Activar cámara';
}

async function scanFrame() {
  if (!scanning || !detector) return;
  try {
    if (video.readyState >= 2) {
      const codes = await detector.detect(video);
      const value = codes[0]?.rawValue?.trim();
      if (value) {
        scanning = false;
        await sendSku(value);
        stopCamera();
        return;
      }
    }
  } catch (_) { message('No se pudo leer el cuadro de cámara.'); }
  if (scanning) setTimeout(scanFrame, 200);
}

cameraButton.addEventListener('click', async () => {
  if (scanning) { stopCamera(); message('Cámara detenida.'); return; }
  if (!window.isSecureContext || !navigator.mediaDevices?.getUserMedia) {
    message('La cámara necesita Chrome en localhost y permiso de cámara.');
    return;
  }
  if (!('BarcodeDetector' in window)) {
    message('Este navegador no admite lectura de códigos. Usa Chrome actualizado o escribe el SKU.');
    return;
  }
  try {
    const preferred = ['ean_13', 'ean_8', 'upc_a', 'upc_e', 'code_128', 'code_39', 'code_93', 'itf', 'codabar', 'qr_code'];
    const supported = await BarcodeDetector.getSupportedFormats();
    const formats = preferred.filter((format) => supported.includes(format));
    if (!formats.length) throw new Error('No hay formatos de código compatibles');
    detector = new BarcodeDetector({ formats });
    stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' }, audio: false });
    video.srcObject = stream;
    await video.play();
    scanning = true;
    cameraButton.textContent = 'Detener cámara';
    message('Cámara activa. Apunta al código.');
    scanFrame();
  } catch (error) {
    stopCamera();
    message(`No se pudo activar la cámara: ${error.message}`);
  }
});

document.addEventListener('visibilitychange', () => { if (document.hidden) stopCamera(); });
window.addEventListener('pagehide', stopCamera);
