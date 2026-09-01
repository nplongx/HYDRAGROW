import { Settings2, RefreshCw, Sparkles, AlertTriangle, FlaskConical, Activity, Droplets, Power, Wind } from 'lucide-react';

// --- ZUSTAND, GLEAM & HOOKS ---
import { useDeviceStore } from '../store/useDeviceStore';
import { useDeviceControl } from '../hooks/useDeviceControl';
import { extract_fault_code_str } from '../../gleam_core/build/dev/javascript/gleam_core/fsm.mjs';
import { get_fault_guide } from '../../gleam_core/build/dev/javascript/gleam_core/faults.mjs';

// --- UI COMPONENTS ---
import { AdvancedDeviceControl } from '../components/control/AdvancedDeviceControl';
import { ActiveRecipeStatus } from '../components/recipes/ActiveRecipeStatus';
import { LoadingState } from '../components/ui/LoadingState';
import { PumpStatus } from '../types/models';

const ControlPanel = () => {
  const deviceId = useDeviceStore((s) => s.deviceId);
  const sensorData = useDeviceStore((s) => s.sensorData);
  const deviceStatus = useDeviceStore((s) => s.deviceStatus);
  const isControllerStatusKnown = useDeviceStore((s) => s.isControllerStatusKnown);
  const isLoading = useDeviceStore((s) => s.isLoading);
  const fsmState = useDeviceStore((s) => s.fsmState);
  const settings = useDeviceStore((s) => s.settings);

  const { isProcessing, resetFault } = useDeviceControl(deviceId || '');

  if (isLoading) return <LoadingState message="Đang kết nối trung tâm điều khiển..." />;

  if (!sensorData) {
    return <LoadingState message="Không có tín hiệu cảm biến!" />;
  }

  const isOnline = deviceStatus?.is_online || false;
  const showDisconnected = isControllerStatusKnown && !isOnline;
  const pumps: Partial<PumpStatus> = sensorData.pump_status || {};
  const isEmergency = Boolean(fsmState?.toUpperCase().includes('EMERGENCY') || fsmState?.toUpperCase().includes('FAULT'));
  const isAutoMode = settings?.control_mode === 'auto';
  const canSendCommands = Boolean(deviceId && settings?.backend_url);

  const faultCode = extract_fault_code_str(fsmState || '');
  const faultGuideOpt = faultCode ? get_fault_guide(faultCode) : null;
  const faultGuide = faultGuideOpt && (faultGuideOpt as any)[0] ? (faultGuideOpt as any)[0] : null;

  return (
    <div className="app-page max-w-5xl">
      {/* Header khu vực */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h1 className="text-xl font-bold tracking-tight text-emerald-950 flex items-center gap-2">
            <Settings2 size={20} className="text-emerald-700/75" />
            <span>Điều khiển thiết bị</span>
          </h1>
          <p className="text-sm text-emerald-800/75">Bơm, van và hệ thống phun sương khi cần thao tác bằng tay.</p>
        </div>
        <button
          disabled={!canSendCommands || isProcessing}
          onClick={async () => {
            if (window.confirm("Khôi phục trạng thái hoạt động của hệ thống?")) await resetFault();
          }}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-white text-emerald-900 border border-emerald-100 rounded-xl text-xs font-bold hover:bg-emerald-100 transition-all disabled:opacity-50"
        >
          <RefreshCw size={12} className={isProcessing ? "animate-spin" : "text-emerald-700"} />
          <span>Khôi phục</span>
        </button>
      </div>

      {/* Cảnh báo sự cố / Mất kết nối */}
      <div className="space-y-3 mt-3">
        {showDisconnected && (
          <div className="bg-red-50 border border-red-200 rounded-2xl p-4 flex gap-3 text-red-700">
            <AlertTriangle size={18} className="shrink-0 mt-0.5" />
            <div className="space-y-0.5">
              <h4 className="font-bold text-sm">Hệ thống Ngoại tuyến</h4>
              <p className="text-xs opacity-80 leading-relaxed">Không thể truyền lệnh do mất kết nối Wi-Fi.</p>
            </div>
          </div>
        )}
        {isEmergency && isOnline && !isAutoMode && (
          <div className="bg-amber-50 border border-amber-200 rounded-2xl p-4 flex gap-3 text-amber-800">
            <AlertTriangle size={18} className="shrink-0 mt-0.5" />
            <div className="space-y-1">
              <h4 className="font-bold text-sm">Hệ thống đang ngắt khẩn cấp</h4>
              <p className="text-xs opacity-80 leading-relaxed">{faultGuide?.short || 'Phát hiện sự cố an toàn.'}</p>
              {faultGuide && <p className="text-[11px] font-medium bg-emerald-50 px-2 py-1 rounded-lg border border-emerald-100 mt-1 max-w-max">Khắc phục: {faultGuide.action}</p>}
            </div>
          </div>
        )}
      </div>

      <ActiveRecipeStatus />

      {/* Lưới điều khiển Bento Grid */}
      <div className="relative border border-emerald-100 rounded-3xl p-5 md:p-6 bg-white/80 backdrop-blur-sm space-y-6 overflow-hidden shadow-sm shadow-emerald-950/5 mt-4">
        {/* Frosted Glass Overlay khi ở chế độ Tự Động */}
        {isAutoMode && isOnline && (
          <div className="absolute inset-0 z-40 bg-emerald-50/80 backdrop-blur-[4px] flex flex-col items-center justify-center p-6 text-center animate-fadeIn select-none">
            <div className="p-4 bg-emerald-100 border border-emerald-200 rounded-2xl mb-3 shadow-xl shadow-emerald-950/10">
              <Sparkles size={28} className="text-emerald-700 animate-pulse" />
            </div>
            <h4 className="text-base font-bold text-emerald-950 tracking-tight">Trạm đang chạy Tự Động</h4>
            <p className="text-xs text-emerald-800/80 max-w-xs leading-relaxed mt-1">
              Thuật toán MIMO đang quản lý dinh dưỡng và vi chất. Bật chế độ Thủ Công trong Cài Đặt nếu cần can thiệp.
            </p>
          </div>
        )}

        {/* Nhóm 1: Bơm châm hóa chất */}
        <div className="space-y-3">
          <h2 className="farm-section-title">Châm dinh dưỡng và pH</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <AdvancedDeviceControl deviceId={deviceId} pumpId="PUMP_A" title="Bơm phân A" icon={FlaskConical} currentStatus={Boolean(pumps.pump_a)} allowPwm={true} colorTheme="orange" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
            <AdvancedDeviceControl deviceId={deviceId} pumpId="PUMP_B" title="Bơm phân B" icon={FlaskConical} currentStatus={Boolean(pumps.pump_b)} allowPwm={true} colorTheme="orange" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
            <AdvancedDeviceControl deviceId={deviceId} pumpId="PH_UP" title="Bơm pH Up" icon={Activity} currentStatus={Boolean(pumps.ph_up)} allowPwm={true} colorTheme="purple" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
            <AdvancedDeviceControl deviceId={deviceId} pumpId="PH_DOWN" title="Bơm pH Down" icon={Activity} currentStatus={Boolean(pumps.ph_down)} allowPwm={true} colorTheme="fuchsia" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
          </div>
        </div>

        {/* Nhóm 2: Bơm cấp & Xả nước */}
        <div className="space-y-3">
          <h2 className="farm-section-title">Cấp & Xả nước bồn</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <AdvancedDeviceControl deviceId={deviceId} pumpId="WATER_PUMP_IN" title="Van cấp nước" icon={Droplets} currentStatus={Boolean(pumps.water_pump_in)} allowPwm={false} colorTheme="water" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
            <AdvancedDeviceControl deviceId={deviceId} pumpId="WATER_PUMP_OUT" title="Bơm xả thoát" icon={Droplets} currentStatus={Boolean(pumps.water_pump_out)} allowPwm={false} colorTheme="sky" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
          </div>
        </div>

        {/* Nhóm 3: Phun sương & Tuần hoàn */}
        <div className="space-y-3">
          <h2 className="farm-section-title">Phun sương và Tuần hoàn</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <AdvancedDeviceControl deviceId={deviceId} pumpId="OSAKA" title="Bơm tăng áp" icon={Power} currentStatus={Boolean(pumps.osaka_pump)} allowPwm={true} colorTheme="water" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
            <AdvancedDeviceControl deviceId={deviceId} pumpId="MIST" title="Van phun sương" icon={Wind} currentStatus={Boolean(pumps.mist_valve)} allowPwm={false} colorTheme="sky" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
            <AdvancedDeviceControl deviceId={deviceId} pumpId="MIX" title="Van trộn" icon={Wind} currentStatus={Boolean(pumps.mix_valve)} allowPwm={false} colorTheme="sky" canSendCommands={canSendCommands} isEmergency={isEmergency} isAutoMode={isAutoMode} />
          </div>
        </div>
      </div>
    </div>
  );
};

export default ControlPanel;
