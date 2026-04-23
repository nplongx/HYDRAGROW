import { useState, useMemo, useEffect } from 'react';
import {
  Settings2,
  FlaskConical,
  Droplets,
  Wind,
  Power,
  AlertTriangle,
  Timer,
  Activity,
  RefreshCw,
  Zap,
  Waves,
  Play,
  Target,
} from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';
import { useDeviceControl } from '../hooks/useDeviceControl';
import { PumpStatus } from '../types/models';
import toast from 'react-hot-toast';

const NEON_COLORS: Record<string, any> = {
  orange: {
    bg: 'bg-orange-500',
    text: 'text-orange-400',
    border: 'border-orange-500/50',
    shadow: 'shadow-[0_0_20px_rgba(249,115,22,0.3)]',
    glow: 'from-orange-500/20',
  },
  purple: {
    bg: 'bg-purple-500',
    text: 'text-purple-400',
    border: 'border-purple-500/50',
    shadow: 'shadow-[0_0_20px_rgba(168,85,247,0.3)]',
    glow: 'from-purple-500/20',
  },
  blue: {
    bg: 'bg-blue-500',
    text: 'text-blue-400',
    border: 'border-blue-500/50',
    shadow: 'shadow-[0_0_20px_rgba(59,130,246,0.3)]',
    glow: 'from-blue-500/20',
  },
  cyan: {
    bg: 'bg-cyan-500',
    text: 'text-cyan-400',
    border: 'border-cyan-500/50',
    shadow: 'shadow-[0_0_20px_rgba(6,182,212,0.3)]',
    glow: 'from-cyan-500/20',
  },
  indigo: {
    bg: 'bg-indigo-500',
    text: 'text-indigo-400',
    border: 'border-indigo-500/50',
    shadow: 'shadow-[0_0_20px_rgba(99,102,241,0.3)]',
    glow: 'from-indigo-500/20',
  },
  sky: {
    bg: 'bg-sky-500',
    text: 'text-sky-400',
    border: 'border-sky-500/50',
    shadow: 'shadow-[0_0_20px_rgba(14,165,233,0.3)]',
    glow: 'from-sky-500/20',
  },
};

