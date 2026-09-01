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
    <div className="max-w-2xl mx-auto p-6">
      <div className="flex items-center gap-3 mb-6">
        <Shield className="text-emerald-700" size={24} />
        <h1 className="text-2xl font-bold">Quản Lý Người Dùng & Quyền</h1>
      </div>

      {message && (
        <div className={`mb-4 p-3 rounded-lg text-sm ${
          message.type === 'success' ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-700'
        }`}>
          {message.text}
        </div>
      )}

      <div className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Firebase UID *</label>
          <input
            value={firebaseUid}
            onChange={(e) => setFirebaseUid(e.target.value)}
            className="w-full px-3 py-2 border rounded-lg text-sm"
            placeholder="Lấy từ Firebase Console → Authentication"
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Email *</label>
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="w-full px-3 py-2 border rounded-lg text-sm"
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Tên hiển thị</label>
          <input
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            className="w-full px-3 py-2 border rounded-lg text-sm"
          />
        </div>

        {/* Scope selection */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">Quyền truy cập</label>
          <div className="space-y-2">
            {scopes.filter(s => s.scope !== '*').map((s) => (
              <label key={s.scope} className="flex items-start gap-3 p-3 border rounded-lg cursor-pointer hover:bg-gray-50">
                <input
                  type="checkbox"
                  checked={selectedScopes.includes(s.scope)}
                  onChange={() => toggleScope(s.scope)}
                  className="mt-0.5"
                />
                <div>
                  <p className="text-sm font-mono font-medium text-gray-800">{s.scope}</p>
                  <p className="text-xs text-gray-500 mt-0.5">{s.description}</p>
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
