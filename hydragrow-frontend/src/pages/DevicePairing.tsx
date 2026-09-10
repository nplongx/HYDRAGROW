import { useState, useRef, useEffect } from 'react';
import {
  Plus,
  Trash2,
  CheckCircle,
  Pencil,
  X,
  Check,
  QrCode,
  Camera,
  StopCircle,
  Radio,
  Clock,
  Sparkles,
} from 'lucide-react';
import { Html5Qrcode } from 'html5-qrcode';
import QRCode from 'react-qr-code';
import toast from 'react-hot-toast';
import { apiPost, apiDelete, apiPut, apiGet } from '../lib/apiClient';
import { useOwnedDevices } from '../hooks/useOwnedDevices';
import { useDeviceStore } from '../store/useDeviceStore';
import type { OwnedDevice, StatusPayload } from '../types/models';
import {
  ScanConfirmOverlay,
  deriveConfirmationCode,
} from '../components/pairing/ScanConfirmOverlay';

export function parseDeviceIdFromQr(raw: string): string {
  const trimmed = raw.trim();
  if (trimmed.startsWith('hydragrow://claim/')) {
    return trimmed.replace('hydragrow://claim/', '').split('?')[0].trim();
  }
  try {
    const url = new URL(trimmed);
    const segments = url.pathname.split('/').filter(Boolean);
    const claimIdx = segments.indexOf('claim');
    if (claimIdx !== -1 && segments[claimIdx + 1]) {
      return segments[claimIdx + 1];
    }
  } catch {
    // Không phải URL, giữ nguyên chuỗi
  }
  return trimmed;
}

