export interface AutomationMetrics {
  activeFlows: number;
  alerts24h: number;
  configOverridesToday: number;
  successRatePercent: number | null;
}

interface Props {
  metrics?: Partial<AutomationMetrics>;
}

export function AutomationMetricsBanner({ metrics }: Props) {
  const data: AutomationMetrics = {
    activeFlows: metrics?.activeFlows ?? 0,
    alerts24h: metrics?.alerts24h ?? 0,
    configOverridesToday: metrics?.configOverridesToday ?? 0,
    successRatePercent: metrics?.successRatePercent ?? null,
  };

  return (
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div className="bg-white rounded-2xl border border-line p-4 shadow-sm hover:shadow-md transition-shadow">
        <div className="text-3xl font-bold text-primary-deep">{data.activeFlows}</div>
        <div className="text-xs text-text-muted font-medium mt-1">Flow đang hoạt động</div>
      </div>

      <div className="bg-white rounded-2xl border border-line p-4 shadow-sm hover:shadow-md transition-shadow">
        <div className="text-3xl font-bold text-warning">{data.alerts24h}</div>
        <div className="text-xs text-text-muted font-medium mt-1">Cảnh báo trong 24h</div>
      </div>

      <div className="bg-white rounded-2xl border border-line p-4 shadow-sm hover:shadow-md transition-shadow">
        <div className="text-3xl font-bold text-config">{data.configOverridesToday}</div>
        <div className="text-xs text-text-muted font-medium mt-1">Ghi đè Config hôm nay</div>
      </div>

      <div className="bg-white rounded-2xl border border-line p-4 shadow-sm hover:shadow-md transition-shadow">
        <div className="text-3xl font-bold text-sky-700">{data.successRatePercent == null ? "—" : `${data.successRatePercent}%`}</div>
        <div className="text-xs text-text-muted font-medium mt-1">Tỉ lệ thực thi thành công</div>
      </div>
    </div>
  );
}
