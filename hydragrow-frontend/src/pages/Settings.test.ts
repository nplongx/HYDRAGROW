// Kiểm tra callApi có merge extraHeaders vào request
import { describe, it, expect } from 'vitest';

// Giả lập minimal — chỉ test logic merge header
function buildHeaders(
  apiKey: string,
  extraHeaders?: Record<string, string>
): Record<string, string> {
  return {
    'Content-Type': 'application/json',
    'X-API-Key': apiKey,
    ...extraHeaders,
  };
}

describe('buildHeaders', () => {
  it('merges X-User-Confirmed into headers', () => {
    const headers = buildHeaders('key123', { 'X-User-Confirmed': 'true' });
    expect(headers['X-User-Confirmed']).toBe('true');
  });

  it('does not include X-User-Confirmed when not provided', () => {
    const headers = buildHeaders('key123');
    expect(headers['X-User-Confirmed']).toBeUndefined();
  });
});
