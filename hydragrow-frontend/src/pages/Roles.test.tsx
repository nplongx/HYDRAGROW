import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Roles } from './Roles';
import * as adminApiModule from '../api/admin';

vi.mock('../api/admin', () => ({
  adminApi: {
    listUsers: vi.fn(),
    provisionUser: vi.fn(),
    updateUser: vi.fn(),
  },
}));

const mockUsers: adminApiModule.AdminUser[] = [
  {
    id: 1,
    firebase_uid: 'uid-admin-1',
    email: 'admin@farm.vn',
    display_name: 'Quản Trị Viên Trưởng',
    role: 'admin',
    scopes: ['*'],
    is_active: true,
  },
  {
    id: 2,
    firebase_uid: 'uid-op-2',
    email: 'operator@farm.vn',
    display_name: 'Kỹ Sư Vận Hành',
    role: 'operator',
    scopes: ['read:telemetry', 'write:config'],
    is_active: true,
  },
  {
    id: 3,
    firebase_uid: 'uid-view-3',
    email: 'viewer@farm.vn',
    display_name: 'Khách Tham Quan',
    role: 'viewer',
    scopes: ['read:telemetry'],
    is_active: false,
  },
];

function renderRoles() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <Roles />
    </QueryClientProvider>,
  );
}

describe('Roles Page', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(adminApiModule.adminApi.listUsers).mockResolvedValue(mockUsers);
  });

  it('hiển thị danh sách thành viên với role pill và bảng ma trận năng lực', async () => {
    renderRoles();

    await waitFor(() => {
      expect(screen.getByText('Quản Trị Viên Trưởng')).toBeInTheDocument();
      expect(screen.getByText('Kỹ Sư Vận Hành')).toBeInTheDocument();
      expect(screen.getByText('Khách Tham Quan')).toBeInTheDocument();
    });

    // Check permission matrix rendered
    const matrix = screen.getByTestId('permission-matrix');
    expect(matrix).toBeInTheDocument();
    expect(matrix).toHaveAttribute('role', 'grid');
    expect(screen.getByText(/Ma trận phân quyền \(5 Năng lực × 3 Vai trò\)/)).toBeInTheDocument();
    expect(screen.getByText(/Giám sát & Số liệu/i)).toBeInTheDocument();
    expect(screen.getByText(/Điều khiển & Vận hành/i)).toBeInTheDocument();
    expect(screen.getByText(/Cấu hình nông học & Kịch bản/i)).toBeInTheDocument();
    expect(screen.getByText(/OTA & Device Network/i)).toBeInTheDocument();
    expect(screen.getByText(/Permissions & Member Invitation/i)).toBeInTheDocument();
  });

  it('thay đổi vai trò thành viên gọi apiPatch với đúng role và scopes', async () => {
    vi.mocked(adminApiModule.adminApi.updateUser).mockResolvedValue({ id: 2, role: 'admin' });
    renderRoles();

    await waitFor(() => {
      expect(screen.getByLabelText('Đổi vai trò cho operator@farm.vn')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Đổi vai trò cho operator@farm.vn'), {
      target: { value: 'admin' },
    });

    await waitFor(() => {
      expect(adminApiModule.adminApi.updateUser).toHaveBeenCalledWith(
        2,
        expect.objectContaining({
          role: 'admin',
          scopes: ['*'],
        })
      );
    });
  });

  it('mở modal thêm thành viên và submit gọi apiPost với dữ liệu hợp lệ', async () => {
    vi.mocked(adminApiModule.adminApi.provisionUser).mockResolvedValue({ status: 'ok' });
    renderRoles();

    fireEvent.click(screen.getByRole('button', { name: /Thêm thành viên/i }));

    expect(screen.getByText('Thêm thành viên mới')).toBeInTheDocument();
    expect(screen.getByTestId('group-identity')).toBeInTheDocument();
    expect(screen.getByTestId('group-role')).toBeInTheDocument();
    expect(screen.getByTestId('group-confirmation')).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText('operator@farm.vn'), {
      target: { value: 'newuser@farm.vn' },
    });
    fireEvent.change(screen.getByPlaceholderText('Lấy từ Firebase Console > Authentication'), {
      target: { value: 'uidNew123' },
    });
    fireEvent.change(screen.getByPlaceholderText('Kỹ sư nông học A'), {
      target: { value: 'Kỹ sư C' },
    });

    fireEvent.click(screen.getByRole('button', { name: /Xác nhận cấp quyền/i }));

    await waitFor(() => {
      expect(adminApiModule.adminApi.provisionUser).toHaveBeenCalledWith(
        expect.objectContaining({
          email: 'newuser@farm.vn',
          firebase_uid: 'uidNew123',
          display_name: 'Kỹ sư C',
          scopes: expect.arrayContaining(['read:telemetry']),
        })
      );
    });
  });

  it('huỷ modal thêm thành viên sẽ đóng modal', async () => {
    renderRoles();

    fireEvent.click(screen.getByRole('button', { name: /Thêm thành viên/i }));
    expect(screen.getByText('Thêm thành viên mới')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Huỷ/i }));
    await waitFor(() => {
      expect(screen.queryByText('Thêm thành viên mới')).not.toBeInTheDocument();
    });
  });

  it('khoá/mở khoá thành viên gọi apiPatch is_active', async () => {
    vi.mocked(adminApiModule.adminApi.updateUser).mockResolvedValue({ id: 1, is_active: false });
    renderRoles();

    await waitFor(() => {
      expect(screen.getByText('Quản Trị Viên Trưởng')).toBeInTheDocument();
    });

    const lockButtons = screen.getAllByRole('button', { name: 'Khóa' });
    fireEvent.click(lockButtons[0]);

    await waitFor(() => {
      expect(adminApiModule.adminApi.updateUser).toHaveBeenCalledWith(
        1,
        expect.objectContaining({
          is_active: false,
        })
      );
    });
  });
});
