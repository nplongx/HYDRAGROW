import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { RuleBuilder } from "./RuleBuilder";

function makeBuilder(kind: "alert" | "action_command" | "recipe_override" = "action_command") {
  return {
    kind,
    setKind: () => {},
    nodes: [
      { id: "trigger", type: "trigger", position: { x: 0, y: 0 }, data: { trigger: { type: "sensor" } } },
      { id: "2", type: "condition", position: { x: 0, y: 0 }, data: { conditions: [] } },
      { id: "3", type: "action", position: { x: 0, y: 0 }, data: { actions: [{ type: "dose", pump: "PUMP_A", doseMl: 10, pwm: 60 }] } },
    ],
    updateNodeData: () => {},
  };
}

describe("RuleBuilder", () => {
  it("renders progressive four-part workflow content and safety boundary", () => {
    render(<RuleBuilder builder={makeBuilder()} />);
    expect(screen.getByText("Khi nào?")).toBeInTheDocument();
    expect(screen.getByText("Điều kiện?")).toBeInTheDocument();
    expect(screen.getByText("Làm gì?")).toBeInTheDocument();
    expect(screen.getByText("Chưa có điều kiện. Rule chưa thể validate/enable.")).toBeInTheDocument();
    expect(screen.getByText(/Automation không tắt\/bypass safety/)).toBeInTheDocument();
    expect(screen.queryByText("Emergency stop")).not.toBeInTheDocument();
  });

  it("exposes only source-backed trigger choices", () => {
    render(<RuleBuilder builder={makeBuilder("alert")} />);
    expect(screen.getByText("Cảm biến")).toBeInTheDocument();
    expect(screen.getByText("Giai đoạn FSM")).toBeInTheDocument();
    expect(screen.getByText("Lịch Cron")).toBeInTheDocument();
    expect(screen.getByText("Webhook")).toBeInTheDocument();
  });
});

