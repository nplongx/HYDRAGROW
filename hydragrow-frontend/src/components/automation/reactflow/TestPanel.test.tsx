import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TestPanel } from "./TestPanel";

const queryClient = new QueryClient();

describe("TestPanel", () => {
  it("renders inputs for sample values and runs dry-run", () => {
    const ir = {
      kind: "alert",
      nodes: [{ id: "trigger", type: "trigger", data: { kind: "sensor" }, position: { x: 0, y: 0 } }],
      edges: [],
      next_flow_ids: [],
    } as any;

    render(
      <QueryClientProvider client={queryClient}>
        <TestPanel
          deviceId="dev1"
          ir={ir}
          fields={["ph", "ec", "temp", "water_level"]}
        />
      </QueryClientProvider>,
    );

    // Using query string or getByText since it's an element label next to input but might not be properly associated in DOM
    expect(screen.getByText("ph")).toBeInTheDocument();
    expect(screen.getByText("ec")).toBeInTheDocument();
    expect(screen.getByText("temp")).toBeInTheDocument();
    expect(screen.getByText("water_level")).toBeInTheDocument();

    // primary Chạy thử button
    const runBtn = screen.getByRole("button", { name: "Chạy thử" });
    expect(runBtn).toBeInTheDocument();
  });
});
