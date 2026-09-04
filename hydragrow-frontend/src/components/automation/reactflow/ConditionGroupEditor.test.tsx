import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { ConditionGroupEditor } from "./ConditionGroupEditor";
import type { ConditionGroup } from "../../../lib/automation/ir";

describe("ConditionGroupEditor", () => {
  it("renders nested condition groups", () => {
    // ((ph < 5.5 OR ph > 7.5) AND ec > 3.0)
    const tree: ConditionGroup = {
      op: "and",
      children: [
        {
          op: "or",
          children: [
            { sensor: "ph", operator: "<", value: 5.5, mode: "instant" },
            { sensor: "ph", operator: ">", value: 7.5, mode: "instant" },
          ],
        },
        { sensor: "ec", operator: ">", value: 3.0, mode: "instant" },
      ],
    };

    const mockOnChange = vi.fn();

    render(
      <ConditionGroupEditor
        group={tree}
        fields={["ph", "ec"]}
        onChange={mockOnChange}
        isRoot={true}
      />,
    );

    // root AND selected; nested group OR selected;
    const andButtons = screen.getAllByRole("button", { name: "AND" });
        expect(andButtons[0]).toHaveClass("bg-emerald-600"); // Assuming selected styling for root

    // leaf rows render
    expect(screen.getAllByDisplayValue("ph").length).toBe(2);
    expect(screen.getByDisplayValue("ec")).toBeInTheDocument();
    expect(screen.getByDisplayValue("5.5")).toBeInTheDocument();
    expect(screen.getByDisplayValue("7.5")).toBeInTheDocument();
    expect(screen.getByDisplayValue("3")).toBeInTheDocument();

    // remove buttons exist
    const removeButtons = screen.getAllByRole("button", { name: /✕/i });
    expect(removeButtons.length).toBeGreaterThan(0);

    // + Thêm điều kiện and + Thêm nhóm con (AND/OR) exist
    expect(
      screen.getAllByRole("button", { name: "+ Thêm điều kiện" }).length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByRole("button", { name: "+ Thêm nhóm con (AND/OR)" })
        .length,
    ).toBeGreaterThan(0);
  });
});
