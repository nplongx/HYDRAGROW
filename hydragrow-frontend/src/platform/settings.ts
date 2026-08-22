// src/platform/settings.ts
import { invoke } from '@tauri-apps/api/core';
import { AppSettings } from '../types/models';

const SETTINGS_STORAGE_KEY = 'hydragrow_app_settings';
const SESSION_API_KEY_STORAGE_KEY = 'hydragrow_session_api_key';

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
  const sessionApiKey = sessionStorage.getItem(SESSION_API_KEY_STORAGE_KEY)?.trim() || '';
  
  // 1. Ưu tiên cấu hình inject từ window
  const winConfig = normalizeSettings((window as any).__APP_CONFIG__);
  if (winConfig) return { ...winConfig, api_key: winConfig.api_key || sessionApiKey };

  // 2. Đọc từ localStorage
  const localRaw = localStorage.getItem(SETTINGS_STORAGE_KEY);
  if (localRaw) {
    try {
      const parsed = JSON.parse(localRaw);
      const localSettings = normalizeSettings(parsed);
      if (localSettings) {
        return {
          ...localSettings,
          api_key: sessionApiKey || localSettings.api_key || '',
        };
      }
    } catch (_) {}
  }

  // 3. Đọc từ file static /config.json (nếu có)
  // try {
  //   const res = await window.fetch('/config.json');
  //   if (res.ok) {
  //     const json = await res.json();
  //     const remoteSettings = normalizeSettings(json);
  //     if (remoteSettings) {
  //       return { ...remoteSettings, api_key: sessionApiKey || remoteSettings.api_key || '' };
  //     }
  //   }
  // } catch (_) {}

    return sessionApiKey ? { backend_url: window.location.origin, api_key: sessionApiKey, device_id: '' } : null;
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
  // Lưu cấu hình bao gồm cả API key vào localStorage để duy trì trạng thái đăng nhập
  localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));

  if (settings.api_key?.trim()) {
    sessionStorage.setItem(SESSION_API_KEY_STORAGE_KEY, settings.api_key.trim());
  } else {
    sessionStorage.removeItem(SESSION_API_KEY_STORAGE_KEY);
  }
};

export const forgetStoredApiKey = async (): Promise<void> => {
  if (!isBrowser) return;

  if (isTauriRuntime()) {
    await invoke('forget_api_key');
    return;
  }

  sessionStorage.removeItem(SESSION_API_KEY_STORAGE_KEY);
  const localRaw = localStorage.getItem(SETTINGS_STORAGE_KEY);
  if (localRaw) {
    try {
      const parsed = JSON.parse(localRaw);
      const { api_key: _, ...safeSettings } = parsed;
      localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(safeSettings));
    } catch (_) {}
  }
};

export const hasRequiredRemoteConfig = (settings: AppSettings | null) => {
  return Boolean(settings?.api_key);
};

export const saveAppSettings = async (settings: AppSettings): Promise<void> => {
  if (isTauriRuntime()) {
    await invoke('save_settings', {
      apiKey: settings.api_key,
      backendUrl: settings.backend_url,
      deviceId: settings.device_id,
    });
    return;
  }

  saveWebSettings(settings);
};
