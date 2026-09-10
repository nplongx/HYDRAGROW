import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Switch } from './Switch';

describe('Switch', () => {
  it('render track kích thước chuẩn 38x22 (Figma) với màu OFF là bg-toggleoff', () => {
    render(<Switch checked={false} onChange={() => {}} />);
    const track = screen.getByRole('switch');
    expect(track).toHaveAttribute('aria-checked', 'false');
    expect(track.className).toContain('w-[38px]');
    expect(track.className).toContain('h-[22px]');
    expect(track.className).toContain('bg-toggleoff');
    expect(track.className).toContain('rounded-full');
  });

  it('toggle bật dùng bg-primary (#15803D) và thumb di chuyển sang trái', () => {
    render(<Switch checked={true} onChange={() => {}} />);
    const track = screen.getByRole('switch');
    expect(track.className).toContain('bg-primary');
    expect(track.className).not.toContain('bg-toggleoff');
    const thumb = track.querySelector('span');
    expect(thumb?.className).toContain('translate-x-4');
    expect(thumb?.className).toContain('w-[18px] h-[18px]');
  });

  it('gọi onChange khi click toggle', () => {
    const onChange = vi.fn();
    render(<Switch checked={false} onChange={onChange} />);
    screen.getByRole('switch').click();
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it('không gọi onChange khi disabled', () => {
    const onChange = vi.fn();
    render(<Switch checked={false} onChange={onChange} disabled />);
    screen.getByRole('switch').click();
    expect(onChange).not.toHaveBeenCalled();
  });

  it('hiển thị label với màu chữ primary-deep', () => {
    render(<Switch checked={true} onChange={() => {}} label="Bật bơm" />);
    const label = screen.getByText('Bật bơm');
    expect(label.className).toContain('text-primary-deep');
  });
});