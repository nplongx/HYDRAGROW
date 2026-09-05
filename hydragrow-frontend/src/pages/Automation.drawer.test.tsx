import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { Automation } from "./Automation";
import { useDeviceStore } from "../store/useDeviceStore";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const queryClient = new QueryClient();

class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}

global.ResizeObserver = ResizeObserverMock as any;

const mockCloseEditor = vi.fn();
let mockSelectedScript: any = null;

vi.mock("../hooks/useAutomationScripts", () => ({
  useAutomationScripts: () => ({
    data: [{ id: "1", name: "Flow 1", kind: "alert", enabled: true }],
    isLoading: false,
    isError: false,
  }),
  useApplyTemplate: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useConfigOverrides: () => ({ data: { active: [], history: [] }, isLoading: false }),
  useRevertConfigOverride: () => ({ mutate: vi.fn(), isPending: false }),
}));

vi.mock("../hooks/useMediaQuery", () => ({
  useMediaQuery: () => true, // Force desktop mode
}));

vi.mock("../hooks/useFlowCanvas", () => ({
  useFlowCanvas: () => ({
    nodes: [],
    edges: [],
    onNodesChange: vi.fn(),
    onEdgesChange: vi.fn(),
    selectedScript: mockSelectedScript,
    openEditor: vi.fn(),
    closeEditor: mockCloseEditor,
  }),
}));

vi.mock("../components/automation/FlowDetailDrawer", () => ({
  FlowDetailDrawer: ({ onClose }: { onClose: () => void }) => (
    <div data-testid="flow-detail-drawer-stub">
      <button onClick={onClose}>Close Drawer</button>
    </div>
  ),
}));

describe("Automation Page Desktop Drawer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDeviceStore.setState({ deviceId: "dev1" });
  });

  it("renders backdrop and max-w-xl container when a script is selected in desktop mode", () => {
    mockSelectedScript = "new";
    render(
      <QueryClientProvider client={queryClient}>
        <Automation />
      </QueryClientProvider>
    );

    const backdrop = screen.getByTestId("drawer-backdrop");
    expect(backdrop).toBeInTheDocument();

    const drawerContainer = backdrop.nextElementSibling;
    expect(drawerContainer).toHaveClass("max-w-xl");

    fireEvent.click(backdrop);
    expect(mockCloseEditor).toHaveBeenCalledTimes(1);
  });

  it("does not render drawer or backdrop when no script is selected", () => {
    mockSelectedScript = null;
    render(
      <QueryClientProvider client={queryClient}>
        <Automation />
      </QueryClientProvider>
    );

    expect(screen.queryByTestId("drawer-backdrop")).not.toBeInTheDocument();
    expect(screen.queryByTestId("flow-detail-drawer-stub")).not.toBeInTheDocument();
  });
});
