import { useAutomationScripts } from "../hooks/useAutomationScripts";
import { FlowDetailDrawer } from "../components/automation/FlowDetailDrawer";
import { AutomationPageHeader } from "../components/automation/AutomationPageHeader";
import { LoadingState } from "../components/ui/LoadingState";
import { FaultExplanation } from "../components/ui/FaultExplanation";
import {
  ReactFlow,
  Background,
  Controls,
  ReactFlowProvider,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useFlowCanvas } from "../hooks/useFlowCanvas";
import { FlowSummaryNode } from "../components/automation/reactflow/FlowSummaryNode";
import { useMediaQuery } from '../hooks/useMediaQuery';
import { AutomationMultiDeviceTemplatePanel } from '../components/automation/AutomationMultiDeviceTemplatePanel';
import { useDeviceStore } from "../store/useDeviceStore";

const nodeTypes = {
  flowSummary: FlowSummaryNode,
};

export function Automation() {
  const deviceId = useDeviceStore((s) => s.deviceId) ?? "";
  const { data: scripts, isLoading, isError } = useAutomationScripts(deviceId, {
    enabled: !!deviceId,
  });
  const isDesktop = useMediaQuery("(min-width: 1024px)");
  const canvas = useFlowCanvas(scripts ?? []);

  if (!deviceId) {
    return (
      <div className="absolute inset-0 flex items-center justify-center text-gray-500">
        Chưa chọn thiết bị — vào Cài đặt để chọn thiết bị đang hoạt động.
      </div>
    );
  }

  if (isLoading) return <LoadingState />;
  if (isError)
    return <FaultExplanation code="FETCH_ERROR" onClose={() => {}} />;

  const isEmpty = !scripts || scripts.length === 0;

  return (
    <div className="app-page h-[calc(100vh-4rem)] flex flex-col">
      <AutomationPageHeader onNewFlow={() => canvas.openEditor("new")} />

      <div className="ui-card flex-1 flex overflow-hidden relative">
        {isEmpty ? (
          <div className="absolute inset-0 flex items-center justify-center text-gray-500">
            Chưa có Flow nào
          </div>
        ) : isDesktop ? (
          <div className="w-full h-full min-h-[500px]">
            <ReactFlowProvider>
              <ReactFlow
                nodes={canvas.nodes}
                edges={canvas.edges}
                nodeTypes={nodeTypes}
                onNodeClick={(_, node) => {
                  if (node.type === "flowSummary" && node.data.script) {
                    canvas.openEditor(node.data.script as any);
                  }
                }}
                fitView
                fitViewOptions={{ padding: 0.2 }}
              >
                <Background />
                <Controls />
              </ReactFlow>
            </ReactFlowProvider>
          </div>
        ) : (
          <div className="w-full h-full overflow-y-auto p-4 flex flex-col gap-4 bg-gray-50">
            <ReactFlowProvider>
              {scripts.map((script) => (
                <div key={script.id} className="relative">
                  <FlowSummaryNode
                    data={{
                      script,
                      onClick: () => canvas.openEditor(script),
                    }}
                  />
                </div>
              ))}
            </ReactFlowProvider>
          </div>
        )}

        {/* Desktop Drawer */}
        {isDesktop && canvas.selectedScript && (
          <div className="fixed inset-0 z-40 flex justify-end">
            <div
              data-testid="drawer-backdrop"
              onClick={canvas.closeEditor}
              className="fixed inset-0 bg-black/20 transition-opacity"
            />
            <div className="relative z-50 h-full w-full max-w-xl border-l bg-white shadow-2xl">
              <FlowDetailDrawer
                deviceId={deviceId}
                script={canvas.selectedScript}
                onClose={canvas.closeEditor}
              />
            </div>
          </div>
        )}
      </div>

      {/* Multi-device Template Application */}
      {isDesktop &&
        !canvas.selectedScript &&
        !isEmpty &&
        scripts &&
        scripts.length > 0 && (
          <div className="mt-6">
            <AutomationMultiDeviceTemplatePanel currentScript={scripts[0]} />
          </div>
        )}

      {/* Mobile Modal/Drawer Full Screen */}
      {!isDesktop && canvas.selectedScript && (
        <div className="fixed inset-0 z-50 bg-white flex flex-col overflow-y-auto">
          <FlowDetailDrawer
            deviceId={deviceId}
            script={canvas.selectedScript}
            onClose={canvas.closeEditor}
          />
        </div>
      )}
    </div>
  );
}
