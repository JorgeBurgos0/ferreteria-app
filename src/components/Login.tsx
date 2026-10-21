import { useEffect, useState, type FormEvent } from 'react';
import { Paper, Title, TextInput, PasswordInput, Button, Stack, Alert, Center, Text } from '@mantine/core';
import { IconAlertCircle } from '@tabler/icons-react';
import { invoke, setSessionToken, clearSessionToken } from '../lib/invoke';

interface UsuarioLogueado {
  id: number;
  username: string;
  rol: string;
  permisos: string[];
  token: string;
  requiere_cambio_password: boolean;
}

interface LoginProps {
  onLoginSuccess: (usuario: UsuarioLogueado) => void;
}

export function Login({ onLoginSuccess }: LoginProps) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [hasUsers, setHasUsers] = useState<boolean | null>(null);
  const [pendingUser, setPendingUser] = useState<UsuarioLogueado | null>(null);

  useEffect(() => {
    invoke<boolean>('hay_usuarios').then(setHasUsers).catch((reason) => {
      setError(String(reason));
    });
  }, []);

  const handleLogin = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    try {
      const user = await invoke<UsuarioLogueado>('login', { username, password });
      setSessionToken(user.token);
      if (user.requiere_cambio_password) {
        setPendingUser(user);
      } else {
        setPassword('');
        onLoginSuccess(user);
      }
    } catch (reason) {
      clearSessionToken();
      setError(String(reason));
    }
  };

  const handleInitialUser = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    if (newPassword !== confirmPassword) {
      setError('Las contraseñas no coinciden');
      return;
    }
    try {
      await invoke('crear_usuario_inicial', { password: newPassword });
      setNewPassword('');
      setConfirmPassword('');
      setUsername('admin');
      setHasUsers(true);
    } catch (reason) {
      setError(String(reason));
    }
  };

  const handlePasswordChange = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    if (newPassword !== confirmPassword) {
      setError('Las contraseñas no coinciden');
      return;
    }
    try {
      await invoke('cambiar_password', { passwordActual: password, passwordNueva: newPassword });
      setPassword('');
      setNewPassword('');
      setConfirmPassword('');
      if (pendingUser) {
        onLoginSuccess({ ...pendingUser, requiere_cambio_password: false });
      }
    } catch (reason) {
      setError(String(reason));
    }
  };

  return (
    <Center style={{ minHeight: '100vh' }}>
      <Paper withBorder shadow="md" p={30} radius="md" style={{ width: 400 }}>
        <Title ta="center" order={2} mb="md">
          {hasUsers === false ? 'Configurar administrador' : pendingUser ? 'Cambiar contraseña' : 'Iniciar sesión'}
        </Title>
        {hasUsers === null && !error ? <Text ta="center">Comprobando usuarios…</Text> : null}
        {hasUsers === false ? (
          <form onSubmit={handleInitialUser}>
            <Stack>
              <Text>La primera cuenta se llamará admin. Elige una contraseña de al menos 12 caracteres.</Text>
              <PasswordInput label="Contraseña nueva" value={newPassword} onChange={(event) => setNewPassword(event.currentTarget.value)} required minLength={12} />
              <PasswordInput label="Confirmar contraseña" value={confirmPassword} onChange={(event) => setConfirmPassword(event.currentTarget.value)} required minLength={12} />
              {error && <Alert icon={<IconAlertCircle size="1rem" />} color="red">{error}</Alert>}
              <Button type="submit">Crear administrador</Button>
            </Stack>
          </form>
        ) : pendingUser ? (
          <form onSubmit={handlePasswordChange}>
            <Stack>
              <Text>Debes cambiar la contraseña inicial o temporal para continuar.</Text>
              <PasswordInput label="Contraseña nueva" value={newPassword} onChange={(event) => setNewPassword(event.currentTarget.value)} required minLength={12} />
              <PasswordInput label="Confirmar contraseña" value={confirmPassword} onChange={(event) => setConfirmPassword(event.currentTarget.value)} required minLength={12} />
              {error && <Alert icon={<IconAlertCircle size="1rem" />} color="red">{error}</Alert>}
              <Button type="submit">Guardar contraseña</Button>
            </Stack>
          </form>
        ) : hasUsers === true ? (
          <form onSubmit={handleLogin}>
            <Stack>
              <TextInput label="Usuario" value={username} onChange={(event) => setUsername(event.currentTarget.value)} required />
              <PasswordInput label="Contraseña" value={password} onChange={(event) => setPassword(event.currentTarget.value)} required />
              {error && <Alert icon={<IconAlertCircle size="1rem" />} color="red">{error}</Alert>}
              <Button type="submit">Entrar</Button>
            </Stack>
          </form>
        ) : error ? <Alert icon={<IconAlertCircle size="1rem" />} color="red">{error}</Alert> : null}
      </Paper>
    </Center>
  );
}
