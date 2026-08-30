import { describe, expect, it, beforeEach } from "vitest";
import * as Blockly from "blockly/core";
import { registerHydragrowBlocks } from "./blocks";
import { extractActions, extractConditions } from "./extractIr";
import { hydrateWorkspace } from "./hydrateIr";

describe("hydrateWorkspace", () => {
  let workspace: Blockly.Workspace;

  beforeEach(() => {
    registerHydragrowBlocks();
    workspace = new Blockly.Workspace();
  });

  it("round-trips a condition and an alert action through extract", () => {
    hydrateWorkspace(
      workspace,
      [{ sensor: "ph", operator: ">", value: 7.5 }],
      [{ type: "alert", level: "warning", message: "pH cao" }],
    );
    expect(extractConditions(workspace)).toEqual([
      { sensor: "ph", operator: ">", value: 7.5 },
    ]);
    expect(extractActions(workspace)).toEqual([
      { type: "alert", level: "warning", message: "pH cao", notifyFcm: true },
    ]);
  });

  it("round-trips an advance_stage action", () => {
    hydrateWorkspace(
      workspace,
      [],
      [{ type: "advance_stage", targetStageOffset: 2, reason: "Đủ 24h" }],
    );
    expect(extractActions(workspace)).toEqual([
      { type: "advance_stage", targetStageOffset: 2, reason: "Đủ 24h" },
    ]);
  });

  it("does nothing on empty input", () => {
    hydrateWorkspace(workspace, [], []);
    expect(extractConditions(workspace)).toEqual([]);
    expect(extractActions(workspace)).toEqual([]);
  });

  it("round-trips multiple conditions in order", () => {
    hydrateWorkspace(
      workspace,
      [
        { sensor: "ph", operator: ">", value: 7.5 },
        { sensor: "ec", operator: "<", value: 1.0 },
      ],
      [{ type: "alert", level: "error", message: "Nguy hiểm" }],
    );
    expect(extractConditions(workspace)).toHaveLength(2);
  });

  it("round-trips a dose action", () => {
    hydrateWorkspace(
      workspace,
      [],
      [{ type: "dose", pump: "PH_DOWN", doseMl: 3, pwm: 80 }],
    );
    expect(extractActions(workspace)).toEqual([
      { type: "dose", pump: "PH_DOWN", doseMl: 3, pwm: 80 },
    ]);
  });

  it("round-trips a water_on action", () => {
    hydrateWorkspace(
      workspace,
      [],
      [{ type: "water_on", pump: "WATER_PUMP_IN", durationSec: 30 }],
    );
    expect(extractActions(workspace)).toEqual([
      { type: "water_on", pump: "WATER_PUMP_IN", durationSec: 30 },
    ]);
  });

  it("round-trips a water_off action", () => {
    hydrateWorkspace(
      workspace,
      [],
      [{ type: "water_off", pump: "MIST_VALVE" }],
    );
    expect(extractActions(workspace)).toEqual([
      { type: "water_off", pump: "MIST_VALVE" },
    ]);
  });

  it("round-trips an emergency_stop action", () => {
    hydrateWorkspace(workspace, [], [{ type: "emergency_stop" }]);
    expect(extractActions(workspace)).toEqual([{ type: "emergency_stop" }]);
  });

  it("round-trips an alert action with notifyFcm explicitly true", () => {
    hydrateWorkspace(
      workspace,
      [],
      [{ type: "alert", level: "info", message: "x", notifyFcm: true }],
    );
    expect(extractActions(workspace)).toEqual([
      { type: "alert", level: "info", message: "x", notifyFcm: true },
    ]);
  });

  it("round-trips an alert action with notifyFcm explicitly false", () => {
    hydrateWorkspace(
      workspace,
      [],
      [{ type: "alert", level: "error", message: "x", notifyFcm: false }],
    );
    expect(extractActions(workspace)).toEqual([
      { type: "alert", level: "error", message: "x", notifyFcm: false },
    ]);
  });

  it("defaults the checkbox to the old severity rule when hydrating a pre-existing alert without notifyFcm", () => {
    // ir_json cũ (trước Phase 2) không có notifyFcm — hydrate phải tự suy ra giá trị
    // checkbox khớp với hành vi cũ (level=error/warning → coi như đã bật FCM), để
    // canvas không "nói dối" người dùng là FCM đang tắt trong khi backend vẫn gửi.
    hydrateWorkspace(
      workspace,
      [],
      [{ type: "alert", level: "warning", message: "x" }],
    );
    // Sau khi hydrate rồi extract lại, giá trị đã trở thành tường minh (đúng thiết kế —
    // xem note ở Step 3): so khớp với true vì level=warning từng ngầm định gửi FCM.
    expect(extractActions(workspace)).toEqual([
      { type: "alert", level: "warning", message: "x", notifyFcm: true },
    ]);
  });
});
