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
    expect(screen.getByText('Đang gửi…')).toBeInTheDocument();
  });

  it('hiển thị "✓ Xác nhận" khi commandStatus là accepted', () => {
    render(<StatusPill commandStatus="accepted" />);
    expect(screen.getByText('✓ Xác nhận')).toBeInTheDocument();
  });

  it.each(['network_error', 'rate_limited', 'HTTP 500', 'safety_blocked'])(
    'hiển thị "⚠ Lỗi phản hồi" khi commandStatus là %s',
    (status) => {
      render(<StatusPill commandStatus={status} />);
      expect(screen.getByText('⚠ Lỗi phản hồi')).toBeInTheDocument();
    },
  );
});
