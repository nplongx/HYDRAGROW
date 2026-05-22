import { useState, useEffect, useRef } from 'react';
import {
  Settings2, Droplets, Wind, Power, AlertTriangle, Timer, Activity, RefreshCw,
  Lock, ChevronDown, FlaskConical, Sparkles, ShieldAlert
} from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';
import { useDeviceControl } from '../hooks/useDeviceControl';
import { PumpStatus } from '../types/models';
import toast from 'react-hot-toast';
import { LoadingState } from '../components/ui/LoadingState';
import { Switch } from '../components/ui/Switch';
import { extractFaultCode } from '../components/ui/FsmStatusBadge';
import { getFaultGuide } from '../components/ui/FaultExplanation';

// ─── COMPONENT CARD THIẾT BỊ NGOẠI VI SMART HOME CẤP CAO ──────────────────────
const AdvancedDeviceControl = ({
  deviceId, pumpId, title, icon: Icon, currentStatus, allowPwm = false, isOnline, isEmergency, isAutoMode, colorTheme
}: any) => {
  const { togglePump, setPwm, forceOn } = useDeviceControl(deviceId);
  const { pwmPreferences, savePwmPreference } = useDeviceContext();

  const [pwmValue, setPwmValue] = useState(pwmPreferences[pumpId] || 100);
  const [duration, setDuration] = useState<number | ''>('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [isToggling, setIsToggling] = useState(false);
  const pendingTargetRef = useRef<boolean | null>(null);

  const isLocked = isAutoMode || (isEmergency && !currentStatus);

  // Khởi tạo các mã màu sắc phát quang (Glow effects) theo danh mục thiết bị
  const themeClasses: Record<string, { activeIcon: string; glow: string; border: string }> = {
    orange: { activeIcon: 'bg-orange-500 text-white shadow-orange-500/30', glow: 'shadow-[0_0_15px_rgba(249,115,22,0.05)] border-orange-500/40 bg-orange-950/5', border: 'border-orange-500/40' },
    fuchsia: { activeIcon: 'bg-fuchsia-500 text-white shadow-fuchsia-500/30', glow: 'shadow-[0_0_15px_rgba(217,70,239,0.05)] border-fuchsia-500/40 bg-fuchsia-950/5', border: 'border-fuchsia-500/40' },
    blue: { activeIcon: 'bg-blue-500 text-white shadow-blue-500/30', glow: 'shadow-[0_0_15px_rgba(59,130,246,0.05)] border-blue-500/40 bg-blue-950/5', border: 'border-blue-500/40' },
    indigo: { activeIcon: 'bg-indigo-500 text-white shadow-indigo-500/30', glow: 'shadow-[0_0_15px_rgba(99,102,241,0.05)] border-indigo-500/40 bg-indigo-950/5', border: 'border-indigo-500/40' },
    sky: { activeIcon: 'bg-sky-500 text-white shadow-sky-500/30', glow: 'shadow-[0_0_15px_rgba(14,165,233,0.05)] border-sky-500/40 bg-sky-950/5', border: 'border-sky-500/40' },
  };
  const activeTheme = themeClasses[colorTheme] || themeClasses.blue;

  useEffect(() => {
    if (pwmPreferences[pumpId] !== undefined) setPwmValue(pwmPreferences[pumpId]);
  }, [pwmPreferences, pumpId]);

  useEffect(() => {
    if (pendingTargetRef.current === null) return;
    if (currentStatus === pendingTargetRef.current) {
      setIsToggling(false);
      pendingTargetRef.current = null;
    }
  }, [currentStatus]);

  const handleToggle = async () => {
    if (isAutoMode) {
      toast.error("Hệ thống đang tự trị thông minh, không thể can thiệp thủ công");
      return;
    }
    if (isEmergency && !currentStatus) {
      toast.error(`Hệ thống đang khóa bảo vệ. Vui lòng mở rộng mục nâng cao để Ép Chạy.`);
      setShowAdvanced(true);
      return;
    }
    if (isToggling) return;

    setIsToggling(true);
    setDuration('');
    const targetAction = currentStatus ? 'off' : 'on';
    try {
      pendingTargetRef.current = targetAction === 'on';
      const success = await togglePump(pumpId, targetAction);
      if (!success) {
        pendingTargetRef.current = null;
        setIsToggling(false);
      }
    } catch (error) {
      pendingTargetRef.current = null;
      setIsToggling(false);
      toast.error(`Lỗi đường truyền tín hiệu tới thiết bị.`);
    }
  };

  const handleAdvancedRun = async () => {
    setIsProcessing(true);
    const time = Number(duration);
    try {
      if (allowPwm) {
        await setPwm(pumpId, pwmValue, time > 0 ? time : undefined);
        savePwmPreference(pumpId, pwmValue);
        toast.success(`Đã đồng bộ công suất ${pwmValue}% lên thiết bị.`);
      } else if (time > 0) {
        await forceOn(pumpId, time);
      } else {
        await togglePump(pumpId, 'on');
      }
    } catch (error) {
      toast.error(`Không thể lưu cấu hình.`);
    } finally {
      setIsProcessing(false);
      if (time > 0) setDuration('');
    }
  };

  const handleEmergencyForceOn = async () => {
    const time = Number(duration);
    if (!time || time <= 0) {
      toast.error("Vui lòng nhập số giây muốn kích hoạt cưỡng chế.");
      return;
    }
    if (!window.confirm(`CẢNH BÁO NGUY HIỂM: Bạn đang chuẩn bị ép chạy thiết bị và bỏ qua tất cả rào chắn bảo vệ của AI. Xác nhận kích hoạt?`)) return;

    setIsProcessing(true);
    try {
      await forceOn(pumpId, time);
    } catch (error) {
      toast.error(`Lỗi thực thi lệnh cưỡng chế.`);
    } finally {
      setIsProcessing(false);
      setDuration('');
    }
  };

  return (
    <div className={`border rounded-2xl overflow-hidden transition-all duration-300 shadow-sm ${currentStatus ? activeTheme.glow : 'border-slate-800/80 bg-slate-900/40'}`}>
      <div className="p-4 flex flex-col gap-3.5">
        {/* Hàng điều khiển chính */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`p-2 rounded-xl transition-all duration-300 shadow-md ${currentStatus ? activeTheme.activeIcon : 'bg-slate-950 text-slate-500 border border-slate-800/60'}`}>
              <Icon size={16} />
            </div>
            <div>
              <h3 className={`text-xs font-bold ${currentStatus ? 'text-slate-100' : 'text-slate-300'}`}>{title}</h3>
              <p className="text-[10px] text-slate-500 font-semibold tracking-wide">{currentStatus ? 'Đang hoạt động' : 'Tạm dừng'}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {isLocked && !currentStatus && <Lock size={12} className="text-slate-600 mr-0.5" />}
            <Switch
              isOn={currentStatus}
              disabled={!isOnline || isToggling || isProcessing || isLocked}
              onClick={handleToggle}
              colorClass={currentStatus ? (pumpId.startsWith('PH') ? 'bg-fuchsia-500' : 'bg-blue-500') : 'bg-slate-700'}
            />
          </div>
        </div>

        {/* Ngăn chứa bảng thông số ẩn (Progressive Disclosure) */}
        <div className="border-t border-slate-800/60 pt-2.5">
          <button
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="flex items-center gap-1 text-[10px] font-bold uppercase tracking-wider text-slate-500 hover:text-slate-400 transition-colors"
          >
            <ChevronDown size={12} className={`transition-transform duration-200 ${showAdvanced ? 'rotate-180' : ''}`} />
            <span>{isEmergency ? 'Thiết lập khẩn cấp' : 'Tùy chỉnh kỹ thuật'}</span>
          </button>

          {showAdvanced && (
            <div className="mt-3 animate-in slide-in-from-top-2 duration-200 bg-slate-950/60 p-3 rounded-xl border border-slate-800/80 space-y-3.5">
              {isEmergency ? (
                <div className="space-y-2">
                  <div className="flex items-center gap-1.5 text-rose-400 text-[10px] font-bold uppercase tracking-wide">
                    <ShieldAlert size={12} />
                    <span>Cưỡng chế phần cứng (Bỏ qua AI)</span>
                  </div>
                  <div className="flex gap-2">
                    <input
                      type="number" placeholder="Nhập số giây muốn ép chạy..."
                      value={duration} onChange={(e) => setDuration(e.target.value === '' ? '' : Number(e.target.value))}
                      disabled={isProcessing}
                      className="flex-1 bg-slate-900 border border-rose-950 text-slate-200 text-xs rounded-xl px-3 py-1.5 outline-none focus:border-rose-500 placeholder:text-slate-600 font-medium"
                    />
                    <button
                      onClick={handleEmergencyForceOn} disabled={isProcessing || !duration}
                      className="px-3.5 py-1.5 bg-rose-500/10 text-rose-400 border border-rose-500/20 text-xs font-bold rounded-xl hover:bg-rose-500 hover:text-white transition-all whitespace-nowrap active:scale-95"
                    >
                      Kích hoạt
                    </button>
                  </div>
                </div>
              ) : (
                <div className="space-y-3.5">
                  {allowPwm && (
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-[10px] text-slate-400 font-bold uppercase tracking-wider">
                        <span>Cường độ dòng chảy (PWM)</span>
                        <span className="text-blue-400 font-mono font-bold">{pwmValue}%</span>
                      </div>
                      <input
                        type="range" min="20" max="100" step="5"
                        value={pwmValue} onChange={(e) => setPwmValue(parseInt(e.target.value))}
                        disabled={isProcessing}
                        className="w-full h-1 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-blue-500"
                      />
                    </div>
                  )}

                  <div className="space-y-1.5">
                    <div className="flex justify-between text-[10px] text-slate-400 font-bold uppercase tracking-wider">
                      <span>Thời gian hẹn giờ (Giây)</span>
                    </div>
                    <div className="flex gap-2">
                      <div className="relative flex-1">
                        <input
                          type="number" placeholder="Để trống để mở vô hạn..."
                          value={duration} onChange={(e) => setDuration(e.target.value === '' ? '' : Number(e.target.value))}
                          disabled={isProcessing}
                          className="w-full bg-slate-900 border border-slate-800 text-slate-200 text-xs rounded-xl pl-8 pr-3 py-1.5 outline-none focus:border-blue-500 placeholder:text-slate-600 font-medium"
                        />
                        <Timer size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-500" />
                      </div>
                      <button
                        onClick={handleAdvancedRun} disabled={isProcessing}
                        className="px-4 py-1.5 bg-slate-800 hover:bg-slate-700 text-white text-xs font-bold rounded-xl border border-slate-700 transition-all whitespace-nowrap active:scale-95"
                      >
                        Áp dụng
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

// ─── BẢNG ĐIỀU KHIỂN ĐỒ HỌA TRANG CHÍNH ───────────────────────────────────────
const ControlPanel = () => {
  const { deviceId, sensorData, deviceStatus, isControllerStatusKnown, isLoading, fsmState, settings } = useDeviceContext();
  const { isProcessing, resetFault } = useDeviceControl(deviceId || "");

  if (isLoading || !sensorData) return <LoadingState message="Đang kết nối trung tâm điều khiển phần cứng..." />;

  const isOnline = deviceStatus?.is_online || false;
  const showDisconnected = isControllerStatusKnown && !isOnline;
  const pumps: Partial<PumpStatus> = isOnline ? (sensorData.pump_status || {}) : {};

  const isEmergency = Boolean(
    fsmState?.toUpperCase().includes('EMERGENCY') ||
    fsmState?.toUpperCase().includes('FAULT') ||
    fsmState?.toUpperCase().includes('LỖI')
  );

  const isAutoMode = settings?.control_mode === 'auto';
  const faultCode = extractFaultCode(fsmState || undefined);
  const faultGuide = getFaultGuide(faultCode || undefined);

  return (
    <div className="p-4 md:p-8 max-w-4xl mx-auto space-y-6 pb-28 text-slate-200">

      {/* Header khu vực */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h1 className="text-xl font-bold tracking-tight text-slate-100 flex items-center gap-2">
            <Settings2 size={20} className="text-slate-500" />
            <span>Bảng vận hành thiết bị</span>
          </h1>
        </div>

        <button
          disabled={!isOnline || isProcessing}
          onClick={async () => {
            if (window.confirm("Bác nông dân có chắc chắn muốn xóa lịch sử cảnh báo lỗi và khôi phục chu trình chạy tự động không?")) await resetFault();
          }}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-900 text-slate-300 border border-slate-800/80 rounded-xl text-xs font-bold hover:bg-slate-800 transition-all active:scale-95 disabled:opacity-40"
        >
          <RefreshCw size={12} className={isProcessing ? "animate-spin" : "text-emerald-400"} />
          <span>Khôi phục lỗi</span>
        </button>
      </div>

      {/* THÔNG BÁO SỰ CỐ / THỜI TIẾT TỔNG HỢP */}
      <div className="space-y-3">
        {showDisconnected && (
          <div className="bg-rose-500/10 border border-rose-500/20 rounded-2xl p-4 flex gap-3 text-rose-400 animate-fadeIn">
            <AlertTriangle size={18} className="shrink-0 mt-0.5" />
            <div className="space-y-0.5">
              <h4 className="font-bold text-sm">Hệ thống đang Ngoại tuyến</h4>
              <p className="text-xs opacity-80 leading-relaxed">Không thể truyền phát lệnh điều khiển do mất tín hiệu Wifi của hộp tổng phần cứng.</p>
            </div>
          </div>
        )}

        {isEmergency && isOnline && !isAutoMode && (
          <div className="bg-amber-500/10 border border-amber-500/20 rounded-2xl p-4 flex gap-3 text-amber-400 animate-fadeIn">
            <AlertTriangle size={18} className="shrink-0 mt-0.5" />
            <div className="space-y-1">
              <h4 className="font-bold text-sm">Hệ thống đang tự khóa bảo vệ</h4>
              <p className="text-xs opacity-80 leading-relaxed">{faultGuide?.short || 'Phím bật thông thường đã tạm dừng để cứu bồn. Hãy mở rộng từng ô thiết bị và dùng nút "Ép chạy" nếu cần cứu cây gấp.'}</p>
              {faultGuide && <p className="text-[11px] opacity-70 font-medium bg-slate-950/40 px-2 py-1 rounded-lg border border-slate-800/40 mt-1 max-w-max">Chỉ dẫn: {faultGuide.action}</p>}
            </div>
          </div>
        )}
      </div>

      {/* LƯỚI GRID ĐIỀU KHIỂN PHẦN CỨNG THỦ CÔNG */}
      <div className="relative border border-slate-800/60 rounded-3xl p-5 md:p-6 bg-slate-900/10 backdrop-blur-sm space-y-6 overflow-hidden">

        {/* 🌟 LỚP KÍNH FROSTED GLASS OVERLAY KHI Ở CHẾ ĐỘ AUTO (AI IS MANAGING) */}
        {isAutoMode && isOnline && (
          <div className="absolute inset-0 z-40 bg-slate-950/60 backdrop-blur-[4px] flex flex-col items-center justify-center p-6 text-center animate-fadeIn select-none">
            <div className="p-4 bg-blue-500/10 border border-blue-500/20 rounded-2xl mb-3 shadow-xl">
              <Sparkles size={28} className="text-blue-400 animate-pulse" />
            </div>
            <h4 className="text-base font-bold text-slate-100 tracking-tight">Trí tuệ nhân tạo đang quản lý bồn</h4>
            <p className="text-xs text-slate-400 max-w-xs leading-relaxed mt-1">
              Hệ thống tự động MIMO đang điều tiết dinh dưỡng và vi khí hậu. Bảng điều khiển thủ công được khóa để chống nhấn nhầm làm sốc rễ cây.
            </p>
          </div>
        )}

        {/* Nhóm thiết bị 1: Châm hóa chất */}
        <div className="space-y-3">
          <h2 className="text-[10px] font-bold text-slate-500 uppercase tracking-wider pl-1">Mạch định lượng dinh dưỡng & pH</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <AdvancedDeviceControl
              deviceId={deviceId} pumpId="PUMP_A" title="Bơm vi chất phân A" icon={FlaskConical}
              currentStatus={pumps.pump_a} allowPwm={true} colorTheme="orange"
              isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
            />
            <AdvancedDeviceControl
              deviceId={deviceId} pumpId="PUMP_B" title="Bơm vi chất phân B" icon={FlaskConical}
              currentStatus={pumps.pump_b} allowPwm={true} colorTheme="orange"
              isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
            />
            <AdvancedDeviceControl
              deviceId={deviceId} pumpId="PH_UP" title="Bơm trung hòa kiềm (pH Up)" icon={Activity}
              currentStatus={pumps.ph_up} allowPwm={true} colorTheme="purple"
              isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
            />
            <AdvancedDeviceControl
              deviceId={deviceId} pumpId="PH_DOWN" title="Bơm cân bằng axit (pH Down)" icon={Activity}
              currentStatus={pumps.ph_down} allowPwm={true} colorTheme="fuchsia"
              isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
            />
          </div>
        </div>

        {/* Nhóm thiết bị 2: Thủy lực nước */}
        <div className="space-y-3">
          <h2 className="text-[10px] font-bold text-slate-500 uppercase tracking-wider pl-1">Hệ thống thủy lực nguồn nước</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <AdvancedDeviceControl
              deviceId={deviceId} pumpId="WATER_PUMP_IN" title="Van mở cấp nước sạch" icon={Droplets}
              currentStatus={pumps.water_pump_in} allowPwm={false} colorTheme="blue"
              isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
            />
            <AdvancedDeviceControl
              deviceId={deviceId} pumpId="WATER_PUMP_OUT" title="Bơm kích xả thoát nước" icon={Droplets}
              currentStatus={pumps.water_pump_out} allowPwm={false} colorTheme="sky"
              isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
            />
          </div>
        </div>

        {/* Nhóm thiết bị 3: Khí hậu và hòa trộn */}
        <div className="space-y-3">
          <h2 className="text-[10px] font-bold text-slate-500 uppercase tracking-wider pl-1">Hệ thống điều hòa và trộn đều</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <AdvancedDeviceControl
              deviceId={deviceId} pumpId="OSAKA" title="Mô-tơ đảo nước tuần hoàn" icon={Power}
              currentStatus={pumps.osaka_pump} allowPwm={true} colorTheme="indigo"
              isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
            />
            <AdvancedDeviceControl
              deviceId={deviceId} pumpId="MIST" title="Hệ thống béc phun sương lá" icon={Wind}
              currentStatus={pumps.mist_valve} allowPwm={false} colorTheme="sky"
              isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
            />
          </div>
        </div>

      </div>
    </div>
  );
};

export default ControlPanel;
