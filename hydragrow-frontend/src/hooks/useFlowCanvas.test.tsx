import { renderHook } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { useFlowCanvas } from "./useFlowCanvas";
import type { UserScript } from "../types/automation";

describe("useFlowCanvas", () => {
  it("creates one animated edge from next_flow_ids", () => {
    const flowA: UserScript = {
      id: "flow-a",
      name: "Flow A",
      kind: "alert",
      enabled: true,
      source: "",
      device_id: "dev1",
      ir_json: {
        kind: "alert",
        nodes: [],
        edges: [],
        next_flow_ids: ["flow-b"],
      },
    };
    const flowB: UserScript = {
      id: "flow-b",
      name: "Flow B",
      kind: "action_command",
      enabled: true,
      source: "",
      device_id: "dev1",
      ir_json: {
        kind: "action_command",
        nodes: [],
        edges: [],
        next_flow_ids: [],
      },
    };

    const { result } = renderHook(() => useFlowCanvas([flowA, flowB]));

    expect(result.current.edges).toEqual([
      expect.objectContaining({
        id: `flow-a->flow-b`,
        source: "flow-a",
        target: "flow-b",
        animated: true,
      }),
    ]);
  });
});
