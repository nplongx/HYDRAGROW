import { fireEvent, render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { NextFlowSelector } from "./NextFlowSelector";
import * as flowCycle from "../../lib/automation/flowCycle";

describe("NextFlowSelector", () => {
  it("shows candidate and cycle-disabled candidate", () => {
    const candidate = {
      id: "1",
      name: "Safe Flow",
      kind: "action",
      enabled: true,
    };
    const cyclicCandidate = {
      id: "2",
      name: "Cyclic Flow",
      kind: "action",
      enabled: true,
    };

    vi.spyOn(flowCycle, "wouldCreateCycle").mockImplementation(
      (_currentId, _selected, candidateId) => {
        return candidateId === "2";
      },
    );

    render(
      <NextFlowSelector
        scripts={[candidate, cyclicCandidate] as any}
        selectedIds={[]}
        currentScriptId="3"
        onToggle={() => {}}
      />,
    );

    expect(
      screen.getByRole("checkbox", { name: candidate.name }),
    ).not.toBeDisabled();
    expect(
      screen.getByRole("checkbox", { name: /Cyclic Flow/ }),
    ).toBeDisabled();
    expect(
      screen.getByText("không cho phép — sẽ tạo vòng lặp"),
    ).toBeInTheDocument();
  });

  it('renders the pass-context-variables toggle and reports changes', () => {
    const onTogglePassContext = vi.fn();
    render(
      <NextFlowSelector
        scripts={[]}
        selectedIds={[]}
        currentScriptId="3"
        onToggle={() => {}}
        passContextVariables={false}
        onTogglePassContext={onTogglePassContext}
      />,
    );

    const checkbox = screen.getByLabelText('Truyền biến ngữ cảnh sang flow tiếp theo');
    expect(checkbox).not.toBeChecked();
    fireEvent.click(checkbox);
    expect(onTogglePassContext).toHaveBeenCalledWith(true);
  });

  it('defaults passContextVariables to false when not provided', () => {
    render(
      <NextFlowSelector scripts={[]} selectedIds={[]} currentScriptId="3" onToggle={() => {}} />,
    );
    expect(screen.getByLabelText('Truyền biến ngữ cảnh sang flow tiếp theo')).not.toBeChecked();
  });
});
