import { useState } from 'react';
import { Download, Upload, DatabaseBackup } from 'lucide-react';
import { useDeviceStore } from '../store/useDeviceStore';
import { apiGet, apiPost } from '../lib/apiClient';

export function ConfigBackup() {
  const deviceId = useDeviceStore((s) => s.deviceId);
  const [importing, setImporting] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  async function handleExport() {
    try {
      const backup = await apiGet(`/devices/${deviceId}/admin/backup`);
      const blob = new Blob([JSON.stringify(backup, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `hydragrow_backup_${deviceId}_${new Date().toISOString().split('T')[0]}.json`;
      a.click();
      URL.revokeObjectURL(url);
      setMessage({ type: 'success', text: 'Đã xuất backup thành công!' });
    } catch (e: any) {
      setMessage({ type: 'error', text: e.message });
    }
  }

  async function handleImport(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    setImporting(true);
    try {
      const text = await file.text();
      const backup = JSON.parse(text);
      if (!confirm(`Import backup từ ${backup.exported_at}?\nThiết bị: ${backup.device_id}\nThao tác này sẽ ghi đè cấu hình hiện tại.`)) {
        return;
      }
      await apiPost(`/devices/${deviceId}/admin/restore`, backup);
      setMessage({ type: 'success', text: 'Import thành công! Cấu hình đang được áp dụng.' });
    } catch (e: any) {
      setMessage({ type: 'error', text: `Lỗi import: ${e.message}` });
    } finally {
      setImporting(false);
      e.target.value = '';
    }
  }

  return (
    <div className="app-page">
      <div className="page-header">
        <div className="page-header-main">
          <div className="page-header-icon">
            <DatabaseBackup size={20} />
          </div>
          <div>
            <h1 className="page-header-title">Backup & Restore Cấu Hình</h1>
            <p className="page-header-subtitle">
              Sao lưu và khôi phục cấu hình thiết bị để đảm bảo an toàn dữ liệu.
            </p>
          </div>
        </div>
      </div>

      {message && (
        <div className={`mb-4 p-3 rounded-lg text-sm ${
          message.type === 'success' ? 'bg-pill text-status' : 'bg-danger-bg text-error'
        }`}>
          {message.text}
        </div>
      )}

      <div className="space-y-4">
        <div className="ui-card">
          <div className="flex items-center gap-3 mb-3">
            <Download className="text-water" size={20} />
            <h2 className="font-semibold">Xuất Backup</h2>
          </div>
          <p className="text-sm text-text-muted mb-4">
            Tải xuống file JSON chứa toàn bộ cấu hình thiết bị và recipe hiện tại.
          </p>
          <button
            onClick={handleExport}
            className="flex items-center gap-2 px-4 py-2 ui-btn-primary"
          >
            <Download size={16} /> Xuất Backup
          </button>
        </div>

        <div className="ui-card">
          <div className="flex items-center gap-3 mb-3">
            <Upload className="text-warning" size={20} />
            <h2 className="font-semibold">Import Backup</h2>
          </div>
          <p className="text-sm text-text-muted mb-4">
            Khôi phục cấu hình từ file backup. Thao tác này sẽ ghi đè cấu hình hiện tại của thiết bị.
          </p>
          <label className={`flex items-center gap-2 px-4 py-2 border border-warning text-warning rounded-lg text-sm cursor-pointer hover:bg-warning-bg ${importing ? 'opacity-50 pointer-events-none' : ''}`}>
            <Upload size={16} /> {importing ? 'Đang import...' : 'Chọn file backup'}
            <input type="file" accept=".json" onChange={handleImport} className="hidden" disabled={importing} />
          </label>
        </div>
      </div>
    </div>
  );
}
