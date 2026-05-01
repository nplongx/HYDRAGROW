import { useState, useMemo, useEffect } from 'react';
import {
  Settings2, FlaskConical, Droplets, Wind, Power, AlertTriangle, Timer, Activity, RefreshCw,
  Play, Target, Lock, ChevronDown
} from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';
import { useDeviceControl } from '../hooks/useDeviceControl';
import { PumpStatus } from '../types/models';
import toast from 'react-hot-toast';
import { LoadingState } from '../components/ui/LoadingState';
import { Switch } from '../components/ui/Switch';
import { InputGroup } from '../components/ui/InputGroup';

// --- Component: Trợ Lý Châm Bán Thủ Công ---
const SemiAutoDosingAssistant = ({ deviceId, isOnline, dosingCalibration, sensorData, isAutoMode }: any) => {
  const { forceOn } = useDeviceControl(deviceId);
  const [selectedPump, setSelectedPump] = useState('PUMP_A');
  const [targetValue, setTargetValue] = useState<number | ''>('');
  const [volumeMl, setVolumeMl] = useState<number>(0);
  const [capacityMlPerSec, setCapacityMlPerSec] = useState<number>(1.2);
  const [isProcessing, setIsProcessing] = useState(false);

  const currentEC = sensorData?.ec || 0;
  const currentPH = sensorData?.ph || 0;
  const isEC = selectedPump === 'PUMP_A' || selectedPump === 'PUMP_B';
  // const currentValue = isEC ? currentEC : currentPH;
  const unit = isEC ? 'mS/cm' : 'pH';

  const getCalibratedCapacity = (pumpId: string) => {
    if (!dosingCalibration) return 1.2;
    switch (pumpId) {
      case 'PUMP_A': return dosingCalibration.pump_a_capacity_ml_per_sec || 1.2;
      case 'PUMP_B': return dosingCalibration.pump_b_capacity_ml_per_sec || 1.2;
      case 'PH_UP': return dosingCalibration.pump_ph_up_capacity_ml_per_sec || 1.2;
      case 'PH_DOWN': return dosingCalibration.pump_ph_down_capacity_ml_per_sec || 1.2;
      default: return 1.2;
    }
  };

  useEffect(() => {
    setCapacityMlPerSec(getCalibratedCapacity(selectedPump));
    setTargetValue('');
    setVolumeMl(0);
  }, [selectedPump, dosingCalibration]);

  useEffect(() => {
    if (targetValue === '' || typeof targetValue !== 'number') return;
    let calcMl = 0;
    if (isEC) {
      const diff = targetValue - currentEC;
      const gain = dosingCalibration?.ec_gain_per_ml || 0.01;
      calcMl = diff > 0 ? diff / gain : 0;
    } else if (selectedPump === 'PH_UP') {
      const diff = targetValue - currentPH;
      const gain = dosingCalibration?.ph_shift_up_per_ml || 0.01;
      calcMl = diff > 0 ? diff / gain : 0;
    } else if (selectedPump === 'PH_DOWN') {
      const diff = currentPH - targetValue;
      const gain = dosingCalibration?.ph_shift_down_per_ml || 0.01;
      calcMl = diff > 0 ? diff / gain : 0;
    }
    setVolumeMl(Math.round(calcMl * 10) / 10);
  }, [targetValue, selectedPump, currentEC, currentPH, dosingCalibration]);

  const durationSec = useMemo(() => {
    if (capacityMlPerSec <= 0) return 0;
    return Math.max(1, Math.round(volumeMl / capacityMlPerSec));
  }, [volumeMl, capacityMlPerSec]);

  const handleDose = async () => {
    if (!window.confirm(`Châm ${volumeMl}mL trong ${durationSec} giây để đạt ${targetValue} ${unit}?`)) return;
    setIsProcessing(true);
    await forceOn(selectedPump, durationSec);
    setIsProcessing(false);
  };

  return (
    <div className={`bg-slate-900 border border-slate-800 rounded-xl p-5 transition-opacity ${isAutoMode ? 'opacity-50 pointer-events-none' : ''}`}>
      <div className="flex items-center gap-3 mb-5 border-b border-slate-800 pb-4">
        <div className="p-2 rounded-lg bg-blue-600 text-white"><Target size={18} /></div>
        <div>
          <h3 className="font-semibold text-slate-100 text-sm">Trợ lý châm thông minh</h3>
          <p className="text-xs text-slate-500 font-medium">Tự động tính toán số mL cần bơm dựa trên ngưỡng đích</p>
        </div>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-5 gap-4 items-end">
        <div className="col-span-2 md:col-span-1">
          <label className="text-xs font-medium text-slate-400 mb-1 block">Dung dịch</label>
          <select
            value={selectedPump} onChange={(e) => setSelectedPump(e.target.value)} disabled={!isOnline || isProcessing}
            className="w-full bg-slate-950 border border-slate-800 text-slate-200 text-sm rounded-lg px-3 py-2 outline-none focus:border-blue-500"
          >
            <option value="PUMP_A">Phân A</option>
            <option value="PUMP_B">Phân B</option>
            <option value="PH_UP">Tăng pH</option>
            <option value="PH_DOWN">Giảm pH</option>
          </select>
        </div>

        <InputGroup label={`Đích đến (${unit})`} step="0.1" value={targetValue} onChange={(e: any) => setTargetValue(e.target.value === '' ? '' : Number(e.target.value))} />
        <InputGroup label="Thể tích (mL)" step="1" value={volumeMl} onChange={(e: any) => setVolumeMl(Number(e.target.value))} />
        <InputGroup label="Lưu lượng (mL/s)" step="0.1" value={capacityMlPerSec} onChange={(e: any) => setCapacityMlPerSec(Number(e.target.value))} />

        <button
          onClick={handleDose}
          disabled={!isOnline || isProcessing || volumeMl <= 0 || capacityMlPerSec <= 0}
          className="w-full h-[38px] col-span-2 md:col-span-1 flex items-center justify-center gap-2 bg-blue-600 text-white font-medium text-xs rounded-lg hover:bg-blue-500 transition-colors disabled:opacity-50"
        >
          <Play size={14} className={isProcessing ? "animate-pulse" : ""} />
          Bơm {durationSec}s
        </button>
      </div>
    </div>
  );
};

