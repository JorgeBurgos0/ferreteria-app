import { useState, type FormEvent } from 'react';
import { Alert, Button, Center, Checkbox, Paper, PasswordInput, Radio, Stack, Text, Title } from '@mantine/core';
import { invoke } from '../lib/invoke';

export interface CryptoStatus { first_run: boolean; encrypted: boolean; locked: boolean }

export function AccesoBase({ status, onReady }: { status: CryptoStatus; onReady: () => void }) {
  const [choice, setChoice] = useState<'encrypted' | 'plain'>('encrypted');
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [accepted, setAccepted] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const firstRun = status.first_run;

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    const encrypt = firstRun && choice === 'encrypted';
    if (encrypt && (password.length < 12 || password !== confirmation || !accepted)) {
      setError('Confirma la contraseña maestra y acepta el aviso de recuperación.');
      return;
    }
    setBusy(true);
    try {
      if (firstRun) {
        await invoke('configurar_base_inicial', { encrypted: encrypt, password: encrypt ? password : null });
      } else {
        await invoke('desbloquear_base', { password });
      }
      setPassword('');
      setConfirmation('');
      onReady();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  return <Center style={{ minHeight: '100vh' }}>
    <Paper withBorder shadow="md" p={30} radius="md" style={{ width: 440 }}>
      <form onSubmit={submit}>
        <Stack gap="lg">
          <Title order={2} ta="center">{firstRun ? 'Preparar base de datos' : 'Desbloquear base de datos'}</Title>
          {firstRun ? <>
            <Text size="sm">Los datos se guardarán solo en este equipo. Elige cómo proteger la base local antes de crear la cuenta administradora.</Text>
            <Radio.Group label="Protección de datos" value={choice} onChange={(value) => setChoice(value as 'encrypted' | 'plain')}>
              <Stack mt="md" gap="md">
                <Radio value="encrypted" label="Cifrar la base (recomendado)" styles={{ label: { paddingLeft: 14, lineHeight: 1.5 } }} />
                <Radio value="plain" label="Guardar sin cifrar" styles={{ label: { paddingLeft: 14, lineHeight: 1.5 } }} />
              </Stack>
            </Radio.Group>
          </> : <Text size="sm">Introduce la contraseña maestra adicional para abrir los datos locales.</Text>}
          {(!firstRun || choice === 'encrypted') && <>
            <Alert color="red" title="Sin recuperación">Si pierdes la contraseña maestra, no podrás recuperar esta base ni sus respaldos cifrados. Guárdala en un lugar seguro.</Alert>
            <PasswordInput label="Contraseña maestra" autoFocus={!firstRun} autoComplete="off" value={password} onChange={(event) => setPassword(event.currentTarget.value)} required minLength={firstRun ? 12 : undefined} />
            {firstRun && <PasswordInput label="Confirmar contraseña maestra" autoComplete="off" value={confirmation} onChange={(event) => setConfirmation(event.currentTarget.value)} required minLength={12} />}
            {firstRun && <Checkbox label="Entiendo que perder esta contraseña impide recuperar los datos cifrados" checked={accepted} onChange={(event) => setAccepted(event.currentTarget.checked)} styles={{ label: { paddingLeft: 14, lineHeight: 1.5 } }} />}
          </>}
          {error && <Alert color="red">{error}</Alert>}
          <Button type="submit" loading={busy} disabled={firstRun && choice === 'encrypted' && !accepted}>{firstRun ? 'Crear base local' : 'Desbloquear'}</Button>
        </Stack>
      </form>
    </Paper>
  </Center>;
}
