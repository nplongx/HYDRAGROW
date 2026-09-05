import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { AutomationMetricsBanner } from "./AutomationMetricsBanner";

describe("AutomationMetricsBanner", () => {
  it("renders default KPI metric values and labels", () => {
    render(<AutomationMetricsBanner />);

    expect(screen.getByText("Flow đang hoạt động")).toBeInTheDocument();
    expect(screen.getByText("Cảnh báo trong 24h")).toBeInTheDocument();
    expect(screen.getByText("Ghi đè Config hôm nay")).toBeInTheDocument();
    expect(screen.getByText("100%")).toBeInTheDocument();
    expect(screen.getByText("Tỉ lệ thực thi thành công")).toBeInTheDocument();
  });

  it("renders custom provided metric values", () => {
    render(
      <AutomationMetricsBanner
        metrics={{
          activeFlows: 20,
          alerts24h: 1,
          configOverridesToday: 8,
          successRatePercent: 98.5,
        }}
      />
    );

    expect(screen.getByText("20")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getByText("8")).toBeInTheDocument();
    expect(screen.getByText("98.5%")).toBeInTheDocument();
  });
});
