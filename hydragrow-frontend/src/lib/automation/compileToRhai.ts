import type { Action, AutomationIr, Condition } from "./ir";

// Rhai string literals: escape backslash first, then double quotes.
function rhaiString(s: string): string {
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function conditionToRhai(c: Condition): string {
  return `input.${c.sensor} ${c.operator} ${c.value}`;
}

// Rhai has no `&&`-free early-exit guard clause style here — we build a single
// `if (!(A && B && ...)) { return (); }` guard, matching the hand-written scripts
// already accepted by ScriptEngine::eval_alert / eval_recipe_override.
function guardClause(conditions: Condition[]): string {
  const joined = conditions.map(conditionToRhai).join(" && ");
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
        "#{",
        ` "target_stage_index": ${offsetExpr},`,
        ` "reason": "${rhaiString(action.reason)}"`,
        "}",
      ].join("\n ");
    }
    case "dose":
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
