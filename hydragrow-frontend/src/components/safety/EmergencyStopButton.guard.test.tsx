// src/components/safety/EmergencyStopButton.guard.test.tsx
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import fs from "node:fs";
import path from "node:path";
import { EmergencyStopButton } from "./EmergencyStopButton";

const emergencyStop = vi.fn().mockResolvedValue(true);
let whoami = { scopes: ['control:emergency'], is_active: true };
let safetyState: 'CONFIRMED_OFF' | 'UNRESOLVED' = 'CONFIRMED_OFF';

vi.mock("../../hooks/useDeviceControl", () => ({
  useDeviceControl: () => ({
    emergencyStop,
    emergencyStopLifecycle: null,
    emergencyStopSafetyState: safetyState,
  }),
}));

vi.mock("../../hooks/useWhoami", () => ({
  useWhoami: () => ({
    data: whoami,
    isLoading: false,
    isError: false,
  }),
}));

vi.mock("../../contexts/StationContext", () => ({
  useStationContext: () => ({
    status: "Selected",
    selectedDeviceId: "dev-001",
    selectedDevice: null,
    availableDevices: [],
    error: null,
    selectDevice: vi.fn(),
    switchDevice: vi.fn(),
    clearSelection: vi.fn(),
    refreshAvailableDevices: vi.fn(),
  }),
}));

describe("EmergencyStopButton — Fitts's Law / Von Restorff regression guard", () => {
  beforeEach(() => {
    whoami = { scopes: ['control:emergency'], is_active: true };
    safetyState = 'CONFIRMED_OFF';
    emergencyStop.mockClear();
  });

  it("touch target is at least 48px (w-14/h-14 or larger)", () => {
    render(
      <QueryClientProvider client={new QueryClient()}>
        <EmergencyStopButton deviceId={null} variant="floating" />
      </QueryClientProvider>,
    );
    const button = screen.getByRole("button", {
      name: /dừng khẩn cấp|emergency/i,
    });
    const sizeClasses = ["w-14", "h-14", "w-16", "h-16"];
    expect(sizeClasses.some((c) => button.className.includes(c))).toBe(true);
  });

  it("no other bg-red-*/bg-danger/bg-error element exists on Dashboard.tsx or Operations.tsx source (Von Restorff uniqueness)", () => {
    const dashboardSrc = fs.readFileSync(
      path.resolve(__dirname, "../../pages/Dashboard.tsx"),
      "utf-8",
    );
    const operationsSrc = fs.readFileSync(
      path.resolve(__dirname, "../../pages/Operations.tsx"),
      "utf-8",
    );
    const dangerPattern = /bg-red-\d{2,3}|bg-danger|bg-error/;
    expect(dangerPattern.test(dashboardSrc)).toBe(false);
    expect(dangerPattern.test(operationsSrc)).toBe(false);
  });

  it("E-STOP executes immediately without a generic confirmation dialog", async () => {
    emergencyStop.mockClear();
    render(
      <QueryClientProvider client={new QueryClient()}>
        <EmergencyStopButton deviceId="dev-001" variant="floating" />
      </QueryClientProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: /dừng khẩn cấp/i }));
    await waitFor(() => expect(emergencyStop).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("does not dispatch when the button identity disagrees with StationContext", async () => {
    emergencyStop.mockClear();
    render(
      <QueryClientProvider client={new QueryClient()}>
        <EmergencyStopButton deviceId="dev-002" variant="floating" />
      </QueryClientProvider>,
    );

    const button = screen.getByRole("button", { name: /dừng khẩn cấp/i });
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(emergencyStop).not.toHaveBeenCalled();
  });

  it("gates dispatch by the control:emergency capability", () => {
    whoami = { scopes: ['control:pump'], is_active: true };
    render(
      <QueryClientProvider client={new QueryClient()}>
        <EmergencyStopButton deviceId="dev-001" variant="floating" />
      </QueryClientProvider>,
    );

    const button = screen.getByRole("button", { name: /dừng khẩn cấp/i });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("data-capability", "denied");
    fireEvent.click(button);
    expect(emergencyStop).not.toHaveBeenCalled();
  });

  it("exposes lifecycle and unresolved safety state separately from command dispatch", () => {
    safetyState = 'UNRESOLVED';
    render(
      <QueryClientProvider client={new QueryClient()}>
        <EmergencyStopButton deviceId="dev-001" variant="bar" />
      </QueryClientProvider>,
    );

    const button = screen.getByRole("button", { name: /dừng khẩn cấp/i });
    expect(button).toHaveAttribute("data-estop-lifecycle", "IDLE");
    expect(button).toHaveAttribute("data-safety-state", "UNRESOLVED");
    expect(button).toHaveAttribute("data-capability", "control:emergency");
    expect(button).toHaveAccessibleName(/chưa xác nhận trạng thái an toàn/i);
  });
});
