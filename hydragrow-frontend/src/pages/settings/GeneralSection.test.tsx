import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { GeneralSection } from './GeneralSection';

describe('GeneralSection', () => {
  const renderSection = () =>
    render(
      <MemoryRouter initialEntries={['/settings']}>
        <Routes>
          <Route
            path="/settings"
            element={
              <GeneralSection
                userEmail="test@hydragrow.dev"
                onLogout={vi.fn()}
                onGoToPairing={() => {}}
                isAdvancedMode={false}
                onToggleAdvancedMode={() => {}}
              />
            }
          />
          <Route path="/pairing" element={<div>Pairing page</div>} />
        </Routes>
      </MemoryRouter>
    );

  it('gọi onLogout khi bấm Đăng xuất', () => {
    const onLogout = vi.fn();
    render(
      <MemoryRouter>
        <GeneralSection
          userEmail="test@hydragrow.dev"
          onLogout={onLogout}
          onGoToPairing={() => {}}
          isAdvancedMode={false}
          onToggleAdvancedMode={() => {}}
        />
      </MemoryRouter>
    );
    fireEvent.click(screen.getByText('Đăng xuất'));
    expect(onLogout).toHaveBeenCalledOnce();
  });

  it('điều hướng tới trang ghép thiết bị khi bấm Ghép thiết bị mới', () => {
    renderSection();
    fireEvent.click(screen.getByText('Ghép thiết bị mới'));
    expect(screen.getByText('Pairing page')).toBeInTheDocument();
  });
});
