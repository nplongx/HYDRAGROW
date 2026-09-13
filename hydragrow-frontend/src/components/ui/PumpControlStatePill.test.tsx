// hydragrow-frontend/src/components/ui/PumpControlStatePill.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { PumpControlStatePill } from './PumpControlStatePill';

describe('PumpControlStatePill', () => {
  it('idle → "Sẵn sàng", token bg-surface-muted/text-faint', () => {
    render(<PumpControlStatePill state="idle" reason={null} />);
    const pill = screen.getByText('Sẵn sàng');
    expect(pill.className).toContain('bg-surface-muted');
    expect(pill.className).toContain('text-faint');
  });

  it('running → "Đang chạy", token bg-success-bg/text-status', () => {
    render(<PumpControlStatePill state="running" reason={null} />);
    const pill = screen.getByText('Đang chạy');
    expect(pill.className).toContain('bg-success-bg');
    expect(pill.className).toContain('text-status');
  });

  it('locked (auto_mode) → "Đã khoá", tooltip đúng lý do', () => {
    render(<PumpControlStatePill state="locked" reason="auto_mode" />);
    const pill = screen.getByText('Đã khoá');
    expect(pill.className).toContain('bg-warning-bg');
    expect(pill.className).toContain('text-warn-deep');
    expect(pill).toHaveAttribute('title', 'Tự động (MIMO) đang quản lý bơm này');
  });

  it('locked (interlock) → tooltip đúng lý do interlock', () => {
    render(<PumpControlStatePill state="locked" reason="interlock" />);
    expect(screen.getByText('Đã khoá')).toHaveAttribute(
      'title',
      'Đã khoá vì thiết bị xung khắc đang chạy — tránh trung hoà lẫn nhau',
    );
  });
});
