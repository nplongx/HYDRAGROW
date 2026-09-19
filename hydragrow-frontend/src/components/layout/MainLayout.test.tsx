import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, it, expect, vi } from "vitest";
import MainLayout from "./MainLayout";

vi.mock("../../hooks/useDeviceSync", () => ({ useDeviceSync: () => {} }));
vi.mock("../../hooks/useDeviceTelemetry", () => ({
  useDeviceTelemetry: () => ({ data: { availability: "ONLINE" } }),
}));
vi.mock("../../hooks/useSystemEvents", () => ({
  useSystemEvents: () => ({ data: [] }),
}));

vi.mock("../../contexts/StationContext", () => ({
  useStationContext: () => ({
    status: "Selected",
    selectedDeviceId: "test-device-123",
    selectedDevice: null,
    availableDevices: [],
    error: null,
    selectDevice: vi.fn(),
    switchDevice: vi.fn(),
    clearSelection: vi.fn(),
    refreshAvailableDevices: vi.fn(),
  }),
}));

describe("MainLayout sidebar", () => {
  it("không khóa toàn bộ ứng dụng khi cấu hình thiết bị chưa tồn tại", () => {
    render(
      <MemoryRouter initialEntries={["/dashboard"]}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/dashboard" element={<div>dashboard-content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByText("dashboard-content")).toBeInTheDocument();
    expect(screen.queryByText("Chưa kết nối máy chủ")).not.toBeInTheDocument();
  });
  it("đánh dấu mục Tổng quan là active khi ở /dashboard", () => {
    render(
      <MemoryRouter initialEntries={["/dashboard"]}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/dashboard" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );
    const activeItem = screen.getAllByRole("button", { name: /Tổng quan/i })[0];
    expect(activeItem.className).toContain("bg-pill");
    expect(activeItem.className).toContain("text-primary-deep");
  });

  it("hiển thị brand HydraGrow và thông tin thiết bị trong sidebar", () => {
    render(
      <MemoryRouter initialEntries={["/dashboard"]}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/dashboard" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );
    expect(screen.getAllByText("HydraGrow").length).toBeGreaterThan(0);
    expect(screen.getByText("Trạm Online")).toBeInTheDocument();
    expect(screen.getByText("ID: test-device-123")).toBeInTheDocument();
  });

  it("header mobile hiển thị trạng thái kết nối với semantic surface", () => {
    render(
      <MemoryRouter initialEntries={["/dashboard"]}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/dashboard" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );
    const pill = screen.getByText("Đang kết nối");
    expect(pill.className).toContain("bg-pill");
    expect(pill.className).toContain("text-status");
    expect(pill.className).toContain("rounded-full");
  });

  it("bottom nav mobile là dải điều hướng nổi với icon + nhãn", () => {
    render(
      <MemoryRouter initialEntries={["/dashboard"]}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/dashboard" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );
    const pill = document.querySelector('nav [class*="bg-white"]');
    expect(pill).toBeDefined();
    expect(pill!.className).toContain("bg-white/95");
    expect(pill!.className).toContain("rounded-2xl");
    ["Tổng quan", "Vận hành", "Canh tác", "Nhật ký", "Cài đặt"].forEach(
      (label) => {
        expect(screen.getAllByText(label).length).toBeGreaterThanOrEqual(1);
      },
    );
  });

  it("đánh dấu route con của trang chính là active theo router", () => {
    render(
      <MemoryRouter initialEntries={["/settings/integration"]}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/settings/*" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    expect(
      screen.getAllByRole("button", { name: /Cài đặt/i })[0].className,
    ).toContain("bg-pill");
  });

  it("desktop và mobile dùng cùng route target cho mục chính", () => {
    render(
      <MemoryRouter initialEntries={["/dashboard"]}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="*" element={<div>content</div>} />
          </Route>
          <Route path="/operations" element={<div>operations</div>} />
        </Routes>
      </MemoryRouter>,
    );

    const operations = screen.getAllByRole("button", { name: /Vận hành/i });
    fireEvent.click(operations[0]);
    expect(screen.getByText("operations")).toBeInTheDocument();
  });

  it("giữ shell landmarks và mobile safe-area contract", () => {
    render(
      <MemoryRouter initialEntries={["/dashboard"]}>
        <Routes>
          <Route element={<MainLayout />}>
            <Route path="/dashboard" element={<div>content</div>} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByRole("main")).toBeInTheDocument();
    expect(screen.getByRole("complementary")).toHaveClass("w-64");
    expect(screen.getByRole("main")).toHaveClass("lg:ml-64");
    expect(screen.getByRole("banner")).toHaveClass("lg:hidden");
    expect(
      screen.getByRole("navigation", { name: "Điều hướng chính" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("navigation", { name: "Điều hướng chính mobile" }),
    ).toHaveClass("lg:hidden");
    expect(
      screen.getByRole("navigation", { name: "Điều hướng chính mobile" })
        .className,
    ).toContain("safe-area-inset-bottom");
  });
});
