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

  const themeClasses: Record<string, { activeIcon: string; glow: string; border: string }> = {
    orange: { activeIcon: 'bg-orange-600 text-white', glow: 'border-orange-200 bg-orange-50', border: 'border-orange-300' },
    fuchsia: { activeIcon: 'bg-fuchsia-600 text-white', glow: 'border-fuchsia-200 bg-fuchsia-50', border: 'border-fuchsia-300' },
    blue: { activeIcon: 'bg-blue-600 text-white', glow: 'border-blue-200 bg-blue-50', border: 'border-blue-300' },
    indigo: { activeIcon: 'bg-indigo-600 text-white', glow: 'border-indigo-200 bg-indigo-50', border: 'border-indigo-300' },
    sky: { activeIcon: 'bg-sky-600 text-white', glow: 'border-sky-200 bg-sky-50', border: 'border-sky-300' },
  };
  const activeTheme = themeClasses[colorTheme] || themeClasses.blue;
  const disabledReason = !isOnline
    ? 'Trạm đang mất kết nối'
    : isToggling || isProcessing
      ? 'Đang gửi lệnh tới thiết bị'
      : isAutoMode
        ? 'Đang ở chế độ tự động để tránh nhấn nhầm'
        : isEmergency && !currentStatus
          ? 'Đang khóa bảo vệ do lỗi an toàn'
          : '';

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
    <div className={`border rounded-2xl overflow-hidden transition-all duration-300 shadow-sm shadow-emerald-950/5 ${currentStatus ? activeTheme.glow : 'border-emerald-100 bg-white'}`}>
      <div className="p-4 flex flex-col gap-3.5">
        {/* Hàng điều khiển chính */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`p-2 rounded-xl transition-all duration-300 shadow-md ${currentStatus ? activeTheme.activeIcon : 'bg-white text-emerald-700/75 border border-emerald-100'}`}>
              <Icon size={16} />
            </div>
            <div>
              <h3 className={`text-xs font-bold ${currentStatus ? 'text-emerald-950' : 'text-emerald-900'}`}>{title}</h3>
              <p className="text-[10px] text-emerald-700/75 font-semibold tracking-wide">{currentStatus ? 'Đang hoạt động' : disabledReason || 'Tạm dừng'}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {isLocked && !currentStatus && <Lock size={12} className="text-emerald-700/60 mr-0.5" />}
            <Switch
              isOn={currentStatus}
              disabled={!isOnline || isToggling || isProcessing || isLocked}
              onClick={handleToggle}
              colorClass={currentStatus ? (pumpId.startsWith('PH') ? 'bg-fuchsia-600' : 'bg-emerald-600') : 'bg-emerald-200'}
            />
          </div>
        </div>

        {/* Ngăn chứa bảng thông số ẩn (Progressive Disclosure) */}
        <div className="border-t border-emerald-100 pt-2.5">
          <button
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="flex items-center gap-1 text-[10px] font-bold uppercase tracking-wider text-emerald-700/75 hover:text-emerald-900 transition-colors cursor-pointer focus:outline-none focus:ring-2 focus:ring-emerald-500/25 rounded-md"
          >
            <ChevronDown size={12} className={`transition-transform duration-200 ${showAdvanced ? 'rotate-180' : ''}`} />
            <span>{isEmergency ? 'Thiết lập khẩn cấp' : 'Tùy chỉnh kỹ thuật'}</span>
          </button>

          {showAdvanced && (
            <div className="mt-3 animate-in slide-in-from-top-2 duration-200 bg-emerald-50/80 p-3 rounded-xl border border-emerald-100 space-y-3.5">
              {isEmergency ? (
                <div className="space-y-2">
                  <div className="flex items-center gap-1.5 text-red-700 text-[10px] font-bold uppercase tracking-wide">
                    <ShieldAlert size={12} />
                    <span>Cưỡng chế phần cứng (Bỏ qua AI)</span>
                  </div>
                  <div className="flex gap-2">
                    <input
                      type="number" placeholder="Nhập số giây muốn ép chạy..."
                      value={duration} onChange={(e) => setDuration(e.target.value === '' ? '' : Number(e.target.value))}
                      disabled={isProcessing}
                      className="flex-1 bg-white border border-red-200 text-emerald-950 text-xs rounded-xl px-3 py-1.5 outline-none focus:border-red-500 focus:ring-2 focus:ring-red-500/20 placeholder:text-emerald-700/60 font-medium"
                    />
                    <button
                      onClick={handleEmergencyForceOn} disabled={isProcessing || !duration}
                      className="px-3.5 py-1.5 bg-red-50 text-red-700 border border-red-200 text-xs font-bold rounded-xl hover:bg-red-600 hover:text-white transition-all whitespace-nowrap disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      Kích hoạt
                    </button>
                  </div>
                </div>
              ) : (
                <div className="space-y-3.5">
                  {allowPwm && (
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-[10px] text-emerald-800/80 font-bold uppercase tracking-wider">
                        <span>Cường độ dòng chảy (PWM)</span>
                        <span className="text-emerald-700 font-mono font-bold">{pwmValue}%</span>
                      </div>
                      <input
                        type="range" min="20" max="100" step="5"
                        value={pwmValue} onChange={(e) => setPwmValue(parseInt(e.target.value))}
                        disabled={isProcessing}
                        className="w-full h-1 bg-emerald-100 rounded-lg appearance-none cursor-pointer accent-emerald-600"
                      />
                    </div>
                  )}

                  <div className="space-y-1.5">
                    <div className="flex justify-between text-[10px] text-emerald-800/80 font-bold uppercase tracking-wider">
                      <span>Thời gian hẹn giờ (Giây)</span>
                    </div>
                    <div className="flex gap-2">
                      <div className="relative flex-1">
                        <input
                          type="number" placeholder="Để trống để mở vô hạn..."
                          value={duration} onChange={(e) => setDuration(e.target.value === '' ? '' : Number(e.target.value))}
                          disabled={isProcessing}
                          className="w-full bg-white border border-emerald-100 text-emerald-950 text-xs rounded-xl pl-8 pr-3 py-1.5 outline-none focus:border-emerald-600 placeholder:text-emerald-700/60 font-medium"
                        />
                        <Timer size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-emerald-700/75" />
                      </div>
                      <button
                        onClick={handleAdvancedRun} disabled={isProcessing}
                        className="px-4 py-1.5 bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold rounded-xl border border-emerald-600 transition-all whitespace-nowrap disabled:opacity-50 disabled:cursor-not-allowed"
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
    <div className="app-page max-w-5xl">

      {/* Header khu vực */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h1 className="text-xl font-bold tracking-tight text-emerald-950 flex items-center gap-2">
            <Settings2 size={20} className="text-emerald-700/75" />
            <span>Điều khiển thiết bị khí canh</span>
          </h1>
          <p className="text-sm text-emerald-800/75">Bật tắt bơm, van và phun sương khi cần thao tác thủ công.</p>
        </div>

        <button
          disabled={!isOnline || isProcessing}
          onClick={async () => {
            if (window.confirm("Bác nông dân có chắc chắn muốn xóa lịch sử cảnh báo lỗi và khôi phục chu trình chạy tự động không?")) await resetFault();
          }}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-white text-emerald-900 border border-emerald-100 rounded-xl text-xs font-bold hover:bg-emerald-100 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <RefreshCw size={12} className={isProcessing ? "animate-spin" : "text-emerald-700"} />
          <span>Khôi phục lỗi</span>
        </button>
      </div>

      {/* THÔNG BÁO SỰ CỐ / THỜI TIẾT TỔNG HỢP */}
      <div className="space-y-3">
        {showDisconnected && (
          <div className="bg-red-50 border border-red-200 rounded-2xl p-4 flex gap-3 text-red-700">
            <AlertTriangle size={18} className="shrink-0 mt-0.5" />
            <div className="space-y-0.5">
              <h4 className="font-bold text-sm">Hệ thống đang Ngoại tuyến</h4>
              <p className="text-xs opacity-80 leading-relaxed">Không thể truyền phát lệnh điều khiển do mất tín hiệu Wifi của hộp tổng phần cứng.</p>
            </div>
          </div>
        )}

        {isEmergency && isOnline && !isAutoMode && (
          <div className="bg-amber-50 border border-amber-200 rounded-2xl p-4 flex gap-3 text-amber-800">
            <AlertTriangle size={18} className="shrink-0 mt-0.5" />
            <div className="space-y-1">
              <h4 className="font-bold text-sm">Hệ thống đang tự khóa bảo vệ</h4>
              <p className="text-xs opacity-80 leading-relaxed">{faultGuide?.short || 'Phím bật thông thường đã tạm dừng để cứu bồn. Hãy mở rộng từng ô thiết bị và dùng nút "Ép chạy" nếu cần cứu cây gấp.'}</p>
              {faultGuide && <p className="text-[11px] opacity-70 font-medium bg-emerald-50/70 px-2 py-1 rounded-lg border border-emerald-100 mt-1 max-w-max">Chỉ dẫn: {faultGuide.action}</p>}
            </div>
          </div>
        )}
      </div>

      {/* LƯỚI GRID ĐIỀU KHIỂN PHẦN CỨNG THỦ CÔNG */}
      <div className="relative border border-emerald-100 rounded-3xl p-5 md:p-6 bg-white/80 backdrop-blur-sm space-y-6 overflow-hidden shadow-sm shadow-emerald-950/5">

        {/* 🌟 LỚP KÍNH FROSTED GLASS OVERLAY KHI Ở CHẾ ĐỘ AUTO (AI IS MANAGING) */}
        {isAutoMode && isOnline && (
          <div className="absolute inset-0 z-40 bg-emerald-50/80 backdrop-blur-[4px] flex flex-col items-center justify-center p-6 text-center animate-fadeIn select-none">
            <div className="p-4 bg-emerald-100 border border-emerald-200 rounded-2xl mb-3 shadow-xl shadow-emerald-950/10">
              <Sparkles size={28} className="text-emerald-700 animate-pulse" />
            </div>
            <h4 className="text-base font-bold text-emerald-950 tracking-tight">Trí tuệ nhân tạo đang quản lý bồn</h4>
            <p className="text-xs text-emerald-800/80 max-w-xs leading-relaxed mt-1">
              Hệ thống tự động MIMO đang điều tiết dinh dưỡng và vi khí hậu. Bảng điều khiển thủ công được khóa để chống nhấn nhầm làm sốc rễ cây.
            </p>
          </div>
        )}

        {/* Nhóm thiết bị 1: Châm hóa chất */}
        <div className="space-y-3">
          <h2 className="farm-section-title">Châm dinh dưỡng và cân pH</h2>
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
          <h2 className="farm-section-title">Cấp và xả nước</h2>
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
          <h2 className="farm-section-title">Phun sương và trộn tuần hoàn</h2>
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
