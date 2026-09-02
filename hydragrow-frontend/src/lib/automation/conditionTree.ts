import type { Condition, ConditionGroup, ConditionOrGroup } from './ir';

function isGroup(c: ConditionOrGroup): c is ConditionGroup {
  return 'op' in c;
}

export function toEditorRoot(stored: ConditionOrGroup[]): ConditionGroup {
  if (stored.length === 1 && isGroup(stored[0])) {
    return stored[0];
  }
  return { op: 'and', children: stored };
}

export function fromEditorRoot(root: ConditionGroup): ConditionOrGroup[] {
  return root.op === 'and' ? root.children : [root];
}

export function countLeafConditions(items: ConditionOrGroup[]): number {
  return items.reduce(
    (sum, c) => sum + (isGroup(c) ? countLeafConditions(c.children) : 1),
    0,
  );
}

function summarizeLeaf(c: Condition): string {
  return `${c.sensor} ${c.operator} ${c.value}`;
}

function summarizeOne(c: ConditionOrGroup): string {
  if (!isGroup(c)) return summarizeLeaf(c);
  const inner = c.children.map(summarizeOne).join(c.op === 'and' ? ' và ' : ' hoặc ');
  return c.children.length > 1 ? `(${inner})` : inner;
}

export function summarizeConditionTree(items: ConditionOrGroup[]): string {
  if (items.length === 0) return 'Chưa cấu hình';
  return items.map(summarizeOne).join(' và ');
}
