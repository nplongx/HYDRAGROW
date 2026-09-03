import { useState } from 'react';
import toast from 'react-hot-toast';
import { useOwnedDevices } from '../../hooks/useOwnedDevices';
import { useApplyFlowTemplate, useSyncFlowTemplate } from '../../hooks/useAutomationScripts';
import type { UserScript } from '../../types/automation';

export interface MultiDeviceApplyDialogProps {
  sourceDeviceId: string;
  sourceScript: UserScript;
  onClose: () => void;
}

export function MultiDeviceApplyDialog({ sourceDeviceId, sourceScript, onClose }: MultiDeviceApplyDialogProps) {
  const { devices, loading: devicesLoading } = useOwnedDevices();
  const [selectedDevices, setSelectedDevices] = useState<Set<string>>(new Set());

  const applyTemplate = useApplyFlowTemplate(sourceDeviceId, sourceScript.id);
  const syncTemplate = useSyncFlowTemplate(sourceDeviceId, sourceScript.id);

  // Exclude current device from candidates
  const candidates = (devices || []).filter(d => d.device_id !== sourceDeviceId);

  const toggleDevice = (id: string) => {
    setSelectedDevices(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleApply = async () => {
    if (selectedDevices.size === 0) return;
    try {
      await applyTemplate.mutateAsync({
        target_device_ids: Array.from(selectedDevices),
      });
      toast.success(`Đã áp dụng Flow cho ${selectedDevices.size} thiết bị`);
      setSelectedDevices(new Set());
    } catch (e: any) {
      toast.error(`Lỗi áp dụng Flow: ${e.message}`);
    }
  };

  const handleSync = async () => {
    if (!confirm('Đồng bộ Flow gốc tới tất cả các bản sao đã áp dụng (giữ lại các trường bị override)?')) return;
    try {
      const res = await syncTemplate.mutateAsync();
      toast.success(`Đã đồng bộ ${res.synced_devices_count} thiết bị`);
    } catch (e: any) {
      toast.error(`Lỗi đồng bộ Flow: ${e.message}`);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
      <div className="flex w-full max-w-md flex-col gap-4 rounded-2xl bg-white p-6 shadow-xl">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-emerald-950">Quản lý Multi-device Flow</h2>
          <button className="text-sm text-emerald-700/70" onClick={onClose}>
            Đóng
          </button>
        </div>

        <div className="text-sm text-emerald-800">
          Flow gốc: <span className="font-semibold">{sourceScript.name}</span>
        </div>

        <div className="flex flex-col gap-2 rounded-xl border border-emerald-100 bg-emerald-50/50 p-3">
          <p className="text-sm font-medium text-emerald-900">Chọn thiết bị để áp dụng bản sao:</p>
          {devicesLoading ? (
            <p className="text-xs text-emerald-700/70">Đang tải...</p>
          ) : candidates.length === 0 ? (
            <p className="text-xs text-emerald-700/70">Không có thiết bị khác</p>
          ) : (
            <div className="flex max-h-40 flex-col gap-1 overflow-y-auto">
              {candidates.map(d => (
                <label key={d.device_id} className="flex cursor-pointer items-center gap-2 text-sm text-emerald-800">
                  <input
                    type="checkbox"
                    checked={selectedDevices.has(d.device_id)}
                    onChange={() => toggleDevice(d.device_id)}
                    className="rounded border-emerald-300 text-emerald-600 focus:ring-emerald-500"
                  />
                  {d.label || d.device_id}
                </label>
              ))}
            </div>
          )}

          <button
            className="ui-btn-primary mt-2 w-full justify-center"
            disabled={selectedDevices.size === 0 || applyTemplate.isPending}
            onClick={handleApply}
          >
            {applyTemplate.isPending ? 'Đang xử lý...' : 'Tạo bản sao'}
          </button>
        </div>

        <div className="mt-4 flex flex-col gap-2 border-t border-emerald-100 pt-4">
          <p className="text-sm font-medium text-emerald-900">Đồng bộ thay đổi từ Flow này</p>
          <p className="text-xs text-emerald-700/80">
            Cập nhật lại toàn bộ thiết bị đang dùng bản sao của Flow này (giữ nguyên các cấu hình mà người dùng đã tùy chỉnh trên thiết bị đích).
          </p>
          <button
            className="ui-btn-primary mt-1 w-full justify-center bg-indigo-600 hover:bg-indigo-700 focus-visible:ring-indigo-500"
            disabled={syncTemplate.isPending}
            onClick={handleSync}
          >
            {syncTemplate.isPending ? 'Đang đồng bộ...' : 'Đồng bộ thiết bị đích'}
          </button>
        </div>
      </div>
    </div>
  );
}
