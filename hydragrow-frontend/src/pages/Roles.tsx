import React, { useState, useEffect } from 'react';
import { Users, UserPlus, Check, X, Shield, RefreshCw } from 'lucide-react';
import toast from 'react-hot-toast';
import { apiGet, apiPost, apiPatch } from '../lib/apiClient';

export type UserRole = 'admin' | 'operator' | 'viewer';

export interface UserItem {
  id: number;
  firebase_uid: string;
  email: string;
  display_name: string | null;
  role: UserRole | null;
  scopes: string[];
  is_active: boolean;
  created_at?: string;
  updated_at?: string;
}

export const ROLE_DEFAULT_SCOPES: Record<UserRole, string[]> = {
  admin: ['*'],
  operator: [
    'read:telemetry',
    'write:config',
    'control:pump',
    'control:emergency',
    'device:ota',
    'device:network',
    'script:write',
    'recipe:write',
  ],
  viewer: ['read:telemetry'],
};

export const ROLE_DISPLAY_NAMES: Record<UserRole, string> = {
  admin: 'Quản trị viên',
  operator: 'Vận hành viên',
  viewer: 'Người xem',
};

export const CAPABILITIES = [
  {
    id: 'telemetry',
    title: 'Giám sát & Số liệu (Telemetry)',
    description: 'Xem thông số cảm biến thời gian thực, biểu đồ lịch sử và nhật ký sự kiện',
    admin: true,
    operator: true,
    viewer: true,
  },
  {
    id: 'control',
    title: 'Điều khiển & Vận hành (Pumps / E-Stop)',
    description: 'Bật/tắt bơm dinh dưỡng, phun sương, xả tràn và kích hoạt dừng khẩn cấp E-stop',
    admin: true,
    operator: true,
    viewer: false,
  },
  {
    id: 'recipes',
    title: 'Cấu hình nông học & Kịch bản (Recipes / Config)',
    description: 'Tạo và chỉnh sửa công thức mùa vụ, ngưỡng pH/EC, lịch trình chiếu sáng',
    admin: true,
    operator: true,
    viewer: false,
  },
  {
    id: 'admin',
    title: 'Quản trị hệ thống & Phân quyền (Roles / OTA / WiFi)',
    description: 'Cập nhật firmware OTA, thiết lập mạng WiFi trạm, mời và đổi vai trò thành viên',
    admin: true,
    operator: false,
    viewer: false,
  },
];