// --- Component: Khối Điều Khiển Từng Thiết Bị ---
const AdvancedDeviceControl = ({
  deviceId, pumpId, title, icon: Icon, currentStatus, allowPwm = false, updatePumpStatusOptimistically, isOnline, isEmergency, isAutoMode
}: any) => {
  const { togglePump, setPwm, forceOn } = useDeviceControl(deviceId);
  const { pwmPreferences, savePwmPreference } = useDeviceContext();

  const [pwmValue, setPwmValue] = useState(pwmPreferences[pumpId] || 100);
  const [duration, setDuration] = useState<number | ''>('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);

  const stateKey = pumpId.toLowerCase();
  const isLocked = isAutoMode || (isEmergency && !currentStatus);

  useEffect(() => {
    if (pwmPreferences[pumpId] !== undefined) setPwmValue(pwmPreferences[pumpId]);
  }, [pwmPreferences, pumpId]);

  const handleToggle = async () => {
    if (isAutoMode) return;
    if (isEmergency && !currentStatus) {
      toast.error(`Không thể bật thường khi hệ thống đang lỗi. Hãy dùng chức năng Chạy Cưỡng Bức.`);
      return;
    }
    setIsProcessing(true);
    const targetAction = currentStatus ? 'off' : 'on';
    updatePumpStatusOptimistically(stateKey, targetAction === 'on');
    try {
      const success = await togglePump(pumpId, targetAction);
      if (!success) updatePumpStatusOptimistically(stateKey, currentStatus);
    } catch (error) {
      updatePumpStatusOptimistically(stateKey, currentStatus);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleForceOn = async () => {
    const time = Number(duration);
    if (!time || time <= 0) { toast.error("Vui lòng nhập số giây hợp lệ."); return; }
    if (!window.confirm(`Bạn chắc chắn muốn chạy ${title} trong ${time} giây?`)) return;

    setIsProcessing(true);
    updatePumpStatusOptimistically(stateKey, true);
    try {
      const success = await forceOn(pumpId, time);
      if (!success) updatePumpStatusOptimistically(stateKey, false);
    } catch (error) {
      updatePumpStatusOptimistically(stateKey, false);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleSetPwm = async () => {
    if (isAutoMode || isEmergency) return;
    setIsProcessing(true);
    if (!currentStatus) updatePumpStatusOptimistically(stateKey, true);
    await setPwm(pumpId, pwmValue);
    savePwmPreference(pumpId, pwmValue);
    setIsProcessing(false);
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
              <p className="text-[10px] text-slate-500 font-medium">{currentStatus ? 'Đang chạy' : 'Đã tắt'}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {isLocked && !currentStatus && <Lock size={14} className="text-slate-500" />}
            <Switch isOn={currentStatus} disabled={!isOnline || isProcessing || isLocked} onClick={handleToggle} colorClass="bg-blue-500" />
          </div>
        </div>

        {/* Cấu hình nâng cao (Mở rộng) */}
        {(allowPwm || !isAutoMode) && (
          <div className="border-t border-slate-800 pt-3">
            <button onClick={() => setShowAdvanced(!showAdvanced)} className="flex items-center gap-1.5 text-xs font-medium text-slate-400 hover:text-slate-200">
              <ChevronDown size={14} className={`transition-transform ${showAdvanced ? 'rotate-180' : ''}`} /> Tùy chọn thời gian & công suất
            </button>

            {showAdvanced && (
              <div className="mt-4 space-y-4 animate-in slide-in-from-top-2 duration-200">

                {/* PWM Slider */}
                {allowPwm && (
                  <div className={`space-y-2 ${isAutoMode || isEmergency ? 'opacity-50 pointer-events-none' : ''}`}>
                    <div className="flex justify-between text-xs text-slate-400">
                      <span>Công suất (PWM)</span>
                      <span className="text-slate-200 font-medium">{pwmValue}%</span>
                    </div>
                    <div className="flex items-center gap-3">
                      <input
                        type="range" min="10" max="100" step="1"
                        value={pwmValue} onChange={(e) => setPwmValue(parseInt(e.target.value))}
                        className="flex-1 h-1.5 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-blue-500"
                      />
                      <button onClick={handleSetPwm} disabled={isProcessing} className="px-2.5 py-1 bg-slate-800 text-slate-200 text-xs font-medium rounded border border-slate-700 hover:bg-slate-700 transition-colors">
                        Lưu
                      </button>
                    </div>
                  </div>
                )}

                {/* Hẹn giờ / Cưỡng bức */}
                {!isAutoMode && (
                  <div className="space-y-2">
                    <div className="flex justify-between text-xs text-slate-400">
                      <span className="flex items-center gap-1.5"><Timer size={12} /> {isEmergency ? 'Chạy Cưỡng Bức' : 'Bật Theo Hẹn Giờ'}</span>
                    </div>
                    <div className="flex gap-2">
                      <input
                        type="number" placeholder="Nhập số giây..." value={duration} onChange={(e) => setDuration(e.target.value === '' ? '' : Number(e.target.value))}
                        className="flex-1 bg-slate-950 border border-slate-800 text-slate-200 text-xs rounded-lg px-3 py-1.5 outline-none focus:border-amber-500"
                      />
                      <button
                        onClick={handleForceOn} disabled={isProcessing || !duration}
                        className="px-3 py-1.5 bg-amber-500/10 text-amber-500 border border-amber-500/20 text-xs font-semibold rounded-lg hover:bg-amber-500 hover:text-white transition-colors disabled:opacity-50"
                      >
                        Bật ngay
                      </button>
                    </div>
                  </div>
                )}

              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

// --- Bảng Điều Khiển Chính ---
const ControlPanel = () => {
  const { deviceId, sensorData, deviceStatus, isControllerStatusKnown, isLoading, updatePumpStatusOptimistically, fsmState, settings } = useDeviceContext();
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

  return (
    <div className="p-4 md:p-8 max-w-5xl mx-auto space-y-6 pb-28">

      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="space-y-1">
          <h1 className="text-2xl font-semibold text-slate-100 flex items-center gap-2">
            Điều khiển <Settings2 size={22} className="text-slate-500" />
          </h1>
          <p className="text-sm text-slate-500">Can thiệp và vận hành thiết bị thủ công.</p>
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
            <p className="text-xs opacity-80 mt-0.5">Hệ thống đang lỗi. Phím bật thường bị khóa. Hãy dùng "Chạy Cưỡng Bức" (có tính giờ) nếu cần thiết.</p>
          </div>
        </div>
      )}

      {/* KHỐI CHÂM BÁN THỦ CÔNG */}
      <SemiAutoDosingAssistant deviceId={deviceId} isOnline={isOnline} dosingCalibration={settings?.dosing_calibration} sensorData={sensorData} isAutoMode={isAutoMode} />

      {/* Máy Pha Phân */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold text-slate-500 uppercase tracking-wider pl-1">Máy pha dinh dưỡng</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <AdvancedDeviceControl deviceId={deviceId} pumpId="PUMP_A" title="Bơm Phân A" icon={FlaskConical} currentStatus={pumps.pump_a} allowPwm={true} updatePumpStatusOptimistically={updatePumpStatusOptimistically} isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode} />
          <AdvancedDeviceControl deviceId={deviceId} pumpId="PUMP_B" title="Bơm Phân B" icon={FlaskConical} currentStatus={pumps.pump_b} allowPwm={true} updatePumpStatusOptimistically={updatePumpStatusOptimistically} isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode} />
          <AdvancedDeviceControl deviceId={deviceId} pumpId="PH_UP" title="Bơm Tăng pH" icon={Activity} currentStatus={pumps.ph_up} allowPwm={true} updatePumpStatusOptimistically={updatePumpStatusOptimistically} isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode} />
          <AdvancedDeviceControl deviceId={deviceId} pumpId="PH_DOWN" title="Bơm Giảm pH" icon={Activity} currentStatus={pumps.ph_down} allowPwm={true} updatePumpStatusOptimistically={updatePumpStatusOptimistically} isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode} />
        </div>
      </div>

      {/* Nước và Khí Hậu */}
      <div className="space-y-3 pt-4 border-t border-slate-800/50">
        <h2 className="text-xs font-semibold text-slate-500 uppercase tracking-wider pl-1">Bơm nước & Khí hậu</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <AdvancedDeviceControl deviceId={deviceId} pumpId="WATER_PUMP_IN" title="Cấp Nước" icon={Droplets} currentStatus={pumps.water_pump_in} allowPwm={false} updatePumpStatusOptimistically={updatePumpStatusOptimistically} isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode} />
          <AdvancedDeviceControl deviceId={deviceId} pumpId="WATER_PUMP_OUT" title="Xả Nước" icon={Droplets} currentStatus={pumps.water_pump_out} allowPwm={false} updatePumpStatusOptimistically={updatePumpStatusOptimistically} isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode} />
          <AdvancedDeviceControl deviceId={deviceId} pumpId="OSAKA" title="Trộn Osaka" icon={Power} currentStatus={pumps.osaka_pump} allowPwm={true} updatePumpStatusOptimistically={updatePumpStatusOptimistically} isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode} />
          <AdvancedDeviceControl deviceId={deviceId} pumpId="MIST" title="Phun Sương" icon={Wind} currentStatus={pumps.mist_valve} allowPwm={false} updatePumpStatusOptimistically={updatePumpStatusOptimistically} isOnline={isOnline} isEmergency={isEmergency} isAutoMode={isAutoMode} />
        </div>
      </div>

    </div>
  );
};

export default ControlPanel;
