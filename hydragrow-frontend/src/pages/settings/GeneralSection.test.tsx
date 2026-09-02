import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { GeneralSection } from './GeneralSection';

describe('GeneralSection', () => {
  it('gọi onLogout khi bấm Đăng xuất', () => {
    const onLogout = vi.fn();
    render(
      <GeneralSection
        userEmail="test@hydragrow.dev"
        onLogout={onLogout}
        onGoToPairing={() => {}}
        isAdvancedMode={false}
        onToggleAdvancedMode={() => {}}
      />
    );
    fireEvent.click(screen.getByText('Đăng xuất'));
    expect(onLogout).toHaveBeenCalledOnce();
  });
});
