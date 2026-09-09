import { useState, useMemo } from 'react';
import { ShieldCheck, Box, Download, AlertTriangle } from 'lucide-react';
import toast from 'react-hot-toast';
import { useQuery } from '@tanstack/react-query';

import { useDeviceStore } from '../store/useDeviceStore';
import { escape_field_str } from '../../gleam_core/build/dev/javascript/gleam_core/csv.mjs';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { httpFetch } from '../platform/http';
import { saveTextFile } from '../platform/file';
import {
  totalMlToday,
  hourlyBuckets,
  sevenDayAverage,
  detectAnomalies,
  DosingHistoryRangeRecord,
} from '../lib/dosing/dosingAggregates';
import { DosingTotalCard } from '../components/dosing/DosingTotalCard';
import { DosingAnomalyBanner } from '../components/dosing/DosingAnomalyBanner';

const DosingHistory = ({ variant = 'standalone' }: { variant?: 'standalone' | 'embedded' }) => {
  const deviceId = useDeviceStore((s) => s.deviceId);
  const settings = useDeviceStore((s) => s.settings);
  const [range, setRange] = useState<'today' | '7d' | '30d'>('today');

  const { start, end } = useMemo(() => {
    const now = new Date();
    const end = now.toISOString();
    const start = new Date(now);
    if (range === 'today') start.setHours(0, 0, 0, 0);
    else if (range === '7d') start.setDate(start.getDate() - 7);
    else start.setDate(start.getDate() - 30);
    return { start: start.toISOString(), end };
  }, [range]);

  const { data: records = [], isLoading, isError, error } = useQuery<DosingHistoryRangeRecord[]>({
    queryKey: ['dosing-history-range', deviceId, range],
    queryFn: async () => {
      if (!deviceId || !settings?.backend_url) return [];
      const res = await httpFetch(
        `${settings.backend_url}/api/devices/${deviceId}/analytics/dosing-history?start=${start}&end=${end}`,
        { headers: { 'X-API-Key': settings.api_key || '' } },
      );
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const json = await res.json();
      return json.data || [];
    },
    enabled: Boolean(deviceId && settings?.backend_url),
  });

  const totalToday = totalMlToday(records);
  const hourlyValues = hourlyBuckets(records);
  const { averageMlPerDay, changePercentVsPrevious7Days } = sevenDayAverage(records);
  const anomalies = detectAnomalies(records);

  const handleExportCSV = async () => {
    if (records.length === 0) return toast.error('Không có dữ liệu để xuất!');
    try {
      const headers = [
        'Mã Thiết Bị',
        'Thời Gian',
        'Phân A (ml)',
        'Phân B (ml)',
        'pH Up (ml)',
        'pH Down (ml)',
      ];

      const csvRows = records.map((row) =>
        [
          escape_field_str(deviceId || ''),
          escape_field_str(new Date(row.created_at).toLocaleString('vi-VN')),
          escape_field_str(String(row.pump_a_ml)),
          escape_field_str(String(row.pump_b_ml)),
          escape_field_str(String(row.ph_up_ml)),
          escape_field_str(String(row.ph_down_ml)),
        ].join(','),
      );

      const csvContent = '\uFEFF' + [headers.join(','), ...csvRows].join('\n');
      const saved = await saveTextFile(`lich-su-cham-phan-${range}.csv`, csvContent);
      if (saved) toast.success('Xuất file Excel/CSV thành công!');
    } catch {
      toast.error('Lỗi khi xuất file!');
    }
  };

  const contentNode = (
    <div className="space-y-6">
      <div className="flex items-center justify-between gap-4">
        <div className="flex gap-2">
          {([['today', 'Hôm nay'], ['7d', '7 ngày'], ['30d', '30 ngày']] as const).map(
            ([id, label]) => (
              <button
                key={id}
                onClick={() => setRange(id)}
                className={`px-4 py-2 rounded-xl text-xs font-bold transition-colors ${
                  range === id
                    ? 'bg-emerald-700 text-white'
                    : 'bg-emerald-50 text-emerald-800 hover:bg-emerald-100'
                }`}
              >
                {label}
              </button>
            ),
          )}
        </div>

        <button
          onClick={handleExportCSV}
          disabled={records.length === 0}
          className="flex items-center justify-center gap-2 bg-emerald-100 hover:bg-emerald-200 disabled:opacity-40 text-emerald-900 px-4 py-2 rounded-xl border border-emerald-200 transition-all font-bold text-xs uppercase tracking-wider shrink-0 shadow-sm active:scale-95"
        >
          <Download size={14} className="text-emerald-700" />
          <span>Xuất Excel</span>
        </button>
      </div>

      <DosingTotalCard
        totalMlToday={totalToday}
        hourlyValues={hourlyValues}
        averageMlPerDay={averageMlPerDay}
        changePercent={changePercentVsPrevious7Days}
      />

      <DosingAnomalyBanner anomalies={anomalies} />

      {isError && (
        <StateView
          icon={AlertTriangle}
          title={(error as Error)?.message || 'Không thể tải lịch sử'}
          className="animate-in fade-in"
        />
      )}

      <div className="bg-white border border-emerald-100 rounded-2xl p-4 space-y-2 shadow-sm">
        <p className="text-[10px] font-bold uppercase tracking-wider text-emerald-700/70">
          Chu kỳ gần đây
        </p>
        {isLoading ? (
          <div className="flex flex-col items-center justify-center gap-3 py-16 text-emerald-700/75">
            <div className="w-5 h-5 border-2 border-emerald-200 border-t-emerald-600 rounded-full animate-spin" />
            <span className="text-xs font-bold tracking-widest uppercase">Đang tải nhật ký châm...</span>
          </div>
        ) : records.length === 0 && !isError ? (
          <StateView
            icon={Box}
            title="Chưa có chu kỳ châm phân"
            description="Hệ thống sẽ ghi nhận khi chu kỳ châm dinh dưỡng được kích hoạt."
          />
        ) : (
          <div className="divide-y divide-emerald-50">
            {records.flatMap((r, i) =>
              (['pump_a_ml', 'pump_b_ml', 'ph_up_ml', 'ph_down_ml'] as const)
                .filter((field) => r[field] > 0)
                .map((field) => (
                  <div key={`${i}-${field}`} className="flex items-center justify-between py-3 text-sm">
                    <span className="text-emerald-950">
                      {new Date(r.created_at).toLocaleTimeString('vi-VN', {
                        hour: '2-digit',
                        minute: '2-digit',
                      })}{' '}
                      ·{' '}
                      {
                        {
                          pump_a_ml: 'Bơm phân A',
                          pump_b_ml: 'Bơm phân B',
                          ph_up_ml: 'Bơm pH Up',
                          ph_down_ml: 'Bơm pH Down',
                        }[field]
                      }
                    </span>
                    <span className="font-bold text-emerald-800">{r[field]} ml</span>
                  </div>
                )),
            )}
          </div>
        )}
      </div>
    </div>
  );

  if (variant === 'embedded') return contentNode;

  return (
    <div className="p-4 md:p-8 space-y-6 pb-28 max-w-4xl mx-auto text-emerald-950">
      <PageHeader
        icon={ShieldCheck}
        title="Lịch Sử Châm Phân"
        subtitle="Theo dõi chi tiết lượng phân bón & vi chất đã cấp cho cây trồng"
      />
      {contentNode}
    </div>
  );
};

export default DosingHistory;