export function Roles() {
  const [users, setUsers] = useState<UserItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showInviteModal, setShowInviteModal] = useState(false);

  // Form states
  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteUid, setInviteUid] = useState('');
  const [inviteDisplayName, setInviteDisplayName] = useState('');
  const [inviteRole, setInviteRole] = useState<UserRole>('operator');

  useEffect(() => {
    if (!showInviteModal) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setShowInviteModal(false);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [showInviteModal]);

  const fetchUsers = async () => {
    try {
      setIsLoading(true);
      const res = await apiGet<UserItem[]>('/admin/users');
      setUsers(Array.isArray(res) ? res : []);
    } catch {
      // Endpoint may return forbidden if not admin or empty
      setUsers([]);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchUsers();
  }, []);

  const handleCreateUser = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!inviteEmail.trim() || !inviteUid.trim()) {
      toast.error('Email và Firebase UID là bắt buộc');
      return;
    }

    setIsSubmitting(true);
    try {
      const scopes = ROLE_DEFAULT_SCOPES[inviteRole];
      await apiPost('/admin/users', {
        firebase_uid: inviteUid.trim(),
        email: inviteEmail.trim(),
        display_name: inviteDisplayName.trim() || null,
        scopes,
      });

      toast.success(`Đã thêm thành viên ${inviteEmail} với vai trò ${ROLE_DISPLAY_NAMES[inviteRole]}`);
      setShowInviteModal(false);
      setInviteEmail('');
      setInviteUid('');
      setInviteDisplayName('');
      setInviteRole('operator');
      await fetchUsers();
    } catch (err: any) {
      toast.error(err?.message || 'Không thể thêm thành viên');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleRoleChange = async (userId: number, newRole: UserRole) => {
    const scopes = ROLE_DEFAULT_SCOPES[newRole];
    try {
      await apiPatch(`/admin/users/${userId}`, {
        role: newRole,
        scopes,
      });
      toast.success(`Đã đổi vai trò thành ${ROLE_DISPLAY_NAMES[newRole]}`);
      setUsers((prev) =>
        prev.map((u) => (u.id === userId ? { ...u, role: newRole, scopes } : u))
      );
    } catch (err: any) {
      toast.error(err?.message || 'Không thể đổi vai trò');
    }
  };

  const handleToggleActive = async (userId: number, currentActive: boolean) => {
    try {
      await apiPatch(`/admin/users/${userId}`, {
        is_active: !currentActive,
      });
      toast.success(!currentActive ? 'Đã kích hoạt tài khoản' : 'Đã tạm dừng tài khoản');
      setUsers((prev) =>
        prev.map((u) => (u.id === userId ? { ...u, is_active: !currentActive } : u))
      );
    } catch (err: any) {
      toast.error(err?.message || 'Không thể cập nhật trạng thái');
    }
  };

  return (
    <div className="farm-page-shell space-y-8 max-w-6xl mx-auto">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h1 className="farm-title flex items-center gap-2.5">
            <Users className="text-primary" size={26} /> Quản lý thành viên &amp; Vai trò
          </h1>
          <p className="farm-subtitle">
            Phân quyền tài khoản trong hệ thống trạm thuỷ canh và ma trận năng lực truy cập
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={fetchUsers}
            disabled={isLoading}
            className="ui-btn-md border border-line text-primary-deep bg-white hover:bg-soft flex items-center gap-2"
            title="Làm mới danh sách"
          >
            <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
            Làm mới
          </button>
          <button
            type="button"
            onClick={() => setShowInviteModal(true)}
            className="ui-btn-primary flex items-center gap-2"
          >
            <UserPlus size={18} /> Thêm thành viên
          </button>
        </div>
      </div>

      {/* Invite Modal / Inline Form */}
      {showInviteModal && (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="invite-modal-title"
          className="fixed inset-0 z-50 bg-black/40 backdrop-blur-sm flex items-center justify-center p-4"
        >
          <div className="ui-card max-w-md w-full p-6 space-y-4 shadow-xl border border-line animate-in fade-in zoom-in-95">
            <div className="flex items-center justify-between">
              <h2 id="invite-modal-title" className="farm-section-title flex items-center gap-2">
                <UserPlus size={20} className="text-primary" /> Thêm thành viên mới
              </h2>
              <button
                type="button"
                onClick={() => setShowInviteModal(false)}
                className="text-text-muted hover:text-primary-deep p-1 cursor-pointer"
                aria-label="Đóng"
              >
                <X size={20} />
              </button>
            </div>

            <form onSubmit={handleCreateUser} className="space-y-4">
              <div>
                <label htmlFor="invite-email" className="block text-xs font-semibold text-primary-deep mb-1">
                  Email tài khoản *
                </label>
                <input
                  id="invite-email"
                  name="inviteEmail"
                  type="email"
                  required
                  value={inviteEmail}
                  onChange={(e) => setInviteEmail(e.target.value)}
                  placeholder="operator@farm.vn"
                  className="w-full px-3 py-2 text-sm rounded-xl border border-line bg-white focus:outline-none focus:border-primary"
                />
              </div>

              <div>
                <label htmlFor="invite-uid" className="block text-xs font-semibold text-primary-deep mb-1">
                  Firebase UID *
                </label>
                <input
                  id="invite-uid"
                  name="inviteUid"
                  type="text"
                  required
                  value={inviteUid}
                  onChange={(e) => setInviteUid(e.target.value)}
                  placeholder="Lấy từ Firebase Console > Authentication"
                  className="w-full px-3 py-2 text-sm rounded-xl border border-line bg-white focus:outline-none focus:border-primary font-mono text-xs"
                />
              </div>

              <div>
                <label htmlFor="invite-name" className="block text-xs font-semibold text-primary-deep mb-1">
                  Tên hiển thị
                </label>
                <input
                  id="invite-name"
                  name="inviteDisplayName"
                  type="text"
                  value={inviteDisplayName}
                  onChange={(e) => setInviteDisplayName(e.target.value)}
                  placeholder="Kỹ sư nông học A"
                  className="w-full px-3 py-2 text-sm rounded-xl border border-line bg-white focus:outline-none focus:border-primary"
                />
              </div>

              <div>
                <label htmlFor="invite-role" className="block text-xs font-semibold text-primary-deep mb-1">
                  Vai trò phân bổ *
                </label>
                <select
                  id="invite-role"
                  name="inviteRole"
                  value={inviteRole}
                  onChange={(e) => setInviteRole(e.target.value as UserRole)}
                  className="w-full px-3 py-2 text-sm rounded-xl border border-line bg-white focus:outline-none focus:border-primary font-medium"
                >
                  <option value="operator">Vận hành viên (Khuyên dùng - Điều khiển &amp; Kịch bản)</option>
                  <option value="viewer">Người xem (Chỉ xem số liệu)</option>
                  <option value="admin">Quản trị viên (Toàn quyền hệ thống)</option>
                </select>
              </div>

              <div className="pt-2 flex items-center justify-end gap-3">
                <button
                  type="button"
                  onClick={() => setShowInviteModal(false)}
                  className="ui-btn-md border border-line text-primary-deep bg-white hover:bg-soft"
                >
                  Huỷ
                </button>
                <button
                  type="submit"
                  disabled={isSubmitting}
                  className="ui-btn-primary"
                >
                  {isSubmitting ? 'Đang thêm...' : 'Xác nhận cấp quyền'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Member List */}
      <div className="ui-card space-y-4">
        <div className="flex items-center justify-between border-b border-line pb-3">
          <h2 className="farm-section-title flex items-center gap-2">
            <Shield size={18} className="text-primary" /> Danh sách thành viên ({users.length})
          </h2>
          <span className="text-xs text-text-muted">
            Vai trò quyết định tập quyền (scopes) gửi lệnh xuống trạm
          </span>
        </div>

        {isLoading ? (
          <div className="py-12 text-center text-sm text-text-muted">Đang tải danh sách thành viên...</div>
        ) : users.length === 0 ? (
          <div className="py-12 text-center space-y-2">
            <p className="text-sm font-semibold text-primary-deep">Chưa có thành viên nào được cấp quyền</p>
            <p className="text-xs text-text-muted">Bấm "+ Thêm thành viên" để phân vai trò cho tài khoản Firebase.</p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-left border-collapse" data-testid="members-table">
              <thead>
                <tr className="border-b border-line text-xs font-semibold text-text-muted uppercase tracking-wider">
                  <th className="py-3 px-4">Thành viên</th>
                  <th className="py-3 px-4">Vai trò hiện tại</th>
                  <th className="py-3 px-4">Đổi vai trò</th>
                  <th className="py-3 px-4">Trạng thái</th>
                  <th className="py-3 px-4 text-right">Thao tác</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-line text-sm">
                {users.map((user) => {
                  const roleKey: UserRole = user.role || 'viewer';
                  return (
                    <tr key={user.id} className="hover:bg-soft/40 transition-colors">
                      <td className="py-3 px-4">
                        <div className="font-semibold text-primary-deep">
                          {user.display_name || user.email.split('@')[0]}
                        </div>
                        <div className="text-xs text-text-muted font-mono">{user.email}</div>
                      </td>
                      <td className="py-3 px-4">
                        <span
                          className={`inline-flex items-center px-2.5 py-1 rounded-full text-xs font-semibold ${
                            roleKey === 'admin'
                              ? 'bg-amber-100 text-amber-800'
                              : roleKey === 'operator'
                              ? 'bg-pill text-status'
                              : 'bg-surface-muted text-text-muted'
                          }`}
                        >
                          {ROLE_DISPLAY_NAMES[roleKey]}
                        </span>
                      </td>
                      <td className="py-3 px-4">
                        <select
                          value={roleKey}
                          onChange={(e) => handleRoleChange(user.id, e.target.value as UserRole)}
                          className="px-2.5 py-1.5 text-xs rounded-lg border border-line bg-white font-medium focus:outline-none focus:border-primary"
                          aria-label={`Đổi vai trò cho ${user.email}`}
                        >
                          <option value="admin">Quản trị viên</option>
                          <option value="operator">Vận hành viên</option>
                          <option value="viewer">Người xem</option>
                        </select>
                      </td>
                      <td className="py-3 px-4">
                        <span
                          className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium ${
                            user.is_active
                              ? 'bg-emerald-50 text-emerald-700'
                              : 'bg-rose-50 text-rose-700'
                          }`}
                        >
                          <span
                            className={`w-1.5 h-1.5 rounded-full ${
                              user.is_active ? 'bg-emerald-500' : 'bg-rose-500'
                            }`}
                          />
                          {user.is_active ? 'Hoạt động' : 'Tạm dừng'}
                        </span>
                      </td>
                      <td className="py-3 px-4 text-right">
                        <button
                          type="button"
                          onClick={() => handleToggleActive(user.id, user.is_active)}
                          className={`text-xs font-medium px-3 py-1.5 rounded-lg border transition-colors ${
                            user.is_active
                              ? 'border-line text-text-muted hover:bg-rose-50 hover:text-rose-700 hover:border-rose-200'
                              : 'border-emerald-200 bg-emerald-50 text-emerald-700 hover:bg-emerald-100'
                          }`}
                        >
                          {user.is_active ? 'Khóa' : 'Mở khóa'}
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Permission Matrix (4 capabilities × 3 roles) */}
      <div className="ui-card space-y-4">
        <div className="border-b border-line pb-3">
          <h2 className="farm-section-title">Ma trận phân quyền (4 Năng lực × 3 Vai trò)</h2>
          <p className="text-xs text-text-muted mt-0.5">
            Bản ánh xạ quyền hạn chi tiết giữa các vai trò hệ thống trạm HydraGrow
          </p>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse" data-testid="permission-matrix">
            <thead>
              <tr className="border-b border-line text-xs font-semibold text-text-muted uppercase tracking-wider">
                <th className="py-3 px-4 w-1/2">Năng lực hệ thống</th>
                <th className="py-3 px-4 text-center">Quản trị viên</th>
                <th className="py-3 px-4 text-center">Vận hành viên</th>
                <th className="py-3 px-4 text-center">Người xem</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-line text-sm">
              {CAPABILITIES.map((cap) => (
                <tr key={cap.id} className="hover:bg-soft/30 transition-colors">
                  <td className="py-3 px-4">
                    <div className="font-semibold text-primary-deep">{cap.title}</div>
                    <div className="text-xs text-text-muted mt-0.5">{cap.description}</div>
                  </td>
                  <td className="py-3 px-4 text-center">
                    {cap.admin ? (
                      <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-pill text-status">
                        <Check size={14} />
                      </span>
                    ) : (
                      <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-surface-muted text-text-muted">
                        <X size={14} />
                      </span>
                    )}
                  </td>
                  <td className="py-3 px-4 text-center">
                    {cap.operator ? (
                      <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-pill text-status">
                        <Check size={14} />
                      </span>
                    ) : (
                      <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-surface-muted text-text-muted">
                        <X size={14} />
                      </span>
                    )}
                  </td>
                  <td className="py-3 px-4 text-center">
                    {cap.viewer ? (
                      <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-pill text-status">
                        <Check size={14} />
                      </span>
                    ) : (
                      <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-surface-muted text-text-muted">
                        <X size={14} />
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

export default Roles;
