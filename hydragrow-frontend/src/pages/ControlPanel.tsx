import { Settings2, RefreshCw, Sparkles, FlaskConical, Activity, Droplets, Power, Wind } from 'lucide-react';

// --- ZUSTAND, GLEAM & HOOKS ---
import { useDeviceStore } from '../store/useDeviceStore';
import { useDeviceControl, INTERLOCK_PAIRS } from '../hooks/useDeviceControl';
import { extract_fault_code_str } from '../../gleam_core/build/dev/javascript/gleam_core/fsm.mjs';
import { get_fault_guide } from '../../gleam_core/build/dev/javascript/gleam_core/faults.mjs';

// --- UI COMPONENTS ---
import { AdvancedDeviceControl } from '../components/control/AdvancedDeviceControl';
import { ActiveRecipeStatus } from '../components/recipes/ActiveRecipeStatus';
import { LoadingState } from '../components/ui/LoadingState';
import { Banner } from '../components/ui/Banner';
import { Button } from '../components/ui/Button';
import { PumpStatus } from '../types/models';

const PUMP_DISPLAY_LABEL: Record<string, string> = {
  PH_UP: 'Bơm pH Up',
  PH_DOWN: 'Bơm pH Down',
  WATER_PUMP_IN: 'Van cấp nước',
  WATER_PUMP_OUT: 'Bơm xả thoát',
};

const PUMP_STATUS_KEY: Record<string, string> = {
  PH_UP: 'ph_up',
  PH_DOWN: 'ph_down',
  WATER_PUMP_IN: 'water_pump_in',
  WATER_PUMP_OUT: 'water_pump_out',
};

const lockedByFor = (pumpId: string, pumps: Partial<PumpStatus>) => {
  const partnerId = INTERLOCK_PAIRS[pumpId];
  if (!partnerId) return undefined;
  const partnerKey = PUMP_STATUS_KEY[partnerId];
  const partnerRunning = partnerKey ? Boolean((pumps as any)[partnerKey]) : false;
  return partnerRunning ? partnerId : undefined;
};

const ControlPanel = ({ variant = 'standalone' }: { variant?: 'standalone' | 'embedded' }) => {
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

  const content = (
    <>
      {/* Cảnh báo sự cố / Mất kết nối */}
      <div className="space-y-3 mt-3">
        {showDisconnected && (
          <Banner tone="danger" title="Hệ thống Ngoại tuyến">
            Không thể truyền lệnh do mất kết nối Wi-Fi.
          </Banner>
        )}
        {isEmergency && isOnline && !isAutoMode && (
          <Banner tone="warning" title="Hệ thống đang ngắt khẩn cấp">
            {faultGuide?.short || 'Phát hiện sự cố an toàn.'}
            {faultGuide && (
              <span className="inline-block mt-1 bg-pill px-2 py-1 rounded-lg border border-line max-w-max text-[11px] font-medium text-text-muted">
                Khắc phục: {faultGuide.action}
              </span>
            )}
          </Banner>
        )}
      </div>

      <ActiveRecipeStatus />

      {/* Lưới điều khiển Bento Grid */}
      <div className="relative border border-line rounded-3xl p-5 md:p-6 bg-white/80 backdrop-blur-sm space-y-6 overflow-hidden shadow-sm shadow-primary/5 mt-4">
        {/* Frosted Glass Overlay khi ở chế độ Tự Động */}
        {isAutoMode && isOnline && (
          <div className="absolute inset-0 z-40 bg-emerald-50/80 backdrop-blur-[4px] flex flex-col items-center justify-center p-6 text-center animate-fadeIn select-none">
            <div className="p-4 bg-pill border border-pill rounded-2xl mb-3 shadow-xl shadow-primary/10">
              <Sparkles size={28} className="text-primary animate-pulse" />
            </div>
            <h4 className="text-base font-bold text-primary-deep tracking-tight">Trạm đang chạy Tự Động</h4>
            <p className="text-xs text-text-muted max-w-xs leading-relaxed mt-1">
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
            <AdvancedDeviceControl
              deviceId={deviceId}
              pumpId="PH_UP"
              title="Bơm pH Up"
              icon={Activity}
              currentStatus={Boolean(pumps.ph_up)}
              allowPwm={true}
              colorTheme="purple"
              canSendCommands={canSendCommands}
              isEmergency={isEmergency}
              isAutoMode={isAutoMode}
              lockedByPumpId={lockedByFor('PH_UP', pumps)}
              lockedByPumpLabel={PUMP_DISPLAY_LABEL[lockedByFor('PH_UP', pumps) || '']}
            />
            <AdvancedDeviceControl
              deviceId={deviceId}
              pumpId="PH_DOWN"
              title="Bơm pH Down"
              icon={Activity}
              currentStatus={Boolean(pumps.ph_down)}
              allowPwm={true}
              colorTheme="fuchsia"
              canSendCommands={canSendCommands}
              isEmergency={isEmergency}
              isAutoMode={isAutoMode}
              lockedByPumpId={lockedByFor('PH_DOWN', pumps)}
              lockedByPumpLabel={PUMP_DISPLAY_LABEL[lockedByFor('PH_DOWN', pumps) || '']}
            />
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
    </>
  );

  if (variant === 'embedded') {
    return content;
  }

  return (
    <div className="app-page max-w-5xl">
      {/* Header khu vực */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h1 className="text-xl font-bold tracking-tight text-primary-deep flex items-center gap-2">
            <Settings2 size={20} className="text-primary/75" />
            <span>Điều khiển thiết bị</span>
          </h1>
          <p className="text-sm text-text-muted">Bơm, van và hệ thống phun sương khi cần thao tác bằng tay.</p>
        </div>
        <Button
          size="sm"
          variant="secondary"
          disabled={!canSendCommands || isProcessing}
          onClick={async () => {
            if (window.confirm("Khôi phục trạng thái hoạt động của hệ thống?")) await resetFault();
          }}
        >
          <RefreshCw size={12} className={isProcessing ? "animate-spin" : "text-primary"} />
          <span>Khôi phục</span>
        </Button>
      </div>

      {content}
    </div>
  );
};

export default ControlPanel;
