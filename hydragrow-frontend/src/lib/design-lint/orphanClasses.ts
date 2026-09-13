import fs from 'node:fs';
import path from 'node:path';

function listTsxFiles(dir: string): string[] {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  return entries.flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return listTsxFiles(full);
    if (entry.name.endsWith('.tsx') && !entry.name.endsWith('.test.tsx')) return [full];
    return [];
  });
}

export function findOrphanClasses(srcDir: string, appCssContent: string): string[] {
  const usedClasses = new Set<string>();
  const CLASS_REF = /\b(ui|farm)-[a-z-]+\b/g;
  for (const file of listTsxFiles(srcDir)) {
    const content = fs.readFileSync(file, 'utf-8');
    const matches = content.match(CLASS_REF);
    matches?.forEach((m) => usedClasses.add(m));
  }

  const definedClasses = new Set<string>();
  // Matches ".ui-foo {" or ".farm-bar," at any indentation level inside @layer blocks.
  const CLASS_DEF = /^\s*\.((?:ui|farm)-[a-z-]+)/gm;
  let defMatch: RegExpExecArray | null;
  while ((defMatch = CLASS_DEF.exec(appCssContent)) !== null) {
    definedClasses.add(defMatch[1]);
  }

  return [...usedClasses].filter((c) => !definedClasses.has(c)).sort();
}
