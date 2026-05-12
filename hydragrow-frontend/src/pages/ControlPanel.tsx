import { useState, useEffect, useRef } from 'react';
import {
  Settings2, Droplets, Wind, Power, AlertTriangle, Timer, Activity, RefreshCw,
  Lock, ChevronDown,
  FlaskConical
} from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';
import { useDeviceControl } from '../hooks/useDeviceControl';
import { PumpStatus } from '../types/models';
import toast from 'react-hot-toast';
import { LoadingState } from '../components/ui/LoadingState';
import { Switch } from '../components/ui/Switch';
import { extractFaultCode } from '../components/ui/FsmStatusBadge';
import { getFaultGuide } from '../components/ui/FaultExplanation';

// --- Component: Khối Điều Khiển Từng Thiết Bị (ĐÃ SỬA LỖI FLICKER & AUTO MODE DISABLE) ---
const AdvancedDeviceControl = ({
  deviceId, pumpId, title, icon: Icon, currentStatus, allowPwm = false, isOnline, isEmergency, isAutoMode
}: any) => {
  const { togglePump, setPwm, forceOn } = useDeviceControl(deviceId);
  const { pwmPreferences, savePwmPreference } = useDeviceContext();

  const [pwmValue, setPwmValue] = useState(pwmPreferences[pumpId] || 100);
  const [duration, setDuration] = useState<number | ''>('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);

  // State để disable nút toggle khi đang chờ phản hồi từ backend
  const [isToggling, setIsToggling] = useState(false);
  const pendingTargetRef = useRef<boolean | null>(null);

  // isLocked dùng để khóa nút Switch:
  // Khóa nếu đang ở Auto Mode HOẶC đang lỗi (trừ khi đang bật thì cho phép tắt)
  const isLocked = isAutoMode || (isEmergency && !currentStatus);

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

  // Nút Switch Bật/Tắt
  const handleToggle = async () => {
    if (isAutoMode) {
      toast.error("Hệ thống đang ở chế độ Tự động, không thể điều khiển thủ công");
      return;
    }
    if (isEmergency && !currentStatus) {
      toast.error(`Hệ thống đang báo lỗi. Vui lòng mở rộng và dùng "Chạy Cưỡng Bức".`);
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
        toast.error(`Không thể ${targetAction === 'on' ? 'bật' : 'tắt'} ${title}`);
      }
    } catch (error) {
      pendingTargetRef.current = null;
      setIsToggling(false);
      console.error(error);
      toast.error(`Lỗi điều khiển ${title}`);
    }
  };

  // Xử lý Hẹn giờ + Công suất kết hợp
  const handleAdvancedRun = async () => {
    if (isAutoMode) {
      toast.error("Hệ thống đang ở chế độ Tự động");
      return;
    }
    if (isEmergency) {
      toast.error("Không thể chạy nâng cao khi đang có lỗi, hãy dùng 'Chạy Cưỡng Bức'");
      return;
    }
    setIsProcessing(true);
    const time = Number(duration);

    try {
      if (allowPwm) {
        await setPwm(pumpId, pwmValue, time > 0 ? time : undefined);
        savePwmPreference(pumpId, pwmValue);
        toast.success(`Đã gửi lệnh chạy ${title} với công suất ${pwmValue}%${time > 0 ? ` trong ${time}s` : ''}`);
      } else {
        if (time > 0) {
          await forceOn(pumpId, time);
          toast.success(`Đã gửi lệnh cưỡng bức ${title} trong ${time}s`);
        } else {
          await togglePump(pumpId, 'on');
          toast.success(`Đã bật ${title}`);
        }
      }
    } catch (error) {
      toast.error(`Lỗi khi chạy ${title}`);
    } finally {
      setIsProcessing(false);
      if (time > 0) setDuration('');
    }
  };

  // Xử lý Cưỡng bức khẩn cấp
  const handleEmergencyForceOn = async () => {
    const time = Number(duration);
    if (!time || time <= 0) {
      toast.error("Vui lòng nhập số giây (Bắt buộc khi cưỡng bức).");
      return;
    }
    if (!window.confirm(`NGUY HIỂM: Bỏ qua cảm biến để chạy ${title} trong ${time} giây?`)) return;

    setIsProcessing(true);
    try {
      await forceOn(pumpId, time);
      toast.success(`Đã kích hoạt cưỡng bức ${title} trong ${time}s`);
    } catch (error) {
      toast.error(`Lỗi cưỡng bức ${title}`);
    } finally {
      setIsProcessing(false);
      setDuration('');
    }
  };

  return (
    <div className={`bg-slate-900 border rounded-xl overflow-hidden transition-colors duration-300 ${currentStatus ? 'border-blue-500/50 bg-slate-800/40' : 'border-slate-800'}`}>
      <div className="p-4 flex flex-col gap-4">

        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`p-2 rounded-lg transition-colors ${currentStatus ? 'bg-blue-500 text-white' : 'bg-slate-950 text-slate-500 border border-slate-800'}`}>
              <Icon size={18} />
            </div>
            <div>
              <h3 className={`text-sm font-semibold ${currentStatus ? 'text-slate-100' : 'text-slate-300'}`}>{title}</h3>
              <p className="text-[10px] text-slate-500 font-medium">{currentStatus ? 'Đang hoạt động' : 'Đang tắt'}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {isLocked && !currentStatus && <Lock size={14} className="text-slate-500" />}
            <Switch
              isOn={currentStatus}
              disabled={!isOnline || isToggling || isProcessing || isLocked}
              onClick={handleToggle}
              colorClass="bg-blue-500"
            />
          </div>
        </div>

        {/* Cấu hình nâng cao */}
        <div className="border-t border-slate-800 pt-3">
          <button onClick={() => setShowAdvanced(!showAdvanced)} className="flex items-center gap-1.5 text-xs font-medium text-slate-400 hover:text-slate-200">
            <ChevronDown size={14} className={`transition-transform ${showAdvanced ? 'rotate-180' : ''}`} />
            {isAutoMode ? 'Thông số hiện tại (Đã khóa vì đang ở chế độ Auto)' : (isEmergency ? 'Mở khóa điều khiển khẩn cấp' : 'Tùy chỉnh thông số')}
          </button>

          {showAdvanced && (
            <div className="mt-4 animate-in slide-in-from-top-2 duration-200 bg-slate-950/50 p-3 rounded-lg border border-slate-800/80">

              {/* TRƯỜNG HỢP 1: ĐANG CẤP CỨU / LỖI */}
              {isEmergency ? (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 text-amber-500 text-xs font-medium">
                    <AlertTriangle size={14} />
                    <span>Chạy Cưỡng Bức (Bỏ qua an toàn)</span>
                  </div>
                  <div className="flex gap-2">
                    <input
                      type="number" placeholder="Bắt buộc nhập số giây..."
                      value={duration} onChange={(e) => setDuration(e.target.value === '' ? '' : Number(e.target.value))}
                      disabled={isAutoMode || isProcessing}
                      className="flex-1 bg-slate-900 border border-red-900/50 text-slate-200 text-xs rounded-lg px-3 py-2 outline-none focus:border-red-500 placeholder:text-slate-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    />
                    <button
                      onClick={handleEmergencyForceOn} disabled={isAutoMode || isProcessing || !duration}
                      className="px-4 py-2 bg-red-500/10 text-red-500 border border-red-500/20 text-xs font-semibold rounded-lg hover:bg-red-500 hover:text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
                    >
                      Ép chạy
                    </button>
                  </div>
                </div>
              ) : (
                <div className="space-y-4">
                  {/* Thanh Slider PWM */}
                  {allowPwm && (
                    <div className="space-y-2">
                      <div className="flex justify-between text-[11px] text-slate-400 font-medium uppercase tracking-wider">
                        <span>Công suất bơm (PWM)</span>
                        <span className="text-blue-400">{pwmValue}%</span>
                      </div>
                      <input
                        type="range" min="10" max="100" step="1"
                        value={pwmValue} onChange={(e) => setPwmValue(parseInt(e.target.value))}
                        disabled={isAutoMode || isProcessing}
                        className="w-full h-1.5 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
                      />
                    </div>
                  )}

                  {/* Ô nhập thời gian & Nút hành động */}
                  <div className="space-y-2">
                    <div className="flex justify-between text-[11px] text-slate-400 font-medium uppercase tracking-wider">
                      <span>Thời gian chạy (Tùy chọn)</span>
                    </div>
                    <div className="flex gap-2">
                      <div className="relative flex-1">
                        <input
                          type="number" placeholder="Để trống để chạy liên tục..."
                          value={duration} onChange={(e) => setDuration(e.target.value === '' ? '' : Number(e.target.value))}
                          disabled={isAutoMode || isProcessing}
                          className="w-full bg-slate-900 border border-slate-700 text-slate-200 text-xs rounded-lg pl-8 pr-3 py-2 outline-none focus:border-blue-500 placeholder:text-slate-600 disabled:opacity-50 disabled:cursor-not-allowed"
                        />
                        <Timer size={14} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-500" />
                      </div>
                      <button
                        onClick={handleAdvancedRun} disabled={isAutoMode || isProcessing}
                        className="px-4 py-2 bg-blue-500 text-white text-xs font-semibold rounded-lg hover:bg-blue-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed shadow-lg shadow-blue-500/20 whitespace-nowrap"
                      >
                        Lưu & Chạy
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

// --- Bảng Điều Khiển Chính ---
const ControlPanel = () => {
  const { deviceId, sensorData, deviceStatus, isControllerStatusKnown, isLoading, fsmState, settings } = useDeviceContext();
  const { isProcessing, resetFault } = useDeviceControl(deviceId || "");

  if (isLoading || !sensorData) return <LoadingState message="Đang tải dữ liệu..." />;

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
    <div className="p-4 md:p-8 max-w-5xl mx-auto space-y-6 pb-28">

      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold text-slate-100 flex items-center gap-2">
            <Settings2 size={22} className="text-slate-500" /> Điều khiển
          </h1>
        </div>

        <button
          disabled={!isOnline || isProcessing}
          onClick={async () => {
            if (window.confirm("Đặt lại lỗi và khởi động lại chu trình FSM?")) await resetFault();
          }}
          className="flex items-center gap-2 px-3 py-2 bg-slate-900 text-slate-300 border border-slate-800 rounded-lg text-xs font-medium hover:bg-slate-800 transition-colors disabled:opacity-50"
        >
          <RefreshCw size={14} className={isProcessing ? "animate-spin" : ""} /> Đặt lại lỗi
        </button>
      </div>

      {/* Cảnh Báo Trạng Thái */}
      {showDisconnected && (
        <div className="bg-red-500/10 border border-red-500/20 rounded-xl p-4 flex gap-3 text-red-400">
          <AlertTriangle size={20} className="shrink-0" />
          <div>
            <h4 className="font-semibold text-sm">Mất kết nối trạm</h4>
            <p className="text-xs opacity-80 mt-0.5">Không thể gửi lệnh. Vui lòng kiểm tra lại kết nối.</p>
          </div>
        </div>
      )}

      {isAutoMode && isOnline && (
        <div className="bg-blue-500/10 border border-blue-500/20 rounded-xl p-4 flex gap-3 text-blue-400">
          <Activity size={20} className="shrink-0" />
          <div>
            <h4 className="font-semibold text-sm">Chế độ Tự Động (Auto) đang bật</h4>
            <p className="text-xs opacity-80 mt-0.5">Hệ thống FSM đang làm chủ. Các lệnh điều khiển thủ công bị khóa để đảm bảo an toàn.</p>
          </div>
        </div>
      )}

      {isEmergency && isOnline && !isAutoMode && (
        <div className="bg-amber-500/10 border border-amber-500/20 rounded-xl p-4 flex gap-3 text-amber-400">
          <AlertTriangle size={20} className="shrink-0" />
          <div>
            <h4 className="font-semibold text-sm">Bảo vệ khẩn cấp</h4>
            <p className="text-xs opacity-80 mt-0.5">{faultGuide?.short || 'Hệ thống đang lỗi. Phím bật thường bị khóa. Hãy dùng "Chạy Cưỡng Bức" nếu cần thiết.'}</p>
            {faultGuide && <p className="text-xs opacity-70 mt-1">Hành động: {faultGuide.action}</p>}
          </div>
        </div>
      )}

      <div className="space-y-3">
        <h2 className="text-xs font-semibold text-slate-500 uppercase tracking-wider pl-1">Pha dinh dưỡng</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <AdvancedDeviceControl
            deviceId={deviceId} pumpId="PUMP_A" title="Bơm Phân A" icon={FlaskConical}
            currentStatus={pumps.pump_a} allowPwm={true}
            isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
          />
          <AdvancedDeviceControl
            deviceId={deviceId} pumpId="PUMP_B" title="Bơm Phân B" icon={FlaskConical}
            currentStatus={pumps.pump_b} allowPwm={true}
            isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
          />
          <AdvancedDeviceControl
            deviceId={deviceId} pumpId="PH_UP" title="Bơm Tăng pH" icon={Activity}
            currentStatus={pumps.ph_up} allowPwm={true}
            isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
          />
          <AdvancedDeviceControl
            deviceId={deviceId} pumpId="PH_DOWN" title="Bơm Giảm pH" icon={Activity}
            currentStatus={pumps.ph_down} allowPwm={true}
            isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
          />
        </div>
      </div>

      <div className="space-y-3">
        <h2 className="text-xs font-semibold text-slate-500 uppercase tracking-wider pl-1">Bơm nước</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <AdvancedDeviceControl
            deviceId={deviceId} pumpId="WATER_PUMP_IN" title="Cấp Nước" icon={Droplets}
            currentStatus={pumps.water_pump_in} allowPwm={false}
            isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
          />
          <AdvancedDeviceControl
            deviceId={deviceId} pumpId="WATER_PUMP_OUT" title="Xả Nước" icon={Droplets}
            currentStatus={pumps.water_pump_out} allowPwm={false}
            isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
          />
        </div>
      </div>

      <div className="space-y-3">
        <h2 className="text-xs font-semibold text-slate-500 uppercase tracking-wider pl-1">Khí hậu</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <AdvancedDeviceControl
            deviceId={deviceId} pumpId="OSAKA" title="Bơm trộn" icon={Power}
            currentStatus={pumps.osaka_pump} allowPwm={true}
            isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
          />
          <AdvancedDeviceControl
            deviceId={deviceId} pumpId="MIST" title="Phun Sương" icon={Wind}
            currentStatus={pumps.mist_valve} allowPwm={false}
            isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode}
          />
        </div>
      </div>

    </div>
  );
};

export default ControlPanel;
