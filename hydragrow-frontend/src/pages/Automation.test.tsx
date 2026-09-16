import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { Automation } from "./Automation";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useStationContext } from "../contexts/StationContext";

const queryClient = new QueryClient();

// Mock dependencies to focus just on layout
vi.mock("../hooks/useAutomationScripts", () => ({
  useAutomationScripts: () => ({
    data: [
      {
        id: "1",
        name: "Saved Alert Node",
        kind: "alert",
        enabled: true,
        device_id: "dev1",
        source: "",
        ir_json: { kind: "alert" },
      },
      {
        id: "2",
        name: "Disabled Flow",
        kind: "action_command",
        enabled: false,
        device_id: "dev1",
        source: "",
        ir_json: { kind: "action_command" },
      },
      {
        id: "3",
        name: "Cron Flow",
        kind: "action_command",
        enabled: true,
        device_id: "dev1",
        source: "",
        ir_json: {
          kind: "action_command",
          nodes: [{ id: "trigger", data: { kind: "cron" } }],
        },
      },
      {
        id: "4",
        name: "Webhook Flow",
        kind: "action_command",
        enabled: true,
        device_id: "dev1",
        source: "",
        ir_json: {
          kind: "action_command",
          nodes: [{ id: "trigger", data: { kind: "webhook" } }],
        },
      },
    ],
    isLoading: false,
    isError: false,
  }),
  useConfigOverrides: () => ({
    data: { active: [], history: [] },
    isLoading: false,
  }),
  useRevertConfigOverride: () => ({ mutate: vi.fn(), isPending: false }),
  useExecutionSuccessRate: () => ({ data: null }),
  useUpdateAutomationScriptById: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

vi.mock("../hooks/useFlowCanvas", () => ({
  useFlowCanvas: () => ({
    nodes: [],
    edges: [],
    onNodesChange: vi.fn(),
    onEdgesChange: vi.fn(),
    selectedScript: null,
    openEditor: vi.fn(),
    closeEditor: vi.fn(),
    getTriggerIconAndColor: vi.fn(),
  }),
}));

vi.mock("../contexts/StationContext", () => ({
  useStationContext: vi.fn(() => ({
    status: "Selected",
    selectedDeviceId: "dev1",
    selectedDevice: null,
    availableDevices: [],
    error: null,
    selectDevice: vi.fn(),
    switchDevice: vi.fn(),
    clearSelection: vi.fn(),
    refreshAvailableDevices: vi.fn(),
  })),
}));

describe("Automation Page", () => {
  beforeEach(() => {
    vi.mocked(useStationContext).mockReturnValue({
      status: "Selected",
      selectedDeviceId: "dev1",
      selectedDevice: null,
      availableDevices: [],
      error: null,
      selectDevice: vi.fn(),
      switchDevice: vi.fn(),
      clearSelection: vi.fn(),
      refreshAvailableDevices: vi.fn(),
    });
  });

  it("renders saved flows", () => {
    // we mocked useMediaQuery in setupTests to return false, so we are in mobile view
    // showing flow cards instead of canvas
    render(
      <QueryClientProvider client={queryClient}>
        <Automation />
      </QueryClientProvider>,
    );

    // a saved alert node shows its kind badge
    expect(screen.queryByText("Cửa sổ (mean)")).not.toBeInTheDocument(); // Make sure nothing weird renders
    expect(screen.getByText("Saved Alert Node")).toBeInTheDocument();

    // a disabled Flow renders muted styling (via opacity or overlay text)
    expect(screen.getByText("Disabled Flow")).toBeInTheDocument();
    expect(screen.getByText("Đã tắt")).toBeInTheDocument();

    // trigger badge prefers CRON or WEBHOOK when configured
    expect(screen.getByText("CRON")).toBeInTheDocument();
    expect(screen.getByText("WEBHOOK")).toBeInTheDocument();
  });

  it("shows prompt when no deviceId is selected in StationContext", () => {
    vi.mocked(useStationContext).mockReturnValue({
      status: "NoSelection",
      selectedDeviceId: null,
      selectedDevice: null,
      availableDevices: [],
      error: null,
      selectDevice: vi.fn(),
      switchDevice: vi.fn(),
      clearSelection: vi.fn(),
      refreshAvailableDevices: vi.fn(),
    });
    render(
      <QueryClientProvider client={queryClient}>
        <Automation />
      </QueryClientProvider>,
    );
    expect(
      screen.getByText(
        /Chưa chọn thiết bị — hãy chọn một trạm từ Tổng Quan Thiết Bị/i,
      ),
    ).toBeInTheDocument();
  });
});
