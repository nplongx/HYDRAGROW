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
  <span className={`flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold tracking-wide border ${color}`}>
    <Zap size={12} className="fill-current" />
    {label}
  </span>
);

const pumpLabels: Record<string, string> = {
  pump_a: 'Dinh dưỡng A',
  pump_b: 'Dinh dưỡng B',
  ph_up: 'pH Up',
  ph_down: 'pH Down',
  osaka_pump: 'Trộn tuần hoàn',
  mist_valve: 'Phun sương',
  water_pump_in: 'Cấp nước',
  water_pump_out: 'Xả nước'
};

const pumpColors: Record<string, string> = {
  pump_a: 'bg-orange-50 text-orange-700 border-orange-200',
  pump_b: 'bg-orange-50 text-orange-700 border-orange-200',
  ph_up: 'bg-fuchsia-50 text-fuchsia-700 border-fuchsia-200',
  ph_down: 'bg-rose-50 text-rose-700 border-rose-200',
  osaka_pump: 'bg-indigo-50 text-indigo-700 border-indigo-200',
  mist_valve: 'bg-sky-50 text-sky-700 border-sky-200',
  water_pump_in: 'bg-blue-50 text-blue-700 border-blue-200',
  water_pump_out: 'bg-cyan-50 text-cyan-700 border-cyan-200'
};

const formatNumber = (value: any, digits = 1) => {
  const num = Number(value);
  if (!Number.isFinite(num)) return '--';
  return num.toFixed(digits);
};

const sensorStatus = (hasError: boolean | undefined, value: any, min?: any, max?: any) => {
  if (hasError) return { label: 'Cần kiểm tra', tone: 'danger' as const };
  const numericValue = Number(value);
  const numericMin = Number(min);
  const numericMax = Number(max);
  if (Number.isFinite(numericValue) && Number.isFinite(numericMin) && numericValue < numericMin) {
    return { label: 'Thấp', tone: 'warn' as const };
  }
  if (Number.isFinite(numericValue) && Number.isFinite(numericMax) && numericValue > numericMax) {
    return { label: 'Cao', tone: 'warn' as const };
  }
  return { label: 'Ổn định', tone: 'good' as const };
};

