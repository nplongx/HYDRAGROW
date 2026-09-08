import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TestPanel } from "./TestPanel";
import { useTestAutomationScript } from "../../../hooks/useAutomationScripts";

vi.mock("../../../hooks/useAutomationScripts", () => ({
  useTestAutomationScript: vi.fn(),
}));

const mockedUseTest = vi.mocked(useTestAutomationScript);

function mockDryRun() {
  mockedUseTest.mockReturnValue({
    data: {
      will_fire: true,
      trace: [],
      actions_preview: [],
    },
    isPending: false,
    mutate: vi.fn(),
  } as any);
}

beforeEach(() => {
  vi.clearAllMocks();
  mockDryRun();
});

function irWithConfigOverwrite(value: string) {
  return {
    kind: "config_override",
    trigger: { type: "manual" },
    conditions: [{ sensor: "ph", operator: ">", value: 7.5 }],
    actions: [{ type: "config_override", key: "ec_target", value }],
    nodes: [],
    edges: [],
    next_flow_ids: [],
    configOverwrite: {
      configKey: "ec_target",
      value,
      readOriginalBeforeWrite: false,
      restoreMode: "on_condition_false",
      priority: 0,
    },
  } as any;
}

function renderPanel(ir: any) {
  const qc = new QueryClient();
  return render(
    <QueryClientProvider client={qc}>
      <TestPanel deviceId="dev1" ir={ir} fields={["ph"]} />
    </QueryClientProvider>,
  );
}

const queryClient = new QueryClient();

describe("TestPanel", () => {
  it("renders inputs for sample values and runs dry-run", () => {
    const ir = {
      kind: "alert",
      actions: [],
      nodes: [{ id: "trigger", type: "trigger", data: { kind: "sensor" }, position: { x: 0, y: 0 } }],
      edges: [],
      next_flow_ids: [],
    } as any;

    render(
      <QueryClientProvider client={queryClient}>
        <TestPanel
          deviceId="dev1"
          ir={ir}
          fields={["ph", "ec", "temp", "water_level"]}
        />
      </QueryClientProvider>,
    );

    // Using query string or getByText since it's an element label next to input but might not be properly associated in DOM
    expect(screen.getByText("ph")).toBeInTheDocument();
    expect(screen.getByText("ec")).toBeInTheDocument();
    expect(screen.getByText("temp")).toBeInTheDocument();
    expect(screen.getByText("water_level")).toBeInTheDocument();

    // primary Chạy thử button
    const runBtn = screen.getByRole("button", { name: "Chạy thử" });
    expect(runBtn).toBeInTheDocument();
  });

  it("renders series input and note for field with mode=mean", () => {
    const irWithMean = {
      kind: "alert",
      actions: [],
      conditions: [
        { sensor: "ph", operator: ">", value: 7.5, mode: "mean", windowSec: 900 },
      ],
      nodes: [],
      edges: [],
      next_flow_ids: [],
    } as any;

    render(
      <QueryClientProvider client={queryClient}>
        <TestPanel
          deviceId="dev1"
          ir={irWithMean}
          fields={["ph", "ec"]}
        />
      </QueryClientProvider>,
    );

    expect(screen.getByText("(mean)")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("vd: 7.0, 7.5, 8.5")).toBeInTheDocument();
    expect(screen.getByText(/Nhập nhiều điểm, cách nhau bởi dấu phẩy/i)).toBeInTheDocument();
  });

  it("shows in-bounds message for config value '1.8'", () => {
    renderPanel(irWithConfigOverwrite("1.8"));
    expect(screen.getByText(/Trong giới hạn cho phép/i)).toBeInTheDocument();
  });

  it("does NOT show in-bounds message for out-of-bounds '99', shows clamp instead", () => {
    renderPanel(irWithConfigOverwrite("99"));
    expect(screen.queryByText(/Trong giới hạn cho phép/i)).not.toBeInTheDocument();
    expect(screen.getByText(/kẹp|clamp/i)).toBeInTheDocument();
  });
});
