import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Select } from './Select';

const OPTIONS = [
  { value: 'auto', label: 'Tự động' },
  { value: 'manual', label: 'Thủ công' },
];

describe('Select', () => {
  it('hiển thị label và các tuỳ chọn', () => {
    render(<Select label="Chế độ" options={OPTIONS} />);
    expect(screen.getByLabelText('Chế độ')).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Tự động' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Thủ công' })).toBeInTheDocument();
  });

  it('hiển thị placeholder khi được truyền', () => {
    render(<Select options={OPTIONS} placeholder="Chọn…" />);
    expect(screen.getByRole('option', { name: 'Chọn…' })).toBeDisabled();
  });

  it('gọi onChange khi đổi lựa chọn', () => {
    const onChange = vi.fn();
    render(<Select label="Chế độ" options={OPTIONS} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText('Chế độ'), { target: { value: 'manual' } });
    expect(onChange).toHaveBeenCalled();
  });

  it('hiển thị thông báo lỗi', () => {
    render(<Select label="Chế độ" options={OPTIONS} error="Bắt buộc chọn" />);
    expect(screen.getByRole('alert')).toHaveTextContent('Bắt buộc chọn');
  });
});