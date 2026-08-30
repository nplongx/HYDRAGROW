import { useEffect, useRef } from 'react';
import * as Blockly from 'blockly/core';
import 'blockly/blocks';
import { registerHydragrowBlocks } from './blockly/blocks';
import { extractActions, extractConditions } from './blockly/extractIr';
import { hydrateWorkspace } from './blockly/hydrateIr';
import { FSM_FIELDS, SENSOR_FIELDS, type Action, type AutomationIr, type Condition } from '../../lib/automation/ir';

export interface BlockLogicEditorProps {
  kind: AutomationIr['kind'];
  onChange: (result: { conditions: Condition[]; actions: Action[] }) => void;
  /** Dữ liệu của một Flow đã lưu, dùng để vẽ lại block khi mở chi tiết một Flow có
   * sẵn (xem hydrateIr.ts). Bỏ trống khi tạo Flow mới. */
  initialConditions?: Condition[];
  initialActions?: Action[];
  className?: string;
}

function toolboxFor(kind: AutomationIr['kind']) {
  if (kind === 'action_command') {
    return {
      kind: 'flyoutToolbox',
      contents: [
        { kind: 'block', type: 'hydragrow_sensor_condition' },
        { kind: 'block', type: 'hydragrow_dose_action' },
        { kind: 'block', type: 'hydragrow_water_action' },
        { kind: 'block', type: 'hydragrow_emergency_stop_action' },
      ],
    };
  }
  return {
    kind: 'flyoutToolbox',
    contents: [
      { kind: 'block', type: 'hydragrow_sensor_condition' },
      { kind: 'block', type: kind === 'alert' ? 'hydragrow_alert_action' : 'hydragrow_advance_stage_action' },
    ],
  };
}

export function BlockLogicEditor({
  kind,
  onChange,
  initialConditions,
  initialActions,
  className,
}: BlockLogicEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const workspaceRef = useRef<Blockly.WorkspaceSvg | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    registerHydragrowBlocks(kind === 'recipe_override' ? FSM_FIELDS : SENSOR_FIELDS);
    const workspace = Blockly.inject(containerRef.current, { toolbox: toolboxFor(kind) });
    workspaceRef.current = workspace;

    if ((initialConditions?.length ?? 0) > 0 || (initialActions?.length ?? 0) > 0) {
      hydrateWorkspace(workspace, initialConditions ?? [], initialActions ?? []);
    }
    const listener = () => {
      onChange({
        conditions: extractConditions(workspace),
        actions: extractActions(workspace),
      });
    };
    workspace.addChangeListener(listener);

    return () => {
      workspace.removeChangeListener(listener);
      workspace.dispose();
    };

    // initialConditions/initialActions/onChange cố tình không nằm trong deps — chúng chỉ
    // dùng để SEED lần mount đầu, không phải để đồng bộ liên tục với parent re-render.
  }, [kind]);

  return <div ref={containerRef} className={className ?? 'h-80 w-full'} />;
}
