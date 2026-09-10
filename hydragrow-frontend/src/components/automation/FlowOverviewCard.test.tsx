import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { FlowOverviewCard } from "./FlowOverviewCard";
import type { UserScript } from "../../types/automation";
import type { AutomationIr } from "../../lib/automation/ir";

function makeScript(overrides: Partial<UserScript> = {}): UserScript {
  return {
    id: "s1",
    device_id: "dev1",
    kind: "alert",
    name: "Flow test",
    source: "",
    enabled: true,
    ir_json: {
      kind: "alert",
      trigger: { type: "sensor" },
      conditions: [{ sensor: "ph", operator: ">", value: 7.5 }],
      actions: [{ type: "alert", level: "warning", message: "pH cao" }],
      nodes: [],
      edges: [],
      next_flow_ids: [],
      chainConfig: { passContextVariables: false, iterationLimit: 5 },
      contextReads: [],
    } as unknown as AutomationIr,
    last_run_at: null,
    created_at: "2026-09-01T00:00:00Z",
    updated_at: "2026-09-01T00:00:00Z",
    ...overrides,
  };
}

const isoDaysAgo = (days: number, hour = 9) => {
  const d = new Date(Date.now() - days * 86400000);
  d.setHours(hour, 0, 0, 0);
  return d.toISOString();
};

describe("FlowOverviewCard last-run footer", () => {
  it("shows Chưa từng chạy when never run", () => {
    render(<FlowOverviewCard script={makeScript()} onClick={() => {}} />);
    expect(screen.getByText("Chưa từng chạy")).toBeTruthy();
  });

  it("shows 'lần cuối: hôm nay' for a run today", () => {
    render(<FlowOverviewCard script={makeScript({ last_run_at: isoDaysAgo(0) })} onClick={() => {}} />);
    expect(screen.getByText("lần cuối: hôm nay")).toBeTruthy();
  });

  it("shows weekday for a run this week", () => {
    render(<FlowOverviewCard script={makeScript({ last_run_at: isoDaysAgo(2) })} onClick={() => {}} />);
    const names = ['Chủ nhật', 'Thứ Hai', 'Thứ Ba', 'Thứ Tư', 'Thứ Năm', 'Thứ Sáu', 'Thứ Bảy'];
    const label = screen.getByText(/lần cuối/).textContent;
    const weekday = new Date(isoDaysAgo(2)).getDay();
    expect(label).toContain(names[weekday]);
  });

  it("shows short date for runs older than a week", () => {
    const run = isoDaysAgo(10);
    render(<FlowOverviewCard script={makeScript({ last_run_at: run })} onClick={() => {}} />);
    const label = screen.getByText(/lần cuối/).textContent;
    expect(label).toContain(new Date(run).toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' }));
  });

  it("renders cron trigger text", () => {
    const script = makeScript({
      ir_json: {
        kind: "alert",
        trigger: { type: "cron", cronExpression: "0 0 6 * * 1", timezone: "Asia/Ho_Chi_Minh" },
        conditions: [{ sensor: "ph", operator: ">", value: 7.5 }],
        actions: [{ type: "alert", level: "warning", message: "pH cao" }],
        nodes: [],
        edges: [],
        next_flow_ids: [],
        chainConfig: { passContextVariables: false, iterationLimit: 5 },
        contextReads: [],
      } as unknown as AutomationIr,
    });
    render(<FlowOverviewCard script={script} onClick={() => {}} />);
    expect(screen.getByText("Trigger: 06:00 Thứ Hai")).toBeTruthy();
  });
});