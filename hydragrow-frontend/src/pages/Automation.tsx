import { useCallback, useState } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  type Edge,
  type Node,
  useEdgesState,
  useNodesState,
  addEdge,
  type Connection,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import toast from 'react-hot-toast';
import { AUTOMATION_NODE_TYPES } from '../components/automation/reactflow/nodeTypes';
import { buildIrFromGraph } from '../components/automation/reactflow/buildIr';
import { compileToRhai } from '../lib/automation/compileToRhai';
import { AutomationIrSchema } from '../lib/automation/ir';
import { useCreateAutomationScript, useValidateAutomationScript } from '../hooks/useAutomationScripts';
import { useDeviceStore } from '../store/useDeviceStore';

const INITIAL_NODES: Node[] = [
  { id: '1', type: 'sensor', position: { x: 250, y: 0 }, data: {} },
  { id: '2', type: 'condition', position: { x: 250, y: 120 }, data: { conditions: [], summary: 'Chưa cấu hình' } },
  { id: '3', type: 'action', position: { x: 250, y: 240 }, data: { actions: [], summary: 'Chưa cấu hình' } },
];
const INITIAL_EDGES: Edge[] = [
  { id: 'e1-2', source: '1', target: '2' },
  { id: 'e2-3', source: '2', target: '3' },
];

export default function Automation() {
  const deviceId = useDeviceStore((s) => s.settings?.device_id ?? '');
  const [nodes, , onNodesChange] = useNodesState(INITIAL_NODES);
  const [edges, setEdges, onEdgesChange] = useEdgesState(INITIAL_EDGES);
  const [name, setName] = useState('Automation mới');
  const validateScript = useValidateAutomationScript(deviceId);
  const createScript = useCreateAutomationScript(deviceId);

  const onConnect = useCallback(
    (connection: Connection) => setEdges((eds) => addEdge(connection, eds)),
    [setEdges],
  );

  const handleSaveAndDeploy = async () => {
    const ir = buildIrFromGraph({ kind: 'alert', nodes, edges });
    const parsed = AutomationIrSchema.safeParse(ir);
    if (!parsed.success) {
      toast.error(`IR không hợp lệ: ${parsed.error.issues[0]?.message}`);
      return;
    }
    const source = compileToRhai(parsed.data);

    const validation = await validateScript.mutateAsync({
      kind: parsed.data.kind,
      name,
      source,
      ir_json: parsed.data,
    });
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
        <input
          className="rounded border px-2 py-1"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <button
          className="rounded bg-emerald-600 px-3 py-1 text-white disabled:opacity-50"
          disabled={createScript.isPending}
          onClick={handleSaveAndDeploy}
        >
          Save &amp; Deploy
        </button>
      </div>
      <div className="flex-1 rounded border">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          nodeTypes={AUTOMATION_NODE_TYPES}
          fitView
        >
          <Background />
          <Controls />
        </ReactFlow>
      </div>
    </div>
  );
}
