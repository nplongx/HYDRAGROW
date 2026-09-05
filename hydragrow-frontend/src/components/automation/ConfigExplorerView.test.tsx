import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { ConfigExplorerView } from "./ConfigExplorerView";
import type { ConfigOverrideActiveItem, ConfigAuditLogEntry } from "../../types/automation";

const TEST_OVERRIDES: ConfigOverrideActiveItem[] = [
  { configKey: "ec_target", deviceId: "dev-a1", deviceName: "Nhà kính A · Kệ 1", originalValue: "2.4", currentValue: "1.8", unit: "mS/cm", flowName: "Hạ ngưỡng EC ban đêm", status: "active" },
  { configKey: "ph_target", deviceId: "dev-b1", deviceName: "Nhà kính B · Kệ 1", originalValue: "6.2", currentValue: "5.8", unit: "", flowName: "Bù pH giai đoạn ra hoa", status: "active" },
];

const TEST_LOGS: ConfigAuditLogEntry[] = [
  { id: "log-1", timestamp: "05/09 22:00", deviceId: "dev-a1", deviceName: "Nhà kính A · Kệ 1", configKey: "ec_target", originalValue: "2.4", overrideValue: "1.8", unit: "mS/cm", reason: 'Điều kiện "Khung giờ ban đêm" chuyển sang đúng', status: "applied" },
];

describe("ConfigExplorerView", () => {
  it("renders page title, KPIs and table headers with empty state when no data", () => {
    const onBack = vi.fn();
    render(<ConfigExplorerView onBack={onBack} activeOverrides={[]} auditLogs={[]} />);

    expect(
      screen.getByText("Config Explorer & Nhật ký ghi đè"),
    ).toBeInTheDocument();
    expect(screen.getByText("Config key đang bị ghi đè")).toBeInTheDocument();
    expect(screen.getByText("Thiết bị có override cục bộ")).toBeInTheDocument();
    expect(screen.getByText("Lượt ghi đè trong 24h")).toBeInTheDocument();
    expect(screen.getByText("Tự khôi phục đúng điều kiện")).toBeInTheDocument();

    // Tables
    expect(
      screen.getByText("Danh sách Config đang hoạt động & Flow kiểm soát"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Nhật ký ghi đè toàn hệ thống (audit log) — xuyên suốt mọi thiết bị",
      ),
    ).toBeInTheDocument();

    // Empty state messages
    expect(screen.getByText("Chưa có bản ghi đè config nào thỏa mãn bộ lọc.")).toBeInTheDocument();
    expect(screen.getByText("Chưa có nhật ký ghi đè nào được ghi nhận trong hệ thống.")).toBeInTheDocument();

    // Click back
    const backBtn = screen.getByRole("button", { name: /Quay lại Flow/i });
    fireEvent.click(backBtn);
    expect(onBack).toHaveBeenCalled();
  });

  it("filters active overrides by search input and filter pills", () => {
    render(
      <ConfigExplorerView
        onBack={vi.fn()}
        activeOverrides={TEST_OVERRIDES}
        auditLogs={TEST_LOGS}
      />
    );

    // Filter pill click
    const ecPill = screen.getByRole("button", { name: "ec_target (1)" });
    fireEvent.click(ecPill);

    expect(screen.getAllByText("ec_target").length).toBeGreaterThan(0);

    // Search filter
    const searchInput = screen.getByPlaceholderText(
      "Tìm theo config key, thiết bị hoặc tên Flow...",
    );
    fireEvent.change(searchInput, { target: { value: "Nhà kính A" } });
    expect(screen.getAllByText("Nhà kính A · Kệ 1").length).toBeGreaterThan(0);
  });
});

