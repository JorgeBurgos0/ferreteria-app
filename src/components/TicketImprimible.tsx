import { Text, Stack, Group } from '@mantine/core';

export interface DatosVentaTicket {
    id: number;
    fecha: string;
    items: {
        nombre: string;
        cantidad: number;
        precio: number;
    }[];
    total: number;
    metodoPago: string;
    nombre_cliente?: string;
}

interface TicketImprimibleProps {
    datos: DatosVentaTicket | null;
    datosEmpresa?: {
        nombre: string;
        direccion: string;
        telefono: string;
        logo: string;
    };
}

export function TicketImprimible({ datos, datosEmpresa }: TicketImprimibleProps) {
    if (!datos) return null;

    const nombre = datosEmpresa?.nombre?.trim() || 'Mi Ferretería';
    const direccion = datosEmpresa?.direccion?.trim();
    const telefono = datosEmpresa?.telefono?.trim();

    return (
        <div id="ticket-imprimible" className="ticket-print-only">
            {/* 1. Encabezado */}
            <Stack gap={2} align="center" mb="sm">
                {datosEmpresa?.logo && (
                    <img
                        src={datosEmpresa.logo}
                        alt="Logo"
                        style={{ maxWidth: '80px', maxHeight: '60px', objectFit: 'contain' }}
                    />
                )}
                <Text fw={900} size="md" style={{ textTransform: 'uppercase' }}>{nombre}</Text>
                {direccion && <Text size="xs">{direccion}</Text>}
                {telefono && <Text size="xs">Tel: {telefono}</Text>}
            </Stack>

            {/* 2. Info Venta */}
            <div style={{ borderBottom: '1px dashed black', margin: '5px 0' }} />
            <Group justify="space-between">
                <Text size="xs">Ticket: #{datos.id}</Text>
                <Text size="xs">{new Date(datos.fecha).toLocaleDateString()} {new Date(datos.fecha).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</Text>
            </Group>
            <div style={{ borderBottom: '1px dashed black', margin: '5px 0' }} />

            {/* 3. Cuerpo (Tabla) */}
            <table style={{ width: '100%', fontSize: '12px', borderCollapse: 'collapse' }}>
                <thead>
                    <tr style={{ textAlign: 'left' }}>
                        <th style={{ width: '10%' }}>Cant</th>
                        <th style={{ width: '60%' }}>Prod</th>
                        <th style={{ width: '30%', textAlign: 'right' }}>Importe</th>
                    </tr>
                </thead>
                <tbody>
                    {datos.items.map((item, index) => (
                        <tr key={index} style={{ verticalAlign: 'top' }}>
                            <td>{item.cantidad}</td>
                            <td>{item.nombre}</td>
                            <td style={{ textAlign: 'right' }}>${(item.cantidad * item.precio).toFixed(2)}</td>
                        </tr>
                    ))}
                </tbody>
            </table>

            <div style={{ borderBottom: '1px dashed black', margin: '10px 0' }} />

            {/* 4. Pie */}
            <Group justify="space-between" mb={5}>
                <Text fw={700} size="sm">TOTAL:</Text>
                <Text fw={700} size="md">${datos.total.toFixed(2)}</Text>
            </Group>

            <Stack gap={0}>
                {/* Parsear metodo de pago para mostrar cambio aparte si es posible, o mostrar string completo limpio */}
                <Text size="xs" fw={500}>Método de Pago:</Text>
                <Text size="xs" style={{ whiteSpace: 'pre-wrap' }}>{datos.metodoPago}</Text>
            </Stack>

            <Text ta="center" size="xs" mt="lg">*** Gracias por su compra ***</Text>
        </div>
    );
}
