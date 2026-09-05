import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { ConfigNodeInspector } from "./ConfigNodeInspector";

describe("ConfigNodeInspector", () => {
  it("renders 3 panels and audit log table", () => {
    const onClose = vi.fn();
    const onSave = vi.fn();

    render(
      <ConfigNodeInspector
        initialKey="ec_target"
        initialValue={1.8}
        onSave={onSave}
        onClose={onClose}
      />,
    );

    // Header & Panels
    expect(screen.getByText("Đọc & Ghi đè Config theo điều kiện")).toBeInTheDocument();
    expect(screen.getByText("(1) Đọc Config")).toBeInTheDocument();
    expect(screen.getByText("(2) Điều kiện áp dụng")).toBeInTheDocument();
    expect(screen.getByText("(3) Ghi đè giá trị")).toBeInTheDocument();
    expect(screen.getByText(/Nhật ký ghi đè \(audit log\)/i)).toBeInTheDocument();

    // Value input
    const input = screen.getByDisplayValue("1.8");
    expect(input).toBeInTheDocument();

    // Clamping: enter value exceeding max (3.2)
    fireEvent.change(input, { target: { value: "5.0" } });
    expect(screen.getByText(/Tự động kẹp về 3.2 mS\/cm/i)).toBeInTheDocument();

    // Save
    const saveBtn = screen.getByRole("button", { name: /Lưu cấu hình Node/i });
    fireEvent.click(saveBtn);

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        configKey: "ec_target",
        overrideValue: 3.2,
      }),
    );
    expect(onClose).toHaveBeenCalled();
  });
});
