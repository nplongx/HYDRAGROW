import { useState, useEffect, useRef } from 'react';
import { Lock, ChevronDown, ShieldAlert, Timer } from 'lucide-react';
import toast from 'react-hot-toast';

import { useDeviceStore } from '../../store/useDeviceStore';
import { useDeviceControl } from '../../hooks/useDeviceControl';
import { Switch } from '../ui/Switch';

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
  colorTheme: 'orange' | 'fuchsia' | 'blue' | 'indigo' | 'sky' | string;
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
  colorTheme
}: AdvancedDeviceControlProps) => {
  const { togglePump, setPwm, forceOn } = useDeviceControl(deviceId || '');
  const pwmPreferences = useDeviceStore((s) => s.pwmPreferences);
  const savePwmPreference = useDeviceStore((s) => s.savePwmPreference);

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

  const disabledReason = !canSendCommands
    ? 'Chưa kết nối máy chủ'
    : isToggling || isProcessing
      ? 'Đang gửi lệnh...'
      : isAutoMode
        ? 'Không thể thao tác — đang chạy tự động'
        : isEmergency && !currentStatus
          ? 'Đang khóa khẩn cấp'
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
      toast.error("Không thể thao tác — đang chạy tự động");
      return;
    }
    if (isEmergency && !currentStatus) {
      toast.error("Đang khóa khẩn cấp. Mở cài đặt kỹ thuật để ép chạy.");
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
    <div className={`ui-card transition-all duration-300 ${currentStatus ? 'border-emerald-300 bg-emerald-50/40' : ''}`}>
      <div className="p-1 flex flex-col gap-3.5">
        {/* Nút bật/tắt chính */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`p-2 rounded-xl transition-all duration-300 shadow-md ${currentStatus ? activeTheme.activeIcon : 'bg-white text-emerald-700/75 border border-emerald-100'}`}>
              <Icon size={16} />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-emerald-900">{title}</h3>
              <p className="text-xs text-emerald-700/60 font-semibold tracking-wide">{currentStatus ? 'Đang hoạt động' : disabledReason || 'Tắt'}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {isLocked && !currentStatus && <Lock size={12} className="text-emerald-700/60 mr-0.5" />}
            <Switch
              isOn={currentStatus}
              disabled={!canSendCommands || isToggling || isProcessing || isLocked}
              onClick={handleToggle}
              colorClass={currentStatus ? (pumpId.startsWith('PH') ? 'bg-fuchsia-600' : 'bg-emerald-600') : 'bg-emerald-200'}
            />
          </div>
        </div>

        {/* Cài đặt kỹ thuật & Hẹn giờ */}
        <div className="border-t border-emerald-100 pt-2.5">
          <button
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="flex items-center gap-1 text-[10px] font-bold uppercase tracking-wider text-emerald-700/75 hover:text-emerald-900 transition-colors cursor-pointer"
          >
            <ChevronDown size={12} className={`transition-transform duration-200 ${showAdvanced ? 'rotate-180' : ''}`} />
            <span>{isEmergency ? 'Thiết lập khẩn cấp' : 'Tùy chỉnh kỹ thuật'}</span>
          </button>

          {showAdvanced && (
            <div className="mt-3 bg-emerald-50/80 p-3 rounded-xl border border-emerald-100 space-y-3.5 animate-in slide-in-from-top-2">
              {isEmergency ? (
                <div className="space-y-2">
                  <div className="flex items-center gap-1.5 text-red-700 text-[10px] font-bold uppercase tracking-wide">
                    <ShieldAlert size={12} />
                    <span>Cưỡng chế chạy (Bỏ qua AI)</span>
                  </div>
                  {allowPwm && (
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-[10px] text-emerald-800 font-bold uppercase">
                        <span>Công suất PWM</span>
                        <span className="text-emerald-700 font-mono">{pwmValue}%</span>
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
                      className="flex-1 bg-white border border-red-200 text-emerald-950 text-xs rounded-xl px-3 py-1.5 outline-none font-medium"
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
                      <div className="flex justify-between text-[10px] text-emerald-800 font-bold uppercase">
                        <span>Công suất (PWM)</span>
                        <span className="text-emerald-700 font-mono">{pwmValue}%</span>
                      </div>
                      <input
                        type="range" min="20" max="100" step="5"
                        value={pwmValue} onChange={(e) => setPwmValue(parseInt(e.target.value))}
                        disabled={isProcessing || !canSendCommands}
                        className="w-full h-1 bg-emerald-100 rounded-lg appearance-none cursor-pointer accent-emerald-600"
                      />
                    </div>
                  )}
                  <div className="space-y-1.5">
                    <div className="flex justify-between text-[10px] text-emerald-800 font-bold uppercase">
                      <span>Thời gian hẹn giờ (Giây)</span>
                    </div>
                    <div className="flex gap-2">
                      <div className="relative flex-1">
                        <input
                          type="number" placeholder="Bắt đầu..."
                          value={duration} onChange={(e) => setDuration(e.target.value === '' ? '' : Number(e.target.value))}
                          disabled={isProcessing || !canSendCommands}
                          className="w-full bg-white border border-emerald-100 text-emerald-950 text-xs rounded-xl pl-8 pr-3 py-1.5 outline-none font-medium"
                        />
                        <Timer size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-emerald-700/75" />
                      </div>
                      <button
                        onClick={handleAdvancedRun} disabled={isProcessing || !canSendCommands}
                        className="px-4 py-1.5 bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold rounded-xl transition-all disabled:opacity-50"
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
