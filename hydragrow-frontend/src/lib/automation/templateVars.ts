const TEMPLATE_TOKEN_RE = /\{\{\s*([a-zA-Z0-9_]+)\s*\}\}/g;

/** Extracts every `{{name}}` token from a template string, in order of first
 * appearance, de-duplicated. Used to warn the user when an Action·Alert
 * message references a variable that isn't in scope. */
export function extractTemplateTokens(text: string): string[] {
  const seen = new Set<string>();
  const ordered: string[] = [];
  let match: RegExpExecArray | null;
  TEMPLATE_TOKEN_RE.lastIndex = 0;
  while ((match = TEMPLATE_TOKEN_RE.exec(text)) !== null) {
    const name = match[1];
    if (!seen.has(name)) {
      seen.add(name);
      ordered.push(name);
    }
  }
  return ordered;
}

/** Renders a preview of a template string against a sample value map.
 * Tokens not present in `sample` are left as literal `{{token}}` text so the
 * editor can visually flag them as unresolved. */
export function renderTemplatePreview(text: string, sample: Record<string, string>): string {
  return text.replace(TEMPLATE_TOKEN_RE, (whole, name: string) =>
    name in sample ? sample[name] : whole,
  );
}
