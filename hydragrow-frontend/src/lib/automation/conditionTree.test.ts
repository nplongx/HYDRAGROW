import { describe, it, expect } from "vitest";
import {
  toEditorRoot,
  fromEditorRoot,
  summarizeConditionTree,
} from "./conditionTree";
import type { Condition, ConditionGroup, ConditionOrGroup } from "./ir";

describe("ConditionTree", () => {
  it("toEditorRoot and fromEditorRoot preserve tree structure losslessly", () => {
    // A single leaf
    const leaf: Condition = { sensor: "ph", operator: "<", value: 5.5 };

    // A nested OR group inside an array
    const nestedOr: ConditionGroup = {
      op: "or",
      children: [
        { sensor: "ph", operator: "<", value: 5.5 },
        { sensor: "ph", operator: ">", value: 7.5 },
      ],
    };

    // Test 1: Single leaf wrapped in an implicit AND root
    const arrayWithLeaf: ConditionOrGroup[] = [leaf];
    const root1 = toEditorRoot(arrayWithLeaf);
    expect(root1.op).toBe("and");
    expect(root1.children).toHaveLength(1);
    expect(fromEditorRoot(root1)).toEqual(arrayWithLeaf);

    // Test 2: Pre-existing single group is preserved as root
    const arrayWithGroup: ConditionOrGroup[] = [nestedOr];
    const root2 = toEditorRoot(arrayWithGroup);
    expect(root2).toEqual(nestedOr); // Because it is the only element and is a group
    expect(fromEditorRoot(root2)).toEqual(arrayWithGroup);

    // Test 3: Multiple siblings wrap in AND root
    const arrayWithSiblings: ConditionOrGroup[] = [
      nestedOr,
      { sensor: "ec", operator: ">", value: 3.0 },
    ];
    const root3 = toEditorRoot(arrayWithSiblings);
    expect(root3.op).toBe("and");
    expect(root3.children).toHaveLength(2);
    expect(fromEditorRoot(root3)).toEqual(arrayWithSiblings);
  });

  it("correctly formats an expression preview", () => {
    const leaf: Condition = { sensor: "ec", operator: ">", value: 3.0 };
    const nestedOr: ConditionGroup = {
      op: "or",
      children: [
        { sensor: "ph", operator: "<", value: 5.5, mode: "instant" },
        { sensor: "ph", operator: ">", value: 7.5, mode: "instant" },
      ],
    };

    // Empty
    expect(summarizeConditionTree([])).toBe("Chưa cấu hình");

    // Single
    expect(summarizeConditionTree([leaf])).toBe("ec > 3");

    // Complex
    expect(summarizeConditionTree([nestedOr, leaf])).toBe(
      "(ph < 5.5 hoặc ph > 7.5) và ec > 3",
    );
  });
});
