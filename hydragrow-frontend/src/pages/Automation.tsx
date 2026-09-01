import { ReactFlow, Background, Controls } from '@xyflow/react';
import { Workflow } from 'lucide-react';
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
    <div className="app-page">
      <div className="page-header">
        <div>
          <h1 className="page-header-title">Tự động hóa</h1>
          <p className="page-header-subtitle">Thiết kế các luồng điều khiển bơm, van và cảm biến cho trạm.</p>
        </div>
        <button className="ui-btn-primary" onClick={flow.openNewFlow}>
          <span aria-hidden="true">＋</span> Flow mới
        </button>
      </div>

      <section className="ui-card relative min-h-[28rem] overflow-hidden p-0">
        <div className="flex min-h-[28rem] flex-col lg:flex-row">
          <div className={`min-h-[18rem] flex-1 transition-all duration-200 ${flow.isDrawerOpen ? 'lg:mr-0' : ''}`}>
            {(scripts ?? []).length === 0 ? (
              <div className="ui-state m-4">
                <Workflow className="mx-auto text-emerald-500" size={40} />
                <h2 className="ui-state-title">Chưa có Flow nào</h2>
                <p className="ui-state-desc">Tạo Flow đầu tiên để tự động hóa vận hành trạm.</p>
                <button className="ui-btn-primary mt-4" onClick={flow.openNewFlow}>Tạo Flow mới</button>
              </div>
            ) : (
              <div className="hidden h-full min-h-[28rem] lg:block">
                <ReactFlow nodes={flow.nodes} edges={flow.edges} nodeTypes={AUTOMATION_FLOW_NODE_TYPES} onNodeClick={(_, node) => flow.openFlow(node.id)} fitView className="h-full w-full">
                  <Background />
                  <Controls />
                </ReactFlow>
              </div>
            )}
          </div>
          {flow.isDrawerOpen && (
            <div className="relative z-20 flex w-full flex-col border-t border-emerald-100 bg-white lg:w-[36rem] lg:border-l lg:border-t-0 lg:shadow-xl">
              <FlowDetailDrawer key={flow.selectedFlowId ?? 'new'} deviceId={deviceId} script={flow.isCreatingNew ? 'new' : flow.selectedScript!} onClose={flow.closeFlow} />
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
