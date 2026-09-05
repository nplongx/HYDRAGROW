import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { ConfigExplorerView } from "./ConfigExplorerView";

describe("ConfigExplorerView", () => {
  it("renders page title, KPIs and table headers", () => {
    const onBack = vi.fn();
    render(<ConfigExplorerView onBack={onBack} />);

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

    // Click back
    const backBtn = screen.getByRole("button", { name: /Quay lại Flow/i });
    fireEvent.click(backBtn);
    expect(onBack).toHaveBeenCalled();
  });

  it("filters active overrides by search input and filter pills", () => {
    render(<ConfigExplorerView onBack={vi.fn()} />);

    // Filter pill click
    const ecPill = screen.getByRole("button", { name: "ec_target 3" });
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
