import { useState } from 'react';
import { AlertOctagon } from 'lucide-react';
import toast from 'react-hot-toast';
import { useStationContext } from '../../contexts/StationContext';
import { useDeviceControl } from '../../hooks/useDeviceControl';
import { useWhoami } from '../../hooks/useWhoami';

interface EmergencyStopButtonProps {
  deviceId: string | null;
  variant: 'floating' | 'bar' | 'status';
}

export const EmergencyStopButton = ({ deviceId, variant }: EmergencyStopButtonProps) => {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { selectedDeviceId } = useStationContext();
  const activeDeviceId = selectedDeviceId === deviceId ? selectedDeviceId : null;
  const {
    emergencyStop,
    emergencyStopLifecycle,
    emergencyStopSafetyState,
  } = useDeviceControl(activeDeviceId || '');
  const {
    data: whoami,
    isLoading: isCapabilityLoading,
    isError: isCapabilityError,
  } = useWhoami();
  const hasEmergencyStopCapability =
    whoami?.scopes.includes('*') || whoami?.scopes.includes('control:emergency') || false;
  const capabilityKnown = !isCapabilityLoading && !isCapabilityError && Boolean(whoami);
  const canEmergencyStop =
    Boolean(activeDeviceId) && capabilityKnown && hasEmergencyStopCapability;

  const handleEmergencyStop = async () => {
    if (!canEmergencyStop || isSubmitting) return;
    setIsSubmitting(true);
    try {
      const success = await emergencyStop();
      if (success) {
        toast.success('Đã gửi lệnh dừng khẩn cấp.');
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  const lifecycleLabel = emergencyStopLifecycle ?? 'IDLE';
  const safetyLabel =
    emergencyStopSafetyState === 'CONFIRMED_OFF'
      ? 'Đã xác nhận tất cả cơ cấu chấp hành đã tắt'
      : 'Chưa xác nhận trạng thái an toàn';
  const capabilityLabel = hasEmergencyStopCapability
    ? 'Có quyền dừng khẩn cấp'
    : 'Tài khoản không có quyền dừng khẩn cấp';
  const statusLabel =
    'Trạng thái an toàn: ' +
    safetyLabel +
    '. Vòng đời lệnh: ' +
    lifecycleLabel +
    '. ' +
    capabilityLabel +
    '.';
  const persistentSafetyMessage =
    emergencyStopLifecycle && emergencyStopLifecycle !== 'CONFIRMED'
      ? emergencyStopLifecycle === 'TIMEOUT'
        ? 'Dừng khẩn cấp: chưa nhận được xác nhận vật lý.'
        : emergencyStopLifecycle === 'FAILED' || emergencyStopLifecycle === 'REJECTED'
          ? 'Dừng khẩn cấp: lệnh không thành công.'
          : 'Dừng khẩn cấp: đang chờ xác nhận vật lý.'
      : emergencyStopSafetyState === 'CONFIRMED_OFF'
        ? 'An toàn: đã xác nhận cơ cấu chấp hành tắt.'
        : null;

  return (
    <>
      {variant === 'status' && (
        <div
          className={`w-full rounded-xl border px-3 py-2 text-xs font-semibold ${
            emergencyStopSafetyState === 'CONFIRMED_OFF'
              ? 'border-status/30 bg-status/5 text-status'
              : emergencyStopLifecycle === 'FAILED' || emergencyStopLifecycle === 'REJECTED' || emergencyStopLifecycle === 'TIMEOUT'
                ? 'border-error/40 bg-error/5 text-error'
                : 'border-warning/50 bg-warning-bg text-warn-deep'
          }`}
          role="status"
          data-estop-status="true"
        >
          Dừng khẩn cấp: {safetyLabel}. Lệnh: {lifecycleLabel}.
        </div>
      )}
      {persistentSafetyMessage && (
        <div className="fixed left-4 right-4 top-4 lg:left-[17rem] lg:right-6 z-40 rounded-xl border border-warning/50 bg-warning-bg px-4 py-2 text-xs font-semibold text-warn-deep shadow-low" role="status" data-estop-persistent-state={emergencyStopSafetyState}>
          {persistentSafetyMessage}
        </div>
      )}
      {variant === 'status' ? null : variant === 'floating' ? (
        <button
          onClick={() => void handleEmergencyStop()}
          disabled={!canEmergencyStop || isSubmitting}
          aria-label={'Dừng khẩn cấp. ' + statusLabel}
          title={statusLabel}
          data-estop-lifecycle={lifecycleLabel}
          data-safety-state={emergencyStopSafetyState}
          data-capability={hasEmergencyStopCapability ? 'control:emergency' : 'denied'}
          className="fixed bottom-[76px] right-4 lg:bottom-6 lg:right-6 z-40 flex items-center justify-center w-14 h-14 rounded-full bg-red-600 hover:bg-red-700 text-white shadow-lg shadow-red-950/30 transition-all active:scale-95 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <AlertOctagon size={24} />
        </button>
      ) : (
        <button
          onClick={() => void handleEmergencyStop()}
          disabled={!canEmergencyStop || isSubmitting}
          aria-label={'Dừng khẩn cấp. ' + statusLabel}
          title={statusLabel}
          data-estop-lifecycle={lifecycleLabel}
          data-safety-state={emergencyStopSafetyState}
          data-capability={hasEmergencyStopCapability ? 'control:emergency' : 'denied'}
          className="fixed bottom-[76px] left-4 right-4 lg:left-[17rem] lg:right-6 lg:bottom-6 z-40 flex items-center justify-center gap-2 px-4 py-3 rounded-2xl bg-red-600 hover:bg-red-700 text-white text-sm font-bold shadow-lg shadow-red-950/30 transition-all active:scale-[0.99] cursor-pointer"
        >
          <AlertOctagon size={16} /> Dừng khẩn cấp
          <span className="sr-only">{statusLabel}</span>
        </button>
      )}
    </>
  );
};
