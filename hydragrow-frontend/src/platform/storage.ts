import { isTauriRuntime } from './settings';

const STORE_FILE = 'device-state.json';

export const getItem = async <T = unknown>(key: string): Promise<T | null> => {
  if (isTauriRuntime()) {
    try {
      const { Store } = await import('@tauri-apps/plugin-store');
      const store = await Store.load(STORE_FILE);
      const value = await store.get<T>(key);
      return value ?? null;
    } catch {
      return null;
    }
  }

  try {
    const raw = window.localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : null;
  } catch {
    return null;
  }
};

export const setItem = async <T = unknown>(key: string, value: T): Promise<void> => {
  if (isTauriRuntime()) {
    const { Store } = await import('@tauri-apps/plugin-store');
    const store = await Store.load(STORE_FILE);
    await store.set(key, value);
    await store.save();
    return;
  }

  window.localStorage.setItem(key, JSON.stringify(value));
};
