import { useState } from 'react';
import { Users, UserPlus, X, Shield, RefreshCw } from 'lucide-react';
import toast from 'react-hot-toast';
import { type AdminUser } from '../api/admin';
import { useAdminUsers } from '../hooks/useAdminUsers';
import { DeviceStatePill } from '../components/ui/DeviceStatePill';
import { RoleBadge, PermissionMatrix } from '../components/roles';
import { InviteForm } from '../components/roles/InviteForm';

export type UserRole = 'admin' | 'operator' | 'viewer';

export type UserItem = AdminUser;

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
    id: 'device',
    title: 'OTA & Mạng thiết bị (OTA & Device Network)',
    description: 'Cập nhật firmware OTA, thiết lập mạng WiFi trạm',
    admin: true,
    operator: true,
    viewer: false,
  },
  {
    id: 'permissions',
    title: 'Phân quyền & Mời thành viên (Permissions & Member Invitation)',
    description: 'Mời và đổi vai trò thành viên',
    admin: true,
    operator: false,
    viewer: false,
  },
];

export function Roles() {
  const [showInviteModal, setShowInviteModal] = useState(false);
  const { users, isLoading, error: usersError, refetch: fetchUsers, provisionUser, updateUser, isProvisioning } = useAdminUsers();

  const handleCreateUser = async (data: {
    firebase_uid: string;
    email: string;
    display_name: string | null;
    role: UserRole;
  }) => {
    try {
      const scopes = ROLE_DEFAULT_SCOPES[data.role];
      await provisionUser({
        firebase_uid: data.firebase_uid,
        email: data.email,
        display_name: data.display_name,
        scopes,
      });

      toast.success(`Đã thêm thành viên ${data.email} với vai trò ${ROLE_DISPLAY_NAMES[data.role]}`);
      setShowInviteModal(false);
      await fetchUsers();
    } catch (err: any) {
      toast.error(err?.message || 'Không thể thêm thành viên');
      throw err;
    }
  };

  const handleRoleChange = async (userId: number, newRole: UserRole) => {
    const scopes = ROLE_DEFAULT_SCOPES[newRole];
    try {
      await updateUser(userId, {
        role: newRole,
        scopes,
      });
      toast.success(`Đã đổi vai trò thành ${ROLE_DISPLAY_NAMES[newRole]}`);
    } catch (err: any) {
      toast.error(err?.message || 'Không thể đổi vai trò');
    }
  };

  const handleToggleActive = async (userId: number, currentActive: boolean) => {
    try {
      await updateUser(userId, {
        is_active: !currentActive,
      });
      toast.success(!currentActive ? 'Đã kích hoạt tài khoản' : 'Đã tạm dừng tài khoản');
    } catch (err: any) {
      toast.error(err?.message || 'Không thể cập nhật trạng thái');
    }
  };

  return (
    <div className="app-page space-y-8 max-w-6xl mx-auto">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h1 className="page-header-title flex items-center gap-2.5">
            <Users className="text-primary" size={26} /> Quản lý thành viên &amp; Vai trò
          </h1>
          <p className="page-header-subtitle">
            Phân quyền tài khoản trong hệ thống trạm thuỷ canh và ma trận năng lực truy cập
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => void fetchUsers()}
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
        <div className="fixed inset-0 z-50 bg-black/40 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="ui-card max-w-lg w-full max-h-[90vh] overflow-y-auto p-6 space-y-4 shadow-xl border border-line animate-in fade-in zoom-in-95">
            <div className="flex items-center justify-between">
              <h2 className="farm-section-title flex items-center gap-2">
                <UserPlus size={20} className="text-primary" /> Thêm thành viên mới
              </h2>
              <button
                type="button"
                onClick={() => setShowInviteModal(false)}
                className="text-text-muted hover:text-primary-deep p-1"
                aria-label="Đóng"
              >
                <X size={20} />
              </button>
            </div>

            <InviteForm
              onSubmit={handleCreateUser}
              onCancel={() => setShowInviteModal(false)}
            isSubmitting={isProvisioning}
            />
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
        ) : usersError ? (
          <div className="py-12 text-center space-y-2">
            <p className="text-sm font-semibold text-primary-deep">Không thể tải danh sách thành viên</p>
            <p className="text-xs text-text-muted">{usersError instanceof Error ? usersError.message : 'Lỗi không xác định'}</p>
          </div>
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
                        <RoleBadge role={roleKey} />
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
                        <DeviceStatePill
                          state={user.is_active ? 'online' : 'offline'}
                          label={user.is_active ? 'Hoạt động' : 'Tạm dừng'}
                        />
                      </td>
                      <td className="py-3 px-4 text-right">
                        <button
                          type="button"
                          onClick={() => handleToggleActive(user.id, user.is_active)}
                          className={`text-xs font-medium px-3 py-1.5 rounded-lg border transition-colors ${
                            user.is_active
                              ? 'border-line text-text-muted hover:bg-danger-bg hover:text-error hover:border-red-200'
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

      {/* Permission Matrix (5 capabilities × 3 roles) */}
      <div className="ui-card space-y-4">
        <div className="border-b border-line pb-3">
          <h2 className="farm-section-title">Ma trận phân quyền (5 Năng lực × 3 Vai trò)</h2>
          <p className="text-xs text-text-muted mt-0.5">
            Bản ánh xạ quyền hạn chi tiết giữa các vai trò hệ thống trạm HydraGrow
          </p>
        </div>

        <PermissionMatrix capabilities={CAPABILITIES} roles={['admin', 'operator', 'viewer']} />
      </div>
    </div>
  );
}

export default Roles;
