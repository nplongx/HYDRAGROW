import type { Action, AutomationIr, Condition, ConditionOrGroup } from "./ir";

// Rhai string literals: escape backslash first, then double quotes.
function rhaiString(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function conditionToRhai(c: Condition): string {
  if (c.mode && c.mode !== 'instant') {
    return `fetch_range_stat("${c.sensor}", "${c.mode}", ${c.windowSec}) ${c.operator} ${c.value}`;
  }
  // Khi có valueVariable, so sánh với 1 biến trong execution context (được backend
  // nạp vào `input` từ Config·Read hoặc từ context được Chain truyền tới) thay vì
  // literal `value` — xem hydragrow-backend/src/services/config_context.rs.
  const rhs = c.valueVariable ? `input.${c.valueVariable}` : `${c.value}`;
  return `input.${c.sensor} ${c.operator} ${rhs}`;
}

function isGroup(c: ConditionOrGroup): c is import("./ir").ConditionGroup {
  return "op" in c;
}

function conditionOrGroupToRhai(c: ConditionOrGroup): string {
  if (isGroup(c)) {
    const inner = c.children
      .map(conditionOrGroupToRhai)
      .join(c.op === "and" ? " && " : " || ");
    return `(${inner})`;
  }
  return conditionToRhai(c);
}

// Rhai has no `&&`-free early-exit guard clause style here — we build a single
// `if (!(A && B && ...)) { return (); }` guard, matching the hand-written scripts
// already accepted by ScriptEngine::eval_alert / eval_recipe_override.
function guardClause(conditions: ConditionOrGroup[]): string {
  const joined = conditions.map(conditionOrGroupToRhai).join(" && ");
  return `if !(${joined}) { return (); }`;
}

function actionToRhaiMap(action: Action): string {
  switch (action.type) {
    case "alert": {
      const title = action.title ?? action.message;
      return [
        "#{",
        ` "level": "${rhaiString(action.level)}",`,
        ` "title": "${rhaiString(title)}",`,
        ` "message": "${rhaiString(action.message)}"`,
        "}",
      ].join("\n ");
    }
    case "advance_stage": {
      const offsetExpr =
        action.targetStageOffset === 0
          ? "input.stage_index"
          : action.targetStageOffset > 0
            ? `input.stage_index + ${action.targetStageOffset}`
            : `input.stage_index - ${Math.abs(action.targetStageOffset)}`;
      return [
        '#{',
        ` "action": "advance_stage",`,
        ` "target_stage_index": ${offsetExpr},`,
        ` "reason": "${rhaiString(action.reason)}"`,
        "}",
      ].join("\n ");
    }
    case 'end_season':
      return [
        '#{',
        ` "action": "end_season",`,
        ` "reason": "${rhaiString(action.reason)}"`,
        '}',
      ].join('\n ');
    case 'dose':
      return [
        "#{",
        ` "action": "dose",`,
        ` "pump": "${rhaiString(action.pump)}",`,
        ` "dose_ml": ${action.doseMl},`,
        ` "pwm": ${action.pwm}`,
        "}",
      ].join("\n ");
    case "water_on":
      return [
        "#{",
        ` "action": "water_on",`,
        ` "pump": "${rhaiString(action.pump)}",`,
        ` "duration_sec": ${action.durationSec}`,
        "}",
      ].join("\n ");
    case "water_off":
      return [
        "#{",
        ` "action": "water_off",`,
        ` "pump": "${rhaiString(action.pump)}"`,
        "}",
      ].join("\n ");
    case "emergency_stop":
      return '#{ "action": "emergency_stop" }';
  }
}

/**
 * Compile Automation IR into a Rhai `fn main(input) { ... }` source string matching
 * the contract ScriptEngine::eval_alert / eval_recipe_override already enforce
 * (hydragrow-backend/src/services/script_engine.rs). Only the FIRST action in the
 * IR is compiled — the Rhai contract returns at most one Map per eval, matching
 * eval_recipe_override_scripts' "first non-empty wins" single-authority rule.
 */
export function compileToRhai(ir: AutomationIr): string {
  const guard = guardClause(ir.conditions);
  const action = actionToRhaiMap(ir.actions[0]);
  return `fn main(input) {\n ${guard}\n ${action}\n}\n`;
}
