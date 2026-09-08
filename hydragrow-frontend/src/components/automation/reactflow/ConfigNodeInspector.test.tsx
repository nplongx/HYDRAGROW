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

  it("does not render dead applyMode radios", () => {
    render(<ConfigNodeInspector onClose={vi.fn()} />);
    expect(screen.queryByText(/ÁP DỤNG GHI ĐÈ KHI/i)).not.toBeInTheDocument();
    expect(screen.queryByDisplayValue("during_true")).not.toBeInTheDocument();
  });

  it("displays conditionSummary prop and default copy", () => {
    const { rerender } = render(
      <ConfigNodeInspector onClose={vi.fn()} conditionSummary="pH > 6.0" />,
    );
    expect(screen.getByText("pH > 6.0")).toBeInTheDocument();
    rerender(<ConfigNodeInspector onClose={vi.fn()} />);
    expect(screen.getByText("Chưa cấu hình")).toBeInTheDocument();
  });

  it("saves priority and autoRestore without applyMode", () => {
    const onSave = vi.fn();
    const onClose = vi.fn();
    render(
      <ConfigNodeInspector
        onSave={onSave}
        onClose={onClose}
        initialPriority={5}
        initialAutoRestore={false}
      />,
    );
    const priorityInput = screen.getByLabelText("Priority");
    expect(priorityInput).toBeInTheDocument();
    fireEvent.change(priorityInput, { target: { value: "7" } });
    fireEvent.click(screen.getByRole("button", { name: /Lưu cấu hình Node/i }));
    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({ priority: 7, autoRestore: false }),
    );
    const payload = onSave.mock.calls[0][0] as Record<string, unknown>;
    expect(payload).not.toHaveProperty("applyMode");
  });

  it("shows priority-based conflict warning copy", () => {
    render(<ConfigNodeInspector onClose={vi.fn()} />);
    expect(screen.getByText(/priority cao hơn sẽ thắng/i)).toBeInTheDocument();
    expect(screen.queryByText(/thứ tự trong danh sách/i)).not.toBeInTheDocument();
  });
});
