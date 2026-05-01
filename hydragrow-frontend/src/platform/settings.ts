export type AppSettings = {
  api_key?: string;
  backend_url?: string;
  device_id?: string;
  [key: string]: any;
};

export const isTauriRuntime = (): boolean => typeof window !== 'undefined' && Boolean((window as any).__TAURI__);

export const loadSettings = async (): Promise<AppSettings | null> => {
  if (isTauriRuntime()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<AppSettings>('load_settings');
    } catch {
      return null;
    }
  }

  const envSettings: AppSettings = {
    api_key: (import.meta as any).env?.VITE_API_KEY,
    backend_url: (import.meta as any).env?.VITE_BACKEND_URL,
    device_id: (import.meta as any).env?.VITE_DEVICE_ID
  };

  if (envSettings.backend_url || envSettings.device_id || envSettings.api_key) {
    return envSettings;
  }

  try {
    const response = await window.fetch('/config.json', { method: 'GET' });
    if (!response.ok) return null;
    return await response.json();
  } catch {
    return null;
  }
};
