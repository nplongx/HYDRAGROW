import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { QuickActionBar } from './QuickActionBar';

describe('QuickActionBar', () => {
  it('gọi đúng callback khi bấm từng nút', () => {
    const onDose = vi.fn();
    const onPausePumps = vi.fn();
    const onViewAlerts = vi.fn();
    render(<QuickActionBar onDose={onDose} onPausePumps={onPausePumps} onViewAlerts={onViewAlerts} />);

    fireEvent.click(screen.getByText('Châm dinh dưỡng'));
    fireEvent.click(screen.getByText('Tạm dừng bơm'));
    fireEvent.click(screen.getByText('Xem cảnh báo'));

    expect(onDose).toHaveBeenCalledOnce();
    expect(onPausePumps).toHaveBeenCalledOnce();
    expect(onViewAlerts).toHaveBeenCalledOnce();
  });

  it('hiển thị Tiếp tục bơm khi pumpsPaused là true', () => {
    const onDose = vi.fn();
    const onPausePumps = vi.fn();
    const onViewAlerts = vi.fn();
    render(<QuickActionBar onDose={onDose} onPausePumps={onPausePumps} onViewAlerts={onViewAlerts} pumpsPaused={true} />);

    expect(screen.getByText('Tiếp tục bơm')).toBeInTheDocument();
  });
});
