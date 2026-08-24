import { httpFetch } from '../platform/http';
import { useDeviceStore } from '../store/useDeviceStore';

export async function apiGet<T>(url: string): Promise<T> {
    const settings = useDeviceStore.getState().settings;
    const res = await httpFetch(`${settings?.backend_url}/api${url}`, {
        method: 'GET',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': settings?.api_key || '',
        },
    });
    if (!res.ok) {
        throw new Error(`GET ${url} failed with status ${res.status}`);
    }
    return res.json();
}

export async function apiPut<T>(path: string, body: unknown): Promise<T> {
    const settings = useDeviceStore.getState().settings;
    const res = await httpFetch(`${settings?.backend_url}/api${path}`, {
        method: 'PUT',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': settings?.api_key || '',
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
    const settings = useDeviceStore.getState().settings;
    const res = await httpFetch(`${settings?.backend_url}/api${url}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': settings?.api_key || '',
            ...headers
        },
        body: JSON.stringify(body)
    });
    if (!res.ok) {
        throw new Error(`POST ${url} failed with status ${res.status}`);
    }
    return res.json() as Promise<T>;
}

export async function apiDelete<T>(url: string): Promise<T> {
    const settings = useDeviceStore.getState().settings;
    const res = await httpFetch(`${settings?.backend_url}/api${url}`, {
        method: 'DELETE',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': settings?.api_key || '',
        },
    });
    if (!res.ok) {
        throw new Error(`DELETE ${url} failed with status ${res.status}`);
    }
    return res.json();
}
