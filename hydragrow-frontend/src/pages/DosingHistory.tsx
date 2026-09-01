// src/pages/DosingHistory.tsx
import { useState } from 'react';
import { ShieldCheck, Box, Calendar, ChevronDown, Download, AlertTriangle } from 'lucide-react';
import toast from 'react-hot-toast';
import { useQuery } from '@tanstack/react-query';

// --- ZUSTAND, GLEAM & HOOKS ---
import { useDeviceStore } from '../store/useDeviceStore';
import { escape_field_str } from '../../gleam_core/build/dev/javascript/gleam_core/csv.mjs';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { DosingReportCard, DosingReportRecord } from '../components/dosing/DosingReportCard';
import { httpFetch } from '../platform/http';
import { saveTextFile } from '../platform/file';

const DosingHistory = ({ variant = 'standalone' }: { variant?: 'standalone' | 'embedded' }) => {
  const deviceId = useDeviceStore((s) => s.deviceId);
  const settings = useDeviceStore((s) => s.settings);
  const [selectedSeason, setSelectedSeason] = useState<string | null>(null);

  // 1. Query danh sách Mùa vụ
  const { data: seasons = [] } = useQuery({
    queryKey: ['seasons-list', deviceId],
    queryFn: async () => {
      if (!deviceId || !settings?.backend_url) return [];
      const res = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/seasons`, {
        headers: { 'X-API-Key': settings.api_key || '' }
      });
      if (!res.ok) return [];
      const json = await res.json();
      const list = json.data || json || [];
      if (list.length > 0 && !selectedSeason) {
        setSelectedSeason(list[0].id);
      }
      return list;
    },
    enabled: Boolean(deviceId && settings?.backend_url)
  });

  // 2. Query Báo cáo Châm Phân theo Mùa vụ
  const { data: history = [], isLoading, isError, error } = useQuery<DosingReportRecord[]>({
    queryKey: ['dosing-reports', deviceId, selectedSeason],
    queryFn: async () => {
      if (!deviceId || !settings?.backend_url || !selectedSeason) return [];
      const res = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/dosing-reports?season_id=${selectedSeason}`, {
        headers: { 'X-API-Key': settings.api_key || '' }
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const json = await res.json();
      return json.data || json || [];
    },
    enabled: Boolean(deviceId && settings?.backend_url && selectedSeason)
  });

  // Xuất file CSV nông vụ tinh gọn
  const handleExportCSV = async () => {
    if (history.length === 0) return toast.error("Không có dữ liệu để xuất!");
    try {
      const headers = [
        "Mã Thiết Bị",
        "Mùa Vụ",
        "Thời Gian",
        "Phân A (ml)",
        "Phân B (ml)",
        "pH Up (ml)",
        "pH Down (ml)"
      ];

      const csvRows = history.map(row => [
        escape_field_str(row.device_id),
        escape_field_str(row.season_id || ''),
        escape_field_str(new Date(row.created_at).toLocaleString('vi-VN')),
        escape_field_str(String(row.pump_a_ml)),
        escape_field_str(String(row.pump_b_ml)),
        escape_field_str(String(row.ph_up_ml)),
        escape_field_str(String(row.ph_down_ml))
      ].join(","));

      const csvContent = "\uFEFF" + [headers.join(","), ...csvRows].join("\n");
      const saved = await saveTextFile(`lich-su-cham-phan-${selectedSeason || 'tat-ca'}.csv`, csvContent);
      if (saved) toast.success("Xuất file Excel/CSV thành công!");
    } catch {
      toast.error("Lỗi khi xuất file!");
    }
  };

  const activeSeasonData = seasons.find((s: any) => s.id === selectedSeason);


  const contentNode = (
    <>
      {/* Thanh chọn & Xuất Excel */}
      <div className="bg-white/90 border border-emerald-100 rounded-3xl p-4 flex flex-col md:flex-row items-stretch md:items-center justify-between gap-4 backdrop-blur-md relative z-20">
        <div className="relative flex-1 max-w-xs">
          <label className="text-[10px] font-bold text-emerald-700/75 uppercase tracking-widest flex items-center gap-1.5 mb-1.5 ml-1">
            <Calendar size={12} /> Chọn mùa vụ
          </label>
          <div className="relative">
            <select
              value={selectedSeason || ''}
              onChange={(e) => setSelectedSeason(e.target.value)}
              disabled={seasons.length === 0}
              className="w-full bg-white border border-emerald-100 text-emerald-950 text-sm font-semibold rounded-2xl pl-4 pr-10 py-2.5 appearance-none outline-none focus:border-indigo-500 disabled:opacity-50 cursor-pointer shadow-inner"
            >
              {seasons.length === 0 && <option value="">Chưa có mùa vụ</option>}
              {seasons.map((ss: any) => (
                <option key={ss.id} value={ss.id}>{ss.status === 'active' ? '🌱 ' : '📦 '} {ss.name}</option>
              ))}
            </select>
            <ChevronDown className="absolute right-4 top-1/2 -translate-y-1/2 text-emerald-700/75 pointer-events-none" size={16} />
          </div>
        </div>

        <button
          onClick={handleExportCSV}
          disabled={history.length === 0}
          className="flex items-center justify-center gap-2 bg-emerald-100 hover:bg-emerald-200 disabled:opacity-40 text-emerald-900 px-5 py-2.5 rounded-2xl border border-emerald-200 transition-all font-bold text-xs uppercase tracking-wider shrink-0 mt-auto shadow-sm active:scale-95"
        >
          <Download size={14} className={history.length > 0 ? "text-emerald-700" : "text-emerald-700/75"} />
          <span>Xuất Excel</span>
        </button>
      </div>

      {/* Banner thông tin mùa active */}
      {activeSeasonData && (
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between px-5 py-4 bg-indigo-500/5 border border-indigo-500/10 rounded-2xl">
          <div className="flex items-center gap-3">
            <div className="p-2.5 bg-indigo-500/10 rounded-xl">
              <Calendar size={20} className="text-indigo-700" />
            </div>
            <div>
              <div className="flex items-center gap-2 mb-1">
                <p className="text-[10px] text-emerald-700/75 font-bold uppercase tracking-wider">Niên vụ canh tác</p>
                {activeSeasonData.plant_type && (
                  <span className="px-2 py-0.5 bg-emerald-500/10 text-emerald-700 border border-emerald-500/20 rounded-md text-[9px] font-black uppercase">
                    {activeSeasonData.plant_type}
                  </span>
                )}
              </div>
              <p className="text-sm text-emerald-950 font-semibold">
                {new Date(activeSeasonData.start_time).toLocaleDateString('vi-VN')}
                <span className="text-emerald-700/60 mx-1.5"> → </span>
                {activeSeasonData.end_time ? new Date(activeSeasonData.end_time).toLocaleDateString('vi-VN') : 'Hiện tại'}
              </p>
            </div>
          </div>
        </div>
      )}

      {isError && <StateView icon={AlertTriangle} title={(error as Error)?.message || 'Không thể tải lịch sử'} className="animate-in fade-in" />}

      {/* Dòng thời gian các chu kỳ châm */}
      <div className="relative pt-4 pl-1">
        <div className="absolute left-[29px] top-8 bottom-0 w-0.5 bg-gradient-to-b from-slate-200 to-transparent -z-10" />
        {isLoading ? (
          <div className="flex flex-col items-center justify-center gap-3 py-20 text-emerald-700/75">
            <div className="w-5 h-5 border-2 border-emerald-200 border-t-indigo-500 rounded-full animate-spin" />
            <span className="text-xs font-bold tracking-widest uppercase">Đang tải nhật ký châm...</span>
          </div>
        ) : history.length === 0 && !isError ? (
          <StateView icon={Box} title="Chưa có chu kỳ châm phân" description="Hệ thống sẽ ghi nhận khi chu kỳ châm dinh dưỡng được kích hoạt." />
        ) : (
          <div className="space-y-4">
            {history.map((record, index) => (
              <DosingReportCard key={record.id || index} record={record} index={index} />
            ))}
          </div>
        )}
      </div>
    </>
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
