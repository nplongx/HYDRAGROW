import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DeviceStatePill } from './DeviceStatePill';

describe('DeviceStatePill', () => {
  it.each([
    ['online', 'Trực tuyến', 'bg-success-bg text-status'],
    ['offline', 'Ngoại tuyến', 'bg-surface-muted text-faint'],
    ['warning', 'Cảnh báo', 'bg-warning-bg text-warn-deep'],
    ['dosing', 'Đang châm', 'bg-info-bg text-info-fg'],
    ['auto', 'Tự động', 'bg-pill text-primary'],
    ['manual', 'Thủ công', 'bg-surface-muted text-text-muted'],
  ] as const)('state %s hiển thị nhãn và màu đúng', (state, label, classes) => {
    render(<DeviceStatePill state={state} />);
    const pill = screen.getByText(label);
    for (const clazz of classes.split(' ')) {
      expect(pill.className).toContain(clazz);
    }
  });

  it('cho phép ghi đè nhãn', () => {
    render(<DeviceStatePill state="warning" label="Mực nước thấp" />);
    expect(screen.getByText('Mực nước thấp')).toBeInTheDocument();
  });

  it('state không xác định → fallback manual', () => {
    render(<DeviceStatePill state="unknown-state" />);
    expect(screen.getByText('Thủ công')).toBeInTheDocument();
  });
});