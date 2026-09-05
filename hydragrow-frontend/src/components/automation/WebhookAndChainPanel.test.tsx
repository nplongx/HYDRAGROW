import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { WebhookAndChainPanel } from "./WebhookAndChainPanel";

describe("WebhookAndChainPanel", () => {
  it("renders inbound webhook section and field mappings", () => {
    render(
      <WebhookAndChainPanel
        scripts={[]}
        selectedNextFlowIds={[]}
        onToggleNextFlow={vi.fn()}
      />,
    );

    expect(screen.getByText("Webhook & Chuỗi Flow kế tiếp")).toBeInTheDocument();
    expect(screen.getByText("Webhook đến (Inbound)")).toBeInTheDocument();
    expect(screen.getByText("CHUỖI FLOW KẾ TIẾP")).toBeInTheDocument();
    expect(screen.getByText("body.ph")).toBeInTheDocument();
    expect(screen.getByText("body.night_ec_target")).toBeInTheDocument();
  });

  it("warns about loop when self flow is listed in chain", () => {
    const scripts = [
      {
        id: "flow-self",
        device_id: "dev-01",
        name: "Flow Của Tôi",
        kind: "alert" as const,
        enabled: true,
        source: "",
        ir_json: null,
        created_at: "",
        updated_at: "",
      },
    ];

    render(
      <WebhookAndChainPanel
        currentScriptId="flow-self"
        scripts={scripts}
        selectedNextFlowIds={[]}
        onToggleNextFlow={vi.fn()}
      />,
    );

    expect(screen.getByText(/không cho phép — sẽ tạo vòng lặp/i)).toBeInTheDocument();
  });
});