const budgetPercent = (used: any, limit: any) => {
  const usedNumber = Number(used || 0);
  const limitNumber = Number(limit || 300);
  if (!Number.isFinite(usedNumber) || !Number.isFinite(limitNumber) || limitNumber <= 0) return 0;
  return Math.min(100, Math.round((usedNumber / limitNumber) * 100));
};

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
        <div className="p-6 bg-white rounded-3xl border border-emerald-100 shadow-xl shadow-emerald-950/10">
          <Settings size={40} className="text-emerald-700" />
        </div>
        <div className="space-y-2 max-w-xs">
          <h2 className="text-xl font-bold text-emerald-950">Chưa thiết lập trạm điều khiển</h2>
          <p className="text-sm text-emerald-800/75 leading-relaxed">
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
  const doseLimit = Number((settings as any)?.max_dose_per_hour || 300);
  const modeLabel = settings?.control_mode === 'auto' ? 'Tự động chăm sóc' : 'Thủ công';
  const ecStatus = sensorStatus(sensorData?.err_ec, sensorData?.ec, (settings as any)?.min_ec_limit, (settings as any)?.max_ec_limit);
  const phStatus = sensorStatus(sensorData?.err_ph, sensorData?.ph, (settings as any)?.min_ph_limit, (settings as any)?.max_ph_limit);
  const tempStatus = sensorStatus(sensorData?.err_temp, sensorData?.temp, (settings as any)?.min_temp_limit, (settings as any)?.max_temp_limit);
  const waterStatus = sensorStatus(sensorData?.err_water, sensorData?.water_level, (settings as any)?.water_level_min, (settings as any)?.water_level_max);
  const nextAction = !isOnline
    ? 'Kiểm tra nguồn điện và Wi-Fi của hộp điều khiển trước khi gửi lệnh.'
    : !isSensorOnline
      ? 'Đầu dò đang mất tín hiệu; số liệu có thể là lần đo cuối. Kiểm tra dây cảm biến và nguồn node cảm biến.'
      : faultGuide?.action || (permission !== 'granted' ? 'Bật thông báo để nhận cảnh báo khi bồn cạn, mất nước hoặc châm lỗi.' : 'Không cần thao tác. Tiếp tục theo dõi vườn.');

  return (
    <div className="app-page">
      <div className="ui-card relative overflow-hidden p-6 md:p-8">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="space-y-4 max-w-2xl">
            <div className="flex flex-wrap items-center gap-2">
              <span className={`farm-status-pill ${isOnline ? 'bg-emerald-50 text-emerald-700 border-emerald-200' : 'bg-red-50 text-red-700 border-red-200'}`}>
                <Wifi size={13} />
                {isOnline ? 'Trạm đang online' : 'Trạm mất kết nối'}
              </span>
              <span className="farm-status-pill bg-sky-50 text-sky-700 border-sky-200">
                <Cpu size={13} />
                {modeLabel}
              </span>
              <span className="farm-status-pill bg-white text-emerald-800 border-emerald-200">
                ID: {deviceId}
              </span>
            </div>
            <div>
              <h1 className="text-2xl md:text-3xl font-bold tracking-tight text-emerald-950">
                {friendlyState.label}
              </h1>
              <p className="text-sm md:text-base text-emerald-800/80 leading-relaxed mt-2">
                {friendlyState.description}
              </p>
            </div>
            <div className={`rounded-2xl border p-4 flex gap-3 items-start ${faultCode || !isOnline || !isSensorOnline ? 'bg-amber-50 border-amber-200' : 'bg-emerald-50 border-emerald-200'}`}>
              <AlertTriangle className={`${faultCode || !isOnline || !isSensorOnline ? 'text-amber-700' : 'text-emerald-700'} shrink-0 mt-0.5`} size={18} />
              <div>
                <h2 className="text-sm font-bold text-emerald-950">Việc nên làm lúc này</h2>
                <p className="text-xs md:text-sm text-emerald-800/80 leading-relaxed mt-1">{nextAction}</p>
              </div>
            </div>
          </div>
          <div className="grid grid-cols-2 gap-3 w-full lg:w-72">
            <div className="rounded-2xl border border-emerald-100 bg-emerald-50 p-4 text-center">
              <span className="text-[10px] text-emerald-800/70 font-bold uppercase tracking-wider">Sức khỏe trạm</span>
              <div className={`text-4xl font-black font-mono tracking-tight mt-1 ${computedHealth.score >= 90 ? 'text-emerald-700' : computedHealth.score >= 60 ? 'text-amber-700' : 'text-red-700'}`}>
                {computedHealth.score}%
              </div>
              <p className="text-xs font-semibold text-emerald-900 mt-1">{computedHealth.label}</p>
            </div>
            <div className="rounded-2xl border border-sky-100 bg-sky-50 p-4 text-center">
              <span className="text-[10px] text-sky-800/70 font-bold uppercase tracking-wider">Đầu dò</span>
              <div className={`text-2xl font-black mt-3 ${isSensorOnline ? 'text-emerald-700' : 'text-red-700'}`}>
                {isSensorOnline ? 'Tốt' : 'Mất'}
              </div>
              <p className="text-xs font-semibold text-sky-900 mt-2">{isSensorOnline ? 'Đang đo' : 'Cần kiểm tra'}</p>
            </div>
          </div>
        </div>
      </div>

      {faultCode && faultGuide && (
        <div className="bg-red-50 border border-red-200 rounded-2xl p-4 flex gap-3 items-start shadow-sm">
          <AlertTriangle className="text-red-700 shrink-0 mt-0.5" size={18} />
          <div className="space-y-1">
            <h4 className="text-sm font-bold text-red-800">{faultGuide.short}</h4>
            <p className="text-xs text-red-800/80 leading-relaxed">{faultGuide.action}</p>
          </div>
        </div>
      )}

      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="farm-section-title">
            <LineChart size={14} />
            <span>Thông số bồn chứa thực thời</span>
          </h3>
          {!isSensorOnline && sensorData && (
            <div className="farm-status-pill bg-red-50 text-red-700 border-red-200">
              <span className="w-1.5 h-1.5 rounded-full bg-red-600"></span>
              <span>Mất tín hiệu đầu dò</span>
            </div>
          )}
        </div>

        <div className={`grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 transition-all duration-500 ${!isSensorOnline ? 'opacity-60 grayscale' : ''}`}>
          <SensorBentoCard
            title="Dinh dưỡng EC"
            value={sensorData?.err_ec === true ? "Bảo trì" : formatNumber(sensorData?.ec, 2)}
            unit={sensorData?.err_ec === true ? "" : "mS/cm"}
            icon={Activity}
            theme={sensorData?.err_ec === true ? "rose" : "blue"}
            statusLabel={ecStatus.label}
            statusTone={ecStatus.tone}
            rangeLabel={`Mục tiêu ${formatNumber((settings as any)?.ec_target, 2)} ± ${formatNumber((settings as any)?.ec_tolerance, 2)}`}
            description={sensorData?.err_ec === true ? 'Cần vệ sinh hoặc hiệu chuẩn đầu dò EC.' : 'Cho biết độ đậm dinh dưỡng trong bồn.'}
          />
          <SensorBentoCard
            title="Cân bằng pH"
            value={sensorData?.err_ph === true ? "Cần rửa" : formatNumber(sensorData?.ph, 2)}
            unit=""
            icon={Droplets}
            theme={sensorData?.err_ph === true ? "rose" : "fuchsia"}
            statusLabel={phStatus.label}
            statusTone={phStatus.tone}
            rangeLabel={`Mục tiêu ${formatNumber((settings as any)?.ph_target, 2)} ± ${formatNumber((settings as any)?.ph_tolerance, 2)}`}
            description={sensorData?.err_ph === true ? 'Đầu dò pH cần rửa hoặc hiệu chuẩn lại.' : 'Giữ vùng rễ hấp thụ dinh dưỡng ổn định.'}
          />
          <SensorBentoCard
            title="Nhiệt độ nước"
            value={sensorData?.err_temp === true ? "Lỗi số" : formatNumber(sensorData?.temp, 1)}
            unit={sensorData?.err_temp === true ? "" : "°C"}
            icon={Thermometer}
            theme={sensorData?.err_temp === true ? "rose" : "orange"}
            statusLabel={tempStatus.label}
            statusTone={tempStatus.tone}
            rangeLabel={`An toàn ${formatNumber((settings as any)?.min_temp_limit, 0)}-${formatNumber((settings as any)?.max_temp_limit, 0)}°C`}
            description="Theo dõi nhiệt trong bồn để tránh sốc rễ."
          />
          <SensorBentoCard
            title="Mực nước bồn"
            value={sensorData?.err_water === true ? "Kẹt phao" : formatNumber(sensorData?.water_level, 0)}
            unit={sensorData?.err_water === true ? "" : "%"}
            icon={Waves}
            theme={sensorData?.err_water === true ? "rose" : "cyan"}
            statusLabel={waterStatus.label}
            statusTone={waterStatus.tone}
            rangeLabel={`Giữ quanh ${formatNumber((settings as any)?.water_level_target, 0)}%`}
            description={sensorData?.err_water === true ? 'Kiểm tra cảm biến mực nước.' : 'Đảm bảo bơm không chạy khô.'}
          />
        </div>
      </div>

      <div className="ui-card space-y-3">
        <h3 className="farm-section-title"><Zap size={14} /> Thiết bị đang chạy</h3>
        <div className="flex flex-wrap gap-2">
          {Object.values(pumps).some(v => v === true) ? (
            Object.entries(pumps).map(([key, isRunning]) => {
              if (!isRunning) return null;
              return <ActiveDeviceTag key={key} label={pumpLabels[key] || key} color={pumpColors[key] || 'bg-emerald-50 text-emerald-700 border-emerald-200'} />;
            })
          ) : (
            <span className="farm-status-pill bg-emerald-50 text-emerald-700 border-emerald-200">
              Không có bơm hoặc van đang chạy
            </span>
          )}
        </div>
      </div>

      {isOnline && (
        <div className="ui-card overflow-hidden p-0">
          <button
            onClick={() => setShowAdvancedDiag(!showAdvancedDiag)}
            className="w-full flex items-center justify-between px-5 py-4 text-xs font-bold text-emerald-800 hover:bg-emerald-50 transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-emerald-500/25"
          >
            <div className="flex items-center gap-2">
              <ShieldCheck size={16} className={computedHealth.score >= 90 ? 'text-emerald-700' : 'text-amber-700'} />
              <span>Chi tiết chẩn đoán kỹ thuật</span>
            </div>
            {showAdvancedDiag ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
          </button>

          {showAdvancedDiag && (
            <div className="p-5 border-t border-emerald-100 bg-emerald-50/40 space-y-4">
              <p className="text-xs text-emerald-800/80 leading-relaxed">
                {computedHealth.description}
              </p>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {[
                  ['Dinh dưỡng dùng trong 1 giờ', budgets.ec_ml, 'bg-blue-600'],
                  ['Dung dịch pH dùng trong 1 giờ', budgets.ph_ml, 'bg-fuchsia-600']
                ].map(([label, used, barClass]) => {
                  const percent = budgetPercent(used, doseLimit);
                  return (
                    <div key={String(label)} className="p-4 bg-white border border-emerald-100 rounded-xl space-y-2">
                      <div className="flex justify-between items-center text-xs gap-3">
                        <span className="text-emerald-800 font-semibold">{label}</span>
                        <span className="text-emerald-950 font-mono font-bold">{Math.round(Number(used || 0))}ml / {doseLimit}ml</span>
                      </div>
                      <div className="w-full h-2 bg-emerald-100 rounded-full overflow-hidden">
                        <div className={`h-full transition-all duration-500 ${barClass}`} style={{ width: `${percent}%` }} />
                      </div>
                    </div>
                  );
                })}
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-[11px] font-mono text-emerald-800">
                <div className="flex items-center gap-1.5"><Wifi size={13} /> Sóng RF: <span className="text-emerald-950 font-bold">{controllerHealth?.rssi || '--'} dBm</span></div>
                <div className="flex items-center gap-1.5"><Cpu size={13} /> RAM trống: <span className="text-emerald-950 font-bold">{controllerHealth?.free_heap ? `${(controllerHealth.free_heap / 1024).toFixed(0)} KB` : '--'}</span></div>
                <div className="flex items-center gap-1.5"><Clock size={13} /> Chạy: <span className="text-emerald-950 font-bold">{controllerHealth?.uptime ? `${Math.floor(controllerHealth.uptime / 3600)} giờ` : '--'}</span></div>
                <div className="flex items-center gap-1.5"><LineChart size={13} /> Lượt học: <span className="text-emerald-950 font-bold">{controllerHealth?.matrix_update_count ?? '--'}</span></div>
                <div className="flex items-center gap-1.5"><ShieldCheck size={13} /> Ma trận: <span className="text-emerald-950 font-bold">{controllerHealth?.matrix_is_warm ? 'Ổn định' : 'Đang học'}</span></div>
                <div className="flex items-center gap-1.5"><Activity size={13} /> Tin rơi: <span className="text-emerald-950 font-bold">{controllerHealth?.log_drop_count ?? '--'}</span></div>
              </div>
            </div>
          )}
        </div>
      )}

      {permission !== 'granted' && (
        <div className="bg-amber-50 border border-amber-200 p-5 rounded-3xl flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div className="space-y-1">
            <h4 className="text-sm font-bold text-amber-950">Nhận cảnh báo khẩn cấp qua điện thoại</h4>
            <p className="text-xs text-amber-900/80 leading-relaxed">Hệ thống sẽ gửi tin nhắn đẩy nếu bồn chứa hết thuốc, mất nước nguồn hoặc trạm gặp lỗi.</p>
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
            className="ui-btn-md shrink-0 rounded-2xl bg-amber-600 hover:bg-amber-700 text-white text-xs"
          >
            Kích hoạt tính năng
          </button>
        </div>
      )}

    </div>
  );
};

export default Dashboard;
