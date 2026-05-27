import { useEffect, useState } from 'react';
import {
  Droplets, Thermometer, Activity, Waves, Settings, Zap, Cpu,
  Wifi, Clock, AlertTriangle, ShieldCheck, ChevronDown, ChevronUp,
  LineChart
} from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';

import { SensorBentoCard } from '../components/ui/SensorBentoCard';
import { LoadingState } from '../components/ui/LoadingState';
import { extractFaultCode } from '../components/ui/FsmStatusBadge';
import { getFaultGuide } from '../components/ui/FaultExplanation';
import { httpFetch } from '../platform/http';
import { loadAppSettings } from '../platform/settings';
import { useFCM } from '../hooks/useFCM';
import toast from 'react-hot-toast';

const ActiveDeviceTag = ({ label, color }: { label: string; color: string }) => (
  <span className={`flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold tracking-wide shadow-sm animate-pulse border ${color}`}>
    <Zap size={12} className="fill-current" />
    {label}
  </span>
);

const Dashboard = () => {
  const {
    deviceId, sensorData, deviceStatus,
    controllerHealth, fsmState, friendlyState, computedHealth,
    isLoading, isSensorOnline, settings
  } = useDeviceContext();

  const { enableNotifications, permission } = useFCM();
  const [showAdvancedDiag, setShowAdvancedDiag] = useState(false);

  useEffect(() => {
    const run = async () => {
      if (!deviceId) return;
      const app = await loadAppSettings();
      const cfg: any = settings || app;
      if (!cfg?.backend_url || !cfg?.api_key) return;
      await httpFetch(`${cfg.backend_url}/api/devices/${deviceId}/events?limit=200`, {
        headers: { 'X-API-Key': cfg.api_key }
      });
    };
    run();
  }, [deviceId, settings]);

  if (isLoading || !sensorData) {
    return <LoadingState message="Đang kết nối kho dữ liệu nông nghiệp thông minh..." />;
  }

  if (!deviceId) {
    return (
      <div className="flex flex-col items-center justify-center h-full min-h-[80vh] space-y-5 p-6 text-center">
        <div className="p-6 bg-slate-900/60 rounded-3xl border border-slate-800 shadow-xl backdrop-blur-md">
          <Settings size={40} className="text-slate-400 animate-spin-slow" />
        </div>
        <div className="space-y-2 max-w-xs">
          <h2 className="text-xl font-bold text-slate-100">Chưa thiết lập Trạm điều khiển</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            Hệ thống cần ID thiết bị để kết nối đám mây. Vui lòng di chuyển đến mục Cài đặt để thiết lập.
          </p>
        </div>
      </div>
    );
  }

  const isOnline = deviceStatus?.is_online;
  const faultCode = extractFaultCode(fsmState || undefined);
  const faultGuide = getFaultGuide(faultCode || undefined);
  const pumps: any = isOnline && sensorData?.pump_status ? sensorData.pump_status : {};
  const budgets = (deviceStatus as any)?.budgets || {};

  return (
    <div className="p-4 md:p-8 space-y-6 pb-28 max-w-4xl mx-auto text-slate-200">

      {/* KHỐI TRẠNG THÁI TRUNG TÂM (HERO CARD HƯỚNG NGƯỜI DÙNG CUỐI) */}
      <div className="relative overflow-hidden bg-gradient-to-br from-slate-900 via-slate-900 to-slate-950 border border-slate-800/80 rounded-3xl p-6 md:p-8 shadow-xl backdrop-blur-md flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
        <div className="absolute top-0 right-0 -mt-10 -mr-10 w-40 h-40 bg-emerald-500/5 blur-3xl rounded-full pointer-events-none" />

        <div className="space-y-3 max-w-lg">
          <div className="flex items-center gap-2.5">
            <span className={`w-2.5 h-2.5 rounded-full ${isOnline ? 'bg-emerald-500 shadow-[0_0_10px_#10b981]' : 'bg-rose-500 shadow-[0_0_10px_#f43f5e]'}`} />
            <h1 className="text-xl font-bold tracking-tight text-slate-100">
              {friendlyState.label}
            </h1>
          </div>
          <p className="text-sm text-slate-400 leading-relaxed">
            {friendlyState.description}
          </p>

          {/* Tóm tắt nhanh hoạt động phần cứng */}
          <div className="flex flex-wrap gap-2 pt-2">
            {Object.values(pumps).some(v => v === true) ? (
              Object.entries(pumps).map(([key, isRunning]) => {
                if (!isRunning) return null;
                const labels: Record<string, string> = {
                  pump_a: 'Dinh dưỡng A', pump_b: 'Dinh dưỡng B',
                  ph_up: 'Kiềm pH Up', ph_down: 'Axit pH Down',
                  osaka_pump: 'Trộn tuần hoàn', mist_valve: 'Phun sương',
                  water_pump_in: 'Cấp nước', water_pump_out: 'Xả nước'
                };
                const colors: Record<string, string> = {
                  pump_a: 'bg-orange-500/15 text-orange-400 border-orange-500/20',
                  pump_b: 'bg-orange-500/15 text-orange-400 border-orange-500/20',
                  ph_up: 'bg-purple-500/15 text-purple-400 border-purple-500/20',
                  ph_down: 'bg-fuchsia-500/15 text-fuchsia-400 border-fuchsia-500/20',
                  osaka_pump: 'bg-indigo-500/15 text-indigo-400 border-indigo-500/20',
                  mist_valve: 'bg-sky-500/15 text-sky-400 border-sky-500/20',
                  water_pump_in: 'bg-blue-500/15 text-blue-400 border-blue-500/20',
                  water_pump_out: 'bg-cyan-500/15 text-cyan-400 border-cyan-500/20'
                };
                return <ActiveDeviceTag key={key} label={labels[key] || key} color={colors[key] || 'bg-slate-800 text-slate-300 border-slate-700'} />;
              })
            ) : (
              <span className="text-xs font-semibold bg-slate-950 text-slate-500 px-3 py-1 rounded-full border border-slate-800/80">
                Hệ thống thủy lực đang nghỉ ngơi tĩnh
              </span>
            )}
          </div>
        </div>

        {/* Điểm sức khỏe dạng Widget Trực Quan */}
        <div className="flex flex-col items-center justify-center bg-slate-950/50 border border-slate-800/60 p-4 rounded-2xl w-full md:w-36 text-center shadow-inner self-stretch md:self-auto">
          <span className="text-[10px] text-slate-500 font-bold uppercase tracking-wider mb-1">Sức khỏe Trạm</span>
          <span className={`text-3xl font-black font-mono tracking-tight ${computedHealth.score >= 90 ? 'text-emerald-400' : computedHealth.score >= 60 ? 'text-amber-400' : 'text-rose-400'}`}>
            {computedHealth.score}%
          </span>
          <span className="text-[11px] text-slate-400 mt-1.5 font-medium">{computedHealth.label}</span>
        </div>
      </div>

      {/* KHỐI TIẾT LỘ TIẾN TRÌNH THÍCH ỨNG (PROGRESSIVE DISCLOSURE FOR DIAGNOSTICS) */}
      {isOnline && (
        <div className="bg-slate-900/40 border border-slate-800/60 rounded-2xl overflow-hidden transition-all duration-300">
          <button
            onClick={() => setShowAdvancedDiag(!showAdvancedDiag)}
            className="w-full flex items-center justify-between px-5 py-3.5 text-xs font-bold text-slate-400 hover:bg-slate-900/60 transition-colors"
          >
            <div className="flex items-center gap-2">
              <ShieldCheck size={16} className={computedHealth.score >= 90 ? 'text-emerald-500' : 'text-amber-500'} />
              <span>Xem chi tiết chẩn đoán tự động của Edge AI</span>
            </div>
            {showAdvancedDiag ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
          </button>

          {showAdvancedDiag && (
            <div className="p-5 border-t border-slate-800/60 bg-slate-950/30 space-y-4 animate-fadeIn">
              <p className="text-xs text-slate-400 leading-relaxed -mt-1">
                {computedHealth.description}
              </p>

              {/* Dự báo tiêu dùng chất lưu dựa trên Budgets */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 pt-1">
                <div className="p-4 bg-slate-950/60 border border-slate-800/60 rounded-xl space-y-2">
                  <div className="flex justify-between items-center text-xs">
                    <span className="text-slate-400 font-semibold">Mức sử dụng Dinh dưỡng (1h)</span>
                    <span className="text-slate-300 font-mono font-bold">{Math.round(budgets.ec_ml || 0)}ml / {Number((settings as any)?.max_dose_per_hour || 300)}ml</span>
                  </div>
                  <div className="w-full h-1.5 bg-slate-900 rounded-full overflow-hidden">
                    <div className="h-full bg-blue-500 transition-all duration-500" style={{ width: `${Math.min(100, ((budgets.ec_ml || 0) / Number((settings as any)?.max_dose_per_hour || 300)) * 100)}%` }} />
                  </div>
                </div>

                <div className="p-4 bg-slate-950/60 border border-slate-800/60 rounded-xl space-y-2">
                  <div className="flex justify-between items-center text-xs">
                    <span className="text-slate-400 font-semibold">Mức sử dụng thuốc khử pH (1h)</span>
                    <span className="text-slate-300 font-mono font-bold">{Math.round(budgets.ph_ml || 0)}ml / {Number((settings as any)?.max_dose_per_hour || 300)}ml</span>
                  </div>
                  <div className="w-full h-1.5 bg-slate-900 rounded-full overflow-hidden">
                    <div className="h-full bg-fuchsia-500 transition-all duration-500" style={{ width: `${Math.min(100, ((budgets.ph_ml || 0) / Number((settings as any)?.max_dose_per_hour || 300)) * 100)}%` }} />
                  </div>
                </div>
              </div>

              {/* Thông tin luồng mạng phần cứng */}
              <div className="grid grid-cols-3 gap-3 text-[11px] font-mono text-slate-500 pt-1">
                <div className="flex items-center gap-1.5"><Wifi size={13} /> Sóng RF: <span className="text-slate-300 font-bold">{controllerHealth?.rssi || '--'} dBm</span></div>
                <div className="flex items-center gap-1.5"><Cpu size={13} /> Bộ nhớ trống: <span className="text-slate-300 font-bold">{controllerHealth?.free_heap ? `${(controllerHealth.free_heap / 1024).toFixed(0)} KB` : '--'}</span></div>
                <div className="flex items-center gap-1.5"><Clock size={13} /> Thời gian chạy: <span className="text-slate-300 font-bold">{controllerHealth?.uptime ? `${Math.floor(controllerHealth.uptime / 3600)} giờ` : '--'}</span></div>
              </div>

              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-[11px] font-mono text-slate-500 pt-1">
                <div className="flex items-center gap-1.5"><LineChart size={13} /> Lượt học: <span className="text-slate-300 font-bold">{controllerHealth?.matrix_update_count ?? '--'}</span></div>
                <div className="flex items-center gap-1.5"><ShieldCheck size={13} /> Ma trận: <span className="text-slate-300 font-bold">{controllerHealth?.matrix_is_warm ? 'Ổn định' : 'Đang học'}</span></div>
                <div className="flex items-center gap-1.5"><Activity size={13} /> Tin rơi: <span className="text-slate-300 font-bold">{controllerHealth?.log_drop_count ?? '--'}</span></div>
                <div className="flex items-center gap-1.5"><Zap size={13} /> Tin cậy EC: <span className="text-slate-300 font-bold">{controllerHealth?.kalman_confidence?.nutrient_a != null ? `${Math.round(controllerHealth.kalman_confidence.nutrient_a * 100)}%` : '--'}</span></div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* KHỐI HIỂN THỊ HƯỚNG DẪN XỬ LÝ LỖI (MÃ LỖI BIÊN CHẤP HÀNH) */}
      {faultCode && faultGuide && (
        <div className="bg-rose-500/10 border border-rose-500/20 rounded-2xl p-4 flex gap-3 items-start shadow-md">
          <AlertTriangle className="text-rose-400 shrink-0 mt-0.5" size={18} />
          <div className="space-y-1">
            <h4 className="text-sm font-bold text-rose-400">{faultGuide.short}</h4>
            <p className="text-xs text-slate-300 leading-relaxed">{faultGuide.action}</p>
          </div>
        </div>
      )}

      {/* LƯỚI CẢM BIẾN MÔI TRƯỜNG TRỰC QUAN HÓA (BYPASS GIÁ TRỊ LỖI -1 THÀNH TEXT THÂN THIỆN) */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 flex items-center gap-1.5">
            <LineChart size={14} className="text-slate-500" />
            <span>Thông số bồn chứa thực thời</span>
          </h3>
          {!isSensorOnline && sensorData && (
            <div className="flex items-center gap-1.5 px-2.5 py-1 bg-rose-500/10 border border-rose-500/20 rounded-full animate-pulse">
              <span className="w-1.5 h-1.5 rounded-full bg-rose-500"></span>
              <span className="text-[10px] font-bold text-rose-400 uppercase tracking-wide">Mất tín hiệu đầu dò</span>
            </div>
          )}
        </div>

        <div className={`grid grid-cols-2 md:grid-cols-4 gap-4 transition-all duration-500 ${!isSensorOnline ? 'opacity-50 grayscale pointer-events-none' : ''}`}>

          {/* TRỤC DINH DƯỠNG EC */}
          <div className="relative">
            <SensorBentoCard
              title="Độ Dinh Dưỡng (EC)"
              value={sensorData?.err_ec === true ? "Bảo trì" : sensorData?.ec}
              unit={sensorData?.err_ec === true ? "" : "mS/cm"}
              icon={Activity}
              theme={sensorData?.err_ec === true ? "rose" : "blue"}
            />
          </div>

          {/* TRỤC CÂN BẰNG PH */}
          <div className="relative">
            <SensorBentoCard
              title="Độ Kiềm (pH)"
              value={sensorData?.err_ph === true ? "Cần rửa" : sensorData?.ph}
              unit=""
              icon={Droplets}
              theme={sensorData?.err_ph === true ? "rose" : "fuchsia"}
            />
          </div>

          {/* TRỤC NHIỆT ĐỘ NƯỚC */}
          <div className="relative">
            <SensorBentoCard
              title="Nhiệt Độ Nước"
              value={sensorData?.err_temp === true ? "Lỗi số" : sensorData?.temp}
              unit={sensorData?.err_temp === true ? "" : "°C"}
              icon={Thermometer}
              theme={sensorData?.err_temp === true ? "rose" : "orange"}
            />
          </div>

          {/* TRỤC MỰC NƯỚC BỒN TỰ NẠP */}
          <div className="relative">
            <SensorBentoCard
              title="Lượng Nước Bồn"
              value={sensorData?.err_water === true ? "Kẹt phao" : sensorData?.water_level}
              unit={sensorData?.err_water === true ? "" : "%"}
              icon={Waves}
              theme={sensorData?.err_water === true ? "rose" : "cyan"}
            />
          </div>

        </div>
      </div>

      {/* PANEL CÀI ĐẶT THÔNG BÁO NHANH */}
      {permission !== 'granted' && (
        <div className="bg-slate-900/60 border border-slate-800/80 p-5 rounded-3xl flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div className="space-y-1">
            <h4 className="text-sm font-bold text-slate-200">Nhận cảnh báo khẩn cấp qua điện thoại</h4>
            <p className="text-xs text-slate-400 leading-relaxed">Hệ thống AI sẽ gửi tin nhắn đẩy ngay lập tức nếu bồn chứa hết thuốc hoặc mất nước nguồn.</p>
          </div>
          <button
            onClick={async () => {
              if (!('Notification' in window)) {
                toast.error("Môi trường mạng không an toàn (HTTP) không hỗ trợ nhận tin nhắn đẩy!");
                return;
              }
              try {
                await enableNotifications();
              } catch (err: any) {
                toast.error("Không thể xin quyền thông báo. Vui lòng mở quyền trong cài đặt trình duyệt.");
              }
            }}
            className="px-5 py-2.5 shrink-0 rounded-2xl bg-blue-500 hover:bg-blue-600 text-white text-xs font-bold transition-all duration-200 shadow-md shadow-blue-500/10 active:scale-95"
          >
            Kích hoạt tính năng
          </button>
        </div>
      )}

    </div>
  );
};

export default Dashboard;
