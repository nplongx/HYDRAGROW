import { ReactFlow, Background, Controls } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { AUTOMATION_FLOW_NODE_TYPES } from '../components/automation/reactflow/FlowSummaryNode';
import { FlowDetailDrawer } from '../components/automation/FlowDetailDrawer';
import { useFlowCanvas } from '../hooks/useFlowCanvas';
import { useAutomationScripts } from '../hooks/useAutomationScripts';
import { useDeviceStore } from '../store/useDeviceStore';

export default function Automation() {
  const deviceId = useDeviceStore((s) => s.settings?.device_id ?? '');
  const { data: scripts } = useAutomationScripts(deviceId);
  const flow = useFlowCanvas(scripts);

  return (
    <div className="relative flex h-full flex-col gap-2 p-4">
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold">Automation Flows</h1>
        <button className="rounded bg-emerald-600 px-3 py-1 text-sm text-white" onClick={flow.openNewFlow}>
          + Flow mới
        </button>
      </div>
      <div className="flex-1 rounded border">
        <ReactFlow
          nodes={flow.nodes}
          edges={flow.edges}
          nodeTypes={AUTOMATION_FLOW_NODE_TYPES}
          onNodeClick={(_, node) => flow.openFlow(node.id)}
          fitView
        >
          <Background />
          <Controls />
        </ReactFlow>
      </div>
      {flow.isDrawerOpen && (
        <FlowDetailDrawer
          key={flow.selectedFlowId ?? 'new'}
          deviceId={deviceId}
          script={flow.isCreatingNew ? 'new' : flow.selectedScript!}
          onClose={flow.closeFlow}
        />
      )}
    </div>
  );
}
