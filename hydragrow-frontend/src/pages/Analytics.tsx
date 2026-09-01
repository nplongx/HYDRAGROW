// src/pages/Analytics.tsx
import { LineChart as ChartIcon, ExternalLink, Activity, Server, Cpu } from 'lucide-react';
import { PageHeader } from '../components/ui/PageHeader';
import { SubCard } from '../components/ui/SubCard';

const Analytics = ({ variant = 'standalone' }: { variant?: 'standalone' | 'embedded' }) => {
  return (
    <div className={variant === "embedded" ? "max-w-4xl" : "app-page max-w-4xl"}>
      {variant !== 'embedded' && (
        <PageHeader
          icon={ChartIcon}
          title="Grafana & Prometheus Observability"
          subtitle="Hệ thống phân tích chuỗi thời gian, thuật toán tự học MIMO/Kalman và tài nguyên phần cứng đã được chuyển đổi tập trung sang Grafana."
        />
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <SubCard title="Trung tâm Giám sát Grafana">
          <div className="space-y-4 text-sm text-emerald-900">
            <p className="leading-relaxed">
              Các biểu đồ chi tiết về biến động EC, pH, nhiệt độ, mực nước, hệ số tự học EMA Gain, ma trận tương tác MIMO và độ tin cậy Kalman hiện được giám sát trực tiếp trên hạ tầng Grafana/Prometheus chuyên dụng.
            </p>
            <a
              href="http://localhost:3000"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 px-5 py-3 bg-sky-600 hover:bg-sky-700 text-white rounded-xl font-medium transition-all shadow-sm"
            >
              <ExternalLink size={16} />
              <span>Mở Grafana Dashboard</span>
            </a>
          </div>
        </SubCard>

        <SubCard title="Các nhóm Metrics chính trên Grafana">
          <div className="space-y-3 text-xs text-emerald-800">
            <div className="flex items-center gap-2.5 p-2.5 bg-emerald-50 rounded-lg border border-emerald-100">
              <Activity size={16} className="text-emerald-700 shrink-0" />
              <span><b>Adaptive Learning:</b> Gain, Step Ratio, Tuner State & Tolerance.</span>
            </div>
            <div className="flex items-center gap-2.5 p-2.5 bg-emerald-50 rounded-lg border border-emerald-100">
              <Server size={16} className="text-sky-700 shrink-0" />
              <span><b>MIMO Matrix & Kalman:</b> Độ tin cậy cơ cấu chấp hành & trạng thái ma trận.</span>
            </div>
            <div className="flex items-center gap-2.5 p-2.5 bg-emerald-50 rounded-lg border border-emerald-100">
              <Cpu size={16} className="text-purple-700 shrink-0" />
              <span><b>ESP32 Telemetry:</b> Free Heap, WiFi RSSI, Uptime & Log Drop Count.</span>
            </div>
          </div>
        </SubCard>
      </div>
    </div>
  );
};

export default Analytics;
