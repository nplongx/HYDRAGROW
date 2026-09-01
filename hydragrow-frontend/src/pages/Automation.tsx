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
    <div className="relative flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b px-4 py-2">
        <h1 className="text-lg font-semibold">Automation Flows</h1>
        <button
          className="flex items-center gap-1 rounded bg-emerald-600 px-3 py-1.5 text-sm text-white transition-colors hover:bg-emerald-700"
          onClick={flow.openNewFlow}
        >
          <span aria-hidden="true">＋</span> Flow mới
        </button>
      </div>

      {/* Split-panel body */}
      <div className="flex flex-1 overflow-hidden">
        {/* Canvas — thu hẹp khi Drawer mở */}
        <div className={`flex-1 transition-all duration-200 ${flow.isDrawerOpen ? 'mr-[36rem]' : ''}`}>
          {(scripts ?? []).length === 0 ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-gray-400">
              <svg className="h-12 w-12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M9 3H5a2 2 0 00-2 2v4m6-6h10a2 2 0 012 2v4M9 3v18m0 0h10a2 2 0 002-2V9M9 21H5a2 2 0 01-2-2V9m0 0h18"
                />
              </svg>
              <p className="text-sm">Chưa có Flow nào. Tạo Flow đầu tiên!</p>
              <button
                className="rounded bg-emerald-600 px-4 py-2 text-sm text-white transition-colors hover:bg-emerald-700"
                onClick={flow.openNewFlow}
              >
                + Tạo Flow mới
              </button>
            </div>
          ) : (
            <ReactFlow
              nodes={flow.nodes}
              edges={flow.edges}
              nodeTypes={AUTOMATION_FLOW_NODE_TYPES}
              onNodeClick={(_, node) => flow.openFlow(node.id)}
              fitView
              className="h-full w-full"
            >
              <Background />
              <Controls />
            </ReactFlow>
          )}
        </div>

        {/* Drawer — absolute right sidebar, không che canvas */}
        {flow.isDrawerOpen && (
          <div className="absolute inset-y-0 right-0 z-20 flex w-[36rem] flex-col border-l bg-white shadow-xl">
            <FlowDetailDrawer
              key={flow.selectedFlowId ?? 'new'}
              deviceId={deviceId}
              script={flow.isCreatingNew ? 'new' : flow.selectedScript!}
              onClose={flow.closeFlow}
            />
          </div>
        )}
      </div>
    </div>
  );
}
