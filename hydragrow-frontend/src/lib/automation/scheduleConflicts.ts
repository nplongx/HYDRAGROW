import type { UserScript } from '../../types/automation';

export interface ScheduleConflict {
  flowA: UserScript;
  flowB: UserScript;
  sharedPumps: string[];
  cronExpression: string;
}

const pumpsControlledBy = (script: UserScript): string[] => {
  const actions = script.ir_json?.actions ?? [];
  const pumps = new Set<string>();
  for (const action of actions) {
    if (action.type === 'dose' || action.type === 'water_on' || action.type === 'water_off') {
      pumps.add(action.pump);
    }
  }
  return Array.from(pumps);
};

export function findScheduleConflicts(scripts: UserScript[]): ScheduleConflict[] {
  const cronScripts = scripts.filter(
    (s): s is UserScript & { ir_json: NonNullable<UserScript['ir_json']> } =>
      Boolean(s.enabled && s.ir_json?.trigger?.type === 'cron'),
  );

  const conflicts: ScheduleConflict[] = [];
  for (let i = 0; i < cronScripts.length; i++) {
    for (let j = i + 1; j < cronScripts.length; j++) {
      const a = cronScripts[i];
      const b = cronScripts[j];
      const cronA = a.ir_json.trigger as { type: 'cron'; cronExpression: string };
      const cronB = b.ir_json.trigger as { type: 'cron'; cronExpression: string };

      if (cronA.cronExpression !== cronB.cronExpression) continue;

      const sharedPumps = pumpsControlledBy(a).filter((p) => pumpsControlledBy(b).includes(p));
      if (sharedPumps.length > 0) {
        conflicts.push({ flowA: a, flowB: b, sharedPumps, cronExpression: cronA.cronExpression });
      }
    }
  }
  return conflicts;
}
