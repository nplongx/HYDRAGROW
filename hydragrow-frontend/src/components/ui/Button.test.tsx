import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Button } from './Button';

describe('Button', () => {
  it('mặc định là variant primary', () => {
    render(<Button>Lưu</Button>);
    const button = screen.getByRole('button', { name: 'Lưu' });
    expect(button.className).toContain('bg-primary-deep text-white');
    expect(button.className).toContain('rounded-xl');
    expect(button).toHaveAttribute('type', 'button');
  });

  it('secondary dùng đường viền primary', () => {
    render(<Button variant="secondary">Huỷ</Button>);
    expect(screen.getByRole('button', { name: 'Huỷ' }).className).toContain('border-primary');
  });

  it('danger dùng nền đỏ', () => {
    render(<Button variant="danger">Dừng</Button>);
    expect(screen.getByRole('button', { name: 'Dừng' }).className).toContain('bg-error');
  });

  it('loading vô hiệu hoá và hiển thị spinner', () => {
    render(<Button loading>Đang gửi</Button>);
    const button = screen.getByRole('button', { name: 'Đang gửi' });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute('aria-busy', 'true');
  });

  it('bắt sự kiện click khi chưa disabled', () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Nhấn</Button>);
    fireEvent.click(screen.getByRole('button', { name: 'Nhấn' }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});