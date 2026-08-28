import { useEffect, useRef } from 'react';
import * as Blockly from 'blockly/core';
import 'blockly/blocks';
import { registerHydragrowBlocks } from './blockly/blocks';
import { extractActions, extractConditions } from './blockly/extractIr';
import type { Action, Condition } from '../../lib/automation/ir';

export interface BlockLogicEditorProps {
  /** Called on every workspace change with the current extracted IR fragment. */
  onChange: (result: { conditions: Condition[]; actions: Action[] }) => void;
  className?: string;
}

const TOOLBOX = {
  kind: 'flyoutToolbox',
  contents: [
    { kind: 'block', type: 'hydragrow_sensor_condition' },
    { kind: 'block', type: 'hydragrow_alert_action' },
  ],
};

export function BlockLogicEditor({ onChange, className }: BlockLogicEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const workspaceRef = useRef<Blockly.WorkspaceSvg | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    registerHydragrowBlocks();
    const workspace = Blockly.inject(containerRef.current, { toolbox: TOOLBOX });
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
    // eslint-disable-next-line react-hooks/exhaustive-deps -- onChange identity churn shouldn't re-mount Blockly
  }, []);

  return <div ref={containerRef} className={className ?? 'h-80 w-full'} />;
}
