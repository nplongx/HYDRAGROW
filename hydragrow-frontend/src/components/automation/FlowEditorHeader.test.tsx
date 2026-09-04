import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { AutomationPageHeader } from "./AutomationPageHeader";

describe("AutomationPageHeader", () => {
  it("renders page title and new-flow action", () => {
    render(<AutomationPageHeader onNewFlow={vi.fn()} />);
    expect(
      screen.getByRole("heading", { name: "Tự động hóa" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Flow mới/i }),
    ).toBeInTheDocument();
  });
});
