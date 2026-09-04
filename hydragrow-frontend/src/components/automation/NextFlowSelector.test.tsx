import { render, screen } from "@testing-library/react";
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
});
