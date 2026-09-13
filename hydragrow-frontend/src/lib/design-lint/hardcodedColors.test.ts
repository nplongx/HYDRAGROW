import { describe, expect, it } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { findHardcodedColorUsages } from './hardcodedColors';
import allowlist from './colorAllowlist.json';

const SRC_DIR = path.resolve(__dirname, '../../');

function listTsxFiles(dir: string): string[] {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  return entries.flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return listTsxFiles(full);
    if (entry.name.endsWith('.tsx') && !entry.name.endsWith('.test.tsx')) return [full];
    return [];
  });
}

describe('hardcoded color drift guard', () => {
  it('finds zero non-allowlisted hardcoded off-brand color utilities', () => {
    const files = listTsxFiles(SRC_DIR);
    const allowedPaths: string[] = allowlist.allowedPathPrefixes;
    const violations = files
      .filter((f) => !allowedPaths.some((prefix) => f.includes(prefix)))
      .flatMap((f) => findHardcodedColorUsages(fs.readFileSync(f, 'utf-8'), f));

    if (violations.length > 0) {
      const summary = violations.map((v) => `${v.file}:${v.line} -> ${v.match}`).join('\n');
      throw new Error(`Found ${violations.length} hardcoded color usage(s) outside the allowlist:\n${summary}`);
    }
    expect(violations).toHaveLength(0);
  });
});
