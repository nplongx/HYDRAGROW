import { Droplets, Thermometer, Activity, Waves, Settings, Zap, Cpu, Wifi, HardDrive, Clock, AlertTriangle, Server, RadioReceiver } from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';
import { useDeviceControl } from '../hooks/useDeviceControl';

import { SensorBentoCard } from '../components/ui/SensorBentoCard';
import { FsmStatusBadge } from '../components/ui/FsmStatusBadge';
import { LoadingState } from '../components/ui/LoadingState';

const ActiveDeviceTag = ({ label, color }: { label: string; color: string }) => (
  <span className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium border ${color}`}>
    <Zap size={14} className="fill-current" />
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
  if (rssi > -60) return "text-emerald-500";
  if (rssi > -75) return "text-amber-500";
  return "text-red-500";
};

const HealthBar = ({ title, icon: Icon, data, isNodeOnline }: { title: string, icon: any, data?: any, isNodeOnline: boolean }) => (
  <div className={`flex flex-col gap-3 p-4 rounded-xl border transition-colors ${isNodeOnline ? 'bg-slate-900 border-slate-800' : 'bg-slate-900/50 border-red-500/30'}`}>
    <div className="flex items-center justify-between">
      <div className="flex items-center gap-2 text-slate-300">
        <Icon size={16} className={isNodeOnline ? "text-blue-500" : "text-red-500"} />
        <span className="text-xs font-semibold">{title}</span>
      </div>
      <div className={`w-2 h-2 rounded-full ${isNodeOnline ? 'bg-emerald-500' : 'bg-red-500'}`}></div>
    </div>

    <div className={`flex items-center gap-2 overflow-x-auto hide-scrollbar ${!isNodeOnline ? 'opacity-50 grayscale' : ''}`}>
      <div className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-slate-950 border border-slate-800 text-[11px] font-medium text-slate-400 whitespace-nowrap">
        <Wifi size={12} className={getWifiColor(data?.rssi)} />
        {data?.rssi ? `${data.rssi} dBm` : '--'}
      </div>
      <div className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-slate-950 border border-slate-800 text-[11px] font-medium text-slate-400 whitespace-nowrap">
        <HardDrive size={12} className="text-slate-400" />
        {data?.free_heap ? `${(data.free_heap / 1024).toFixed(1)} KB` : '--'}
      </div>
      <div className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-slate-950 border border-slate-800 text-[11px] font-medium text-slate-400 whitespace-nowrap">
        <Clock size={12} className="text-slate-400" />
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
      <div className="flex flex-col items-center justify-center h-full min-h-[80vh] space-y-4 p-6 text-center">
        <div className="p-5 bg-slate-900 rounded-2xl border border-slate-800">
          <Settings size={36} className="text-slate-500" />
        </div>
        <div className="space-y-1">
          <h2 className="text-lg font-semibold text-slate-100">Chưa kết nối Trạm</h2>
          <p className="text-sm text-slate-500 max-w-xs mx-auto">
            Vui lòng vào mục Cài đặt để thiết lập ID thiết bị.
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
    <div className="p-4 md:p-8 space-y-6 pb-28 max-w-5xl mx-auto">

      {/* HEADER KHU VỰC TRẠM */}
      <div className="flex flex-col space-y-1.5">
        <h1 className="text-2xl font-semibold text-slate-100 tracking-tight">
          Trạm Trung Tâm
        </h1>
        <div className="flex items-center gap-2">
          <div className={`flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium border ${isOnline ? 'bg-emerald-500/10 border-emerald-500/20 text-emerald-400' :
            isControllerStatusKnown ? 'bg-red-500/10 border-red-500/20 text-red-400' :
              'bg-amber-500/10 border-amber-500/20 text-amber-400'
            }`}>
            <span className={`w-1.5 h-1.5 rounded-full ${isOnline ? 'bg-emerald-500' : (isControllerStatusKnown ? 'bg-red-500' : 'bg-amber-500')}`}></span>
            {isOnline ? 'Đang Hoạt Động' : (isControllerStatusKnown ? 'Mất Kết Nối' : 'Đang Kết Nối...')}
          </div>
          <span className="text-xs text-slate-500">{deviceId}</span>
        </div>
      </div>

      {/* HEALTH BARS */}
      {isOnline && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <HealthBar title="Controller Node" icon={Server} data={controllerHealth} isNodeOnline={true} />
          <HealthBar title="Sensor Node" icon={RadioReceiver} data={sensorData} isNodeOnline={isSensorOnline} />
        </div>
      )}

      {/* TIẾN TRÌNH FSM & HOẠT ĐỘNG BƠM */}
      <div className="bg-slate-900 border border-slate-800 rounded-2xl p-5">
        <div className="flex items-center justify-between mb-5">
          <div className="flex items-center gap-2">
            <Cpu size={18} className="text-slate-500" />
            <span className="text-sm font-semibold text-slate-300">Tiến trình FSM</span>
          </div>
          <FsmStatusBadge state={fsmState} />
        </div>

        <div className="pt-4 border-t border-slate-800">
          <p className="text-xs font-medium text-slate-500 mb-3">Đang tiêu thụ điện:</p>
          <div className="flex flex-wrap gap-2">
            {pumps.pump_a && <ActiveDeviceTag label="Bơm Phân A" color="bg-orange-500/10 text-orange-500 border-orange-500/20" />}
            {pumps.pump_b && <ActiveDeviceTag label="Bơm Phân B" color="bg-orange-500/10 text-orange-500 border-orange-500/20" />}
            {pumps.ph_up && <ActiveDeviceTag label="Tăng pH" color="bg-purple-500/10 text-purple-400 border-purple-500/20" />}
            {pumps.ph_down && <ActiveDeviceTag label="Giảm pH" color="bg-purple-500/10 text-purple-400 border-purple-500/20" />}
            {pumps.osaka_pump && <ActiveDeviceTag label="Bơm Trộn" color="bg-indigo-500/10 text-indigo-400 border-indigo-500/20" />}
            {pumps.mist_valve && <ActiveDeviceTag label="Phun Sương" color="bg-sky-500/10 text-sky-400 border-sky-500/20" />}
            {pumps.water_pump_in && <ActiveDeviceTag label="Cấp Nước" color="bg-blue-500/10 text-blue-400 border-blue-500/20" />}
            {pumps.water_pump_out && <ActiveDeviceTag label="Xả Nước" color="bg-cyan-500/10 text-cyan-400 border-cyan-500/20" />}

            {!Object.values(pumps).some(v => v === true) && (
              <span className="text-xs font-medium text-slate-500 px-2.5 py-1 rounded-md bg-slate-950 border border-slate-800">
                Hệ thống đang nghỉ
              </span>
            )}
          </div>
        </div>
      </div>

      {/* LƯỚI CẢM BIẾN */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="relative">
          {sensorData?.err_ec === true && (
            <div className="absolute -top-1.5 -right-1.5 z-10 bg-red-500 text-white p-1 rounded-md shadow-sm">
              <AlertTriangle size={14} />
            </div>
          )}
          <div className={sensorData?.err_ec === true ? "opacity-60" : ""}>
            <SensorBentoCard
              title="EC"
              value={sensorData?.err_ec === true ? -1 : sensorData?.ec}
              unit="mS/cm"
              icon={Activity}
              theme={sensorData?.err_ec === true ? "rose" : "blue"}
            />
          </div>
        </div>

        <div className="relative">
          {sensorData?.err_ph === true && (
            <div className="absolute -top-1.5 -right-1.5 z-10 bg-red-500 text-white p-1 rounded-md shadow-sm">
              <AlertTriangle size={14} />
            </div>
          )}
          <div className={sensorData?.err_ph === true ? "opacity-60" : ""}>
            <SensorBentoCard
              title="pH"
              value={sensorData?.err_ph === true ? -1 : sensorData?.ph}
              icon={Droplets}
              theme={sensorData?.err_ph === true ? "rose" : "fuchsia"}
            />
          </div>
        </div>

        <div className="relative">
          {sensorData?.err_temp === true && (
            <div className="absolute -top-1.5 -right-1.5 z-10 bg-red-500 text-white p-1 rounded-md shadow-sm">
              <AlertTriangle size={14} />
            </div>
          )}
          <div className={sensorData?.err_temp === true ? "opacity-60" : ""}>
            <SensorBentoCard
              title="Nhiệt độ"
              value={sensorData?.err_temp === true ? -1 : sensorData?.temp}
              unit="°C"
              icon={Thermometer}
              theme={sensorData?.err_temp === true ? "rose" : "orange"}
            />
          </div>
        </div>

        <div className="relative">
          {sensorData?.err_water === true && (
            <div className="absolute -top-1.5 -right-1.5 z-10 bg-red-500 text-white p-1 rounded-md shadow-sm">
              <AlertTriangle size={14} />
            </div>
          )}
          <div className={sensorData?.err_water === true ? "opacity-60" : ""}>
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

      {/* ĐIỀU KHIỂN NHANH (NƯỚC) */}
      <div className="bg-slate-900 border border-slate-800 rounded-2xl p-5">
        <h3 className="text-sm font-semibold text-slate-300 mb-4 flex items-center gap-2">
          <Zap size={16} className="text-slate-500" /> Cưỡng chế bơm nước
        </h3>
        <div className="flex gap-3">
          <button
            disabled={isProcessing || !isOnline}
            onClick={() => handleToggle("WATER_PUMP_IN", pumps.water_pump_in)}
            className={`flex-1 py-3 rounded-xl font-medium text-sm transition-colors border flex items-center justify-center gap-2 disabled:opacity-50 ${pumps.water_pump_in
              ? 'bg-red-500/10 text-red-500 border-red-500/30'
              : 'bg-slate-950 text-blue-400 border-slate-800 hover:border-blue-500/30 hover:bg-slate-800'
              }`}
          >
            <Waves size={16} />
            {pumps.water_pump_in ? 'Ngừng Cấp' : 'Cấp Nước'}
          </button>

          <button
            disabled={isProcessing || !isOnline}
            onClick={() => handleToggle("WATER_PUMP_OUT", pumps.water_pump_out)}
            className={`flex-1 py-3 rounded-xl font-medium text-sm transition-colors border flex items-center justify-center gap-2 disabled:opacity-50 ${pumps.water_pump_out
              ? 'bg-red-500/10 text-red-500 border-red-500/30'
              : 'bg-slate-950 text-cyan-400 border-slate-800 hover:border-cyan-500/30 hover:bg-slate-800'
              }`}
          >
            <Waves size={16} className="rotate-180" />
            {pumps.water_pump_out ? 'Ngừng Xả' : 'Xả Nước'}
          </button>
        </div>
      </div>

    </div>
  );
};

export default Dashboard;
