import { apiGet, apiPatch, apiPost } from '../lib/apiClient';

export type AdminUserRole = 'admin' | 'operator' | 'viewer';

export interface AdminUser {
  id: number;
  firebase_uid: string;
  email: string;
  display_name: string | null;
  role: AdminUserRole | null;
  scopes: string[];
  is_active: boolean;
  created_at?: string;
  updated_at?: string;
}

export interface ScopeInfo {
  scope: string;
  description: string;
}

export interface ProvisionUserPayload {
  firebase_uid: string;
  email: string;
  display_name: string | null;
  scopes: string[];
}

export interface UpdateUserPayload {
  role?: AdminUserRole;
  scopes?: string[];
  is_active?: boolean;
}

type AdminListResponse<T> = { status: string; data: T };

export const adminApi = {
  listUsers: async (signal?: AbortSignal) => {
    const response = await apiGet<AdminListResponse<AdminUser[]>>('/admin/users', { signal });
    return Array.isArray(response.data) ? response.data : [];
  },

  listScopes: () => apiGet<ScopeInfo[]>('/admin/scopes'),

  provisionUser: (payload: ProvisionUserPayload) =>
    apiPost('/admin/users', payload),

  updateUser: (userId: number, payload: UpdateUserPayload) =>
    apiPatch(`/admin/users/${userId}`, payload),
};
