import { useState, useMemo } from "react";
import { Search, LayoutGrid, Network } from "lucide-react";
import {
  ReactFlow,
  Background,
  Controls,
  ReactFlowProvider,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { useAutomationScripts, useConfigOverrides, useRevertConfigOverride } from "../hooks/useAutomationScripts";
import { useQueryClient } from "@tanstack/react-query";
import { apiPut } from "../lib/apiClient";
import toast from "react-hot-toast";
import { FlowDetailDrawer } from "../components/automation/FlowDetailDrawer";
import { AutomationPageHeader } from "../components/automation/AutomationPageHeader";
import { AutomationMetricsBanner } from "../components/automation/AutomationMetricsBanner";
import { FlowOverviewCard } from "../components/automation/FlowOverviewCard";
import { ConfigExplorerWidget } from "../components/automation/ConfigExplorerWidget";
import { ConfigExplorerView } from "../components/automation/ConfigExplorerView";
import { AutomationMultiDeviceTemplatePanel } from "../components/automation/AutomationMultiDeviceTemplatePanel";
import { FlowSummaryNode } from "../components/automation/reactflow/FlowSummaryNode";
import { LoadingState } from "../components/ui/LoadingState";
import { FaultExplanation } from "../components/ui/FaultExplanation";
import { useFlowCanvas } from "../hooks/useFlowCanvas";
import { useMediaQuery } from "../hooks/useMediaQuery";
import { useDeviceStore } from "../store/useDeviceStore";
import type { UserScript } from "../types/automation";

const nodeTypes = {
  flowSummary: FlowSummaryNode,
};

export function Automation() {
  const deviceId = useDeviceStore((s) => s.deviceId) ?? "";
  const { data: scripts, isLoading, isError } = useAutomationScripts(deviceId, {
    enabled: !!deviceId,
  });
  const { data: configOverridesData } = useConfigOverrides(deviceId, {
    enabled: !!deviceId,
  });
  const revertMutation = useRevertConfigOverride(deviceId);
  const queryClient = useQueryClient();

  const isDesktop = useMediaQuery("(min-width: 1024px)");
  const canvas = useFlowCanvas(scripts ?? []);

  const [currentView, setCurrentView] = useState<"overview" | "config_explorer">("overview");
  const [viewMode, setViewMode] = useState<"grid" | "canvas">("grid");
  const [searchQuery, setSearchQuery] = useState("");
  const [filterKind, setFilterKind] = useState<"all" | "alert" | "recipe" | "action" | "config">("all");

  const activeScripts = scripts ?? [];

  const toggleScriptEnabled = async (script: UserScript, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await apiPut(`/devices/${deviceId}/scripts/${script.id}`, {
        name: script.name,
        kind: script.kind,
        source: script.source,
        enabled: !script.enabled,
        ir_json: script.ir_json,
      });
      queryClient.invalidateQueries({ queryKey: ["automation-scripts", deviceId] });
      toast.success(script.enabled ? "Đã tắt Flow" : "Đã bật Flow");
    } catch {
      toast.error("Không thể thay đổi trạng thái Flow");
    }
  };

  const counts = useMemo(() => {
    let alert = 0, recipe = 0, action = 0, config = 0;
    activeScripts.forEach((s) => {
      const isCfg = s.kind === "config_override" || s.ir_json?.kind === "config_override" || s.name.toLowerCase().includes("config") || s.name.toLowerCase().includes("ngưỡng ec");
      if (isCfg) config++;
      else if (s.kind === "alert") alert++;
      else if (s.kind === "recipe_override") recipe++;
      else if (s.kind === "action_command") action++;
    });
    return { all: activeScripts.length, alert, recipe, action, config };
  }, [activeScripts]);

  const filteredScripts = useMemo(() => {
    return activeScripts.filter((s) => {
      const isCfg = s.kind === "config_override" || s.ir_json?.kind === "config_override" || s.name.toLowerCase().includes("config") || s.name.toLowerCase().includes("ngưỡng ec");
      let matchKind = true;
      if (filterKind === "config") matchKind = isCfg;
      else if (filterKind === "alert") matchKind = !isCfg && s.kind === "alert";
      else if (filterKind === "recipe") matchKind = !isCfg && s.kind === "recipe_override";
      else if (filterKind === "action") matchKind = !isCfg && s.kind === "action_command";

      const matchSearch =
        searchQuery === "" ||
        s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.kind.toLowerCase().includes(searchQuery.toLowerCase());

      return matchKind && matchSearch;
    });
  }, [activeScripts, filterKind, searchQuery]);

  if (!deviceId) {
    return (
      <div className="absolute inset-0 flex items-center justify-center text-emerald-800/70">
        Chưa chọn thiết bị — vào Cài đặt để chọn thiết bị đang hoạt động.
      </div>
    );
  }

  if (isLoading) return <LoadingState />;
  if (isError) return <FaultExplanation code="FETCH_ERROR" onClose={() => {}} />;

  if (currentView === "config_explorer") {
    return (
      <div className="app-page min-h-screen p-4 sm:p-6 lg:p-8">
        <ConfigExplorerView
          onBack={() => setCurrentView("overview")}
          activeOverrides={configOverridesData?.active ?? []}
          auditLogs={configOverridesData?.history ?? []}
          onRevert={(id) => revertMutation.mutate(id)}
        />
      </div>
    );
  }

  return (
    <div className="app-page min-h-screen p-4 sm:p-6 lg:p-8 flex flex-col space-y-6">
      {/* Header */}
      <AutomationPageHeader
        onNewFlow={() => canvas.openEditor("new")}
        onOpenConfigExplorer={() => setCurrentView("config_explorer")}
      />

      {/* 4 KPI Cards */}
      <AutomationMetricsBanner
        metrics={{
          activeFlows: activeScripts.filter((s) => s.enabled).length,
          alerts24h: activeScripts.filter((s) => s.kind === "alert" && s.enabled).length,
          configOverridesToday: (configOverridesData?.active?.length ?? 0) + activeScripts.filter((s) => s.kind === "config_override" && s.enabled).length,
          successRatePercent: 100,
        }}
      />

      {/* Search & Filter Bar */}
      <div className="bg-white p-3 rounded-2xl border border-emerald-100 flex flex-col md:flex-row items-center justify-between gap-3 shadow-sm">
        <div className="relative flex-1 w-full">
          <Search className="w-4 h-4 text-emerald-700/50 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            placeholder="Tìm Flow theo tên, cảm biến, config..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="ui-input pl-9 w-full text-xs"
          />
        </div>

        <div className="flex flex-wrap items-center gap-1.5 w-full md:w-auto">
          {[
            { id: "all", label: `Tất cả ${counts.all}` },
            { id: "alert", label: `Cảnh báo ${counts.alert}` },
            { id: "recipe", label: `Recipe ${counts.recipe}` },
            { id: "action", label: `Điều khiển ${counts.action}` },
            { id: "config", label: `Config Override ${counts.config}` },
          ].map((chip) => (
            <button
              key={chip.id}
              type="button"
              onClick={() => setFilterKind(chip.id as any)}
              className={`px-3 py-1.5 rounded-xl text-xs font-semibold transition-colors cursor-pointer ${
                filterKind === chip.id
                  ? "bg-emerald-800 text-white shadow-2xs"
                  : "bg-emerald-50 text-emerald-800 hover:bg-emerald-100/70"
              }`}
            >
              {chip.label}
            </button>
          ))}

          {/* View switcher: Grid vs Canvas */}
          <div className="hidden sm:flex items-center ml-2 pl-2 border-l border-emerald-100 gap-1">
            <button
              type="button"
              onClick={() => setViewMode("grid")}
              className={`p-1.5 rounded-lg text-xs font-medium cursor-pointer ${
                viewMode === "grid"
                  ? "bg-emerald-100 text-emerald-900"
                  : "text-emerald-700 hover:bg-emerald-50"
              }`}
              title="Chế độ lưới danh sách"
            >
              <LayoutGrid className="w-4 h-4" />
            </button>
            <button
              type="button"
              onClick={() => setViewMode("canvas")}
              className={`p-1.5 rounded-lg text-xs font-medium cursor-pointer ${
                viewMode === "canvas"
                  ? "bg-emerald-100 text-emerald-900"
                  : "text-emerald-700 hover:bg-emerald-50"
              }`}
              title="Chế độ sơ đồ React Flow"
            >
              <Network className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        {/* Left Side: Flows Display */}
        <div className="lg:col-span-8 space-y-6">
          {activeScripts.length === 0 ? (
            <div className="bg-white rounded-3xl border border-emerald-100 p-8 sm:p-12 text-center shadow-sm space-y-4">
              <div className="w-16 h-16 rounded-3xl bg-emerald-50 text-emerald-600 flex items-center justify-center mx-auto">
                <Network className="w-8 h-8" />
              </div>
              <div className="max-w-md mx-auto">
                <h3 className="text-lg font-bold text-emerald-950">Chưa có Flow tự động hóa nào</h3>
                <p className="text-xs text-emerald-800/70 mt-1">Chưa có kịch bản nào được kích hoạt trên thiết bị này. Hãy tạo Flow đầu tiên.</p>
              </div>
              <button
                type="button"
                onClick={() => canvas.openEditor("new")}
                className="ui-btn-primary px-6 py-2.5 text-xs font-semibold inline-flex items-center gap-2 cursor-pointer shadow-md"
              >
                <span>+ Tạo Flow tự động hóa đầu tiên</span>
              </button>
            </div>
          ) : viewMode === "grid" ? (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              {filteredScripts.map((script) => (
                <FlowOverviewCard
                  key={script.id}
                  script={script}
                  onClick={() => canvas.openEditor(script)}
                  onToggleEnabled={(e) => toggleScriptEnabled(script, e)}
                />
              ))}
              {filteredScripts.length === 0 && (
                <div className="col-span-2 py-12 text-center text-xs text-emerald-800/60 bg-white rounded-2xl border border-emerald-100">
                  Không tìm thấy Flow nào phù hợp bộ lọc.
                </div>
              )}
            </div>
          ) : (
            <div className="ui-card h-[540px] rounded-3xl overflow-hidden relative border border-emerald-100">
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
          )}

          {/* Multi-Device Rollout Panel */}
          {isDesktop && activeScripts.length > 0 && (
            <AutomationMultiDeviceTemplatePanel currentScript={activeScripts[0]} />
          )}
        </div>

        {/* Right Side: Config Explorer Widget */}
        <div className="lg:col-span-4 sticky top-6">
          <ConfigExplorerWidget
            items={configOverridesData?.active ?? []}
            onOpenFullView={() => setCurrentView("config_explorer")}
          />
        </div>
      </div>


      {/* Flow Editor Centered Modal */}
      {canvas.selectedScript && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-5 lg:p-8">
          <div
            data-testid="drawer-backdrop"
            onClick={canvas.closeEditor}
            className="fixed inset-0 bg-slate-950/40 backdrop-blur-sm transition-opacity"
          />
          <div className="relative z-50 h-[92vh] max-h-[960px] w-full max-w-7xl rounded-3xl bg-white shadow-2xl border border-emerald-100/80 overflow-hidden flex flex-col animate-in fade-in zoom-in-95 duration-200">
            <FlowDetailDrawer
              deviceId={deviceId}
              script={canvas.selectedScript}
              onClose={canvas.closeEditor}
            />
          </div>
        </div>
      )}
    </div>
  );
}
