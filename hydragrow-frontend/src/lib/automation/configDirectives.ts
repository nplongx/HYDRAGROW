import type { GraphNode } from './contextVariables';

export interface ContextReadDirective {
  configKey: string;
  saveToVariable: string;
}

export interface ConfigOverwriteDirective {
  configKey: string;
  value: string;
  readOriginalBeforeWrite: boolean;
  restoreMode: 'on_condition_false';
}

/**
 * Flattens every `config` canvas node into the two directives the backend
 * consumes at eval time (see hydragrow-backend/src/services/config_context.rs):
 * every `variant: 'read'` node becomes a contextRead; only the FIRST
 * `variant: 'overwrite'` node becomes configOverwrite — a v1 limitation
 * matching the existing "only the first action compiles" rule in
 * compileToRhai.ts. Pure and synchronous, safe to call on every save.
 */
export function collectConfigDirectives(nodes: GraphNode[]): {
  contextReads: ContextReadDirective[];
  configOverwrite?: ConfigOverwriteDirective;
} {
  const contextReads: ContextReadDirective[] = [];
  let configOverwrite: ConfigOverwriteDirective | undefined;

  for (const node of nodes) {
    if (node.type !== 'config') continue;
    const configKey = typeof node.data.configKey === 'string' ? node.data.configKey : '';
    if (!configKey) continue;

    if (node.data.variant === 'read') {
      const saveToVariable = typeof node.data.saveToVariable === 'string' ? node.data.saveToVariable : '';
      if (saveToVariable) contextReads.push({ configKey, saveToVariable });
    } else if (node.data.variant === 'overwrite' && !configOverwrite) {
      const value = node.data.overrideValue !== undefined ? String(node.data.overrideValue) : '';
      if (value) {
        configOverwrite = {
          configKey,
          value,
          readOriginalBeforeWrite: Boolean(node.data.readOriginalBeforeWrite),
          restoreMode: 'on_condition_false',
        };
      }
    }
  }

  return { contextReads, configOverwrite };
}
