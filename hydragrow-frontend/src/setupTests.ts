import '@testing-library/jest-dom';
import { vi } from 'vitest';

vi.mock('firebase/messaging', () => ({
  getMessaging: vi.fn().mockReturnValue({}),
  getToken: vi.fn().mockResolvedValue('test-token'),
  onMessage: vi.fn(),
  isSupported: vi.fn().mockResolvedValue(false),
}));
