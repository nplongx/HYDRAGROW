import { describe, expect, it } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';
import { findOrphanClasses } from './orphanClasses';

describe('orphan ui-*/farm-* class guard', () => {
  it('finds zero classes referenced in JSX but never defined in App.css', () => {
    const srcDir = path.resolve(__dirname, '../../');
    const appCss = fs.readFileSync(path.join(srcDir, 'App.css'), 'utf-8');
    const orphans = findOrphanClasses(srcDir, appCss);
    if (orphans.length > 0) {
      throw new Error(`Orphan classes (referenced but never defined): ${orphans.join(', ')}`);
    }
    expect(orphans).toHaveLength(0);
  });
});
