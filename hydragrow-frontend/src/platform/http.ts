import { isTauriRuntime } from './settings';

const DEFAULT_WEB_TIMEOUT_MS = 12_000;

const fetchWithTimeout: typeof fetch = async (input, init = {}) => {
  if (init.signal) {
    return window.fetch(input, init);
  }

  const requestedTimeout = Number((init as any).timeout || (init as any).connectTimeout || DEFAULT_WEB_TIMEOUT_MS);
  const timeoutMs = Number.isFinite(requestedTimeout) && requestedTimeout > 0
    ? requestedTimeout
    : DEFAULT_WEB_TIMEOUT_MS;
  const { timeout: _timeout, connectTimeout: _connectTimeout, ...fetchInit } = init as RequestInit & {
    timeout?: number;
    connectTimeout?: number;
  };

  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), timeoutMs);

  try {
    return await window.fetch(input, { ...fetchInit, signal: controller.signal });
  } finally {
    window.clearTimeout(timeout);
  }
};

export const httpFetch: typeof fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
  if (isTauriRuntime()) {
    const { fetch } = await import('@tauri-apps/plugin-http');
    return fetch(input as any, init as any) as any;
  }

  return fetchWithTimeout(input, init);
};
