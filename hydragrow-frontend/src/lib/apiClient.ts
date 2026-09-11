import { httpFetch } from '../platform/http';
import { useDeviceStore } from '../store/useDeviceStore';

function getBackendUrl(): string {
  const settings = useDeviceStore.getState().settings;
  if (settings?.backend_url) return settings.backend_url;
  try {
    const raw = typeof window !== 'undefined' ? localStorage.getItem('hydragrow_app_settings') : null;
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed.backend_url) return parsed.backend_url;
    }
  } catch {
    // window/localStorage unavailable or corrupt JSON — fall through to default
  }
  return 'http://localhost:8080';
}

function getApiKey(): string {
  const settings = useDeviceStore.getState().settings;
  if (settings?.api_key) return settings.api_key;
  try {
    const raw = typeof window !== 'undefined' ? localStorage.getItem('hydragrow_app_settings') : null;
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed.api_key) return parsed.api_key;
    }
  } catch {
    // window/localStorage unavailable or corrupt JSON — fall through to empty key
  }
  return '';
}

export async function apiGet<T>(url: string): Promise<T> {
    const backendUrl = getBackendUrl();
    const apiKey = getApiKey();
    const res = await httpFetch(`${backendUrl}/api${url}`, {
        method: 'GET',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': apiKey,
        },
    });
    if (!res.ok) {
        throw new Error(`GET ${url} failed with status ${res.status}`);
    }
    return res.json();
}

export async function apiPut<T>(path: string, body: unknown): Promise<T> {
    const backendUrl = getBackendUrl();
    const apiKey = getApiKey();
    const res = await httpFetch(`${backendUrl}/api${path}`, {
        method: 'PUT',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': apiKey,
        },
        body: JSON.stringify(body),
    });
    if (!res.ok) {
        throw new Error(`PUT ${path} failed with status ${res.status}`);
    }
    return res.json() as Promise<T>;
}

// Gửi bulk request: POST đến một endpoint với body chứa array device_ids
export async function apiBulkPost<T>(path: string, body: unknown): Promise<T> {
    return apiPost<T, unknown>(path, body);
}

export async function apiPost<T, B = Record<string, unknown>>(
    url: string,
    body: B,
    headers?: Record<string, string>
): Promise<T> {
    const backendUrl = getBackendUrl();
    const apiKey = getApiKey();
    const res = await httpFetch(`${backendUrl}/api${url}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': apiKey,
            ...headers
        },
        body: JSON.stringify(body)
    });
    if (!res.ok) {
        throw new Error(`POST ${url} failed with status ${res.status}`);
    }
    return res.json() as Promise<T>;
}

export async function apiPatch<T, B = Record<string, unknown>>(
    url: string,
    body: B,
    headers?: Record<string, string>
): Promise<T> {
    const backendUrl = getBackendUrl();
    const apiKey = getApiKey();
    const res = await httpFetch(`${backendUrl}/api${url}`, {
        method: 'PATCH',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': apiKey,
            ...headers
        },
        body: JSON.stringify(body)
    });
    if (!res.ok) {
        throw new Error(`PATCH ${url} failed with status ${res.status}`);
    }
    return res.json() as Promise<T>;
}

export async function apiDelete<T>(url: string): Promise<T> {
    const backendUrl = getBackendUrl();
    const apiKey = getApiKey();
    const res = await httpFetch(`${backendUrl}/api${url}`, {
        method: 'DELETE',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': apiKey,
        },
    });
    if (!res.ok) {
        throw new Error(`DELETE ${url} failed with status ${res.status}`);
    }
    return res.json() as Promise<T>;
}
