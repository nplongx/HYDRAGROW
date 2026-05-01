import { invoke } from '@tauri-apps/api/core';
import { AppSettings } from '../types/models';

const SETTINGS_STORAGE_KEY = 'hydragrow_app_settings';

const isBrowser = typeof window !== 'undefined';

export const isTauriRuntime = () => isBrowser && '__TAURI_INTERNALS__' in window;

const normalizeSettings = (raw: any): AppSettings | null => {
  if (!raw || typeof raw !== 'object') return null;

  const backend_url = typeof raw.backend_url === 'string' ? raw.backend_url.trim() : '';
  const api_key = typeof raw.api_key === 'string' ? raw.api_key.trim() : '';
  const device_id = typeof raw.device_id === 'string' ? raw.device_id.trim() : '';

  return { backend_url, api_key, device_id };
};

const loadWebSettings = async (): Promise<AppSettings | null> => {
  const winConfig = normalizeSettings((window as any).__APP_CONFIG__);
  if (winConfig) return winConfig;

  const localRaw = localStorage.getItem(SETTINGS_STORAGE_KEY);
  if (localRaw) {
    try {
      const localSettings = normalizeSettings(JSON.parse(localRaw));
      if (localSettings) return localSettings;
    } catch (_) { }
  }

  try {
    const res = await window.fetch('/config.json');
    if (res.ok) {
      const json = await res.json();
      const remoteSettings = normalizeSettings(json);
      if (remoteSettings) return remoteSettings;
    }
  } catch (_) { }

  return null;
};

export const loadAppSettings = async (): Promise<AppSettings | null> => {
  if (isTauriRuntime()) {
    const tauriSettings = await invoke<AppSettings | null>('load_settings').catch(() => null);
    return normalizeSettings(tauriSettings);
  }

  return loadWebSettings();
};

export const saveWebSettings = (settings: AppSettings) => {
  if (!isBrowser || isTauriRuntime()) return;
  localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
};

export const hasRequiredRemoteConfig = (settings: AppSettings | null) => {
  return Boolean(settings?.backend_url && settings?.api_key);
};

export const saveAppSettings = async (settings: AppSettings): Promise<void> => {
  if (isTauriRuntime()) {
    await invoke('save_settings', {
      apiKey: settings.api_key,
      backendUrl: settings.backend_url,
      deviceId: settings.device_id
    });
    return;
  }

  saveWebSettings(settings);
};

