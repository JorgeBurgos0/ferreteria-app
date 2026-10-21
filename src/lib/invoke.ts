import { invoke as tauriInvoke } from '@tauri-apps/api/core';

let sessionToken = '';

export function setSessionToken(token: string) {
  sessionToken = token;
}

export function clearSessionToken() {
  sessionToken = '';
}

export function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const normalizedArgs = args && Object.fromEntries(
    Object.entries(args).map(([key, value]) => [
      key.replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase()), value,
    ]),
  );
  return tauriInvoke<T>(command, normalizedArgs, {
    headers: sessionToken ? { 'x-session-token': sessionToken } : {},
  });
}
