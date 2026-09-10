import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { StatusPill } from './StatusPill';

describe('StatusPill', () => {
  it('không hiển thị gì khi không có commandStatus', () => {
    const { container } = render(<StatusPill commandStatus={undefined} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('hiển thị "Đang gửi…" khi commandStatus là sending', () => {
    render(<StatusPill commandStatus="sending" />);
    const pill = screen.getByText('Đang gửi…');
    expect(pill).toBeInTheDocument();
    expect(pill.className).toContain('bg-[#FFFBEB]');
    expect(pill.className).toContain('text-warn-deep');
    expect(pill.className).toContain('rounded-full');
  });

  it('hiển thị "✓ Xác nhận" khi commandStatus là accepted', () => {
    render(<StatusPill commandStatus="accepted" />);
    const pill = screen.getByText('✓ Xác nhận');
    expect(pill).toBeInTheDocument();
    expect(pill.className).toContain('bg-pill');
    expect(pill.className).toContain('text-status');
  });

  it.each(['network_error', 'rate_limited', 'HTTP 500', 'safety_blocked'])(
    'hiển thị "⚠ Lỗi phản hồi" khi commandStatus là %s',
    (status) => {
      render(<StatusPill commandStatus={status} />);
      expect(screen.getByText('⚠ Lỗi phản hồi')).toBeInTheDocument();
    },
  );
});