export function DevicePairing() {
  const { devices, loading, error, refresh } = useOwnedDevices();

  const activeDeviceId = useDeviceStore((s) => s.deviceId);
  const setDeviceId = useDeviceStore((s) => s.setDeviceId);

  const [newDeviceId, setNewDeviceId] = useState('');
  const [newLabel, setNewLabel] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [qrPayload, setQrPayload] = useState<string | null>(null);

  // Scanner states
  const [isScanning, setIsScanning] = useState(false);
  const scannerRef = useRef<Html5Qrcode | null>(null);

  // Confirmation overlay states
  const [pendingDeviceId, setPendingDeviceId] = useState<string | null>(null);
  const [showConfirmOverlay, setShowConfirmOverlay] = useState(false);

  // Inline rename state
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');

  // Device status map (online/offline)
  const [deviceStatuses, setDeviceStatuses] = useState<Record<string, StatusPayload>>({});

  useEffect(() => {
    return () => {
      if (scannerRef.current) {
        if (scannerRef.current.isScanning) {
          scannerRef.current.stop().catch(() => {});
        }
        scannerRef.current.clear();
      }
    };
  }, []);

  // Fetch status for all claimed devices
  useEffect(() => {
    if (!devices.length) return;
    devices.forEach((d) => {
      apiGet<StatusPayload>(`/devices/${d.device_id}/status`)
        .then((res) => {
          setDeviceStatuses((prev) => ({ ...prev, [d.device_id]: res }));
        })
        .catch(() => {
          // ignore status fetch failure
        });
    });
  }, [devices]);

  const startScanner = async () => {
    setFormError(null);
    setIsScanning(true);
    // Give react time to render the #qr-reader element
    setTimeout(async () => {
      try {
        const html5QrCode = new Html5Qrcode('qr-reader');
        scannerRef.current = html5QrCode;
        await html5QrCode.start(
          { facingMode: 'environment' },
          { fps: 10, qrbox: { width: 250, height: 250 } },
          (decodedText) => {
            const extracted = parseDeviceIdFromQr(decodedText);
            stopScanner();
            setPendingDeviceId(extracted);
            setShowConfirmOverlay(true);
          },
          () => {
            // ignore per-frame parse failures
          }
        );
      } catch (err: any) {
        toast.error('Không thể mở camera: ' + (err?.message || err));
        setIsScanning(false);
      }
    }, 150);
  };

  const stopScanner = async () => {
    if (scannerRef.current) {
      try {
        if (scannerRef.current.isScanning) {
          await scannerRef.current.stop();
        }
        scannerRef.current.clear();
      } catch {
        // ignore stop errors
      }
      scannerRef.current = null;
    }
    setIsScanning(false);
  };

  async function executeClaim(deviceIdToClaim: string, labelToClaim: string | null) {
    setSubmitting(true);
    setFormError(null);
    try {
      const res = await apiPost<{ device_id: string; label: string | null; qr_payload: string }>(
        '/devices/claim',
        { device_id: deviceIdToClaim.trim(), label: labelToClaim?.trim() || null }
      );
      toast.success(`Đã ghép nối thiết bị ${res.device_id} thành công!`);
      setQrPayload(res.qr_payload);
      setNewDeviceId('');
      setNewLabel('');
      setShowConfirmOverlay(false);
      setPendingDeviceId(null);
      if (!activeDeviceId) setDeviceId(res.device_id);
      await refresh();
    } catch (e: any) {
      setFormError(e.message || 'Lỗi ghép nối thiết bị');
      toast.error(e.message || 'Lỗi ghép nối thiết bị');
    } finally {
      setSubmitting(false);
    }
  }

  async function unclaimDevice(deviceId: string) {
    if (!confirm(`Xác nhận huỷ liên kết thiết bị ${deviceId}?`)) return;
    setSubmitting(true);
    try {
      await apiDelete(`/devices/${deviceId}/claim`);
      toast.success(`Đã huỷ liên kết ${deviceId}`);
      if (activeDeviceId === deviceId) setDeviceId(null);
      await refresh();
    } catch (e: any) {
      setFormError(e.message);
      toast.error(e.message);
    } finally {
      setSubmitting(false);
    }
  }

  async function saveRename(deviceId: string) {
    try {
      await apiPut(`/devices/${deviceId}/label`, { label: renameValue.trim() || null });
      toast.success('Đã cập nhật tên trạm');
      setRenamingId(null);
      await refresh();
    } catch (e: any) {
      setFormError(e.message);
      toast.error(e.message);
    }
  }

  function startRename(d: OwnedDevice) {
    setRenamingId(d.device_id);
    setRenameValue(d.label ?? '');
  }

  return (
    <div className="farm-page-shell max-w-4xl mx-auto space-y-6">
      {/* HEADER */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h1 className="farm-title flex items-center gap-2.5">
            <QrCode className="text-primary" size={26} /> Quản lý Thiết Bị Của Tôi
          </h1>
          <p className="farm-subtitle">
            Ghép nối trạm thuỷ canh mới qua mã QR hoặc nhập Device ID thủ công
          </p>
        </div>
        <div>
          {!isScanning ? (
            <button
              type="button"
              onClick={startScanner}
              className="ui-btn-primary flex items-center gap-2"
            >
              <Camera size={18} /> Quét mã QR trạm
            </button>
          ) : (
            <button
              type="button"
              onClick={stopScanner}
              className="ui-btn-md border border-rose-300 text-rose-600 bg-rose-50 hover:bg-rose-100 flex items-center gap-2"
            >
              <StopCircle size={18} /> Dừng quét camera
            </button>
          )}
        </div>
      </div>

      {(error || formError) && (
        <div className="p-3.5 bg-rose-50 border border-rose-200 text-rose-700 rounded-xl text-xs font-medium">
          {error || formError}
        </div>
      )}

      {/* CAMERA QR SCANNER VIEWPORT */}
      {isScanning && (
        <div className="ui-card p-4 space-y-3 animate-in fade-in" data-testid="qr-scanner-card">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Camera size={18} className="text-primary animate-pulse" />
              <h2 className="farm-section-title">Hướng camera vào mã QR trên trạm thuỷ canh</h2>
            </div>
            <button
              type="button"
              onClick={stopScanner}
              className="text-text-muted hover:text-primary-deep"
              aria-label="Đóng camera"
            >
              <X size={20} />
            </button>
          </div>
          <div className="relative mx-auto max-w-sm overflow-hidden rounded-2xl border-2 border-dashed border-primary/50 bg-black/5 p-2">
            <div id="qr-reader" className="w-full overflow-hidden rounded-xl" />
          </div>
          <p className="text-center text-xs text-text-muted">
            Hệ thống sẽ tự động nhận diện và trích xuất mã thiết bị (hydragrow://claim/...)
          </p>
        </div>
      )}

      {/* CONFIRMATION OVERLAY */}
      {showConfirmOverlay && pendingDeviceId && (
        <ScanConfirmOverlay
          deviceId={pendingDeviceId}
          initialLabel={newLabel}
          isSubmitting={submitting}
          onConfirm={(devId, label) => executeClaim(devId, label || null)}
          onCancel={() => {
            setShowConfirmOverlay(false);
            setPendingDeviceId(null);
          }}
        />
      )}

      {/* DANH SÁCH THIẾT BỊ ĐÃ GHÉP NỐI */}
      <div className="ui-card space-y-4">
        <div className="flex items-center justify-between border-b border-line pb-3">
          <h2 className="farm-section-title flex items-center gap-2">
            <Radio size={18} className="text-primary" /> Trạm đã liên kết ({devices.length})
          </h2>
          <span className="text-xs text-text-muted">
            Chọn trạm đang làm việc để tải dữ liệu giám sát
          </span>
        </div>

        {devices.length === 0 && !loading && (
          <div className="py-12 text-center space-y-2">
            <p className="text-sm font-semibold text-primary-deep">Chưa có thiết bị nào được liên kết</p>
            <p className="text-xs text-text-muted">
              Quét mã QR trên thân trạm hoặc nhập Device ID bên dưới để bắt đầu.
            </p>
          </div>
        )}

        <div className="space-y-3">
          {devices.map((d) => {
            const isActive = activeDeviceId === d.device_id;
            const status = deviceStatuses[d.device_id];
            const isOnline = status?.is_online ?? false;
            const confirmCode = deriveConfirmationCode(d.device_id);

            return (
              <div
                key={d.device_id}
                className={`flex flex-col sm:flex-row sm:items-center sm:justify-between p-4 rounded-2xl border transition-all gap-4 ${
                  isActive
                    ? 'border-primary bg-pill/50 shadow-sm'
                    : 'border-line hover:border-primary/40 bg-white'
                }`}
                data-testid={`device-card-${d.device_id}`}
              >
                <div className="flex-1 min-w-0 space-y-1.5">
                  {renamingId === d.device_id ? (
                    <div className="flex items-center gap-2">
                      <input
                        className="flex-1 px-3 py-1.5 border border-primary rounded-xl text-sm bg-white focus:outline-none"
                        value={renameValue}
                        onChange={(e) => setRenameValue(e.target.value)}
                        onKeyDown={(e) => e.key === 'Enter' && saveRename(d.device_id)}
                        autoFocus
                      />
                      <button
                        onClick={() => saveRename(d.device_id)}
                        className="p-1.5 text-primary hover:bg-soft rounded-lg"
                        title="Lưu"
                      >
                        <Check size={16} />
                      </button>
                      <button
                        onClick={() => setRenamingId(null)}
                        className="p-1.5 text-text-muted hover:bg-soft rounded-lg"
                        title="Huỷ"
                      >
                        <X size={16} />
                      </button>
                    </div>
                  ) : (
                    <div className="flex items-center gap-2">
                      <p className="font-bold text-sm text-primary-deep truncate">
                        {d.label ?? d.device_id}
                      </p>
                      <button
                        onClick={() => startRename(d)}
                        className="text-text-muted hover:text-primary transition-colors p-1"
                        title="Đổi tên gợi nhớ"
                      >
                        <Pencil size={13} />
                      </button>
                    </div>
                  )}

                  <div className="flex flex-wrap items-center gap-2.5 text-xs text-text-muted">
                    <span className="font-mono bg-surface-muted px-2 py-0.5 rounded text-[11px] border border-line">
                      {d.device_id}
                    </span>
                    <span className="font-mono text-primary font-semibold text-[11px]" title="Mã bảo mật">
                      Mã: {confirmCode}
                    </span>
                    <span className="flex items-center gap-1">
                      <span
                        className={`w-2 h-2 rounded-full ${
                          isOnline ? 'bg-emerald-500 animate-pulse' : 'bg-slate-300'
                        }`}
                      />
                      <span className={isOnline ? 'text-status font-medium' : 'text-text-muted'}>
                        {isOnline ? 'Trực tuyến' : 'Ngoại tuyến'}
                      </span>
                    </span>
                    {status?.last_seen && (
                      <span className="flex items-center gap-1 text-[11px] text-faint">
                        <Clock size={12} /> {new Date(status.last_seen).toLocaleTimeString('vi-VN')}
                      </span>
                    )}
                  </div>
                </div>

                <div className="flex items-center gap-2.5 self-end sm:self-center flex-shrink-0">
                  <button
                    onClick={() => setDeviceId(d.device_id)}
                    disabled={isActive}
                    className={`flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl text-xs font-semibold transition ${
                      isActive
                        ? 'bg-primary text-white shadow-sm cursor-default'
                        : 'border border-line text-primary-deep hover:border-primary hover:bg-soft'
                    }`}
                  >
                    <CheckCircle size={14} />
                    {isActive ? 'Đang kích hoạt' : 'Chọn trạm này'}
                  </button>

                  <button
                    onClick={() => unclaimDevice(d.device_id)}
                    disabled={submitting}
                    className="p-2 text-text-muted hover:text-rose-600 hover:bg-rose-50 rounded-xl transition-colors"
                    title="Huỷ liên kết trạm"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* FORM THÊM THỦ CÔNG */}
      <div className="ui-card p-5 space-y-4">
        <div className="flex items-center justify-between border-b border-line pb-3">
          <h2 className="farm-section-title flex items-center gap-2">
            <Plus size={18} className="text-primary" /> Nhập mã thiết bị thủ công
          </h2>
          <span className="text-xs text-text-muted">
            Dành cho thiết bị không có camera quét mã QR
          </span>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <label className="block text-xs font-semibold text-primary-deep mb-1">
              Device ID *
            </label>
            <input
              type="text"
              placeholder="Ví dụ: hydra_station_01"
              value={newDeviceId}
              onChange={(e) => setNewDeviceId(e.target.value)}
              className="w-full px-3 py-2 text-sm rounded-xl border border-line bg-white focus:outline-none focus:border-primary font-mono text-xs"
            />
          </div>
          <div>
            <label className="block text-xs font-semibold text-primary-deep mb-1">
              Tên hiển thị gợi nhớ (tuỳ chọn)
            </label>
            <input
              type="text"
              placeholder="Ví dụ: Giàn dâu tây trong nhà"
              value={newLabel}
              onChange={(e) => setNewLabel(e.target.value)}
              className="w-full px-3 py-2 text-sm rounded-xl border border-line bg-white focus:outline-none focus:border-primary"
            />
          </div>
        </div>

        <div className="flex justify-end pt-1">
          <button
            onClick={() => {
              if (newDeviceId.trim()) {
                setPendingDeviceId(newDeviceId.trim());
                setShowConfirmOverlay(true);
              }
            }}
            disabled={submitting || !newDeviceId.trim()}
            className="ui-btn-primary flex items-center gap-2"
          >
            <Sparkles size={16} /> Tiếp tục xác nhận ghép nối
          </button>
        </div>
      </div>

      {/* QR CODE MOBILE APP PAIRING */}
      {qrPayload && (
        <div className="ui-card p-6 bg-soft/50 text-center space-y-3">
          <p className="text-sm text-primary-deep font-bold">
            Mã QR đồng bộ với ứng dụng di động:
          </p>
          <div className="inline-block bg-white p-4 rounded-2xl border border-line shadow-sm">
            <QRCode value={qrPayload} size={180} level="M" />
          </div>
          <p className="text-xs font-mono text-text-muted max-w-sm mx-auto break-all">
            {qrPayload}
          </p>
        </div>
      )}
    </div>
  );
}
