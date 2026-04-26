import { Droplets, Thermometer, Activity, Waves, Settings, Zap, Cpu, Wifi, HardDrive, Clock, AlertTriangle, Server, RadioReceiver } from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';
import { useDeviceControl } from '../hooks/useDeviceControl';

import { SensorBentoCard } from '../components/ui/SensorBentoCard';
import { FsmStatusBadge } from '../components/ui/FsmStatusBadge';
import { LoadingState } from '../components/ui/LoadingState';

const ActiveDeviceTag = ({ label, color, glowColor }: { label: string; color: string; glowColor: string }) => (
  <span
    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl border text-[10px] font-extrabold uppercase tracking-widest backdrop-blur-md ${color}`}
    style={{ boxShadow: `0 0 12px ${glowColor}, inset 0 0 8px ${glowColor}` }}
  >
    <Zap size={12} className="fill-current" />
    {label}
  </span>
);

const formatUptime = (seconds?: number) => {
  if (seconds === undefined || seconds === null) return "--";
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
};

const getWifiColor = (rssi?: number) => {
  if (rssi === undefined) return "text-slate-500";
  if (rssi > -60) return "text-emerald-400";
  if (rssi > -75) return "text-amber-400";
  return "text-rose-500";
};

const HealthBar = ({ title, icon: Icon, data, isNodeOnline }: { title: string, icon: any, data?: any, isNodeOnline: boolean }) => (
  <div className={`flex flex-col gap-2 bg-slate-900/60 border p-3 rounded-2xl shadow-inner transition-colors duration-500 ${isNodeOnline ? 'border-slate-700/80 shadow-sm' : 'border-rose-500/30 shadow-sm'}`}>
    <div className="flex items-center justify-between">
      <div className="flex items-center gap-1.5 text-slate-400">
        <Icon size={14} className={isNodeOnline ? "text-indigo-400" : "text-rose-400"} />
        <span className="text-[10px] font-black uppercase tracking-widest">{title}</span>
      </div>
      {/* Nút báo hiệu Online/Offline nhỏ xíu ở góc */}
      <span className="relative flex h-2 w-2 rounded-full">
        <span className={`absolute inline-flex h-full w-full rounded-full opacity-40 ${isNodeOnline ? 'bg-emerald-400' : 'bg-rose-500'}`}></span>
        <span className={`relative inline-flex rounded-full h-2 w-2 ${isNodeOnline ? 'bg-emerald-500' : 'bg-rose-500'}`}></span>
      </span>
    </div>

    <div className={`flex items-center gap-2 overflow-x-auto hide-scrollbar ${!isNodeOnline ? 'opacity-50 grayscale' : ''}`}>
      <div className="flex items-center gap-1.5 px-2 py-1 rounded-lg border border-slate-800/80 bg-slate-950/80 text-[10px] font-mono text-slate-300 whitespace-nowrap">
        <Wifi size={12} className={getWifiColor(data?.rssi)} />
        {data?.rssi ? `${data.rssi} dBm` : '--'}
      </div>
      <div className="flex items-center gap-1.5 px-2 py-1 rounded-lg border border-slate-800/80 bg-slate-950/80 text-[10px] font-mono text-slate-300 whitespace-nowrap">
        <HardDrive size={12} className="text-cyan-400" />
        {data?.free_heap ? `${(data.free_heap / 1024).toFixed(1)} KB` : '--'}
      </div>
      <div className="flex items-center gap-1.5 px-2 py-1 rounded-lg border border-slate-800/80 bg-slate-950/80 text-[10px] font-mono text-slate-300 whitespace-nowrap">
        <Clock size={12} className="text-emerald-400" />
        {formatUptime(data?.uptime || data?.uptime_sec)}
      </div>
    </div>
  </div>
);

const Dashboard = () => {
  const { deviceId, sensorData, deviceStatus, isControllerStatusKnown, controllerHealth, fsmState, isLoading, updatePumpStatusOptimistically, isSensorOnline } = useDeviceContext();
  const { isProcessing, togglePump } = useDeviceControl(deviceId || "");

  if (isLoading || !sensorData) {
    return <LoadingState message="Đang tải tổng quan thiết bị..." />;
  }

  if (!deviceId) {
    return (
      <div className="flex flex-col items-center justify-center h-full min-h-[80vh] space-y-6 p-6 text-center">
        <div className="relative p-6 bg-slate-900/80 rounded-full border border-slate-700 shadow-md group">
          <div className="absolute inset-0 bg-indigo-500/20 rounded-full blur-2xl group-hover:bg-indigo-500/30 transition-colors"></div>
          <Settings size={48} className="text-indigo-400 relative z-10" />
        </div>
        <div className="space-y-2">
          <h2 className="text-2xl font-bold text-slate-100">
            CHƯA KẾT NỐI TRẠM
          </h2>
          <p className="text-sm text-slate-300 max-w-xs mx-auto leading-7">
            Vui lòng vào mục Cài đặt để nhập Device ID và bắt đầu đồng bộ dữ liệu.
          </p>
        </div>
      </div>
    );
  }

  const isOnline = deviceStatus?.is_online;
  const pumps: any = isOnline && sensorData?.pump_status ? sensorData.pump_status : {};

  const handleToggle = async (pumpId: string, currentStatus: boolean | undefined) => {
    const targetAction = currentStatus ? 'off' : 'on';
    updatePumpStatusOptimistically(pumpId, targetAction === 'on');
    const success = await togglePump(pumpId, targetAction);
    if (!success) updatePumpStatusOptimistically(pumpId, !!currentStatus);
  };

  return (
    <div className="p-4 space-y-6 pb-32 relative min-h-screen">
      
      <div className="flex flex-col relative z-10">
        <div className="flex items-start justify-between mb-4">
          <div className="space-y-1">
            <h1 className="text-3xl font-bold text-slate-100 tracking-tight">
              TRẠM TRUNG TÂM
            </h1>
            <div className="flex items-center mt-1 space-x-2">
              <div className={`flex items-center gap-2 px-2.5 py-1 rounded-full border backdrop-blur-md ${isOnline ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400 shadow-sm' :
                isControllerStatusKnown ? 'bg-rose-500/10 border-rose-500/30 text-rose-400 shadow-sm' :
                  'bg-amber-500/10 border-amber-500/30 text-amber-400 shadow-sm'
                }`}>
                <span className={`relative flex h-2 w-2 rounded-full ${isOnline ? 'bg-emerald-400' : (isControllerStatusKnown ? 'bg-rose-500' : 'bg-amber-500')}`}>
                  
                </span>
                <span className="text-[10px] font-bold uppercase tracking-wider">
                  {isOnline ? 'Đang Hoạt Động' : (isControllerStatusKnown ? 'Mất Kết Nối' : 'Đang Kết Nối...')}
                </span>
              </div>
              <span className="text-xs text-slate-500 font-mono">{deviceId}</span>
            </div>
          </div>
        </div>

        {/* KHU VỰC HIỂN THỊ SỨC KHỎE 2 NODE */}
        {isOnline && (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mt-2">
            <HealthBar title="Controller Node" icon={Server} data={controllerHealth} isNodeOnline={true} />
            <HealthBar title="Sensor Node" icon={RadioReceiver} data={sensorData} isNodeOnline={isSensorOnline} />
          </div>
        )}
      </div>

      <div className="relative bg-slate-900/70 border border-slate-700 rounded-[2rem] p-5 shadow-md overflow-hidden group">

        <div className="relative z-10 flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Cpu size={16} className="text-slate-400" />
            <span className="text-xs font-black uppercase tracking-widest text-slate-400">Tiến trình FSM</span>
          </div>
          <FsmStatusBadge state={fsmState} />
        </div>

        <div className="relative z-10 pt-4 border-t border-slate-700/50">
          <p className="text-xs font-semibold text-slate-400 uppercase tracking-wide mb-3 flex items-center gap-1.5">
            <span className="w-1 h-1 rounded-full bg-slate-500"></span>
            Đang tiêu thụ điện:
          </p>
          <div className="flex flex-wrap gap-2.5">
            {pumps.pump_a && <ActiveDeviceTag label="Bơm Phân A" color="bg-orange-500/10 text-orange-400 border-orange-500/50" glowColor="rgba(249,115,22,0.25)" />}
            {pumps.pump_b && <ActiveDeviceTag label="Bơm Phân B" color="bg-orange-500/10 text-orange-400 border-orange-500/50" glowColor="rgba(249,115,22,0.25)" />}
            {pumps.ph_up && <ActiveDeviceTag label="Tăng pH" color="bg-purple-500/10 text-purple-400 border-purple-500/50" glowColor="rgba(168,85,247,0.25)" />}
            {pumps.ph_down && <ActiveDeviceTag label="Giảm pH" color="bg-purple-500/10 text-purple-400 border-purple-500/50" glowColor="rgba(168,85,247,0.25)" />}
            {pumps.osaka_pump && <ActiveDeviceTag label="Bơm Trộn" color="bg-indigo-500/10 text-indigo-400 border-indigo-500/50" glowColor="rgba(99,102,241,0.25)" />}
            {pumps.mist_valve && <ActiveDeviceTag label="Phun Sương" color="bg-sky-500/10 text-sky-400 border-sky-500/50" glowColor="rgba(14,165,233,0.25)" />}
            {pumps.water_pump_in && <ActiveDeviceTag label="Cấp Nước" color="bg-blue-500/10 text-blue-400 border-blue-500/50" glowColor="rgba(59,130,246,0.25)" />}
            {pumps.water_pump_out && <ActiveDeviceTag label="Xả Nước" color="bg-cyan-500/10 text-cyan-400 border-cyan-500/50" glowColor="rgba(6,182,212,0.25)" />}

            {!Object.values(pumps).some(v => v === true) && (
              <span className="text-sm text-slate-400 font-medium flex items-center gap-2 bg-slate-900/50 px-3 py-1.5 rounded-lg border border-slate-800">
                <div className="w-1.5 h-1.5 rounded-full bg-slate-600"></div>
                Hệ thống đang nghỉ
              </span>
            )}
          </div>
        </div>
      </div>

      {/* 3. LƯỚI CẢM BIẾN */}
      <div className="grid grid-cols-2 gap-4 relative z-10">

        {/* Thẻ Dinh Dưỡng EC */}
        <div className="relative">
          {sensorData?.err_ec === true && (
            <div className="absolute -top-2 -right-2 z-20 bg-rose-500 text-white p-1 rounded-full shadow-md">
              <AlertTriangle size={14} />
            </div>
          )}
          <div className={sensorData?.err_ec === true ? "opacity-50 ring-2 ring-rose-500/50 rounded-[2rem] transition-all duration-300" : "transition-all duration-300"}>
            <SensorBentoCard
              title="Dinh dưỡng (EC)"
              value={sensorData?.err_ec === true ? -1 : sensorData?.ec}
              unit="mS/cm"
              icon={Activity}
              theme={sensorData?.err_ec === true ? "rose" : "blue"}
            />
          </div>
        </div>

        {/* Thẻ pH */}
        <div className="relative">
          {sensorData?.err_ph === true && (
            <div className="absolute -top-2 -right-2 z-20 bg-rose-500 text-white p-1 rounded-full shadow-md">
              <AlertTriangle size={14} />
            </div>
          )}
          <div className={sensorData?.err_ph === true ? "opacity-50 ring-2 ring-rose-500/50 rounded-[2rem] transition-all duration-300" : "transition-all duration-300"}>
            <SensorBentoCard
              title="Độ pH"
              value={sensorData?.err_ph === true ? -1 : sensorData?.ph}
              icon={Droplets}
              theme={sensorData?.err_ph === true ? "rose" : "fuchsia"}
            />
          </div>
        </div>

        {/* Thẻ Nhiệt Độ */}
        <div className="relative">
          {sensorData?.err_temp === true && (
            <div className="absolute -top-2 -right-2 z-20 bg-rose-500 text-white p-1 rounded-full shadow-md">
              <AlertTriangle size={14} />
            </div>
          )}
          <div className={sensorData?.err_temp === true ? "opacity-50 ring-2 ring-rose-500/50 rounded-[2rem] transition-all duration-300" : "transition-all duration-300"}>
            <SensorBentoCard
              title="Nhiệt độ"
              value={sensorData?.err_temp === true ? -1 : sensorData?.temp}
              unit="°C"
              icon={Thermometer}
              theme={sensorData?.err_temp === true ? "rose" : "orange"}
            />
          </div>
        </div>

        {/* Thẻ Mực Nước */}
        <div className="relative">
          {sensorData?.err_water === true && (
            <div className="absolute -top-2 -right-2 z-20 bg-rose-500 text-white p-1 rounded-full shadow-md">
              <AlertTriangle size={14} />
            </div>
          )}
          <div className={sensorData?.err_water === true ? "opacity-50 ring-2 ring-rose-500/50 rounded-[2rem] transition-all duration-300" : "transition-all duration-300"}>
            <SensorBentoCard
              title="Mực nước"
              value={sensorData?.err_water === true ? -1 : sensorData?.water_level}
              unit="%"
              icon={Waves}
              theme={sensorData?.err_water === true ? "rose" : "cyan"}
            />
          </div>
        </div>

      </div>

      <div className="bg-slate-900/40 backdrop-blur-lg border border-slate-800/80 rounded-[2rem] p-5 relative z-10">
        <h3 className="text-[10px] font-black text-slate-500 uppercase tracking-widest mb-4 flex items-center gap-2">
          <Zap size={14} className="text-amber-500" /> Cưỡng chế nhanh
        </h3>
        <div className="flex space-x-3">
          <button
            disabled={isProcessing || !isOnline}
            onClick={() => handleToggle("WATER_PUMP_IN", pumps.water_pump_in)}
            className={`flex-1 py-4 rounded-2xl font-black text-xs uppercase tracking-wider flex items-center justify-center space-x-2 transition-all duration-300 active:scale-95 disabled:opacity-50 disabled:active:scale-100 border ${pumps.water_pump_in
              ? 'bg-rose-500/20 text-rose-400 border-rose-500/50 shadow-[0_0_20px_rgba(244,63,94,0.3)]'
              : 'bg-slate-800/50 text-blue-400 border-blue-500/20 hover:bg-blue-500/10 hover:border-blue-500/40 shadow-inner'
              }`}
          >
            <Waves size={18} className="" />
            <span>{pumps.water_pump_in ? 'NGỪNG CẤP' : 'CẤP NƯỚC'}</span>
          </button>

          <button
            disabled={isProcessing || !isOnline}
            onClick={() => handleToggle("WATER_PUMP_OUT", pumps.water_pump_out)}
            className={`flex-1 py-4 rounded-2xl font-black text-xs uppercase tracking-wider flex items-center justify-center space-x-2 transition-all duration-300 active:scale-95 disabled:opacity-50 disabled:active:scale-100 border ${pumps.water_pump_out
              ? 'bg-rose-500/20 text-rose-400 border-rose-500/50 shadow-[0_0_20px_rgba(244,63,94,0.3)]'
              : 'bg-slate-800/50 text-cyan-400 border-cyan-500/20 hover:bg-cyan-500/10 hover:border-cyan-500/40 shadow-inner'
              }`}
          >
            <Waves size={18} className="rotate-180" />
            <span>{pumps.water_pump_out ? 'NGỪNG XẢ' : 'XẢ NƯỚC'}</span>
          </button>
        </div>
      </div>
    </div>
  );
};

export default Dashboard;
