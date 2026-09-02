import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { AlertBell } from './AlertBell';

describe('AlertBell', () => {
  it('gọi onClick khi bấm chuông', () => {
    const onClick = vi.fn();
    render(<AlertBell unreadCount={0} onClick={onClick} />);
    fireEvent.click(screen.getByRole('button', { name: /cảnh báo/i }));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it('hiển thị badge số lượng khi unreadCount > 0', () => {
    render(<AlertBell unreadCount={3} onClick={() => {}} />);
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('không hiển thị badge khi unreadCount = 0', () => {
    render(<AlertBell unreadCount={0} onClick={() => {}} />);
    expect(screen.queryByText('0')).not.toBeInTheDocument();
  });

  it('hiển thị 9+ khi unreadCount > 9', () => {
    render(<AlertBell unreadCount={12} onClick={() => {}} />);
    expect(screen.getByText('9+')).toBeInTheDocument();
  });
});
