import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { GeneralSection } from './GeneralSection';

describe('GeneralSection', () => {
  it('gọi onLogout khi bấm Đăng xuất', () => {
    const onLogout = vi.fn();
    render(
      <MemoryRouter>
        <GeneralSection
          userEmail="test@hydragrow.dev"
          onLogout={onLogout}
          isAdvancedMode={false}
          onToggleAdvancedMode={() => {}}
        />
      </MemoryRouter>
    );
    fireEvent.click(screen.getByText('Đăng xuất'));
    expect(onLogout).toHaveBeenCalledOnce();
  });

  it('điều hướng tới trang ghép thiết bị khi bấm Ghép thiết bị mới', () => {
    render(
      <MemoryRouter initialEntries={['/settings']}>
        <Routes>
          <Route
            path="/settings"
            element={
              <GeneralSection
                userEmail="test@hydragrow.dev"
                onLogout={vi.fn()}
                isAdvancedMode={false}
                onToggleAdvancedMode={() => {}}
              />
            }
          />
          <Route path="/pairing" element={<div>Pairing page</div>} />
        </Routes>
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Ghép thiết bị mới' }));
    expect(screen.getByText('Pairing page')).toBeInTheDocument();
  });

  it('gọi callback khi đổi chế độ hoạt động', () => {
    const onControlModeChange = vi.fn();
    render(
      <MemoryRouter>
        <GeneralSection
          userEmail="test@hydragrow.dev"
          onLogout={vi.fn()}
          isAdvancedMode={false}
          onToggleAdvancedMode={() => {}}
          controlMode="auto"
          onControlModeChange={onControlModeChange}
        />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole('button', { name: 'Thủ công' }));
    expect(onControlModeChange).toHaveBeenCalledWith('manual');
  });

  it('hiển thị vai trò của người dùng và nút quản lý vai trò', () => {
    render(
      <MemoryRouter initialEntries={['/settings']}>
        <Routes>
          <Route
            path="/settings"
            element={
              <GeneralSection
                userEmail="test@hydragrow.dev"
                userRole="Vận hành viên"
                onLogout={vi.fn()}
                isAdvancedMode={false}
                onToggleAdvancedMode={() => {}}
              />
            }
          />
          <Route path="/roles" element={<div>Roles page</div>} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText('Vai trò của bạn')).toBeInTheDocument();
    expect(screen.getByText('Vận hành viên')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Quản lý thành viên & vai trò/i }));
    expect(screen.getByText('Roles page')).toBeInTheDocument();
  });
});
