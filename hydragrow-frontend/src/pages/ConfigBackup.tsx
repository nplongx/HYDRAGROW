import { useState } from 'react';
import { Download, Upload, DatabaseBackup } from 'lucide-react';
import { useStationContext } from '../contexts/StationContext';
import { BackupArtifact, useConfigBackup } from '../hooks/useConfigBackup';

export function ConfigBackup() {
  const { selectedDeviceId: deviceId } = useStationContext();
  const { exportMutation, previewMutation, restoreMutation } = useConfigBackup(deviceId);
  const [selectedArtifact, setSelectedArtifact] = useState<BackupArtifact | null>(null);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  async function handleExport() {
    try {
      const backup = await exportMutation.mutateAsync();
      const blob = new Blob([JSON.stringify(backup, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `hydragrow_backup_${deviceId}_${new Date().toISOString().split('T')[0]}.json`;
      a.click();
      URL.revokeObjectURL(url);
      setMessage({ type: 'success', text: 'Đã xuất backup thành công!' });
    } catch (e: unknown) {
      setMessage({ type: 'error', text: e instanceof Error ? e.message : String(e) });
    }
  }

  async function handleImport(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      const backup = JSON.parse(text) as BackupArtifact;
      const preview = await previewMutation.mutateAsync(backup);
      setSelectedArtifact(backup);
      const summary = [
        `Thêm: ${preview.additions.length}`,
        `Thay đổi: ${preview.changes.length}`,
        `Không đổi: ${preview.unchanged.length}`,
      ].join(' · ');
      setMessage({ type: 'success', text: `Đã kiểm tra backup. ${summary}` });
    } catch (e: unknown) {
      setMessage({ type: 'error', text: `Backup bị từ chối: ${e instanceof Error ? e.message : String(e)}` });
    } finally {
      e.target.value = '';
    }
  }

  async function handleApply() {
    if (!selectedArtifact) return;
    if (!confirm('Áp dụng backup đã được kiểm tra? Cấu hình sẽ được commit nguyên tử.')) return;
    try {
      const result = await restoreMutation.mutateAsync(selectedArtifact);
      setMessage({ type: 'success', text: `Restore: ${result.status}. Kiểm tra đồng bộ controller nếu cần.` });
      setSelectedArtifact(null);
    } catch (e: unknown) {
      setMessage({ type: 'error', text: `Restore bị từ chối: ${e instanceof Error ? e.message : String(e)}` });
    }
  }

  const busy = exportMutation.isPending || previewMutation.isPending || restoreMutation.isPending;

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
            disabled={busy || !deviceId}
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
            Chọn file để kiểm tra preview trước. Chỉ backup hợp lệ mới được phép áp dụng.
          </p>
          <label className={`flex items-center gap-2 px-4 py-2 border border-warning text-warning rounded-lg text-sm cursor-pointer hover:bg-warning-bg ${busy ? 'opacity-50 pointer-events-none' : ''}`}>
            <Upload size={16} /> {previewMutation.isPending ? 'Đang kiểm tra...' : 'Chọn file backup'}
            <input type="file" accept=".json" onChange={handleImport} className="hidden" disabled={busy || !deviceId} />
          </label>
          {previewMutation.data && (
            <div className="mt-4 text-sm text-text-muted space-y-1">
              <div>Thêm: {previewMutation.data.additions.length}</div>
              <div>Thay đổi: {previewMutation.data.changes.length}</div>
              <div>Không đổi: {previewMutation.data.unchanged.length}</div>
              {previewMutation.data.warnings.map((warning) => <div key={warning}>Cảnh báo: {warning}</div>)}
              <button
                onClick={handleApply}
                disabled={busy || !previewMutation.data.apply_permitted || !selectedArtifact}
                className="mt-3 px-4 py-2 ui-btn-primary"
              >
                {restoreMutation.isPending ? 'Đang restore...' : 'Áp dụng backup'}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
