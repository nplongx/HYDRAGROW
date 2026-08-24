import { useState } from 'react';
import { Plus, Trash2, CheckCircle, Pencil, X, Check } from 'lucide-react';
import { apiPost, apiDelete, apiPut } from '../lib/apiClient';
import QRCode from 'react-qr-code';
import { useOwnedDevices } from '../hooks/useOwnedDevices';
import { useDeviceStore } from '../store/useDeviceStore';
import type { OwnedDevice } from '../types/models';

export function DevicePairing() {
  const { devices, loading, error, refresh } = useOwnedDevices();
  
  // Sửa lỗi: Lấy riêng từng giá trị từ Zustand store
  const activeDeviceId = useDeviceStore((s) => s.deviceId);
  const setDeviceId = useDeviceStore((s) => s.setDeviceId);

  const [newDeviceId, setNewDeviceId] = useState('');
  const [newLabel, setNewLabel] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [qrPayload, setQrPayload] = useState<string | null>(null);

  // Inline rename state
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');

  async function claimDevice() {
    if (!newDeviceId.trim()) return;
    setSubmitting(true);
    setFormError(null);
    try {
      const res = await apiPost<{ device_id: string; label: string | null; qr_payload: string }>(
        '/devices/claim',
        { device_id: newDeviceId.trim(), label: newLabel.trim() || null }
      );
      setQrPayload(res.qr_payload);
      setNewDeviceId('');
      setNewLabel('');
      if (!activeDeviceId) setDeviceId(res.device_id);
      await refresh();
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setSubmitting(false);
    }
  }

  async function unclaimDevice(deviceId: string) {
    setSubmitting(true);
    try {
      await apiDelete(`/devices/${deviceId}/claim`);
      if (activeDeviceId === deviceId) setDeviceId(null);
      await refresh();
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setSubmitting(false);
    }
  }

  async function saveRename(deviceId: string) {
    try {
      await apiPut(`/devices/${deviceId}/label`, { label: renameValue.trim() || null });
      setRenamingId(null);
      await refresh();
    } catch (e: any) {
      setFormError(e.message);
    }
  }

  function startRename(d: OwnedDevice) {
    setRenamingId(d.device_id);
    setRenameValue(d.label ?? '');
  }

  return (
    <div className="max-w-2xl mx-auto p-6">
      <h1 className="text-2xl font-bold mb-6">Thiết Bị Của Tôi</h1>

      {(error || formError) && (
        <div className="mb-4 p-3 bg-red-50 text-red-700 rounded-lg text-sm">
          {error || formError}
        </div>
      )}

      {/* Danh sách thiết bị */}
      <div className="mb-6 space-y-2">
        {devices.length === 0 && !loading && (
          <p className="text-gray-500 text-sm">Chưa có thiết bị nào được liên kết.</p>
        )}
        {devices.map((d) => (
          <div
            key={d.device_id}
            className={`flex items-center justify-between p-4 border rounded-xl transition ${
              activeDeviceId === d.device_id ? 'border-emerald-500 bg-emerald-50' : 'border-gray-200'
            }`}
          >
            <div className="flex-1 min-w-0">
              {renamingId === d.device_id ? (
                <div className="flex items-center gap-2">
                  <input
                    className="flex-1 px-2 py-1 border rounded text-sm"
                    value={renameValue}
                    onChange={(e) => setRenameValue(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && saveRename(d.device_id)}
                    autoFocus
                  />
                  <button onClick={() => saveRename(d.device_id)} className="text-emerald-600">
                    <Check size={16} />
                  </button>
                  <button onClick={() => setRenamingId(null)} className="text-gray-400">
                    <X size={16} />
                  </button>
                </div>
              ) : (
                <div className="flex items-center gap-2">
                  <div>
                    <p className="font-medium truncate">{d.label ?? d.device_id}</p>
                    <p className="text-xs text-gray-400">{d.device_id}</p>
                  </div>
                  <button
                    onClick={() => startRename(d)}
                    className="text-gray-400 hover:text-gray-600 flex-shrink-0"
                  >
                    <Pencil size={14} />
                  </button>
                </div>
              )}
            </div>

            <div className="flex items-center gap-2 ml-4 flex-shrink-0">
              <button
                onClick={() => setDeviceId(d.device_id)}
                disabled={activeDeviceId === d.device_id}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition ${
                  activeDeviceId === d.device_id
                    ? 'bg-emerald-600 text-white cursor-default'
                    : 'border border-emerald-500 text-emerald-600 hover:bg-emerald-50'
                }`}
              >
                <CheckCircle size={14} />
                {activeDeviceId === d.device_id ? 'Đang dùng' : 'Chọn'}
              </button>

              <button
                onClick={() => unclaimDevice(d.device_id)}
                disabled={submitting}
                className="p-2 text-red-400 hover:text-red-600 hover:bg-red-50 rounded transition"
              >
                <Trash2 size={16} />
              </button>
            </div>
          </div>
        ))}
      </div>

      {/* Form thêm thiết bị */}
      <div className="p-4 border rounded-xl space-y-3">
        <h2 className="font-semibold">Liên Kết Thiết Bị Mới</h2>
        <input
          type="text"
          placeholder="Device ID (vd: device_001)"
          value={newDeviceId}
          onChange={(e) => setNewDeviceId(e.target.value)}
          className="w-full px-3 py-2 border rounded-lg text-sm"
        />
        <input
          type="text"
          placeholder="Tên hiển thị (tùy chọn)"
          value={newLabel}
          onChange={(e) => setNewLabel(e.target.value)}
          className="w-full px-3 py-2 border rounded-lg text-sm"
        />
        <button
          onClick={claimDevice}
          disabled={submitting || !newDeviceId.trim()}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg text-sm disabled:opacity-50"
        >
          <Plus size={16} /> Liên Kết
        </button>
      </div>

      {/* QR code */}
      {qrPayload && (
        <div className="mt-4 p-4 bg-blue-50 rounded-lg text-center">
          <p className="text-sm text-blue-700 font-medium mb-3">Quét mã QR trên app mobile:</p>
          <div className="inline-block bg-white p-4 rounded-lg">
            <QRCode value={qrPayload} size={200} level="M" />
          </div>
          <p className="mt-3 text-xs text-gray-500 break-all">{qrPayload}</p>
        </div>
      )}
    </div>
  );
}
