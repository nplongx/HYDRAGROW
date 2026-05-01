import { isTauriRuntime } from './settings';

export const httpFetch: typeof fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
  if (isTauriRuntime()) {
    const { fetch } = await import('@tauri-apps/plugin-http');
    return fetch(input as any, init as any) as any;
  }

  return window.fetch(input, init);
};
