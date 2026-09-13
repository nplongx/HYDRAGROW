import { AutomationIr } from "../../lib/automation/ir";
import { Switch } from "../ui/Switch";

interface FlowEditorHeaderProps {
  name: string;
  kind: AutomationIr["kind"];
  enabled: boolean;
  onChange: (
    updates: Partial<{
      name: string;
      kind: AutomationIr["kind"];
      enabled: boolean;
    }>,
  ) => void;
}

export function FlowEditorHeader({
  name,
  kind,
  enabled,
  onChange,
}: FlowEditorHeaderProps) {
  return (
    <div className="flex justify-between items-center mb-4">
      <div className="flex items-center gap-4">
        <input
          type="text"
          value={name}
          onChange={(e) => onChange({ name: e.target.value })}
          className="ui-input font-semibold text-lg"
        />
        <select
          value={kind}
          onChange={(e) =>
            onChange({ kind: e.target.value as AutomationIr["kind"] })
          }
          className="ui-input"
        >
          <option value="alert">Alert</option>
          <option value="action_command">Action Command</option>
          <option value="recipe_override">Recipe Override</option>
        </select>
      </div>
      <Switch
        checked={enabled}
        onChange={(next) => onChange({ enabled: next })}
        label="Kích hoạt"
      />
    </div>
  );
}
