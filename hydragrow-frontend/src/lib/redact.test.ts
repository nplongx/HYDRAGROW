import { describe, it, expect } from 'vitest';
import { redactSecret } from './redact';

describe('redactSecret', () => {
  it('returns [redacted] when input is null', () => {
    expect(redactSecret(null)).toBe('[redacted]');
  });

  it('returns [redacted] when input is undefined', () => {
    expect(redactSecret(undefined)).toBe('[redacted]');
  });

  it('returns [redacted] when input is an empty string', () => {
    expect(redactSecret('')).toBe('[redacted]');
  });

  it('redacts correctly with default visibleSuffixLength (4)', () => {
    expect(redactSecret('mysecrettoken1234')).toBe('[redacted]...1234');
  });

  it('redacts correctly with custom visibleSuffixLength', () => {
    expect(redactSecret('mysecrettoken1234', 2)).toBe('[redacted]...34');
  });

  it('handles strings shorter than visibleSuffixLength by appending the whole string to [redacted]...', () => {
    expect(redactSecret('abc')).toBe('[redacted]...abc');
  });

  it('handles exact visibleSuffixLength', () => {
    expect(redactSecret('1234')).toBe('[redacted]...1234');
  });

  it('safely converts numbers to strings and redacts', () => {
    expect(redactSecret(123456)).toBe('[redacted]...3456');
  });

  it('safely converts other types to strings (e.g. booleans)', () => {
    expect(redactSecret(true)).toBe('[redacted]...true');
    expect(redactSecret(false)).toBe('[redacted]...alse'); // 'false'.slice(-4) === 'alse'
  });
});
