import type { UserScript } from '../../types/automation';

export function wouldCreateCycle(
  candidateId: string,
  candidateNextIds: string[],
  targetScriptId: string,
  allScripts: UserScript[],
): boolean {
  if (candidateId === targetScriptId) return true;

  const graph = new Map<string, string[]>();
  for (const s of allScripts) {
    if (s.id === candidateId) {
      graph.set(s.id, [...candidateNextIds, targetScriptId]);
    } else {
      graph.set(s.id, s.ir_json?.next_flow_ids ?? []);
    }
  }
  if (!graph.has(candidateId)) {
    graph.set(candidateId, [...candidateNextIds, targetScriptId]);
  }

  const visited = new Set<string>();
  const stack = [candidateId];

  while (stack.length > 0) {
    const curr = stack.pop()!;
    const nexts = graph.get(curr) ?? [];
    for (const nxt of nexts) {
      if (nxt === candidateId) return true;
      if (!visited.has(nxt)) {
        visited.add(nxt);
        stack.push(nxt);
      }
    }
  }

  return false;
}
