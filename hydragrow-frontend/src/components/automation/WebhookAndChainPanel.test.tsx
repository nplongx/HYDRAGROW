import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { WebhookAndChainPanel } from "./WebhookAndChainPanel";

const baseProps = {
  webhookUrl: "https://hydragrow.example/hooks/f-9001",
  mode: "flow" as const,
  onModeChange: vi.fn(),
  mappings: [{ bodyPath: "data.ph", targetField: "ph" }],
  onMappingsChange: vi.fn(),
  currentScriptName: "Cảnh báo bơm ngoài",
  currentScriptKind: "alert",
  configOverwriteSummary: undefined as string | undefined,
  scripts: [],
  selectedNextFlowIds: [],
  onToggleNextFlow: vi.fn(),
};

describe("WebhookAndChainPanel", () => {
  it("renders the real webhookUrl prop, not undefined", () => {
    render(<WebhookAndChainPanel {...baseProps} />);
    expect(screen.getByDisplayValue("https://hydragrow.example/hooks/f-9001")).toBeInTheDocument();
  });

  it("shows the current Flow's real name/kind, not the hardcoded example", () => {
    render(
      <WebhookAndChainPanel {...baseProps} currentScriptName="Tưới buổi sáng" currentScriptKind="action_command" />,
    );
    expect(screen.getByText(/Tưới buổi sáng/)).toBeInTheDocument();
    expect(screen.getByText(/ACTION_COMMAND/)).toBeInTheDocument();
  });

  it("shows the real configOverwrite summary when the Flow has one, and omits that step when it does not", () => {
    const { rerender } = render(
      <WebhookAndChainPanel {...baseProps} configOverwriteSummary="ec_target: 2.4 → 1.8 mS/cm" />,
    );
    expect(screen.getByText(/ec_target: 2\.4 → 1\.8 mS\/cm/)).toBeInTheDocument();
    rerender(<WebhookAndChainPanel {...baseProps} configOverwriteSummary={undefined} />);
    expect(screen.queryByText(/ec_target/)).not.toBeInTheDocument();
  });
});
