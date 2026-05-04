import { useState, useEffect } from 'react';
import {
  ShieldCheck, Clock, Box,
  AlertTriangle, Settings, Calendar, ChevronDown, Download
} from 'lucide-react';
import toast from 'react-hot-toast';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { LoadingState } from '../components/ui/LoadingState';
import { httpFetch } from '../platform/http';
import { saveTextFile } from '../platform/file';
import { loadAppSettings } from '../platform/settings';

interface DosingReportRecord {
  id: number;
  device_id: string;
  season_id?: string;
  action: string;
  created_at: string;
}

interface CropSeason {
  id: string;
  name: string;
  status: 'active' | 'completed';
  start_time: string;
  end_time?: string;
  plant_type?: string;
}

const DosingHistory = () => {
  const [appConfig, setAppConfig] = useState<any>(null);
  const [deviceId, setDeviceId] = useState<string | null>(null);

  // States cho Vụ Mùa & Lịch sử
  const [seasons, setSeasons] = useState<CropSeason[]>([]);
  const [selectedSeason, setSelectedSeason] = useState<string | null>(null);
  const [history, setHistory] = useState<DosingReportRecord[]>([]);

  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // 1. Tải cấu hình và lấy danh sách Vụ Mùa
  useEffect(() => {
    const init = async () => {
      try {
        const settings: any = await loadAppSettings();
        if (settings && settings.device_id) {
          setAppConfig(settings);
          setDeviceId(settings.device_id);
          await fetchSeasons(settings.device_id, settings.backend_url, settings.api_key);
        } else {
          setIsLoading(false);
        }
      } catch (err) {
        console.error("Lỗi khi tải cấu hình:", err);
        setIsLoading(false);
      }
    };
    init();
  }, []);

  // 2. Fetch danh sách vụ mùa
  const fetchSeasons = async (devId: string, backendUrl: string, apiKey: string) => {
    try {
      const url = `${backendUrl}/api/devices/${devId}/seasons`;
      const response = await httpFetch(url, {
        method: 'GET',
        headers: { 'Content-Type': 'application/json', 'X-API-Key': apiKey }
      });

      if (!response.ok) throw new Error("API chưa sẵn sàng");

      const resData = await response.json();
      const actualData = resData.data ? resData.data : resData;
      setSeasons(actualData);

      // Tự động chọn vụ mùa đầu tiên nếu có dữ liệu
      if (actualData.length > 0) setSelectedSeason(actualData[0].id);

    } catch (err) {
      console.warn("Lỗi khi tải dữ liệu vụ mùa:", err);
    }
  };

  // 3. Lắng nghe sự thay đổi của Vụ Mùa để tải lại Lịch sử
  useEffect(() => {
    if (appConfig && selectedSeason) {
      fetchHistory(appConfig.backend_url, appConfig.api_key, selectedSeason);
    }
  }, [selectedSeason, appConfig]);

  // 4. Gọi API lấy lịch sử
  const fetchHistory = async (backendUrl: string, apiKey: string, seasonId: string) => {
    setIsLoading(true);
    setError(null);
    try {
      if (!backendUrl) throw new Error("Chưa cấu hình URL máy chủ.");

      const url = `${backendUrl}/api/devices/${deviceId}/dosing-reports?season_id=${seasonId}`;
      const response = await httpFetch(url, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': apiKey
        }
      });

      if (!response.ok) throw new Error(`Lỗi máy chủ: HTTP ${response.status}`);

      const resData = await response.json();
      const actualData = resData.data ? resData.data : resData;
      setHistory(actualData);
    } catch (err: any) {
      console.error("Lỗi tải nhật ký niêm phong:", err);
      const errMsg = err.message || (typeof err === 'string' ? err : "Không thể tải dữ liệu");
      setError(errMsg);
      toast.error(errMsg);
    } finally {
      setIsLoading(false);
    }
  };

  // 5. HÀM XUẤT FILE CSV
  const handleExportCSV = async () => {
    if (history.length === 0) {
      toast.error("Không có dữ liệu để xuất!");
      return;
    }

    try {
      const headers = ["ID", "Mã Thiết Bị", "Mã Vụ Mùa", "Hành Động", "Thời Gian"];

      const csvRows = history.map(row => [
        row.id,
        row.device_id,
        row.season_id || "",
        row.action.replace(/_/g, ' '),
        new Date(row.created_at).toLocaleString('vi-VN')
      ].map(val => `"${val}"`).join(","));

      const csvContent = "\uFEFF" + [headers.join(","), ...csvRows].join("\n");

      const saved = await saveTextFile(`nhat-ky-niem-phong-${selectedSeason || 'tat-ca'}.csv`, csvContent);
      if (!saved) return;
      toast.success("Đã lưu file thành công!");
    } catch (err: any) {
      console.error("ERROR SAVE FILE:", err);
      toast.error(err?.message || "Lỗi khi lưu file!");
    }
  };

  const formatDate = (isoString: string) => {
    return new Date(isoString).toLocaleDateString('vi-VN', {
      day: '2-digit', month: '2-digit', year: 'numeric'
    });
  };

  const activeSeasonData = seasons.find(s => s.id === selectedSeason);

  if (isLoading && !selectedSeason && seasons.length === 0) {
    return <LoadingState message="Đang tải dữ liệu..." />;
  }

  if (!deviceId) {
    return (
      <div className="flex flex-col items-center justify-center h-screen space-y-4 p-6 text-center animate-in fade-in bg-slate-950">
        <div className="p-4 bg-slate-900 rounded-full border border-slate-800">
          <Settings size={32} className="text-slate-400" />
        </div>
        <h2 className="text-xl font-bold text-white">Chưa cấu hình thiết bị</h2>
        <p className="text-sm text-slate-400 max-w-xs">
          Vui lòng vào mục Cài đặt để nhập Device ID trước khi xem nhật ký.
        </p>
      </div>
    );
  }

  return (
    <div className="app-page animate-in fade-in slide-in-from-bottom-4 duration-500 pb-24 max-w-4xl mx-auto">

      {/* HEADER & CHỌN VỤ MÙA */}
      <div className="ui-card flex flex-col md:flex-row md:items-center justify-between gap-6 border-indigo-500/20">
        <PageHeader
          icon={ShieldCheck}
          title="Nhật Ký Niêm Phong"
          subtitle="Minh bạch và lưu trữ dữ liệu các hoạt động canh tác."
          className="w-full"
        />

        {/* KHU VỰC BỘ LỌC */}
        <div className="flex flex-col sm:flex-row items-end gap-3 shrink-0">

          {/* Nút Xuất CSV */}
          <button
            onClick={handleExportCSV}
            disabled={history.length === 0}
            className="ui-btn-md flex items-center justify-center space-x-2 bg-slate-800 hover:bg-slate-700 disabled:opacity-50 text-white rounded-2xl transition-all border border-slate-700 active:scale-95 h-[42px]"
            title="Xuất dữ liệu ra Excel"
          >
            <Download size={18} className={history.length > 0 ? "text-emerald-400" : "text-slate-500"} />
            <span className="hidden sm:inline">Xuất CSV</span>
          </button>

          {/* Lọc theo Vụ Mùa */}
          <div className="relative min-w-[220px] w-full sm:w-auto">
            <label className="text-[10px] font-bold text-indigo-400 uppercase tracking-widest mb-1.5 block ml-1 flex items-center gap-1.5">
              <Calendar size={12} /> Mẻ trồng (Vụ mùa)
            </label>
            <div className="relative">
              <select
                value={selectedSeason || ''}
                onChange={(e) => setSelectedSeason(e.target.value)}
                disabled={seasons.length === 0}
                className="ui-input h-[42px] bg-slate-950 border-slate-800 hover:border-indigo-500/50 text-white font-semibold rounded-2xl pr-10 appearance-none focus:ring-indigo-500/30 cursor-pointer disabled:opacity-50"
              >
                {seasons.length === 0 && <option value="">Không có dữ liệu</option>}
                {seasons.map(ss => (
                  <option key={ss.id} value={ss.id}>
                    {ss.status === 'active' ? '🟢' : '📦'} {ss.name}
                  </option>
                ))}
              </select>
              <ChevronDown className="absolute right-4 top-1/2 -translate-y-1/2 text-slate-400 pointer-events-none" size={18} />
            </div>
          </div>

        </div>
      </div>

      {/* HIỂN THỊ THÔNG TIN VỤ MÙA ĐANG CHỌN */}
      {activeSeasonData && (
        <div className="flex items-center justify-between px-4 py-3 bg-indigo-500/5 border border-indigo-500/10 rounded-2xl">
          <div className="flex items-center space-x-3">
            <div className="p-2 bg-indigo-500/10 rounded-lg">
              <Calendar size={18} className="text-indigo-400" />
            </div>
            <div>
              <div className="flex items-center gap-2 mb-0.5">
                <p className="text-[10px] text-slate-500 font-bold uppercase tracking-wider">Thời gian canh tác</p>
                {/* Hiển thị giống cây của vụ mùa này */}
                {activeSeasonData.plant_type && (
                  <span className="px-1.5 py-[1px] bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 rounded text-[9px] font-bold uppercase">
                    {activeSeasonData.plant_type}
                  </span>
                )}
              </div>
              <p className="text-sm text-slate-300 font-medium">
                {formatDate(activeSeasonData.start_time)} - {activeSeasonData.end_time ? formatDate(activeSeasonData.end_time) : 'Đang sinh trưởng'}
              </p>
            </div>
          </div>
        </div>
      )}

      {error && <StateView icon={AlertTriangle} variant="error" title={error} className="animate-in fade-in" />}

      {/* Danh sách Timeline */}
      <div className="space-y-6 relative pt-4">
        {/* Đường line dọc */}
        <div className="absolute left-6 top-8 bottom-0 w-px bg-slate-800 -z-10"></div>

        {isLoading ? (
          <LoadingState
            fullscreen={false}
            className="py-8"
            message="Đang tải dữ liệu niêm phong..."
          />
        ) : history.length === 0 && !error ? (
          <StateView icon={Box} title="Chưa có dữ liệu nào được niêm phong cho mẻ trồng này." className="bg-slate-900/30" />
        ) : (
          history.map((record, index) => (
            <div key={record.id || index} className="flex items-start space-x-4 animate-in slide-in-from-right-4 duration-500" style={{ animationDelay: `${index * 50}ms` }}>

              {/* Icon / Node trên timeline */}
              <div className="shrink-0">
                <div className="h-12 w-12 rounded-full bg-slate-900 border-4 border-slate-950 flex items-center justify-center shadow-lg relative z-10">
                  <Box size={18} className="text-indigo-400" />
                </div>
              </div>

              {/* Card nội dung */}
              <div className="flex-1 bg-slate-900/80 backdrop-blur-md border border-slate-800 rounded-2xl p-4 hover:border-indigo-500/40 transition-all hover:shadow-[0_0_20px_rgba(99,102,241,0.1)] group">
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">

                  <div>
                    <h4 className="text-white font-bold text-sm capitalize tracking-wide">
                      {record.action.replace(/_/g, ' ')}
                    </h4>
                    <div className="flex items-center space-x-3 mt-1.5 text-xs text-slate-400 font-medium">
                      <span className="flex items-center">
                        <Clock size={12} className="mr-1.5" />
                        {new Date(record.created_at).toLocaleString('vi-VN', {
                          hour: '2-digit', minute: '2-digit', second: '2-digit',
                          day: '2-digit', month: '2-digit', year: 'numeric'
                        })}
                      </span>
                    </div>
                  </div>

                </div>
              </div>

            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default DosingHistory;
