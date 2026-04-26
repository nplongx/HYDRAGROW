import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { fetch } from '@tauri-apps/plugin-http';
import {
  ShieldCheck, Clock, ExternalLink, Box, Server,
  AlertTriangle, Settings, Calendar, ChevronDown, Download
} from 'lucide-react';
import toast from 'react-hot-toast';
import { writeTextFile } from '@tauri-apps/plugin-fs'; import { save } from '@tauri-apps/plugin-dialog';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
interface BlockchainRecord {
  id: number;
  device_id: string;
  season_id?: string;
  action: string;
  tx_id: string;
  created_at: string;
}

interface CropSeason {
  id: string;
  name: string;
  status: 'active' | 'completed';
  start_time: string;
  end_time?: string;
}

const BlockchainHistory = () => {
  const [appConfig, setAppConfig] = useState<any>(null);
  const [deviceId, setDeviceId] = useState<string | null>(null);

  // States cho Vụ Mùa & Lịch sử
  const [seasons, setSeasons] = useState<CropSeason[]>([]);
  const [selectedSeason, setSelectedSeason] = useState<string | null>(null);
  const [history, setHistory] = useState<BlockchainRecord[]>([]);

  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // 1. Tải cấu hình và lấy danh sách Vụ Mùa
  useEffect(() => {
    const init = async () => {
      try {
        const settings: any = await invoke('load_settings').catch(() => null);
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
      const response = await fetch(url, {
        method: 'GET',
        headers: { 'Content-Type': 'application/json', 'X-API-Key': apiKey }
      });

      if (!response.ok) throw new Error("API chưa sẵn sàng");

      const resData = await response.json();
      const actualData = resData.data ? resData.data : resData;
      setSeasons(actualData);
      if (actualData.length > 0) setSelectedSeason(actualData[0].id);

    } catch (err) {
      console.warn("Lỗi khi tải dữ liệu vụ mùa:", err);
    }
  };

  // 3. Lắng nghe sự thay đổi của Vụ Mùa để tải lại Lịch sử Blockchain
  useEffect(() => {
    if (appConfig && selectedSeason) {
      fetchHistory(appConfig.backend_url, appConfig.api_key, selectedSeason);
    }
  }, [selectedSeason, appConfig]);

  // 4. Gọi API lấy lịch sử Blockchain CÓ LỌC THEO VỤ MÙA
  const fetchHistory = async (backendUrl: string, apiKey: string, seasonId: string) => {
    setIsLoading(true);
    setError(null);
    try {
      if (!backendUrl) throw new Error("Chưa cấu hình URL máy chủ.");

      // Gắn season_id vào URL
      const url = `${backendUrl}/api/devices/${deviceId}/blockchain?season_id=${seasonId}`;
      const response = await fetch(url, {
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
      console.error("Lỗi tải lịch sử blockchain:", err);
      const errMsg = err.message || (typeof err === 'string' ? err : "Không thể tải dữ liệu");
      setError(errMsg);
      toast.error(errMsg);
    } finally {
      setIsLoading(false);
    }
  };

  // 5. API xác thực Transaction On-chain
  const handleVerify = async (txId: string) => {
    const toastId = toast.loading("Đang truy xuất thông tin xác thực trên Solana...");
    try {
      if (!appConfig || !appConfig.backend_url) throw new Error("Lỗi cấu hình hệ thống");

      const url = `${appConfig.backend_url}/api/blockchain/verify/${txId}`;
      const response = await fetch(url, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': appConfig.api_key
        }
      });

      if (!response.ok) throw new Error(`HTTP ${response.status}`);

      const resData = await response.json();
      const data = resData.data ? resData.data : resData;

      toast.success("Xác thực thành công! Đang mở trình duyệt...", { id: toastId });

      setTimeout(() => {
        window.open(data.verification_links.solscan, '_blank');
      }, 500);

    } catch (err: any) {
      toast.error("Lỗi xác thực: " + (err.message || err), { id: toastId });
    }
  };

  // 🟢 6. HÀM XUẤT FILE CSV
  const handleExportCSV = async () => {
    if (history.length === 0) {
      toast.error("Không có dữ liệu để xuất!");
      return;
    }

    try {
      const headers = ["ID", "Mã Thiết Bị", "Mã Vụ Mùa", "Hành Động", "TxID", "Thời Gian"];

      const csvRows = history.map(row => [
        row.id,
        row.device_id,
        row.season_id || "",
        row.action.replace(/_/g, ' '),
        row.tx_id,
        new Date(row.created_at).toLocaleString('vi-VN')
      ].map(val => `"${val}"`).join(","));

      const csvContent = "\uFEFF" + [headers.join(","), ...csvRows].join("\n");

      // 📂 Cho user chọn nơi lưu
      const filePath = await save({
        defaultPath: `nhat-ky-niem-phong-${selectedSeason || 'tat-ca'}.csv`
      });

      if (!filePath) return; // user cancel

      // 💾 Ghi file thật xuống máy
      await writeTextFile(filePath, csvContent);

      toast.success("Đã lưu file thành công!");
    } catch (err: any) {
      console.error("ERROR SAVE FILE:", err);
      toast.error(err?.message || "Lỗi khi lưu file!");
    }
  };

  const truncateTx = (tx: string) => {
    if (!tx || tx.length < 15) return tx;
    return `${tx.slice(0, 6)}...${tx.slice(-6)}`;
  };

  const formatDate = (isoString: string) => {
    return new Date(isoString).toLocaleDateString('vi-VN', {
      day: '2-digit', month: '2-digit', year: 'numeric'
    });
  };

  // Lấy thông tin vụ mùa đang được chọn
  const activeSeasonData = seasons.find(s => s.id === selectedSeason);

  if (isLoading && !selectedSeason) {
    return (
      <div className="flex flex-col items-center justify-center h-screen space-y-4 bg-slate-950">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-500"></div>
        <p className="text-sm text-slate-400 font-medium animate-pulse">Đang đồng bộ sổ cái Solana...</p>
      </div>
    );
  }

  if (!deviceId) {
    return (
      <div className="flex flex-col items-center justify-center h-screen space-y-4 p-6 text-center animate-in fade-in bg-slate-950">
        <div className="p-4 bg-slate-900 rounded-full border border-slate-800">
          <Settings size={32} className="text-slate-400" />
        </div>
        <h2 className="text-xl font-bold text-white">Chưa cấu hình thiết bị</h2>
        <p className="text-sm text-slate-400 max-w-xs">
          Vui lòng vào mục Cài đặt để nhập Device ID trước khi xem lịch sử Blockchain.
        </p>
      </div>
    );
  }

  return (
    <div className="app-page animate-in fade-in slide-in-from-bottom-4 duration-500 pb-24 max-w-4xl mx-auto">

      {/* HEADER & CHỌN VỤ MÙA TÁI THIẾT KẾ */}
      <div className="ui-card flex flex-col md:flex-row md:items-center justify-between gap-4 border-indigo-500/20">
        <PageHeader
          icon={ShieldCheck}
          title="Nhật Ký Niêm Phong"
          subtitle="Minh bạch dữ liệu canh tác. Lưu trữ vĩnh viễn trên mạng Solana."
          className="w-full"
        />

        {/* 🟢 KHU VỰC NÚT XUẤT CSV VÀ DROPDOWN CHỌN MẺ TRỒNG */}
        <div className="flex flex-row items-end gap-3 shrink-0">

          {/* Nút Xuất CSV */}
          <button
            onClick={handleExportCSV}
            disabled={history.length === 0}
            className="ui-btn-md flex items-center justify-center space-x-2 bg-slate-800 hover:bg-slate-700 disabled:opacity-50 text-white rounded-2xl transition-all border border-slate-700 active:scale-95"
            title="Xuất dữ liệu ra Excel"
          >
            <Download size={18} className={history.length > 0 ? "text-emerald-400" : "text-slate-500"} />
            <span className="hidden sm:inline">Xuất CSV</span>
          </button>

          {/* Dropdown Vụ Mùa */}
          <div className="relative min-w-[200px] flex-1 sm:flex-none">
            <label className="text-[10px] font-bold text-indigo-400 uppercase tracking-widest mb-1.5 block ml-1">Lọc theo vụ mùa</label>
            <div className="relative">
              <select
                value={selectedSeason || ''}
                onChange={(e) => setSelectedSeason(e.target.value)}
                className="ui-input bg-slate-950 border-slate-800 hover:border-indigo-500/50 text-white font-semibold rounded-2xl pr-10 appearance-none focus:ring-indigo-500/30 cursor-pointer"
              >
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
              <p className="text-[10px] text-slate-500 font-bold uppercase tracking-wider">Thời gian canh tác</p>
              <p className="text-sm text-slate-300 font-medium">
                {formatDate(activeSeasonData.start_time)} - {activeSeasonData.end_time ? formatDate(activeSeasonData.end_time) : 'Đang sinh trưởng'}
              </p>
            </div>
          </div>
          <div className="hidden sm:flex items-center space-x-2 bg-slate-900 px-3 py-1.5 rounded-full border border-slate-800 shrink-0">
            <Server size={14} className="text-indigo-400" />
            <span className="text-[11px] font-semibold text-slate-400">Solana Devnet</span>
          </div>
        </div>
      )}

      {error && <StateView icon={AlertTriangle} variant="error" title={error} className="animate-in fade-in" />}

      {/* Danh sách Timeline */}
      <div className="space-y-6 relative pt-4">
        {/* Đường line dọc */}
        <div className="absolute left-6 top-8 bottom-0 w-px bg-slate-800 -z-10"></div>

        {isLoading ? (
          <div className="flex justify-center py-10">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-indigo-500"></div>
          </div>
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

                  {/* Phần hiển thị Tx và Nút check */}
                  <div className="flex items-center bg-slate-950 rounded-xl p-1.5 border border-slate-800/80 self-start md:self-auto shrink-0">
                    <span className="px-3 font-mono text-[11px] text-slate-400 select-all">
                      {truncateTx(record.tx_id)}
                    </span>
                    <button
                      onClick={() => handleVerify(record.tx_id)}
                      className="flex items-center space-x-1.5 bg-indigo-600 hover:bg-indigo-500 text-white px-3 py-1.5 rounded-lg text-xs font-bold transition-all shadow-lg shadow-indigo-500/20 active:scale-95"
                    >
                      <ExternalLink size={12} />
                      <span>Xác Thực</span>
                    </button>
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

export default BlockchainHistory;
