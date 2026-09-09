import { useState } from 'react';
import { AlertOctagon } from 'lucide-react';
import toast from 'react-hot-toast';
import { useDeviceControl } from '../../hooks/useDeviceControl';
import { useDeviceStore } from '../../store/useDeviceStore';
import { EmergencyStopConfirmDialog } from './EmergencyStopConfirmDialog';

interface EmergencyStopButtonProps {
  deviceId: string | null;
  variant: 'floating' | 'bar';
}

export const EmergencyStopButton = ({ deviceId, variant }: EmergencyStopButtonProps) => {
  const [open, setOpen] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const pumps = useDeviceStore((s) => s.sensorData?.pump_status) as Record<string, boolean> | undefined;
  const { emergencyStop } = useDeviceControl(deviceId || '');

  const handleConfirm = async () => {
    setIsSubmitting(true);
    try {
      const success = await emergencyStop();
      if (success) {
        toast.success('Đã gửi lệnh dừng khẩn cấp.');
        setOpen(false);
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <>
      {variant === 'floating' ? (
        <button
          onClick={() => setOpen(true)}
          aria-label="Dừng khẩn cấp"
          className="fixed bottom-[76px] right-4 lg:bottom-6 lg:right-6 z-40 flex items-center justify-center w-14 h-14 rounded-full bg-red-600 hover:bg-red-700 text-white shadow-lg shadow-red-950/30 transition-all active:scale-95 cursor-pointer"
        >
          <AlertOctagon size={24} />
        </button>
      ) : (
        <button
          onClick={() => setOpen(true)}
          className="fixed bottom-[76px] left-4 right-4 lg:left-[17rem] lg:right-6 lg:bottom-6 z-40 flex items-center justify-center gap-2 px-4 py-3 rounded-2xl bg-red-600 hover:bg-red-700 text-white text-sm font-bold shadow-lg shadow-red-950/30 transition-all active:scale-[0.99] cursor-pointer"
        >
          <AlertOctagon size={16} /> Dừng khẩn cấp
        </button>
      )}

      <EmergencyStopConfirmDialog
        open={open}
        runningPumps={pumps || {}}
        onCancel={() => setOpen(false)}
        onConfirm={handleConfirm}
        isSubmitting={isSubmitting}
      />
    </>
  );
};
