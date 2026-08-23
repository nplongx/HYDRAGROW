import { useState, useEffect } from 'react';
import { QrCode, Plus, Trash2 } from 'lucide-react';
import { apiGet, apiPost, apiDelete } from '../lib/apiClient';

interface OwnedDevice {
  device_id: string;
  label: string | null;
}

export function DevicePairing() {
  const [devices, setDevices] = useState<OwnedDevice[]>([]);
  const [newDeviceId, setNewDeviceId] = useState('');
  const [newLabel, setNewLabel] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [qrPayload, setQrPayload] = useState<string | null>(null);

  useEffect(() => { fetchDevices(); }, []);

  async function fetchDevices() {
    try {
      const data = await apiGet<OwnedDevice[]>('/devices');
      setDevices(data);
    } catch (e: any) {
      setError(e.message);
    }
  }

  async function claimDevice() {
    if (!newDeviceId.trim()) return;
    setLoading(true); setError(null);
    try {
      const res = await apiPost<{ device_id: string; label: string | null; qr_payload: string }>(
        '/devices/claim',
        { device_id: newDeviceId.trim(), label: newLabel.trim() || null }
      );
      setQrPayload(res.qr_payload);
      setNewDeviceId(''); setNewLabel('');
      await fetchDevices();
    } catch (e: any) {
      setError(e.message);
    } finally { setLoading(false); }
  }

  async function unclaimDevice(deviceId: string) {
    setLoading(true); setError(null);
    try {
      await apiDelete(`/devices/${deviceId}/claim`);
      await fetchDevices();
    } catch (e: any) {
      setError(e.message);
    } finally { setLoading(false); }
  }

  return (
    <div className="max-w-2xl mx-auto p-6">
      <h1 className="text-2xl font-bold mb-6">Thiết Bị Của Tôi</h1>

      {error && (
        <div className="mb-4 p-3 bg-red-50 text-red-700 rounded-lg text-sm">{error}</div>
      )}

      {/* Danh sách thiết bị đã claim */}
      <div className="mb-6 space-y-2">
        {devices.length === 0 && (
          <p className="text-gray-500 text-sm">Chưa có thiết bị nào được liên kết.</p>
        )}
        {devices.map((d) => (
          <div key={d.device_id} className="flex items-center justify-between p-4 border rounded-lg">
            <div>
              <p className="font-medium">{d.label ?? d.device_id}</p>
              <p className="text-xs text-gray-500">{d.device_id}</p>
            </div>
            <button
              onClick={() => unclaimDevice(d.device_id)}
              disabled={loading}
              className="p-2 text-red-500 hover:bg-red-50 rounded"
            >
              <Trash2 size={16} />
            </button>
          </div>
        ))}
      </div>

      {/* Form thêm thiết bị */}
      <div className="p-4 border rounded-lg space-y-3">
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
          disabled={loading || !newDeviceId.trim()}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg text-sm disabled:opacity-50"
        >
          <Plus size={16} /> Liên Kết
        </button>
      </div>

      {/* QR code hiển thị sau khi claim */}
      {qrPayload && (
        <div className="mt-4 p-4 bg-blue-50 rounded-lg text-center">
          <QrCode className="mx-auto mb-2 text-blue-600" size={24} />
          <p className="text-sm text-blue-700 font-medium">Quét mã QR này trên app mobile:</p>
          <code className="text-xs break-all">{qrPayload}</code>
          <button onClick={() => setQrPayload(null)} className="mt-2 text-xs text-blue-500 underline block">Đóng</button>
        </div>
      )}
    </div>
  );
}
