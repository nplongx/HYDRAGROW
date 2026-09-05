import { render, screen, fireEvent } from "@testing-library/react";
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
    const andButtons = screen.getAllByRole("button", { name: "AND — tất cả đúng" });
    expect(andButtons[0]).toHaveClass("bg-emerald-700");
    expect(andButtons[0]).toHaveAttribute("aria-pressed", "true");

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

describe('ConditionGroupEditor variable combobox', () => {
  it('offers context variables as suggestions on the sensor field, merged with fixed fields', () => {
    const tree: ConditionGroup = {
      op: 'and',
      children: [{ sensor: 'ph', operator: '>', value: 7.2, mode: 'instant' }],
    };

    render(
      <ConditionGroupEditor
        group={tree}
        fields={['ph', 'ec']}
        availableVariables={['ph_target_now']}
        onChange={vi.fn()}
        isRoot={true}
      />,
    );

    const sensorInput = screen.getByLabelText('Cảm biến') as HTMLInputElement;
    expect(sensorInput.value).toBe('ph');
    const options = Array.from(document.querySelectorAll(`#${sensorInput.getAttribute('list')} option`)).map(
      (o) => o.getAttribute('value'),
    );
    expect(options).toEqual(['ph', 'ec', 'ph_target_now']);
  });

  it('toggling "dùng biến" switches the value field to a variable combobox and sets valueVariable', () => {
    const tree: ConditionGroup = {
      op: 'and',
      children: [{ sensor: 'ph', operator: '>', value: 7.2, mode: 'instant' }],
    };
    const onChange = vi.fn();

    render(
      <ConditionGroupEditor
        group={tree}
        fields={['ph', 'ec']}
        availableVariables={['ph_target_now']}
        onChange={onChange}
        isRoot={true}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Dùng biến' }));
    fireEvent.change(screen.getByLabelText('Biến giá trị'), {
      target: { value: 'ph_target_now' },
    });

    expect(onChange).toHaveBeenLastCalledWith({
      op: 'and',
      children: [{ sensor: 'ph', operator: '>', value: 7.2, mode: 'instant', valueVariable: 'ph_target_now' }],
    });
  });

  it('toggling back to "dùng số" clears valueVariable', () => {
    const tree: ConditionGroup = {
      op: 'and',
      children: [
        { sensor: 'ph', operator: '>', value: 7.2, mode: 'instant', valueVariable: 'ph_target_now' },
      ],
    };
    const onChange = vi.fn();

    render(
      <ConditionGroupEditor group={tree} fields={['ph', 'ec']} onChange={onChange} isRoot={true} />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Dùng số' }));

    expect(onChange).toHaveBeenLastCalledWith({
      op: 'and',
      children: [{ sensor: 'ph', operator: '>', value: 7.2, mode: 'instant', valueVariable: undefined }],
    });
  });
});

