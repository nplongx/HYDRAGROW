import { useState, useEffect } from 'react';
import { Shield, Save } from 'lucide-react';
import { apiGet, apiPost } from '../lib/apiClient';

interface ScopeInfo {
  scope: string;
  description: string;
}

export function UserManagement() {
  const [scopes, setScopes] = useState<ScopeInfo[]>([]);
  const [firebaseUid, setFirebaseUid] = useState('');
  const [email, setEmail] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [selectedScopes, setSelectedScopes] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  useEffect(() => {
    apiGet<ScopeInfo[]>('/admin/scopes').then(setScopes).catch(() => {});
  }, []);

  function toggleScope(scope: string) {
    setSelectedScopes((prev) =>
      prev.includes(scope) ? prev.filter((s) => s !== scope) : [...prev, scope]
    );
  }

  async function provision() {
    if (!firebaseUid.trim() || !email.trim()) {
      setMessage({ type: 'error', text: 'Firebase UID và email là bắt buộc.' });
      return;
    }
    setLoading(true);
    setMessage(null);
    try {
      await apiPost('/admin/users', {
        firebase_uid: firebaseUid.trim(),
        email: email.trim(),
        display_name: displayName.trim() || null,
        scopes: selectedScopes,
      });
      setMessage({ type: 'success', text: `Đã cấp quyền cho ${email}!` });
      setFirebaseUid(''); setEmail(''); setDisplayName(''); setSelectedScopes([]);
    } catch (e: any) {
      setMessage({ type: 'error', text: e.message });
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="app-page">
      <div className="page-header">
        <div className="page-header-main">
          <div className="page-header-icon">
            <Shield size={20} />
          </div>
          <div>
            <h1 className="page-header-title">Quản Lý Người Dùng & Quyền</h1>
            <p className="page-header-subtitle">
              Quản lý quyền truy cập và vai trò cho người dùng trong hệ thống.
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
        <div>
          <label className="block text-sm font-medium text-primary-deep mb-1">Firebase UID *</label>
          <input
            value={firebaseUid}
            onChange={(e) => setFirebaseUid(e.target.value)}
            className="w-full px-3 py-2 border rounded-lg text-sm"
            placeholder="Lấy từ Firebase Console → Authentication"
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-primary-deep mb-1">Email *</label>
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="w-full px-3 py-2 border rounded-lg text-sm"
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-primary-deep mb-1">Tên hiển thị</label>
          <input
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            className="w-full px-3 py-2 border rounded-lg text-sm"
          />
        </div>

        {/* Scope selection */}
        <div>
          <label className="block text-sm font-medium text-primary-deep mb-2">Quyền truy cập</label>
          <div className="space-y-2">
            {scopes.filter(s => s.scope !== '*').map((s) => (
              <label key={s.scope} className="flex items-start gap-3 p-3 border border-line rounded-lg cursor-pointer hover:bg-soft">
                <input
                  type="checkbox"
                  checked={selectedScopes.includes(s.scope)}
                  onChange={() => toggleScope(s.scope)}
                  className="mt-0.5"
                />
                <div>
                  <p className="text-sm font-mono font-medium text-primary-deep">{s.scope}</p>
                  <p className="text-xs text-text-muted mt-0.5">{s.description}</p>
                </div>
              </label>
            ))}
          </div>
        </div>

        <button
          onClick={provision}
          disabled={loading}
          className="ui-btn-primary flex items-center gap-2 w-full justify-center"
        >
          <Save size={16} /> {loading ? 'Đang lưu...' : 'Cấp Quyền / Cập Nhật'}
        </button>
      </div>
    </div>
  );
}
