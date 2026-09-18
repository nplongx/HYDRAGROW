import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Operations } from "./Operations";

vi.mock("./ControlPanel", () => ({
  default: ({ variant }: { variant?: string }) => (
    <div data-testid="control-panel">ControlPanel Mock variant={variant}</div>
  ),
}));

vi.mock("./Automation", () => ({
  Automation: () => <div data-testid="automation-page">Automation Mock</div>,
}));

vi.mock("../contexts/StationContext", () => ({
  useStationContext: () => ({
    status: "Selected",
    selectedDeviceId: "device-1",
    selectedDevice: null,
    availableDevices: [],
    error: null,
    selectDevice: vi.fn(),
    switchDevice: vi.fn(),
    clearSelection: vi.fn(),
    refreshAvailableDevices: vi.fn(),
  }),
}));

describe("Operations Page", () => {
  function renderOperations() {
    return render(
      <QueryClientProvider client={new QueryClient()}>
        <MemoryRouter>
          <Operations />
        </MemoryRouter>
      </QueryClientProvider>,
    );
  }

  it("renders both Điều khiển and Tự động hóa tabs, defaulting to Điều khiển", () => {
    renderOperations();
    expect(screen.getByRole("tab", { name: /điều khiển/i })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tab", { name: /tự động hóa/i })).toHaveAttribute(
      "aria-selected",
      "false",
    );
    expect(screen.getByTestId("control-panel")).toBeInTheDocument();
    expect(screen.queryByTestId("automation-page")).not.toBeInTheDocument();
  });

  it("switches to Automation tab on click", async () => {
    renderOperations();
    fireEvent.click(screen.getByRole("tab", { name: /tự động hóa/i }));
    expect(screen.getByRole("tab", { name: /tự động hóa/i })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tab", { name: /điều khiển/i })).toHaveAttribute(
      "aria-selected",
      "false",
    );
    await waitFor(() => expect(screen.getByTestId("automation-page")).toBeInTheDocument());
    expect(screen.queryByTestId("control-panel")).not.toBeInTheDocument();
  });
});
