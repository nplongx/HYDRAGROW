import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import SystemLog from "./SystemLog";
import { apiGet } from "../lib/apiClient";
import type { SystemEvent } from "../components/logs/EventLogCard";

vi.mock("../lib/apiClient", () => ({
  apiGet: vi.fn((path: string) => {
    if (path.includes("health-summary")) {
      return Promise.resolve({
        status: "success",
        data: {
          window_seconds: 3600,
          ec_dosing_count: 1,
          ph_dosing_count: 0,
          water_operation_count: 0,
          warning_count: 0,
          critical_count: 0,
          latest_ph_dosing_at: null,
        },
      });
    }
    return Promise.resolve({ status: "success", data: [] });
  }),
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

function makeEvents(
  count: number,
  prefix: string,
  startTimestamp: number,
): SystemEvent[] {
  return Array.from({ length: count }, (_, i) => ({
    id: startTimestamp - i,
    device_id: "device-1",
    level: "info",
    category: "dosing",
    title: `${prefix} ${i}`,
    message: `msg ${i}`,
    timestamp: startTimestamp - i * 1000,
  }));
}

function withQueryClient(children: React.ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

beforeEach(() => {
  vi.mocked(apiGet).mockImplementation(async (path: string) => {
    if (path.includes("health-summary")) {
      return {
        status: "success",
        data: {
          window_seconds: 3600,
          ec_dosing_count: 1,
          ph_dosing_count: 0,
          water_operation_count: 0,
          warning_count: 0,
          critical_count: 0,
          latest_ph_dosing_at: null,
        },
      } as any;
    }
    return { status: "success", data: [] } as any;
  });
});

describe("SystemLog page", () => {
  it("mặc định ở chế độ Quan trọng và hiện thanh tóm tắt sức khoẻ", async () => {
    render(withQueryClient(<SystemLog />));
    await waitFor(() =>
      expect(screen.getByText(/1 lần châm EC/)).toBeInTheDocument(),
    );
    expect(screen.getByText("Quan trọng")).toBeInTheDocument();
  });

  it("bấm toggle chuyển sang Toàn bộ kỹ thuật", async () => {
    render(withQueryClient(<SystemLog />));
    await waitFor(() => expect(screen.getByRole("switch")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("switch"));
    expect(screen.getByText("Toàn bộ kỹ thuật")).toBeInTheDocument();
  });

  it("gõ vào ô tìm kiếm cập nhật giá trị input", async () => {
    render(withQueryClient(<SystemLog />));
    await waitFor(() =>
      expect(screen.getByLabelText("Tìm kiếm nhật ký")).toBeInTheDocument(),
    );
    fireEvent.change(screen.getByLabelText("Tìm kiếm nhật ký"), {
      target: { value: "châm ec" },
    });
    expect(screen.getByLabelText("Tìm kiếm nhật ký")).toHaveValue("châm ec");
  });

  it("có link mở Grafana", async () => {
    render(withQueryClient(<SystemLog />));
    await waitFor(() =>
      expect(screen.getByText("Mở Grafana")).toBeInTheDocument(),
    );
    expect(screen.getByText("Mở Grafana").closest("a")).toHaveAttribute(
      "href",
      "http://localhost:3000",
    );
  });
  it("gửi bộ lọc cấp độ và unresolved vào React Query request", async () => {
    render(withQueryClient(<SystemLog />));
    await waitFor(() => expect(screen.getByLabelText("Lọc cấp độ nhật ký")).toBeInTheDocument());

    fireEvent.change(screen.getByLabelText("Lọc cấp độ nhật ký"), { target: { value: "warning" } });
    fireEvent.click(screen.getAllByRole("button", { name: "Chưa xử lý" })[0]);

    await waitFor(() => {
      const urls = vi.mocked(apiGet).mock.calls.map((call) => String(call[0]));
      expect(urls.some((url) => url.includes("level=warning") && url.includes("unresolved=true"))).toBe(true);
    });
  });

});

describe("SystemLog pagination", () => {
  const page1 = makeEvents(200, "Dosing event", 2_000_000_000_000);
  const page2 = makeEvents(50, "Older dosing event", 1_000_000_000_000);

  beforeEach(() => {
    vi.mocked(apiGet).mockImplementation(async (path: string) => {
      if (path.includes("health-summary")) {
        return {
          status: "success",
          data: {
            window_seconds: 3600,
            ec_dosing_count: 1,
            ph_dosing_count: 0,
            water_operation_count: 0,
            warning_count: 0,
            critical_count: 0,
            latest_ph_dosing_at: null,
          },
        } as any;
      }
      if (path.includes("cursor=cursor-1")) {
        return { status: "success", data: page2, next_cursor: null } as any;
      }
      return { status: "success", data: page1, next_cursor: "cursor-1" } as any;
    });
  });

  it('tải thêm sự kiện cũ hơn khi bấm nút "Tải thêm sự kiện cũ hơn"', async () => {
    render(withQueryClient(<SystemLog />));

    await waitFor(() =>
      expect(screen.getByText("Dosing event 0")).toBeInTheDocument(),
    );
    expect(screen.queryByText("Older dosing event 0")).not.toBeInTheDocument();

    fireEvent.click(screen.getByText("Tải thêm sự kiện cũ hơn"));

    await waitFor(() =>
      expect(screen.getByText("Older dosing event 0")).toBeInTheDocument(),
    );

    const secondCallUrl = vi
      .mocked(apiGet)
      .mock.calls.map((call) => String(call[0]))
      .find((url) => url.includes("cursor=cursor-1"));
    expect(secondCallUrl).toContain("cursor=cursor-1");
  }, 15000);

  it('ẩn nút "Tải thêm" khi trang cuối trả về ít hơn PAGE_SIZE sự kiện', async () => {
    render(withQueryClient(<SystemLog />));

    await waitFor(() =>
      expect(screen.getByText("Dosing event 0")).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByText("Tải thêm sự kiện cũ hơn"));
    await waitFor(() =>
      expect(screen.getByText("Older dosing event 0")).toBeInTheDocument(),
    );
    expect(
      screen.queryByText("Tải thêm sự kiện cũ hơn"),
    ).not.toBeInTheDocument();
  });
});

describe("SystemLog date grouping", () => {
  const DAY = 86400000;
  const now = Date.now();
  const todayEvent = {
    id: 3,
    device_id: "device-1",
    level: "info",
    category: "dosing",
    title: "Sự kiện hôm nay",
    message: "msg",
    timestamp: now,
  };
  const yesterdayEvent = {
    id: 2,
    device_id: "device-1",
    level: "info",
    category: "dosing",
    title: "Sự kiện hôm qua",
    message: "msg",
    timestamp: now - DAY,
  };
  const oldEvent = {
    id: 1,
    device_id: "device-1",
    level: "warning",
    category: "dosing",
    title: "Sự kiện cũ",
    message: "msg",
    timestamp: now - 3 * DAY,
  };

  beforeEach(() => {
    vi.mocked(apiGet).mockImplementation(async (path: string) => {
      if (path.includes("health-summary")) {
        return {
          status: "success",
          data: {
            window_seconds: 3600,
            ec_dosing_count: 1,
            ph_dosing_count: 0,
            water_operation_count: 0,
            warning_count: 0,
            critical_count: 0,
            latest_ph_dosing_at: null,
          },
        } as any;
      }
      return {
        status: "success",
        data: [todayEvent, yesterdayEvent, oldEvent],
      } as any;
    });
  });

  it("nhóm sự kiện theo ngày với header HÔM NAY / HÔM QUA / ngày cụ thể", async () => {
    render(withQueryClient(<SystemLog />));

    await waitFor(() =>
      expect(screen.getByText("Sự kiện hôm nay")).toBeInTheDocument(),
    );
    expect(screen.getByText("HÔM NAY")).toBeInTheDocument();
    expect(screen.getByText("HÔM QUA")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByText("Sự kiện cũ")).toBeInTheDocument(),
    );
  });
});
