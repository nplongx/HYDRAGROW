import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AutomationMultiDeviceTemplatePanel } from "./AutomationMultiDeviceTemplatePanel";

vi.mock("../../hooks/useOwnedDevices", () => ({
  useOwnedDevices: () => ({
    devices: [
      { device_id: "dev1", label: "Device 1" },
      { device_id: "dev2", label: "Device 2" },
    ],
    loading: false,
    error: null,
    refresh: vi.fn(),
  }),
}));

const mutateMock = vi.fn();
vi.mock("../../hooks/useAutomationScripts", () => ({
  useApplyTemplate: () => ({
    mutate: mutateMock,
    isPending: false,
    isSuccess: false,
    isError: false,
  }),
  useAllConfigOverrides: () => ({
    data: [
      {
        configKey: "ec_target",
        deviceId: "dev2",
        originalValue: "1.2",
        currentValue: "2.0",
        flowName: "Flow B",
        status: "active",
      },
    ],
    isLoading: false,
  }),
}));

function renderPanel(irJson: any = { configOverwrite: { configKey: "ec_target", value: "1.8" } }) {
  const queryClient = new QueryClient();
  render(
    <QueryClientProvider client={queryClient}>
      <AutomationMultiDeviceTemplatePanel
        currentScript={{ id: "script1", device_id: "dev-root", name: "Test Script", ir_json: irJson } as any}
      />
    </QueryClientProvider>,
  );
}

describe("AutomationMultiDeviceTemplatePanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("detects real local overrides from useAllConfigOverrides (not device name)", () => {
    renderPanel();
    // dev2 has a real override for ec_target -> badge; dev1 does not
    expect(screen.getByText("Device 2")).toBeInTheDocument();
    expect(screen.getAllByText("Có override cục bộ")).toHaveLength(1);
    expect(screen.getAllByText("Giống gốc")).toHaveLength(1);
    // per-device line shows real current value
    expect(screen.queryByText(/Nhóm:/)).not.toBeInTheDocument();
  });

  it("shows dynamic preview for targetConfigKey/value, with fallback when no configOverwrite", () => {
    renderPanel();
    expect(screen.getByText("ec_target sẽ được ghi đè → 1.8")).toBeInTheDocument();
  });

  it("shows fallback text when the flow has no configOverwrite node", () => {
    renderPanel(null);
    expect(
      screen.getByText(/không chứa node Ghi đè Config/),
    ).toBeInTheDocument();
  });

  it("sends { configOverwrite: null } for preserved devices and {} otherwise", () => {
    renderPanel();
    const checkboxes = screen.getAllByRole("checkbox");
    fireEvent.click(checkboxes[0]);
    fireEvent.click(checkboxes[1]);
    fireEvent.click(screen.getByRole("button", { name: /Áp dụng cho 2 thiết bị đã chọn/i }));
    expect(mutateMock).toHaveBeenCalledWith([
      { device_id: "dev1", overrides: {} },
      { device_id: "dev2", overrides: { configOverwrite: null } },
    ]);
  });
});
