// src/components/logs/EventDetailDrawer.test.tsx
import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { EventDetailDrawer } from './EventDetailDrawer';
import type { SystemEvent } from './EventLogCard';

const event: SystemEvent = {
  id: 42,
  device_id: 'd1',
  level: 'warning',
  category: 'calibration',
  title: 'Cập nhật hệ số EMA',
  message: 'Gain thay đổi do nhiễu thấp',
  timestamp: 1735000000000,
  metadata: { source: 'auto_tune', parameter: 'ec_gain_per_ml', old_value: 1.1, new_value: 1.3 },
};

describe('EventDetailDrawer', () => {
  it('không render gì khi event là null', () => {
    const { container } = render(<EventDetailDrawer event={null} onClose={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('hiển thị tóm tắt dễ hiểu ở trên (tiêu đề + message)', () => {
    render(<EventDetailDrawer event={event} onClose={vi.fn()} />);
    expect(screen.getByText('Cập nhật hệ số EMA')).toBeInTheDocument();
    expect(screen.getByText('Gain thay đổi do nhiễu thấp')).toBeInTheDocument();
  });

  it('JSON thô thu gọn mặc định, chỉ hiện khi bấm mở accordion', () => {
    render(<EventDetailDrawer event={event} onClose={vi.fn()} />);
    expect(screen.queryByText(/"id": 42/)).not.toBeInTheDocument();
    fireEvent.click(screen.getByText('JSON thô'));
    expect(screen.getByText(/"id": 42/)).toBeInTheDocument();
  });

  it('bấm nút đóng gọi onClose', () => {
    const onClose = vi.fn();
    render(<EventDetailDrawer event={event} onClose={onClose} />);
    fireEvent.click(screen.getByLabelText('Đóng chi tiết'));
    expect(onClose).toHaveBeenCalled();
  });
});
