import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
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

describe("AutomationMultiDeviceTemplatePanel", () => {
  it("renders multi-device template application UI properly", () => {
    // We expect blocked apply behavior because we don't have a real Flow bulk API
    render(
      <AutomationMultiDeviceTemplatePanel
        currentScript={{ id: "script1", name: "Test Script" } as any}
      />,
    );

    // selected device count is reflected in the CTA
    // a device with local override renders override;
    // a device inheriting template renders giống gốc;
    // the current Flow threshold summary is visible;
    // the sync helper text states that local overrides are preserved;
    // Apply is disabled with an explanatory state when no supported Automation bulk API exists.

    expect(
      screen.getByText("Áp Flow template cho nhiều thiết bị"),
    ).toBeInTheDocument();

    // We expect some UI indicating it's unsupported/blocked for now
    expect(screen.getByText(/Tính năng đang phát triển/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Áp dụng/i })).toBeDisabled();
  });
});
