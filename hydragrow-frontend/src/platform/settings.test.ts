import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { forgetStoredApiKey } from './settings';
import { invoke } from '@tauri-apps/api/core';

// Mock the invoke function
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const SETTINGS_STORAGE_KEY = 'hydragrow_app_settings';
const SESSION_API_KEY_STORAGE_KEY = 'hydragrow_session_api_key';

describe('forgetStoredApiKey', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sessionStorage.clear();
    localStorage.clear();

    // Default to web environment for most tests
    delete (window as any).__TAURI_INTERNALS__;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Tauri environment', () => {
    it('should invoke forget_api_key command and not touch web storage', async () => {
      // Set up Tauri environment
      (window as any).__TAURI_INTERNALS__ = true;

      const spySessionRemove = vi.spyOn(sessionStorage, 'removeItem');
      const spyLocalGet = vi.spyOn(localStorage, 'getItem');

      await forgetStoredApiKey();

      expect(invoke).toHaveBeenCalledWith('forget_api_key');
      expect(spySessionRemove).not.toHaveBeenCalled();
      expect(spyLocalGet).not.toHaveBeenCalled();
    });
  });

  describe('Web environment', () => {
    it('should remove API key from sessionStorage', async () => {
      sessionStorage.setItem(SESSION_API_KEY_STORAGE_KEY, 'test-key');

      await forgetStoredApiKey();

      expect(sessionStorage.getItem(SESSION_API_KEY_STORAGE_KEY)).toBeNull();
    });

    it('should remove api_key from localStorage settings while keeping other settings', async () => {
      const initialSettings = {
        backend_url: 'http://localhost:8080',
        api_key: 'secret-key',
        device_id: 'device-123'
      };

      localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(initialSettings));

      await forgetStoredApiKey();

      const updatedRaw = localStorage.getItem(SETTINGS_STORAGE_KEY);
      expect(updatedRaw).not.toBeNull();

      const updatedSettings = JSON.parse(updatedRaw!);
      expect(updatedSettings.api_key).toBeUndefined();
      expect(updatedSettings.backend_url).toBe('http://localhost:8080');
      expect(updatedSettings.device_id).toBe('device-123');
    });

    it('should handle missing localStorage settings gracefully', async () => {
      const spyLocalSet = vi.spyOn(localStorage, 'setItem');

      await forgetStoredApiKey();

      expect(spyLocalSet).not.toHaveBeenCalled();
    });

    it('should handle invalid JSON in localStorage gracefully', async () => {
      localStorage.setItem(SETTINGS_STORAGE_KEY, 'invalid-json');
      const spyLocalSet = vi.spyOn(localStorage, 'setItem');

      await forgetStoredApiKey();

      expect(spyLocalSet).not.toHaveBeenCalled();
      expect(localStorage.getItem(SETTINGS_STORAGE_KEY)).toBe('invalid-json');
    });
  });
});
