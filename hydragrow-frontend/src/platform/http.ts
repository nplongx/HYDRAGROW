import { isTauriRuntime } from './settings';
import { getIdToken } from '../lib/authToken';

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

/**
 * Gắn `Authorization: Bearer <Firebase ID token>` vào request, trừ khi caller
 * đã tự đặt header `Authorization` (giữ khả năng override khi cần).
 * Nếu đang dùng mock auth hoặc dev mode, tự động gắn `X-User-Id: 1` để backend nhận diện người dùng.
 */
function withAuthHeader(init?: RequestInit): RequestInit | undefined {
  const token = getIdToken();
  const headers = new Headers(init?.headers);

  if (token && !headers.has('Authorization')) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  if (typeof window !== 'undefined' && !headers.has('X-User-Id')) {
    const mockUserId = localStorage.getItem('mock_user_id') || (localStorage.getItem('mock_auth') === 'true' ? '1' : null);
    if (mockUserId) {
      headers.set('X-User-Id', mockUserId);
    }
  }

  return { ...init, headers };
}

export const httpFetch: typeof fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
  const authedInit = withAuthHeader(init);

  if (isTauriRuntime()) {
    const { fetch } = await import('@tauri-apps/plugin-http');
    return fetch(input as any, authedInit as any) as any;
  }

  return fetchWithTimeout(input, authedInit);
};
