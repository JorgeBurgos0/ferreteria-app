
import { useState, useEffect } from 'react';
import { Paper, Title, Group, Text, NumberInput, Button, Stack, Grid, Card, Alert, Badge } from '@mantine/core';
import { DatePickerInput } from '@mantine/dates';
import { IconCalculator, IconAlertCircle, IconCheck, IconLock } from '@tabler/icons-react';
import { invoke } from '../lib/invoke';
import 'dayjs/locale/es';

interface CorteCajaData {
    id: number;
    fecha: string;
    total_ventas: number;
    esperado_efectivo: number;
    esperado_tarjeta: number;
    contado_efectivo: number;
    contado_tarjeta: number;
    diferencia: number;
    usuario: string;
    fecha_hora_cierre: string;
}

interface ResumenCorte {
    fecha: string;
    total_ventas: number;
    esperado_efectivo: number;
    esperado_tarjeta: number;
    corte_existente: CorteCajaData | null;
}

export function CorteCaja({ username, canReopen }: { username: string; canReopen: boolean }) {
    const [fecha, setFecha] = useState<Date | null>(new Date());
    const [resumen, setResumen] = useState<ResumenCorte | null>(null);
    const [contadoEfectivo, setContadoEfectivo] = useState<number | ''>('');
    const [contadoTarjeta, setContadoTarjeta] = useState<number | ''>('');

    const [corteFinalizado, setCorteFinalizado] = useState(false);

    const cargarCorte = async () => {
        if (!fecha) return;
        try {
            const anio = fecha.getFullYear();
            const mes = (fecha.getMonth() + 1).toString().padStart(2, '0');
            const dia = fecha.getDate().toString().padStart(2, '0');
            const fechaStr = `${anio}-${mes}-${dia}`;

            const datos: ResumenCorte = await invoke('obtener_resumen_corte', { fecha: fechaStr });
            setResumen(datos);

            if (datos.corte_existente) {
                setCorteFinalizado(true);
                setContadoEfectivo(datos.corte_existente.contado_efectivo);
                setContadoTarjeta(datos.corte_existente.contado_tarjeta);
            } else {
                setCorteFinalizado(false);
                setContadoEfectivo('');
                setContadoTarjeta('');
            }
        } catch (error) {
            console.error("Error al cargar corte:", error);
            alert("Error al cargar datos del corte.");
        }
    };

    useEffect(() => {
        cargarCorte();
    }, [fecha]);

    const diferenciaEfectivo = resumen ? (Number(contadoEfectivo) - resumen.esperado_efectivo) : 0;
    const diferenciaTarjeta = resumen ? (Number(contadoTarjeta) - resumen.esperado_tarjeta) : 0;
    const diferenciaTotal = diferenciaEfectivo + diferenciaTarjeta;
    const diferenciaRedondeada = Math.round((diferenciaTotal + Number.EPSILON) * 100) / 100;

    const handleImprimir = () => {
        window.print();
    };

    const handleFinalizarCorte = async () => {
        if (contadoEfectivo === '' || contadoTarjeta === '') {
            alert("Por favor ingresa los montos contados.");
            return;
        }
        if (!resumen || !fecha) return;

        if (confirm("¿Estás seguro de finalizar el corte? Reabrirlo requiere un permiso especial.")) {
            try {
                const anio = fecha.getFullYear();
                const mes = (fecha.getMonth() + 1).toString().padStart(2, '0');
                const dia = fecha.getDate().toString().padStart(2, '0');
                const fechaStr = `${anio}-${mes}-${dia}`;

                await invoke('guardar_corte', {
                    fecha: fechaStr,
                    esperadoEfectivo: resumen.esperado_efectivo,
                    esperadoTarjeta: resumen.esperado_tarjeta,
                    contadoEfectivo: Number(contadoEfectivo),
                    contadoTarjeta: Number(contadoTarjeta),
                    usuario: username
                });

                setCorteFinalizado(true);
                cargarCorte();
                alert("Corte guardado correctamente.");
            } catch (error) {
                console.error("Error al guardar corte:", error);
                alert("Error al guardar el corte.");
            }
        }
    }

    const handleReabrirCorte = async () => {
        if (!confirm("¿Estás seguro de REABRIR el corte? Esto eliminará el registro de cierre actual y permitirá hacer uno nuevo.")) return;

        if (!resumen || !fecha) return;

        try {
            const anio = fecha.getFullYear();
            const mes = (fecha.getMonth() + 1).toString().padStart(2, '0');
            const dia = fecha.getDate().toString().padStart(2, '0');
            const fechaStr = `${anio}-${mes}-${dia}`;

            await invoke('eliminar_corte', { fecha: fechaStr });

            alert("Corte reabierto exitosamente.");
            cargarCorte(); // Recargar para volver al estado inicial
        } catch (error) {
            console.error(error);
            alert("Error al reabrir corte: " + error);
        }
    };

    return (
        <Stack>
            <Group justify="space-between">
                <Title order={2}>Corte de Caja</Title>
                {corteFinalizado && <Badge color="green" size="lg" leftSection={<IconLock size={14} />}>Corte Finalizado</Badge>}
            </Group>

            <Paper p="md" shadow="xs" withBorder className="no-print">
                <Group>
                    <DatePickerInput
                        label="Fecha del Corte"
                        value={fecha}
                        onChange={(val: any) => setFecha(val)}
                        locale="es"
                        maxDate={new Date()}
                    />
                    <Button onClick={cargarCorte} mt={24} leftSection={<IconCalculator size="1rem" />}>
                        Cargar Datos
                    </Button>
                </Group>
            </Paper>

            {resumen && (
                <Grid>
                    <Grid.Col span={6}>
                        <Card shadow="sm" padding="lg" radius="md" withBorder>
                            <Title order={4} mb="md">1. Dinero Contado (Real)</Title>
                            <Stack>
                                <NumberInput
                                    label="Efectivo en Caja"
                                    placeholder="0.00"
                                    prefix="$"
                                    value={contadoEfectivo}
                                    onChange={(val) => setContadoEfectivo(Number(val))}
                                    disabled={corteFinalizado}
                                />
                                <NumberInput
                                    label="Vouchers / Recibos Tarjeta"
                                    placeholder="0.00"
                                    prefix="$"
                                    value={contadoTarjeta}
                                    onChange={(val) => setContadoTarjeta(Number(val))}
                                    disabled={corteFinalizado}
                                />
                            </Stack>
                        </Card>
                    </Grid.Col>

                    <Grid.Col span={6}>
                        <Card shadow="sm" padding="lg" radius="md" withBorder>
                            <Title order={4} mb="md">2. Resumen del Sistema</Title>
                            {corteFinalizado ? (
                                <Stack gap="xs">
                                    <Group justify="space-between">
                                        <Text>Ventas brutas:</Text>
                                        <Text fw={700}>${resumen.total_ventas.toFixed(2)}</Text>
                                    </Group>
                                    <Group justify="space-between">
                                        <Text>Esperado Efectivo:</Text>
                                        <Text fw={700}>${resumen.esperado_efectivo.toFixed(2)}</Text>
                                    </Group>
                                    <Group justify="space-between">
                                        <Text>Esperado Tarjeta:</Text>
                                        <Text fw={700}>${resumen.esperado_tarjeta.toFixed(2)}</Text>
                                    </Group>
                                    <Alert color={diferenciaRedondeada === 0 ? "green" : "red"} title="Diferencia" mt="md" icon={diferenciaRedondeada === 0 ? <IconCheck size={16} /> : <IconAlertCircle size={16} />}>
                                        <Text fw={700} size="lg">
                                            ${diferenciaRedondeada.toFixed(2)}
                                        </Text>
                                        {diferenciaRedondeada !== 0 && <Text size="sm">Revise las transacciones.</Text>}
                                    </Alert>
                                </Stack>
                            ) : (
                                <Text c="dimmed" fs="italic">Ingrese los montos contados para ver el comparativo.</Text>
                            )}
                        </Card>
                    </Grid.Col>
                </Grid>
            )}

            {resumen && !corteFinalizado && (
                <Button size="lg" color="blue" fullWidth onClick={handleFinalizarCorte} mt="xl">
                    Finalizar Corte y Guardar
                </Button>
            )}


            {corteFinalizado && (
                <Stack className="no-print">
                    <Button size="lg" variant="outline" fullWidth onClick={handleImprimir} mt="xl">
                        Imprimir Reporte
                    </Button>

                    {canReopen && (
                        <Button color="red" variant="subtle" onClick={handleReabrirCorte}>
                            Reabrir Turno
                        </Button>
                    )}
                </Stack>
            )}

            <div id="ticket-corte" className="ticket-print-only" style={{ display: 'none' }}>
                {resumen && (
                    <Stack gap="xs" align="center">
                        <Text fw={700} size="lg">CORTE DE CAJA</Text>
                        <Text size="sm">{resumen.fecha}</Text>
                        <Text size="sm">--------------------------------</Text>
                        <Group justify="space-between" w="100%">
                            <Text>Ventas brutas:</Text>
                            <Text>${resumen.total_ventas.toFixed(2)}</Text>
                        </Group>
                        <Text size="sm">--------------------------------</Text>
                        <Text fw={700}>ESPERADO</Text>
                        <Group justify="space-between" w="100%">
                            <Text>Efectivo:</Text>
                            <Text>${resumen.esperado_efectivo.toFixed(2)}</Text>
                        </Group>
                        <Group justify="space-between" w="100%">
                            <Text>Tarjeta:</Text>
                            <Text>${resumen.esperado_tarjeta.toFixed(2)}</Text>
                        </Group>
                        <Text size="sm">--------------------------------</Text>
                        <Text fw={700}>CONTADO</Text>
                        <Group justify="space-between" w="100%">
                            <Text>Efectivo:</Text>
                            <Text>${Number(contadoEfectivo).toFixed(2)}</Text>
                        </Group>
                        <Group justify="space-between" w="100%">
                            <Text>Tarjeta:</Text>
                            <Text>${Number(contadoTarjeta).toFixed(2)}</Text>
                        </Group>
                        <Text size="sm">--------------------------------</Text>
                        <Group justify="space-between" w="100%">
                            <Text fw={700}>DIFERENCIA:</Text>
                            <Text fw={700}>${diferenciaRedondeada.toFixed(2)}</Text>
                        </Group>
                        <Text size="sm" mt="md">Firma Cajero: ________________</Text>
                        <Text size="sm" mt="md">Firma Supervisor: _____________</Text>
                    </Stack>
                )}
            </div>
            <style>{`
                @media print {
                    body {
                        visibility: hidden;
                    }
                    #ticket-corte {
                        display: block !important;
                        visibility: visible;
                        position: fixed;
                        left: 0;
                        top: 0;
                        width: 80mm;
                        background: white;
                        z-index: 9999;
                        padding: 10px;
                        font-size: 12px;
                    }
                    #ticket-corte * {
                        visibility: visible;
                    }
                    .no-print {
                        display: none !important;
                    }
                }
            `}</style>
        </Stack>
    );
}

