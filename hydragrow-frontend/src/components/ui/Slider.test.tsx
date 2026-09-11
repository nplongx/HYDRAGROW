import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Slider } from './Slider';

describe('Slider', () => {
  it('nhận giá trị hiện tại từ prop value', () => {
    render(<Slider value={72} onChange={vi.fn()} ariaLabel="Công suất bơm" />);
    const slider = screen.getByRole('slider', { name: 'Công suất bơm' });
    expect(slider).toHaveValue(String(72));
    expect(slider).toHaveAttribute('min', '0');
    expect(slider).toHaveAttribute('max', '100');
  });

  it('gọi onChange khi đổi giá trị', () => {
    const onChange = vi.fn();
    render(<Slider value={50} onChange={onChange} ariaLabel="Công suất" />);
    fireEvent.change(screen.getByRole('slider', { name: 'Công suất' }), { target: { value: '75' } });
    expect(onChange).toHaveBeenCalledWith(75);
  });

  it('gọi onCommit khi nhả chuột', () => {
    const onCommit = vi.fn();
    render(<Slider value={50} onChange={vi.fn()} onCommit={onCommit} ariaLabel="Công suất" />);
    fireEvent.mouseUp(screen.getByRole('slider', { name: 'Công suất' }));
    expect(onCommit).toHaveBeenCalledWith(50);
  });

  it('bị vô hiệu hoá khi disabled', () => {
    render(<Slider value={50} onChange={vi.fn()} disabled ariaLabel="Công suất" />);
    expect(screen.getByRole('slider', { name: 'Công suất' })).toBeDisabled();
  });

  it('kẹp giá trị trong đoạn [min, max]', () => {
    render(<Slider value={150} min={0} max={100} onChange={vi.fn()} ariaLabel="Công suất" />);
    expect(screen.getByRole('slider', { name: 'Công suất' })).toHaveValue(String(100));
  });
});