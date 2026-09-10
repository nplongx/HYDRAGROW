import { useState, useEffect, useRef } from 'react';
import { Lock, ChevronDown, ShieldAlert, Timer } from 'lucide-react';
import toast from 'react-hot-toast';

import { useDeviceStore } from '../../store/useDeviceStore';
import { useDeviceControl } from '../../hooks/useDeviceControl';
import { Switch } from '../ui/Switch';
import { StatusPill } from '../ui/StatusPill';

// Hệ thống quy đổi PWM(%) -> ml/phút, tạm thời tuyến tính cho hiển thị nhanh trên card.
// TODO(sau khi có dữ liệu hiệu chuẩn DosingCalibration thật): thay bằng giá trị đo thực tế theo từng bơm.
const PWM_TO_ML_PER_MIN: Record<string, number> = {
  PUMP_A: 0.12,
  PUMP_B: 0.12,
  PH_UP: 0.1,
  PH_DOWN: 0.1,
  OSAKA: 0.15,
};

interface AdvancedDeviceControlProps {
  deviceId: string | null;
  pumpId: string;
  title: string;
  icon: React.ElementType;
  currentStatus: boolean;
  allowPwm?: boolean;
  canSendCommands: boolean;
  isEmergency: boolean;
  isAutoMode: boolean;
  colorTheme: 'orange' | 'fuchsia' | 'water' | 'sky' | string;
  lockedByPumpId?: string;
  lockedByPumpLabel?: string;
}

