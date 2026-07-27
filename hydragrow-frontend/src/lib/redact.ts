export const isDevelopment = import.meta.env.DEV;

export function redactSecret(value: unknown, visibleSuffixLength = 4): string {
  if (value === null || value === undefined) return '[redacted]';

  const text = String(value);
  if (!text) return '[redacted]';

  const suffix = text.slice(-visibleSuffixLength);
  return suffix ? `[redacted]...${suffix}` : '[redacted]';
}

export function debugLog(...args: unknown[]) {
  if (isDevelopment) {
    console.log(...args);
  }
}
