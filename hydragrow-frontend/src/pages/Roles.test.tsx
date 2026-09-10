import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Roles } from './Roles';
import * as apiClient from '../lib/apiClient';

vi.mock('../lib/apiClient', () => ({
  apiGet: vi.fn(),
  apiPost: vi.fn(),
  apiPatch: vi.fn(),
}));

const mockUsers = [
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

describe('Roles Page', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(apiClient.apiGet).mockResolvedValue(mockUsers);
  });

  it('hiển thị danh sách thành viên với role pill và bảng ma trận năng lực', async () => {
    render(<Roles />);

    await waitFor(() => {
      expect(screen.getByText('Quản Trị Viên Trưởng')).toBeInTheDocument();
      expect(screen.getByText('Kỹ Sư Vận Hành')).toBeInTheDocument();
      expect(screen.getByText('Khách Tham Quan')).toBeInTheDocument();
    });

    // Check permission matrix rendered
    expect(screen.getByTestId('permission-matrix')).toBeInTheDocument();
    expect(screen.getByText(/Giám sát & Số liệu/i)).toBeInTheDocument();
    expect(screen.getByText(/Điều khiển & Vận hành/i)).toBeInTheDocument();
    expect(screen.getByText(/Cấu hình nông học & Kịch bản/i)).toBeInTheDocument();
    expect(screen.getByText(/Quản trị hệ thống & Phân quyền/i)).toBeInTheDocument();
  });

  it('thay đổi vai trò thành viên gọi apiPatch với đúng role và scopes', async () => {
    vi.mocked(apiClient.apiPatch).mockResolvedValue({ id: 2, role: 'admin' });
    render(<Roles />);

    await waitFor(() => {
      expect(screen.getByLabelText('Đổi vai trò cho operator@farm.vn')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Đổi vai trò cho operator@farm.vn'), {
      target: { value: 'admin' },
    });

    await waitFor(() => {
      expect(apiClient.apiPatch).toHaveBeenCalledWith(
        '/admin/users/2',
        expect.objectContaining({
          role: 'admin',
          scopes: ['*'],
        })
      );
    });
  });

  it('mở modal thêm thành viên và submit gọi apiPost', async () => {
    vi.mocked(apiClient.apiPost).mockResolvedValue({ status: 'ok' });
    render(<Roles />);

    fireEvent.click(screen.getByRole('button', { name: /Thêm thành viên/i }));

    expect(screen.getByText('Thêm thành viên mới')).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText('operator@farm.vn'), {
      target: { value: 'newuser@farm.vn' },
    });
    fireEvent.change(screen.getByPlaceholderText('Lấy từ Firebase Console > Authentication'), {
      target: { value: 'uid-new-123' },
    });
    fireEvent.change(screen.getByPlaceholderText('Kỹ sư nông học A'), {
      target: { value: 'Kỹ sư C' },
    });

    fireEvent.click(screen.getByRole('button', { name: /Xác nhận cấp quyền/i }));

    await waitFor(() => {
      expect(apiClient.apiPost).toHaveBeenCalledWith(
        '/admin/users',
        expect.objectContaining({
          email: 'newuser@farm.vn',
          firebase_uid: 'uid-new-123',
          display_name: 'Kỹ sư C',
          scopes: expect.arrayContaining(['read:telemetry']),
        })
      );
    });
  });

  it('khoá/mở khoá thành viên gọi apiPatch is_active', async () => {
    vi.mocked(apiClient.apiPatch).mockResolvedValue({ id: 1, is_active: false });
    render(<Roles />);

    await waitFor(() => {
      expect(screen.getByText('Quản Trị Viên Trưởng')).toBeInTheDocument();
    });

    const lockButtons = screen.getAllByRole('button', { name: 'Khóa' });
    fireEvent.click(lockButtons[0]);

    await waitFor(() => {
      expect(apiClient.apiPatch).toHaveBeenCalledWith(
        '/admin/users/1',
        expect.objectContaining({
          is_active: false,
        })
      );
    });
  });
});
