import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Badge } from './Badge';

describe('Badge', () => {
  it('mặc định tone neutral', () => {
    render(<Badge>Trạng thái</Badge>);
    const badge = screen.getByText('Trạng thái');
    expect(badge.className).toContain('bg-surface-muted');
    expect(badge.className).toContain('rounded-full');
  });

  it.each([
    ['success', 'bg-success-bg text-status'],
    ['warning', 'bg-warning-bg text-warn-deep'],
    ['danger', 'bg-danger-bg text-error'],
    ['info', 'bg-info-bg text-info-fg'],
  ] as const)('tone %s áp dụng đúng màu', (tone, classes) => {
    render(<Badge tone={tone}>Nhãn</Badge>);
    for (const clazz of classes.split(' ')) {
      expect(screen.getByText('Nhãn').className).toContain(clazz);
    }
  });

  it('hiển thị chấm tròn khi dot=true', () => {
    const { container } = render(<Badge tone="success" dot>Online</Badge>);
    expect(container.querySelector('span[aria-hidden="true"]')).not.toBeNull();
  });
});