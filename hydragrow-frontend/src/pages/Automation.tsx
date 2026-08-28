import { useState } from 'react';
import { ReactFlow, Background, Controls, type Node } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import toast from 'react-hot-toast';
import { AUTOMATION_NODE_TYPES } from '../components/automation/reactflow/nodeTypes';
import { NodePalette } from '../components/automation/reactflow/NodePalette';
import { NodeEditorPanel } from '../components/automation/reactflow/NodeEditorPanel';
import { buildIrFromGraph } from '../components/automation/reactflow/buildIr';
import { BlockLogicEditor } from '../components/automation/BlockLogicEditor';
import { compileToRhai } from '../lib/automation/compileToRhai';
import { AutomationIrSchema, type AutomationIr } from '../lib/automation/ir';
import { useAutomationBuilder } from '../hooks/useAutomationBuilder';
import { useCreateAutomationScript, useValidateAutomationScript } from '../hooks/useAutomationScripts';
import { useDeviceStore } from '../store/useDeviceStore';

export default function Automation() {
  const deviceId = useDeviceStore((s) => s.settings?.device_id ?? '');
  const [name, setName] = useState('Automation mới');
  const validateScript = useValidateAutomationScript(deviceId);
  const createScript = useCreateAutomationScript(deviceId);
  const builder = useAutomationBuilder();

  const handleSaveAndDeploy = async () => {
    const ir: AutomationIr =
      builder.mode === 'reactflow'
        ? buildIrFromGraph({ kind: builder.kind, nodes: builder.nodes, edges: builder.edges })
        : {
            kind: builder.kind,
            trigger: { type: builder.kind === 'alert' ? 'sensor' : 'fsm' },
            conditions: builder.blocklyResult.conditions,
            actions: builder.blocklyResult.actions,
            nodes: [],
            edges: [],
          };
    const parsed = AutomationIrSchema.safeParse(ir);
    if (!parsed.success) {
      toast.error(`IR không hợp lệ: ${parsed.error.issues[0]?.message}`);
      return;
    }
    const source = compileToRhai(parsed.data);

    const validation = await validateScript.mutateAsync({ kind: parsed.data.kind, name, source, ir_json: parsed.data });
    if (!validation.valid) {
      toast.error(`Script không hợp lệ: ${validation.error}`);
      return;
    }

    await createScript.mutateAsync({ kind: parsed.data.kind, name, source, ir_json: parsed.data });
    toast.success('Đã lưu và deploy automation');
  };

  return (
    <div className="flex h-full flex-col gap-2 p-4">
      <div className="flex items-center gap-2">
        <input className="rounded border px-2 py-1" value={name} onChange={(e) => setName(e.target.value)} />
        <select
          className="rounded border px-2 py-1 text-sm"
          value={builder.kind}
          onChange={(e) => builder.setKind(e.target.value as AutomationIr['kind'])}
        >
          <option value="alert">Alert</option>
          <option value="recipe_override">Recipe Override</option>
        </select>
        <div className="flex overflow-hidden rounded border text-xs">
          <button
            className={`px-2 py-1 ${builder.mode === 'reactflow' ? 'bg-slate-800 text-white' : 'bg-white'}`}
            onClick={() => builder.setMode('reactflow')}
          >
            React Flow
          </button>
          <button
            className={`px-2 py-1 ${builder.mode === 'blockly' ? 'bg-slate-800 text-white' : 'bg-white'}`}
            onClick={() => builder.setMode('blockly')}
          >
            Blockly
          </button>
        </div>
        <button
          className="rounded bg-emerald-600 px-3 py-1 text-white disabled:opacity-50"
          disabled={createScript.isPending}
          onClick={handleSaveAndDeploy}
        >
          Save &amp; Deploy
        </button>
      </div>
      {builder.mode === 'reactflow' ? (
        <>
          <NodePalette onAddNode={builder.addNode} />
          <div className="flex flex-1 rounded border">
            <div className="flex-1">
              <ReactFlow
                nodes={builder.nodes}
                edges={builder.edges}
                onNodesChange={builder.onNodesChange}
                onEdgesChange={builder.onEdgesChange}
                onConnect={builder.onConnect}
                onNodeClick={(_, node: Node) => builder.setSelectedNodeId(node.id)}
                nodeTypes={AUTOMATION_NODE_TYPES}
                fitView
              >
                <Background />
                <Controls />
              </ReactFlow>
            </div>
            {builder.selectedNode && (
              <NodeEditorPanel
                kind={builder.kind}
                node={builder.selectedNode}
                onChange={builder.updateNodeData}
                onClose={() => builder.setSelectedNodeId(null)}
              />
            )}
          </div>
        </>
      ) : (
        <div className="flex-1 rounded border p-2">
          <BlockLogicEditor kind={builder.kind} onChange={builder.setBlocklyResult} className="h-full w-full" />
        </div>
      )}
      {/* SCRIPT_LIST_SLOT: Task 3 renders <ScriptListPanel deviceId={deviceId} onLoad={builder.loadFromIr} /> here */}
    </div>
  );
}
