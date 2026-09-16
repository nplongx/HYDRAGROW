import { httpFetch } from '../platform/http';
import { loadAppSettings, getDefaultBackendUrl } from '../platform/settings';

async function getApiConfig(): Promise<{ backendUrl: string; apiKey: string }> {
    const settings = await loadAppSettings();
    return {
        backendUrl: settings?.backend_url || getDefaultBackendUrl(),
        apiKey: settings?.api_key || '',
    };
}

export type ApiRequestOptions = {
    timeoutMs?: number;
    signal?: AbortSignal;
};

export class ApiError extends Error {
    readonly status: number;
    readonly code?: string;
    readonly details?: unknown;
    readonly requestId?: string;

    constructor(input: { status: number; message: string; code?: string; details?: unknown; requestId?: string }) {
        super(input.message);
        this.name = 'ApiError';
        this.status = input.status;
        this.code = input.code;
        this.details = input.details;
        this.requestId = input.requestId;
    }
}

async function throwApiError(res: Response, method: string, path: string): Promise<never> {
    let payload: unknown;
    try { payload = await res.json(); } catch { payload = undefined; }
    const error = payload && typeof payload === 'object'
        ? (payload as { error?: Record<string, unknown> }).error
        : undefined;
    throw new ApiError({
        status: res.status,
        message: typeof error?.message === 'string' ? error.message : `${method} ${path} failed with status ${res.status}`,
        code: typeof error?.code === 'string' ? error.code : undefined,
        details: error?.details,
        requestId: typeof error?.request_id === 'string' ? error.request_id : undefined,
    });
}

export async function apiGet<T>(url: string, options?: ApiRequestOptions): Promise<T> {
    const { backendUrl, apiKey } = await getApiConfig();
    const res = await httpFetch(`${backendUrl}/api${url}`, {
        method: 'GET',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': apiKey,
        },
        ...(options?.timeoutMs ? { timeout: options.timeoutMs } : {}),
        ...(options?.signal ? { signal: options.signal } : {}),
    } as RequestInit & { timeout?: number });
    if (!res.ok) {
        return throwApiError(res, 'GET', url);
    }
    return res.json();
}

export async function apiPut<T>(path: string, body: unknown, options?: ApiRequestOptions): Promise<T> {
    const { backendUrl, apiKey } = await getApiConfig();
    const res = await httpFetch(`${backendUrl}/api${path}`, {
        method: 'PUT',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': apiKey,
        },
        body: JSON.stringify(body),
        ...(options?.timeoutMs ? { timeout: options.timeoutMs } : {}),
        ...(options?.signal ? { signal: options.signal } : {}),
    } as RequestInit & { timeout?: number });
    if (!res.ok) {
        return throwApiError(res, 'PUT', path);
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
    headers?: Record<string, string>,
    options?: ApiRequestOptions,
): Promise<T> {
    const { backendUrl, apiKey } = await getApiConfig();
    const res = await httpFetch(`${backendUrl}/api${url}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': apiKey,
            ...headers
        },
        ...(options?.timeoutMs ? { timeout: options.timeoutMs } : {}),
        ...(options?.signal ? { signal: options.signal } : {}),
        body: JSON.stringify(body)
    } as RequestInit & { timeout?: number });
    if (!res.ok) {
        return throwApiError(res, 'POST', url);
    }
    return res.json() as Promise<T>;
}

export async function apiPatch<T, B = Record<string, unknown>>(
    url: string,
    body: B,
    headers?: Record<string, string>,
    options?: ApiRequestOptions
): Promise<T> {
    const { backendUrl, apiKey } = await getApiConfig();
    const res = await httpFetch(`${backendUrl}/api${url}`, {
        method: 'PATCH',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': apiKey,
            ...headers
        },
        ...(options?.timeoutMs ? { timeout: options.timeoutMs } : {}),
        ...(options?.signal ? { signal: options.signal } : {}),
        body: JSON.stringify(body)
    } as RequestInit & { timeout?: number });
    if (!res.ok) {
        return throwApiError(res, 'PATCH', url);
    }
    return res.json() as Promise<T>;
}

export async function apiDelete<T>(url: string, options?: ApiRequestOptions): Promise<T> {
    const { backendUrl, apiKey } = await getApiConfig();
    const res = await httpFetch(`${backendUrl}/api${url}`, {
        method: 'DELETE',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': apiKey,
        },
        ...(options?.timeoutMs ? { timeout: options.timeoutMs } : {}),
        ...(options?.signal ? { signal: options.signal } : {}),
    } as RequestInit & { timeout?: number });
    if (!res.ok) {
        return throwApiError(res, 'DELETE', url);
    }
    return res.json() as Promise<T>;
}