// --- Component: Trợ Lý Châm Bán Thủ Công (Tự tính từ Ngưỡng EC/pH) ---
const SemiAutoDosingAssistant = ({ deviceId, isOnline, dosingCalibration, sensorData }: any) => {
  const { forceOn } = useDeviceControl(deviceId);
  const [selectedPump, setSelectedPump] = useState('PUMP_A');
  const [targetValue, setTargetValue] = useState<number | ''>(''); // Ngưỡng mong muốn
  const [volumeMl, setVolumeMl] = useState<number>(0); // Cần châm (mL)
  const [capacityMlPerSec, setCapacityMlPerSec] = useState<number>(1.2);
  const [isProcessing, setIsProcessing] = useState(false);

  const currentEC = sensorData?.ec_value || 0;
  const currentPH = sensorData?.ph_value || 0;

  const isEC = selectedPump === 'PUMP_A' || selectedPump === 'PUMP_B';
  const currentValue = isEC ? currentEC : currentPH;
  const unit = isEC ? 'mS/cm' : 'pH';

  // Lấy lưu lượng chuẩn từ cấu hình Backend
  const getCalibratedCapacity = (pumpId: string) => {
    if (!dosingCalibration) return 1.2;
    switch (pumpId) {
      case 'PUMP_A':
        return dosingCalibration.pump_a_capacity_ml_per_sec || 1.2;
      case 'PUMP_B':
        return dosingCalibration.pump_b_capacity_ml_per_sec || 1.2;
      case 'PH_UP':
        return dosingCalibration.pump_ph_up_capacity_ml_per_sec || 1.2;
      case 'PH_DOWN':
        return dosingCalibration.pump_ph_down_capacity_ml_per_sec || 1.2;
      default:
        return 1.2;
    }
  };

  // Reset và cập nhật khi đổi loại bơm
  useEffect(() => {
    setCapacityMlPerSec(getCalibratedCapacity(selectedPump));
    setTargetValue('');
    setVolumeMl(0);
  }, [selectedPump, dosingCalibration]);

  // Tự động tính số mL cần châm khi nhập Ngưỡng Mục Tiêu
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

    // Làm tròn 1 chữ số thập phân cho dễ nhìn
    setVolumeMl(Math.round(calcMl * 10) / 10);
  }, [targetValue, selectedPump, currentEC, currentPH, dosingCalibration]);

  // Quy đổi từ mL sang thời gian (giây)
  const durationSec = useMemo(() => {
    if (capacityMlPerSec <= 0) return 0;
    return Math.max(1, Math.round(volumeMl / capacityMlPerSec));
  }, [volumeMl, capacityMlPerSec]);

  const handleDose = async () => {
    if (
      !window.confirm(
        `Xác nhận châm ${volumeMl}mL (khoảng ${durationSec}s)?\nHệ thống sẽ cố gắng đạt ngưỡng ${targetValue || 'mong muốn'}.`,
      )
    )
      return;

    setIsProcessing(true);
    await forceOn(selectedPump, durationSec);
    setIsProcessing(false);
  };

  return (
    <div className="relative bg-slate-900/60 backdrop-blur-xl rounded-[2rem] p-5 border border-emerald-500/30 shadow-[0_0_30px_rgba(16,185,129,0.05)] overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-br from-emerald-500/10 to-transparent opacity-100 pointer-events-none"></div>

      <div className="relative z-10">
        <div className="flex items-center justify-between mb-5 border-b border-emerald-500/20 pb-4">
          <div className="flex items-center gap-3">
            <div className="p-3 rounded-2xl bg-emerald-500 text-white shadow-[0_0_15px_rgba(16,185,129,0.4)]">
              <Target size={20} />
            </div>
            <div>
              <h3 className="font-black tracking-wide text-white uppercase text-sm">
                Trợ lý châm thông minh (Bán thủ công)
              </h3>
              <p className="text-[10px] text-emerald-400/80 font-medium tracking-wider">
                Chỉ cần nhập ngưỡng đích, hệ thống tự nhẩm số mL
              </p>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-2 lg:grid-cols-5 gap-4 items-end">
          {/* Cột 1: Chọn Dung Dịch */}
          <div className="space-y-1.5 col-span-2 lg:col-span-1">
            <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400 ml-1">
              Loại Dung Dịch
            </label>
            <select
              value={selectedPump}
              onChange={(e) => setSelectedPump(e.target.value)}
              disabled={!isOnline || isProcessing}
              className="w-full bg-slate-800/80 border border-slate-700 text-emerald-300 text-sm font-bold rounded-xl px-4 py-2.5 focus:ring-2 focus:ring-emerald-500 outline-none appearance-none cursor-pointer"
            >
              <option value="PUMP_A">Phân A (Bơm A)</option>
              <option value="PUMP_B">Phân B (Bơm B)</option>
              <option value="PH_UP">Tăng pH (pH Up)</option>
              <option value="PH_DOWN">Giảm pH (pH Down)</option>
            </select>
          </div>

          {/* Cột 2: Nhập Ngưỡng */}
          <div className="space-y-1.5 col-span-1 lg:col-span-1">
            <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400 ml-1 flex items-center justify-between">
              <span>Đích đến ({unit})</span>
              <span className="text-sky-300 bg-sky-500/10 px-1.5 py-0.5 rounded shadow-sm">
                Now: {currentValue.toFixed(2)}
              </span>
            </label>
            <input
              type="number"
              step="0.1"
              placeholder="Vd: 1.5"
              value={targetValue}
              onChange={(e) => setTargetValue(e.target.value === '' ? '' : Number(e.target.value))}
              disabled={!isOnline || isProcessing}
              className="w-full bg-slate-800/80 border border-sky-500/50 text-white text-sm font-bold rounded-xl px-4 py-2.5 focus:ring-2 focus:ring-sky-500 outline-none shadow-[0_0_10px_rgba(14,165,233,0.1)]"
            />
          </div>

          {/* Cột 3: Thể Tích */}
          <div className="space-y-1.5 col-span-1 lg:col-span-1">
            <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400 ml-1">
              Tính ra (mL)
            </label>
            <input
              type="number"
              min="0"
              step="1"
              value={volumeMl}
              onChange={(e) => setVolumeMl(Number(e.target.value))}
              disabled={!isOnline || isProcessing}
              className="w-full bg-slate-800/80 border border-slate-700 text-amber-300 text-sm font-bold rounded-xl px-4 py-2.5 focus:ring-2 focus:ring-amber-500 outline-none"
            />
          </div>

          {/* Cột 4: Lưu lượng */}
          <div className="space-y-1.5 col-span-1 lg:col-span-1">
            <label className="text-[10px] font-bold uppercase tracking-widest text-slate-400 ml-1 flex justify-between">
              <span>Lưu lượng (mL/s)</span>
              {capacityMlPerSec === getCalibratedCapacity(selectedPump) && (
                <span className="text-emerald-500 hidden xl:block">Đồng bộ</span>
              )}
            </label>
            <input
              type="number"
              min="0.1"
              step="0.1"
              value={capacityMlPerSec}
              onChange={(e) => setCapacityMlPerSec(Number(e.target.value))}
              disabled={!isOnline || isProcessing}
              className={`w-full bg-slate-800/80 border text-sm font-bold rounded-xl px-4 py-2.5 focus:ring-2 outline-none transition-colors ${
                capacityMlPerSec === getCalibratedCapacity(selectedPump)
                  ? 'border-emerald-500/50 text-emerald-300 focus:ring-emerald-500'
                  : 'border-amber-500/50 text-amber-300 focus:ring-amber-500'
              }`}
            />
          </div>

          {/* Cột 5: Nút Bấm */}
          <button
            onClick={handleDose}
            disabled={!isOnline || isProcessing || volumeMl <= 0 || capacityMlPerSec <= 0}
            className="w-full h-[42px] col-span-2 lg:col-span-1 flex items-center justify-center gap-2 bg-emerald-500 text-white font-black text-[11px] uppercase tracking-widest rounded-xl hover:bg-emerald-400 hover:shadow-[0_0_20px_rgba(16,185,129,0.4)] transition-all disabled:opacity-50 disabled:hover:shadow-none active:scale-95"
          >
            <Play size={14} className={isProcessing ? 'animate-pulse' : ''} />
            Châm {durationSec} Giây
          </button>
        </div>
      </div>
    </div>
  );
};

