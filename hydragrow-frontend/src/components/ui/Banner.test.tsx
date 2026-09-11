import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Button } from './Button';
import { Banner } from './Banner';

describe('Banner', () => {
  it.each([
    ['info', 'bg-info-bg', 'text-info-fg'],
    ['warning', 'bg-warning-bg', 'text-warn-deep'],
    ['danger', 'bg-danger-bg', 'text-error'],
  ] as const)('tone %s áp dụng màu đúng', (tone, panelClass, textClass) => {
    render(<Banner tone={tone} title="Thông báo" />);
    const alert = screen.getByRole('alert');
    expect(alert.className).toContain(panelClass);
    expect(alert.className).toContain('rounded-xl');
    expect(screen.getByText('Thông báo').className).toContain(textClass);
  });

  it('hiển thị tiêu đề và mô tả', () => {
    render(
      <Banner tone="danger" title="Mất tín hiệu cảm biến">
        Kiểm tra kết nối trạm
      </Banner>,
    );
    expect(screen.getByText('Mất tín hiệu cảm biến')).toBeInTheDocument();
    expect(screen.getByText('Kiểm tra kết nối trạm')).toBeInTheDocument();
  });

  it('cho phép đính kèm action', () => {
    render(
      <Banner tone="warning" title="Cần xử lý" action={<Button size="sm" variant="secondary">Xem</Button>}>
        Chi tiết
      </Banner>,
    );
    expect(screen.getByRole('button', { name: 'Xem' })).toBeInTheDocument();
  });
});