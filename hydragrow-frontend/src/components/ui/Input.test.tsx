import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Input } from './Input';

describe('Input', () => {
  it('hiển thị label và nhãn đi kèm', () => {
    render(<Input label="Nhiệt độ" hint="Đơn vị °C" />);
    expect(screen.getByLabelText('Nhiệt độ')).toBeInTheDocument();
    expect(screen.getByText('Đơn vị °C')).toBeInTheDocument();
  });

  it('hiển thị lỗi và đánh dấu aria-invalid', () => {
    render(<Input label="EC" error="Giá trị không hợp lệ" />);
    const input = screen.getByLabelText('EC');
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(input.className).toContain('border-error');
    expect(screen.getByRole('alert')).toHaveTextContent('Giá trị không hợp lệ');
  });

  it('không hiển thị hint khi có lỗi', () => {
    render(<Input label="pH" hint="0–14" error="Bắt buộc" />);
    expect(screen.queryByText('0–14')).not.toBeInTheDocument();
  });

  it('chuyển tiếp props còn lại xuống input', () => {
    render(<Input placeholder="Nhập…" disabled />);
    const input = screen.getByPlaceholderText('Nhập…');
    expect(input).toBeDisabled();
  });
});