// --- Component: Khối Điều Khiển Pha Lê ---
const AdvancedDeviceControl = ({
  deviceId,
  pumpId,
  title,
  icon: Icon,
  colorTheme,
  currentStatus,
  allowPwm = false,
  updatePumpStatusOptimistically,
  isOnline,
  isEmergency,
}: any) => {
  const { togglePump, setPwm, forceOn } = useDeviceControl(deviceId);

  const { pwmPreferences, savePwmPreference } = useDeviceContext();

  const [pwmValue, setPwmValue] = useState(pwmPreferences[pumpId] || 100);

  const [duration, setDuration] = useState(120);
  const [isProcessing, setIsProcessing] = useState(false);

  const theme = NEON_COLORS[colorTheme];
  const stateKey = pumpId.toLowerCase();

  useEffect(() => {
    if (pwmPreferences[pumpId] !== undefined) {
      setPwmValue(pwmPreferences[pumpId]);
    }
  }, [pwmPreferences, pumpId]);

  const handleToggle = async () => {
    if (isEmergency && !currentStatus) {
      toast.error(`🚨 FSM đang bảo vệ! Chỉ có thể dùng FORCE để bật ${title}.`);
      return;
    }

    setIsProcessing(true);
    const targetAction = currentStatus ? 'off' : 'on';
    const isNowOn = targetAction === 'on';

    updatePumpStatusOptimistically(stateKey, isNowOn);

    try {
      const success = await togglePump(pumpId, targetAction);
      if (!success) {
        updatePumpStatusOptimistically(stateKey, !isNowOn);
        toast.error(`Lỗi khi điều khiển ${title}!`);
      }
    } catch (error) {
      updatePumpStatusOptimistically(stateKey, !isNowOn);
      toast.error(`Lỗi mạng khi điều khiển ${title}!`);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleForceOn = async () => {
    if (!window.confirm(`⚠️ CẢNH BÁO: Bật cưỡng chế ${title} trong ${duration}s?`)) return;
    setIsProcessing(true);

    updatePumpStatusOptimistically(stateKey, true);

    try {
      const success = await forceOn(pumpId, duration);
      if (!success) {
        updatePumpStatusOptimistically(stateKey, false);
        toast.error(`Lỗi khi cưỡng chế ${title}!`);
      }
    } catch (error) {
      updatePumpStatusOptimistically(stateKey, false);
      toast.error(`Lỗi kết nối khi cưỡng chế ${title}!`);
    } finally {
      setIsProcessing(false);
    }
  };

  const handleSetPwm = async () => {
    if (isEmergency) {
      toast.error(`🚨 FSM đang bảo vệ! Lệnh PWM bình thường bị vô hiệu hóa.`);
      return;
    }

    setIsProcessing(true);
    if (!currentStatus) {
      updatePumpStatusOptimistically(stateKey, true);
    }

    await setPwm(pumpId, pwmValue);

    savePwmPreference(pumpId, pwmValue);

    setIsProcessing(false);
  };

  return (
    <div
      className={`relative bg-slate-900/40 backdrop-blur-xl rounded-[2rem] p-5 transition-all duration-500 overflow-hidden group ${
        currentStatus ? `border ${theme.border} ${theme.shadow}` : 'border border-white/5'
      }`}
    >
      <div
        className={`absolute inset-0 bg-gradient-to-br ${theme.glow} to-transparent opacity-0 transition-opacity duration-500 pointer-events-none ${currentStatus ? 'opacity-100' : ''}`}
      ></div>

      <div className="relative z-10">
        <div className="flex items-center justify-between mb-5">
          <div className="flex items-center space-x-3">
            <div
              className={`p-3 rounded-2xl transition-all duration-500 ${
                currentStatus
                  ? `${theme.bg} text-white shadow-lg`
                  : 'bg-slate-800 text-slate-400 group-hover:bg-slate-700'
              }`}
            >
              <Icon size={20} className={currentStatus ? 'animate-pulse' : ''} />
            </div>
            <div>
              <h3
                className={`font-black tracking-wide ${currentStatus ? 'text-white' : 'text-slate-300'}`}
              >
                {title}
              </h3>
              <p className="text-[10px] text-slate-500 font-mono tracking-widest">{pumpId}</p>
            </div>
          </div>

          <button
            disabled={!isOnline || isProcessing || (isEmergency && !currentStatus)}
            onClick={handleToggle}
            className={`px-5 py-2.5 rounded-xl font-black text-xs uppercase tracking-widest transition-all duration-300 active:scale-95 disabled:opacity-50 ${
              currentStatus
                ? 'bg-rose-500 text-white shadow-[0_0_15px_rgba(244,63,94,0.5)]'
                : isEmergency && !currentStatus
                  ? 'bg-slate-800/50 text-slate-600 border border-slate-800 cursor-not-allowed'
                  : 'bg-slate-800 text-slate-300 hover:bg-slate-700 hover:text-white border border-slate-700'
            }`}
          >
            {currentStatus ? 'TẮT' : 'BẬT'}
          </button>
        </div>

        <div
          className={`space-y-4 pt-4 border-t transition-all duration-500 ${currentStatus ? `border-${colorTheme}-500/20` : 'border-slate-800/50'}`}
        >
          {allowPwm && (
            <div className="space-y-2.5">
              <div className="flex justify-between items-center text-[10px] font-bold uppercase tracking-widest text-slate-400">
                <span className="flex items-center gap-1.5">
                  <Activity size={12} className={currentStatus ? theme.text : ''} /> Công suất (PWM)
                </span>
                <span className={currentStatus ? theme.text : 'text-white'}>{pwmValue}%</span>
              </div>
              <div className="flex items-center gap-3">
                <input
                  type="range"
                  min="10"
                  max="100"
                  step="10"
                  value={pwmValue}
                  onChange={(e) => setPwmValue(parseInt(e.target.value))}
                  disabled={!isOnline || isProcessing || isEmergency}
                  className={`flex-1 h-2 rounded-lg appearance-none bg-slate-800 accent-${colorTheme}-500 outline-none ${isEmergency ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`}
                />
                <button
                  onClick={handleSetPwm}
                  disabled={!isOnline || isProcessing || isEmergency}
                  className={`px-3 py-1.5 text-[10px] font-black uppercase tracking-wider rounded-lg transition-all disabled:opacity-50 ${
                    currentStatus && !isEmergency
                      ? `${theme.bg} text-white`
                      : 'bg-slate-800 text-slate-400 border border-slate-700 hover:bg-slate-700 hover:text-white'
                  }`}
                >
                  SET
                </button>
              </div>
            </div>
          )}

          <div className="space-y-2.5 pt-1">
            <div className="flex justify-between items-center text-[10px] font-bold uppercase tracking-widest text-slate-400">
              <span className="flex items-center gap-1.5">
                <Timer size={12} className="text-amber-500" /> Hẹn giờ chạy
              </span>
            </div>
            <div className="flex gap-2">
              <select
                value={duration}
                onChange={(e) => setDuration(Number(e.target.value))}
                disabled={!isOnline || isProcessing}
                className="bg-slate-800/50 border border-slate-700 text-slate-200 text-xs font-bold rounded-xl px-3 py-2 flex-1 focus:ring-2 focus:ring-amber-500 outline-none appearance-none"
              >
                <option value={30}>30 Giây</option>
                <option value={60}>1 Phút</option>
                <option value={120}>2 Phút</option>
                <option value={300}>5 Phút</option>
              </select>
              <button
                onClick={handleForceOn}
                disabled={!isOnline || isProcessing}
                className="flex items-center gap-1.5 px-4 py-2 bg-amber-500/10 text-amber-500 border border-amber-500/30 rounded-xl text-[10px] font-black uppercase tracking-wider hover:bg-amber-500 hover:text-white hover:shadow-[0_0_15px_rgba(245,158,11,0.5)] transition-all disabled:opacity-50"
              >
                <Zap size={14} className={isProcessing ? 'animate-pulse' : ''} /> FORCE
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

// --- Bảng Điều Khiển Chính ---
const ControlPanel = () => {
  const {
    deviceId,
    sensorData,
    deviceStatus,
    isControllerStatusKnown,
    isLoading,
    updatePumpStatusOptimistically,
    fsmState,
    settings,
  } = useDeviceContext();
  const { isProcessing, resetFault } = useDeviceControl(deviceId || '');

  if (isLoading || !sensorData) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-slate-950">
        <div className="animate-spin rounded-full h-10 w-10 border-t-2 border-b-2 border-emerald-500 shadow-[0_0_15px_rgba(16,185,129,0.5)]"></div>
      </div>
    );
  }

  const isOnline = deviceStatus?.is_online || false;
  const showDisconnected = isControllerStatusKnown && !isOnline;
  const pumps: Partial<PumpStatus> = isOnline ? sensorData.pump_status || {} : {};

  const isEmergency = Boolean(
    fsmState?.toUpperCase().includes('EMERGENCY') ||
    fsmState?.toUpperCase().includes('FAULT') ||
    fsmState?.toUpperCase().includes('LỖI'),
  );

  return (
    <div className="p-4 space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700 pb-32 relative min-h-screen">
      <div className="absolute top-0 right-0 w-[80%] h-64 bg-gradient-to-bl from-indigo-500/10 via-transparent to-transparent pointer-events-none blur-3xl"></div>

      {/* Header */}
      <div className="flex items-start justify-between relative z-10">
        <div className="space-y-1">
          <h1 className="text-3xl font-black bg-clip-text text-transparent bg-gradient-to-r from-emerald-400 to-indigo-400 tracking-tight flex items-center gap-3">
            ĐIỀU KHIỂN
            <Settings2 size={28} className="text-indigo-400" />
          </h1>
          <p className="text-sm text-slate-400 font-medium">Bảng can thiệp thủ công Override</p>
        </div>

        <button
          disabled={!isOnline || isProcessing}
          onClick={async () => {
            if (
              window.confirm(
                'CẢNH BÁO: Reset lỗi sẽ khởi động lại toàn bộ chu trình FSM. Xác nhận?',
              )
            )
              await resetFault();
          }}
          className="flex items-center gap-2 px-4 py-2.5 bg-rose-500/10 text-rose-500 border border-rose-500/30 rounded-xl font-black text-[10px] uppercase tracking-widest hover:bg-rose-500 hover:text-white hover:shadow-[0_0_15px_rgba(244,63,94,0.5)] transition-all disabled:opacity-50 disabled:hover:bg-rose-500/10 disabled:hover:text-rose-500 disabled:hover:shadow-none"
        >
          <RefreshCw size={14} className={isProcessing ? 'animate-spin' : ''} />
          Reset Lỗi
        </button>
      </div>

      {showDisconnected && (
        <div className="bg-rose-500/10 backdrop-blur-md border border-rose-500/30 rounded-2xl p-4 flex items-center gap-3 text-rose-400 shadow-[0_0_20px_rgba(244,63,94,0.15)] relative z-10 animate-in zoom-in">
          <AlertTriangle size={24} className="animate-pulse" />
          <div>
            <h4 className="font-bold text-sm uppercase tracking-wider">Mất kết nối trạm</h4>
            <p className="text-xs text-rose-400/80">Không thể gửi lệnh điều khiển lúc này.</p>
          </div>
        </div>
      )}

      {isEmergency && isOnline && (
        <div className="bg-amber-500/10 backdrop-blur-md border border-amber-500/30 rounded-2xl p-4 flex items-center gap-3 text-amber-400 shadow-[0_0_20px_rgba(245,158,11,0.15)] relative z-10 animate-in zoom-in">
          <AlertTriangle size={24} className="animate-pulse" />
          <div>
            <h4 className="font-bold text-sm uppercase tracking-wider">
              Hệ Thống Đang Lỗi / Khẩn Cấp
            </h4>
            <p className="text-xs text-amber-400/80">
              Các lệnh bật bình thường và set PWM đã bị khóa. Hãy sử dụng lệnh <b>FORCE</b>.
            </p>
          </div>
        </div>
      )}

      {/* KHỐI CHÂM BÁN THỦ CÔNG */}
      <SemiAutoDosingAssistant
        deviceId={deviceId}
        isOnline={isOnline}
        dosingCalibration={settings?.dosing_calibration}
        sensorData={sensorData}
      />

      {/* Khu Vực Dinh Dưỡng */}
      <div className="space-y-4 relative z-10">
        <h2 className="text-[10px] uppercase tracking-widest font-black text-slate-500 pl-3 flex items-center gap-2">
          <FlaskConical size={14} className="text-orange-500" /> Khu Vực Châm Phân & pH
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
          <AdvancedDeviceControl
            deviceId={deviceId}
            pumpId="PUMP_A"
            title="Bơm Phân A"
            icon={FlaskConical}
            colorTheme="orange"
            currentStatus={pumps.pump_a}
            allowPwm={true}
            updatePumpStatusOptimistically={updatePumpStatusOptimistically}
            isOnline={isOnline}
            isEmergency={isEmergency}
          />
          <AdvancedDeviceControl
            deviceId={deviceId}
            pumpId="PUMP_B"
            title="Bơm Phân B"
            icon={FlaskConical}
            colorTheme="orange"
            currentStatus={pumps.pump_b}
            allowPwm={true}
            updatePumpStatusOptimistically={updatePumpStatusOptimistically}
            isOnline={isOnline}
            isEmergency={isEmergency}
          />
          <AdvancedDeviceControl
            deviceId={deviceId}
            pumpId="PH_UP"
            title="Bơm Tăng pH"
            icon={Activity}
            colorTheme="purple"
            currentStatus={pumps.ph_up}
            allowPwm={true}
            updatePumpStatusOptimistically={updatePumpStatusOptimistically}
            isOnline={isOnline}
            isEmergency={isEmergency}
          />
          <AdvancedDeviceControl
            deviceId={deviceId}
            pumpId="PH_DOWN"
            title="Bơm Giảm pH"
            icon={Activity}
            colorTheme="purple"
            currentStatus={pumps.ph_down}
            allowPwm={true}
            updatePumpStatusOptimistically={updatePumpStatusOptimistically}
            isOnline={isOnline}
            isEmergency={isEmergency}
          />
        </div>
      </div>

      {/* Khu Vực Bơm Nước & Khí Hậu */}
      <div className="space-y-4 pt-4 relative z-10">
        <h2 className="text-[10px] uppercase tracking-widest font-black text-slate-500 pl-3 flex items-center gap-2">
          <Waves size={14} className="text-cyan-500" /> Bơm Nước & Sục Trộn
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
          <AdvancedDeviceControl
            deviceId={deviceId}
            pumpId="WATER_PUMP_IN"
            title="Cấp Nước"
            icon={Droplets}
            colorTheme="blue"
            currentStatus={pumps.water_pump_in}
            allowPwm={false}
            updatePumpStatusOptimistically={updatePumpStatusOptimistically}
            isOnline={isOnline}
            isEmergency={isEmergency}
          />
          <AdvancedDeviceControl
            deviceId={deviceId}
            pumpId="WATER_PUMP_OUT"
            title="Xả Nước"
            icon={Droplets}
            colorTheme="cyan"
            currentStatus={pumps.water_pump_out}
            allowPwm={false}
            updatePumpStatusOptimistically={updatePumpStatusOptimistically}
            isOnline={isOnline}
            isEmergency={isEmergency}
          />
          <AdvancedDeviceControl
            deviceId={deviceId}
            pumpId="OSAKA"
            title="Trộn Osaka"
            icon={Power}
            colorTheme="indigo"
            currentStatus={pumps.osaka_pump}
            allowPwm={true}
            updatePumpStatusOptimistically={updatePumpStatusOptimistically}
            isOnline={isOnline}
            isEmergency={isEmergency}
          />
          <AdvancedDeviceControl
            deviceId={deviceId}
            pumpId="MIST"
            title="Phun Sương"
            icon={Wind}
            colorTheme="sky"
            currentStatus={pumps.mist_valve}
            allowPwm={false}
            updatePumpStatusOptimistically={updatePumpStatusOptimistically}
            isOnline={isOnline}
            isEmergency={isEmergency}
          />
        </div>
      </div>
    </div>
  );
};

export default ControlPanel;
