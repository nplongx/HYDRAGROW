import { apiDelete, apiGet, apiPost, apiPut } from '../lib/apiClient';
import type { CropSeason } from '../types/models';

export type SeasonPayload = { name: string; plant_type?: string; description?: string; recipe_id?: string };

export interface SeasonPhoto {
  id: string;
  day_offset: number;
  cloudinary_url: string;
}

export interface SeasonPhotoSignResponse {
  api_key: string;
  timestamp: number;
  signature: string;
  folder: string;
  signature_algorithm: string;
  cloud_name: string;
}

export const seasonsApi = {
  active: (deviceId: string, signal?: AbortSignal) => apiGet<{ data?: CropSeason }>(`/devices/${deviceId}/seasons/active`, { signal }).then((r) => r.data ?? null),
  history: (deviceId: string, signal?: AbortSignal) => apiGet<{ data?: CropSeason[] }>(`/devices/${deviceId}/seasons`, { signal }).then((r) => r.data ?? []),
  create: (deviceId: string, payload: SeasonPayload) => apiPost(`/devices/${deviceId}/seasons`, payload),
  updateActive: (deviceId: string, payload: { name: string; plant_type: string; description: string }) => apiPut(`/devices/${deviceId}/seasons/active`, payload),
  end: (deviceId: string) => apiPut(`/devices/${deviceId}/seasons/active/end`, {}),
  remove: (deviceId: string, seasonId: string) => apiDelete(`/devices/${deviceId}/seasons/${seasonId}`),
};

export const seasonPhotosApi = {
  list: (deviceId: string, seasonId: string, signal?: AbortSignal) =>
    apiGet<{ data?: SeasonPhoto[] }>(`/devices/${deviceId}/seasons/${seasonId}/photos`, { signal }).then((r) => r.data ?? []),
  signUpload: (deviceId: string, seasonId: string) =>
    apiPost<{ data: SeasonPhotoSignResponse }>(`/devices/${deviceId}/seasons/${seasonId}/photos/sign`, {}),
  create: (deviceId: string, seasonId: string, payload: { cloudinary_public_id: string; cloudinary_url: string; day_offset: number }) =>
    apiPost(`/devices/${deviceId}/seasons/${seasonId}/photos`, payload),
  remove: (deviceId: string, seasonId: string, photoId: string) =>
    apiDelete(`/devices/${deviceId}/seasons/${seasonId}/photos/${photoId}`),
};