export const AdvancedDeviceControl = ({
  deviceId,
  pumpId,
  title,
  icon: Icon,
  currentStatus,
  allowPwm = false,
  canSendCommands,
  isEmergency,
  isAutoMode,
  colorTheme,
  lockedByPumpId,
  lockedByPumpLabel,
}: AdvancedDeviceControlProps) => {
  const { togglePump, setPwm, forceOn, commandStatus } = useDeviceControl(deviceId || '');
  const pwmPreferences = useDeviceStore((s) => s.pwmPreferences);
  const savePwmPreference = useDeviceStore((s) => s.savePwmPreference);

  const [pwmValue, setPwmValue] = useState(pwmPreferences[pumpId] || 100);
  const [duration, setDuration] = useState<number | ''>('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [isToggling, setIsToggling] = useState(false);
  const pendingTargetRef = useRef<boolean | null>(null);

  const isLocked = isAutoMode || (isEmergency && !currentStatus) || Boolean(lockedByPumpId);

  const themeClasses: Record<string, { activeIcon: string; glow: string; border: string }> = {
    orange: { activeIcon: 'bg-orange-600 text-white', glow: 'border-orange-200 bg-orange-50', border: 'border-orange-300' },
    fuchsia: { activeIcon: 'bg-fuchsia-600 text-white', glow: 'border-fuchsia-200 bg-fuchsia-50', border: 'border-fuchsia-300' },
    water: { activeIcon: 'bg-sky-600 text-white', glow: 'border-sky-200 bg-sky-50', border: 'border-sky-300' },
    sky: { activeIcon: 'bg-sky-600 text-white', glow: 'border-sky-200 bg-sky-50', border: 'border-sky-300' },
  };
  const activeTheme = themeClasses[colorTheme] || themeClasses.water;

  const disabledReason = !canSendCommands
    ? 'Chưa kết nối máy chủ'
    : isToggling || isProcessing
      ? 'Đang gửi lệnh...'
      : isAutoMode
        ? 'Tự động (MIMO) quản lý'
        : isEmergency && !currentStatus
          ? 'Đang ngắt do sự cố an toàn'
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
      toast.error("Hệ thống đang chạy Tự Động, không thể can thiệp thủ công.");
      return;
    }
    if (isEmergency && !currentStatus) {
      toast.error("Hệ thống đang ngắt do sự cố. Mở cài đặt kỹ thuật để ép chạy.");
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
      } else {
        setTimeout(() => {
          if (pendingTargetRef.current !== null) {
            pendingTargetRef.current = null;
            setIsToggling(false);
          }
        }, 8000);
      }
    } catch {
      pendingTargetRef.current = null;
      setIsToggling(false);
      toast.error("Lỗi khi gửi lệnh.");
    }
  };

  const handleAdvancedRun = async () => {
    setIsProcessing(true);
    const time = Number(duration);
    try {
      if (allowPwm) {
        if (time > 0) {
          await forceOn(pumpId, time, pwmValue);
        } else {
          await setPwm(pumpId, pwmValue);
        }
        savePwmPreference(pumpId, pwmValue);
        toast.success(`Đã đồng bộ công suất ${pwmValue}%`);
      } else if (time > 0) {
        await forceOn(pumpId, time);
      } else {
        await togglePump(pumpId, 'on');
      }
    } catch {
      toast.error("Không thể thực thi.");
    } finally {
      setIsProcessing(false);
      if (time > 0) setDuration('');
    }
  };

  const handleEmergencyForceOn = async () => {
    const time = Number(duration);
    if (!time || time <= 0) {
      toast.error("Vui lòng nhập thời gian ép chạy.");
      return;
    }
    if (!window.confirm("CẢNH BÁO: Bỏ qua kiểm tra an toàn AI. Xác nhận kích hoạt?")) return;
    setIsProcessing(true);
    try {
      await forceOn(pumpId, time, allowPwm ? pwmValue : undefined);
      if (allowPwm) savePwmPreference(pumpId, pwmValue);
    } catch {
      toast.error("Lỗi thực thi lệnh cưỡng chế.");
    } finally {
      setIsProcessing(false);
      setDuration('');
    }
  };

  return (
    <div className={`border rounded-2xl overflow-hidden transition-all duration-300 shadow-sm shadow-primary/5 ${currentStatus ? activeTheme.glow : 'border-line bg-white'}`}>
      <div className="p-4 flex flex-col gap-3.5">
        {/* Nút bật/tắt chính */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`p-2 rounded-xl transition-all duration-300 shadow-md ${currentStatus ? activeTheme.activeIcon : 'bg-white text-primary/75 border border-line'}`}>
              <Icon size={16} />
            </div>
            <div>
              <h3 className={`text-xs font-bold ${currentStatus ? 'text-primary-deep' : 'text-primary-deep'}`}>{title}</h3>
              <p className="text-[10px] text-text-muted font-semibold tracking-wide">{currentStatus ? 'Đang hoạt động' : disabledReason || 'Tắt'}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <StatusPill commandStatus={commandStatus[pumpId]} />
            {isLocked && !currentStatus && <Lock size={12} className="text-primary/60 mr-0.5" />}
            <Switch
              isOn={currentStatus}
              disabled={!canSendCommands || isToggling || isProcessing || isLocked}
              onClick={handleToggle}
              colorClass={currentStatus ? (pumpId.startsWith('PH') ? 'bg-fuchsia-600' : 'bg-primary') : undefined}
            />
          </div>
        </div>

        {lockedByPumpId && (
          <div className="flex items-center gap-1.5 text-[10px] font-semibold text-text-muted bg-soft border border-line rounded-lg px-2.5 py-1.5">
            <Lock size={11} className="shrink-0" />
            <span>Đã khoá vì {lockedByPumpLabel || 'thiết bị xung khắc'} đang chạy — tránh trung hoà lẫn nhau</span>
          </div>
        )}

        {allowPwm && currentStatus && (
          <div className="flex items-center justify-between text-[10px] font-semibold text-primary-deep bg-soft border border-line rounded-lg px-2.5 py-1.5">
            <span>Công suất</span>
            <span className="font-mono">
              {pwmValue}% ≈ {((pwmValue * (PWM_TO_ML_PER_MIN[pumpId.toUpperCase()] ?? 0.12))).toFixed(1)} ml/phút
            </span>
          </div>
        )}

        {/* Cài đặt kỹ thuật & Hẹn giờ */}
        <div className="border-t border-line pt-2.5">
          <button
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="flex items-center gap-1 text-[10px] font-bold uppercase tracking-wider text-text-muted hover:text-primary-deep transition-colors cursor-pointer"
          >
            <ChevronDown size={12} className={`transition-transform duration-200 ${showAdvanced ? 'rotate-180' : ''}`} />
            <span>{isEmergency ? 'Thiết lập khẩn cấp' : 'Tùy chỉnh kỹ thuật'}</span>
          </button>

          {showAdvanced && (
            <div className="mt-3 bg-soft p-3 rounded-xl border border-line space-y-3.5 animate-in slide-in-from-top-2">
              {isEmergency ? (
                <div className="space-y-2">
                  <div className="flex items-center gap-1.5 text-red-700 text-[10px] font-bold uppercase tracking-wide">
                    <ShieldAlert size={12} />
                    <span>Cưỡng chế chạy (Bỏ qua AI)</span>
                  </div>
                  {allowPwm && (
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-[10px] text-primary-deep font-bold uppercase">
                        <span>Công suất PWM</span>
                        <span className="text-primary font-mono">{pwmValue}%</span>
                      </div>
                      <input
                        type="range" min="20" max="100" step="5"
                        value={pwmValue} onChange={(e) => setPwmValue(parseInt(e.target.value))}
                        disabled={isProcessing || !canSendCommands}
                        className="w-full h-1 bg-red-100 rounded-lg appearance-none cursor-pointer accent-red-600"
                      />
                    </div>
                  )}
                  <div className="flex gap-2">
                    <input
                      type="number" placeholder="Số giây ép chạy..."
                      value={duration} onChange={(e) => setDuration(e.target.value === '' ? '' : Number(e.target.value))}
                      disabled={isProcessing || !canSendCommands}
                      className="flex-1 bg-white border border-red-200 text-primary-deep text-xs rounded-xl px-3 py-1.5 outline-none font-medium"
                    />
                    <button
                      onClick={handleEmergencyForceOn} disabled={isProcessing || !duration || !canSendCommands}
                      className="px-3.5 py-1.5 bg-red-50 text-red-700 border border-red-200 text-xs font-bold rounded-xl hover:bg-red-600 hover:text-white transition-all disabled:opacity-50"
                    >
                      Kích hoạt
                    </button>
                  </div>
                </div>
              ) : (
                <div className="space-y-3.5">
                  {allowPwm && (
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-[10px] text-primary-deep font-bold uppercase">
                        <span>Công suất (PWM)</span>
                        <span className="text-primary font-mono">{pwmValue}%</span>
                      </div>
                      <input
                        type="range" min="20" max="100" step="5"
                        value={pwmValue} onChange={(e) => setPwmValue(parseInt(e.target.value))}
                        disabled={isProcessing || !canSendCommands}
                        className="w-full h-1 bg-line rounded-lg appearance-none cursor-pointer accent-primary"
                      />
                    </div>
                  )}
                  <div className="space-y-1.5">
                    <div className="flex justify-between text-[10px] text-primary-deep font-bold uppercase">
                      <span>Thời gian hẹn giờ (Giây)</span>
                    </div>
                    <div className="flex gap-2">
                      <div className="relative flex-1">
                        <input
                          type="number" placeholder="Bắt đầu..."
                          value={duration} onChange={(e) => setDuration(e.target.value === '' ? '' : Number(e.target.value))}
                          disabled={isProcessing || !canSendCommands}
                          className="w-full bg-white border border-line text-primary-deep text-xs rounded-xl pl-8 pr-3 py-1.5 outline-none font-medium"
                        />
                        <Timer size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-primary/75" />
                      </div>
                      <button
                        onClick={handleAdvancedRun} disabled={isProcessing || !canSendCommands}
                        className="px-4 py-1.5 bg-primary hover:bg-primary-deep text-white text-xs font-bold rounded-xl transition-all disabled:opacity-50"
                      >
                        Chạy
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
