import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { WebhookFieldMappingEditor } from "./WebhookFieldMappingEditor";
import type { WebhookTriggerConfig } from "../../../lib/automation/ir";

describe("WebhookFieldMappingEditor", () => {
  it("supports flow/direct mode and field mappings", () => {
    const config: WebhookTriggerConfig = {
      type: "webhook",
      mode: "flow",
      fieldMappings: [],
    };

    const mockOnChange = vi.fn();

    render(
      <WebhookFieldMappingEditor config={config} onChange={mockOnChange} />,
    );

    // mode selector
    const flowRadio = screen.getByLabelText(/Chạy qua Flow/i);
    expect(flowRadio).toBeInTheDocument();

    const directRadio = screen.getByLabelText(/Gọi lệnh trực tiếp/i);
    expect(directRadio).toBeInTheDocument();

    // add mapping
    const addBtn = screen.getByRole("button", { name: /\+ Thêm ánh xạ/i });
    fireEvent.click(addBtn);

    expect(mockOnChange).toHaveBeenCalled();
    const newConfig = mockOnChange.mock.calls[0][0] as WebhookTriggerConfig;
    expect(newConfig.fieldMappings).toHaveLength(1);
    expect(newConfig.fieldMappings[0].bodyPath).toBe("");
    expect(newConfig.fieldMappings[0].targetField).toBe("");

    // We shouldn't strictly test what sourcePath vs bodyPath is used until we implement it properly or just verify what exists
    // it seems the existing code uses bodyPath, not sourcePath.
  });
});
