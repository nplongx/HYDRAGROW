import { useEffect, useRef } from 'react';
import * as Blockly from 'blockly/core';
import 'blockly/blocks';
import { registerHydragrowBlocks } from './blockly/blocks';
import { extractActions, extractConditions } from './blockly/extractIr';
import { FSM_FIELDS, SENSOR_FIELDS, type Action, type AutomationIr, type Condition } from '../../lib/automation/ir';

export interface BlockLogicEditorProps {
  kind: AutomationIr['kind'];
  onChange: (result: { conditions: Condition[]; actions: Action[] }) => void;
  className?: string;
}

function toolboxFor(kind: AutomationIr['kind']) {
  return {
    kind: 'flyoutToolbox',
    contents: [
      { kind: 'block', type: 'hydragrow_sensor_condition' },
      { kind: 'block', type: kind === 'alert' ? 'hydragrow_alert_action' : 'hydragrow_advance_stage_action' },
    ],
  };
}

export function BlockLogicEditor({ kind, onChange, className }: BlockLogicEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const workspaceRef = useRef<Blockly.WorkspaceSvg | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    registerHydragrowBlocks(kind === 'alert' ? SENSOR_FIELDS : FSM_FIELDS);
    const workspace = Blockly.inject(containerRef.current, { toolbox: toolboxFor(kind) });
    workspaceRef.current = workspace;

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
    // eslint-disable-next-line react-hooks/exhaustive-deps -- re-mount only when kind changes, not on every onChange
  }, [kind]);

  return <div ref={containerRef} className={className ?? 'h-80 w-full'} />;
}
