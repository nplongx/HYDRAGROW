import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AutomationMultiDeviceTemplatePanel } from "./AutomationMultiDeviceTemplatePanel";

// Mock dependencies
vi.mock("../../hooks/useOwnedDevices", () => ({
  useOwnedDevices: () => ({
    data: [
      { id: "dev1", name: "Device 1", online: true },
      { id: "dev2", name: "Device 2 (Local Override)", online: true },
    ],
    isLoading: false,
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
}));

const queryClient = new QueryClient();

describe("AutomationMultiDeviceTemplatePanel", () => {
  it("renders multi-device template application UI and allows applying to selected devices", () => {
    render(
      <QueryClientProvider client={queryClient}>
        <AutomationMultiDeviceTemplatePanel
          currentScript={{ id: "script1", device_id: "dev-root", name: "Test Script" } as any}
        />
      </QueryClientProvider>,
    );

    expect(
      screen.getByText("Áp Flow template cho nhiều thiết bị"),
    ).toBeInTheDocument();

    expect(screen.getByText("Device 1")).toBeInTheDocument();
    expect(screen.getByText("Device 2 (Local Override)")).toBeInTheDocument();

    const checkboxes = screen.getAllByRole("checkbox");
    expect(checkboxes).toHaveLength(2);

    // Initial state: 0 selected, button disabled
    const applyButton = screen.getByRole("button", { name: /Áp dụng cho 0 thiết bị đã chọn/i });
    expect(applyButton).toBeDisabled();

    // Check first device
    fireEvent.click(checkboxes[0]);
    expect(screen.getByRole("button", { name: /Áp dụng cho 1 thiết bị đã chọn/i })).not.toBeDisabled();

    // Click apply
    fireEvent.click(screen.getByRole("button", { name: /Áp dụng cho 1 thiết bị đã chọn/i }));
    expect(mutateMock).toHaveBeenCalledWith([
      { device_id: "dev1", overrides: {} },
    ]);
  });
});
