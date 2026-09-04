import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { Automation } from "./Automation";
import { useDeviceStore } from "../store/useDeviceStore";

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

describe("Automation Page", () => {
  beforeEach(() => {
    useDeviceStore.setState({ deviceId: "dev1" });
  });

  it("renders saved flows", () => {
    // we mocked useMediaQuery in setupTests to return false, so we are in mobile view
    // showing flow cards instead of canvas
    render(<Automation />);

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

  it("shows prompt when no deviceId is selected in useDeviceStore", () => {
    useDeviceStore.setState({ deviceId: null });
    render(<Automation />);
    expect(
      screen.getByText(/Chưa chọn thiết bị — vào Cài đặt để chọn thiết bị đang hoạt động/i)
    ).toBeInTheDocument();
  });
});
