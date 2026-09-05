import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  Badge,
  ConfigCard,
  FieldGroup,
  Chip,
  ChipsRow,
  Segmented,
  ToggleRow,
  SafeNote,
} from "./ConfigPanelUI";

describe("ConfigPanelUI", () => {
  it("renders a badge with tone-based classes", () => {
    render(<Badge tone="sky">TRIGGER · SENSOR</Badge>);
    const badge = screen.getByText("TRIGGER · SENSOR");
    expect(badge.className).toMatch(/sky/);
  });

  it("renders an emphasized config card with thicker border", () => {
    render(
      <ConfigCard tone="indigo" emphasized>
        <p>content</p>
      </ConfigCard>,
    );
    expect(screen.getByText("content").parentElement?.className).toMatch(
      /border-\[1\.5px\]|border-2/,
    );
  });

  it("renders FieldGroup and associates label with input", () => {
    render(
      <FieldGroup label="Field Label">
        <input type="text" />
      </FieldGroup>,
    );
    expect(screen.getByLabelText("Field Label")).toBeInTheDocument();
  });

  it("renders Chip and triggers onRemove when clicked", () => {
    const onRemove = vi.fn();
    render(
      <ChipsRow>
        <Chip tone="emerald" onRemove={onRemove}>
          Tag
        </Chip>
      </ChipsRow>,
    );
    expect(screen.getByText("Tag")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Xóa" }));
    expect(onRemove).toHaveBeenCalledTimes(1);
  });

  it("renders Segmented control with aria-pressed and calls onChange", () => {
    const onChange = vi.fn();
    render(
      <Segmented
        options={[
          { value: "a", label: "Option A" },
          { value: "b", label: "Option B" },
        ]}
        value="a"
        onChange={onChange}
      />,
    );
    const btnA = screen.getByRole("button", { name: "Option A" });
    const btnB = screen.getByRole("button", { name: "Option B" });
    expect(btnA).toHaveAttribute("aria-pressed", "true");
    expect(btnB).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(btnB);
    expect(onChange).toHaveBeenCalledWith("b");
  });

  it("renders ToggleRow switch and fires onChange", () => {
    const onChange = vi.fn();
    render(<ToggleRow label="Enable feature" checked={false} onChange={onChange} />);
    const toggle = screen.getByRole("switch");
    expect(toggle).not.toBeChecked();
    fireEvent.click(toggle);
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("renders SafeNote with checkmark icon", () => {
    render(<SafeNote>Safe note content</SafeNote>);
    expect(screen.getByText("Safe note content")).toBeInTheDocument();
    expect(screen.getByText("✓")).toBeInTheDocument();
  });
});
