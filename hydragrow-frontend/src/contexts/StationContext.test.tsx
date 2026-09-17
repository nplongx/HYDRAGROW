import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StationProvider, useStationContext } from "./StationContext";

const { apiGetMock, syncStatusMock } = vi.hoisted(() => ({ apiGetMock: vi.fn(), syncStatusMock: vi.fn() }));

vi.mock("../lib/apiClient", () => ({ apiGet: apiGetMock }));
vi.mock("../api/config", () => ({ configApi: { syncStatus: syncStatusMock } }));

const devices = [
  {
    id: 1,
    user_id: 1,
    device_id: "device-a",
    label: "Station A",
    claimed_at: "2026-09-15T00:00:00Z",
  },
  {
    id: 2,
    user_id: 1,
    device_id: "device-b",
    label: "Station B",
    claimed_at: "2026-09-15T00:00:00Z",
  },
];

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <StationProvider>{children}</StationProvider>
  </QueryClientProvider>
);

describe("StationContext", () => {
  beforeEach(() => {
    apiGetMock.mockReset();
    syncStatusMock.mockReset();
    syncStatusMock.mockResolvedValue({});
    localStorage.clear();
  });

  it("loads with no selection when there is no persisted preference", async () => {
    apiGetMock.mockResolvedValue(devices);

    const { result } = renderHook(() => useStationContext(), { wrapper });

    await waitFor(() => expect(result.current.status).toBe("NoSelection"));
    expect(result.current.selectedDeviceId).toBeNull();
    expect(result.current.selectedDevice).toBeNull();
  });

  it("exposes durable configuration synchronization for the selected station", async () => {
    localStorage.setItem(
      "hydragrow_selected_device_id",
      JSON.stringify("device-b"),
    );
    apiGetMock.mockResolvedValue(devices);
    syncStatusMock.mockResolvedValue({
      device_id: "device-b", config_version: 9, overall_state: "published",
      controller: { state: "applied", attempts: 1, last_attempt_at: null, applied_at: "2026-09-16T01:00:00Z" },
      sensor: { state: "published", attempts: 1, last_attempt_at: "2026-09-16T01:00:00Z", applied_at: null },
      last_error: null, updated_at: "2026-09-16T01:00:00Z",
    });

    const { result } = renderHook(() => useStationContext(), { wrapper });

    await waitFor(() => expect(result.current.configurationSync?.version).toBe(9));
    expect(result.current.configurationSync?.status).toBe("published");
    expect(result.current.configurationSync?.controller).toBe("applied");
    expect(result.current.configurationSync?.sensor).toBe("published");
  });

  it("restores a valid persisted selection", async () => {
    localStorage.setItem(
      "hydragrow_selected_device_id",
      JSON.stringify("device-b"),
    );
    apiGetMock.mockResolvedValue(devices);

    const { result } = renderHook(() => useStationContext(), { wrapper });

    await waitFor(() => expect(result.current.status).toBe("Selected"));
    expect(result.current.selectedDeviceId).toBe("device-b");
    expect(result.current.selectedDevice?.device_id).toBe("device-b");
  });

  it("rejects an invalid selection without destroying a valid selection", async () => {
    apiGetMock.mockResolvedValue(devices);

    const { result } = renderHook(() => useStationContext(), { wrapper });
    await waitFor(() => expect(result.current.status).toBe("NoSelection"));

    act(() => result.current.selectDevice("device-a"));
    expect(result.current.selectedDeviceId).toBe("device-a");
    expect(result.current.status).toBe("Selected");

    act(() => result.current.selectDevice("unknown-device"));
    expect(result.current.selectedDeviceId).toBe("device-a");
    expect(result.current.status).toBe("Selected");
  });

  it("clears the selection and persisted preference", async () => {
    apiGetMock.mockResolvedValue(devices);

    const { result } = renderHook(() => useStationContext(), { wrapper });
    await waitFor(() => expect(result.current.status).toBe("NoSelection"));

    act(() => result.current.selectDevice("device-a"));
    act(() => result.current.clearSelection());

    expect(result.current.status).toBe("NoSelection");
    expect(result.current.selectedDeviceId).toBeNull();
    expect(localStorage.getItem("hydragrow_selected_device_id")).toBeNull();
  });

  it("does not replace a selected device with another device after a refresh", async () => {
    apiGetMock
      .mockResolvedValueOnce(devices)
      .mockResolvedValueOnce([devices[1]]);

    const { result } = renderHook(() => useStationContext(), { wrapper });
    await waitFor(() => expect(result.current.status).toBe("NoSelection"));

    act(() => result.current.selectDevice("device-a"));
    await act(async () => {
      await result.current.refreshAvailableDevices();
    });

    expect(result.current.selectedDeviceId).toBe("device-a");
    expect(result.current.status).toBe("InvalidSelection");
  });

  it("does not turn a refresh failure into an empty success state", async () => {
    apiGetMock.mockResolvedValue(devices);
    const { result } = renderHook(() => useStationContext(), { wrapper });
    await waitFor(() => expect(result.current.status).toBe("NoSelection"));

    act(() => result.current.selectDevice("device-a"));

    apiGetMock.mockRejectedValueOnce(new Error("network down"));
    await act(async () => {
      await result.current.refreshAvailableDevices();
    });

    expect(result.current.status).toBe("Selected");
    expect(result.current.selectedDeviceId).toBe("device-a");
    expect(result.current.availableDevices).toEqual(devices);
    expect(result.current.error?.message).toBe("network down");
  });

  it("maps initial 403 device-list denial to PermissionDenied without selecting a device", async () => {
    const error = Object.assign(new Error("Device access denied"), { status: 403 });
    apiGetMock.mockRejectedValue(error);

    const { result } = renderHook(() => useStationContext(), { wrapper });

    await waitFor(() => expect(result.current.status).toBe("PermissionDenied"));
    expect(result.current.selectedDeviceId).toBeNull();
    expect(result.current.selectedDevice).toBeNull();
    expect(result.current.availableDevices).toEqual([]);
  });

  it("does not expose a previous device after the selected device becomes unavailable", async () => {
    apiGetMock
      .mockResolvedValueOnce(devices)
      .mockResolvedValueOnce([devices[1]]);

    const { result } = renderHook(() => useStationContext(), { wrapper });
    await waitFor(() => expect(result.current.status).toBe("NoSelection"));

    act(() => result.current.selectDevice("device-a"));
    expect(result.current.selectedDevice?.device_id).toBe("device-a");

    await act(async () => {
      await result.current.refreshAvailableDevices();
    });

    expect(result.current.selectedDeviceId).toBe("device-a");
    expect(result.current.selectedDevice).toBeNull();
    expect(result.current.status).toBe("InvalidSelection");
    expect(result.current.availableDevices).toEqual([devices[1]]);
  });
});
