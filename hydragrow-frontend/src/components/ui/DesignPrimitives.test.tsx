import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Activity, Info } from "lucide-react";
import { AccordionSection } from "./AccordionSection";
import { InputGroup } from "./InputGroup";
import { LoadingState } from "./LoadingState";
import { PageHeader } from "./PageHeader";
import { SensorBentoCard } from "./SensorBentoCard";
import { StateView } from "./StateView";
import { SubCard } from "./SubCard";

describe("P3.2 design primitives", () => {
  it("PageHeader exposes a semantic header landmark", () => {
    render(<PageHeader title="Tổng quan" subtitle="Trạng thái trạm" />);
    expect(screen.getByRole("banner")).toHaveTextContent("Tổng quan");
  });

  it("LoadingState exposes an explicit busy status", () => {
    render(<LoadingState message="Đang tải telemetry..." />);
    expect(screen.getByRole("status")).toHaveAttribute("aria-busy", "true");
    expect(screen.getByText("Đang tải telemetry...")).toBeInTheDocument();
  });

  it("StateView uses semantic tone tokens instead of page-local color", () => {
    render(<StateView icon={Info} title="Không khả dụng" tone="danger" />);
    expect(screen.getByText("Không khả dụng").closest(".ui-state")).toHaveClass(
      "border-error/40",
    );
  });

  it("InputGroup links label and helper text to its input", () => {
    render(<InputGroup label="EC mục tiêu" helperText="Đơn vị ppm" />);
    const input = screen.getByLabelText("EC mục tiêu");
    const describedBy = input.getAttribute("aria-describedby");
    expect(describedBy).toBeTruthy();
    expect(document.getElementById(describedBy!)).toHaveTextContent(
      "Đơn vị ppm",
    );
  });

  it("AccordionSection exposes expanded state and controlled content relationship", () => {
    render(
      <AccordionSection id="advanced" title="Nâng cao" defaultOpen={false}>
        Chi tiết
      </AccordionSection>,
    );
    const button = screen.getByRole("button", { name: "Nâng cao" });
    expect(button).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(button);
    expect(button).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Chi tiết")).toBeInTheDocument();
    expect(button).toHaveAttribute("aria-controls", "advanced-content");
  });

  it("SensorBentoCard exposes metric identity and preserves unavailable value semantics", () => {
    render(
      <SensorBentoCard
        title="EC"
        value={null}
        icon={Activity}
        theme="blue"
        statusLabel="Chưa có dữ liệu"
      />,
    );
    expect(screen.getByRole("article", { name: "EC" })).toHaveTextContent("--");
    expect(screen.getByText("Chưa có dữ liệu")).toBeInTheDocument();
  });

  it("SubCard remains a semantic reusable section container", () => {
    render(<SubCard title="Thông số">Nội dung</SubCard>);
    expect(screen.getByText("Thông số")).toBeInTheDocument();
    expect(screen.getByText("Nội dung")).toBeInTheDocument();
  });
});
