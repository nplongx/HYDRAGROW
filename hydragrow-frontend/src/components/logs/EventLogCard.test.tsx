import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { EventLogCard } from './EventLogCard';
import type { SystemEvent } from './EventLogCard';

const technicalEvent: SystemEvent = {
  id: 7,
  device_id: 'd1',
  level: 'info',
  category: 'sensor',
  title: 'Đọc cảm biến EC',
  message: 'EC=1.8mS',
  timestamp: 1000,
  metadata: { source: 'sensor_ec', raw_adc: 512 },
};

describe('EventLogCard', () => {
  it('không hiện nút mở JSON thô khi không truyền onOpenDetail', () => {
    render(<EventLogCard ev={technicalEvent} idx={0} />);
    expect(screen.queryByText('Xem JSON thô')).not.toBeInTheDocument();
  });

  it('hiện nút mở JSON thô khi có onOpenDetail và có metadata', () => {
    render(<EventLogCard ev={technicalEvent} idx={0} onOpenDetail={vi.fn()} />);
    expect(screen.getByText('Xem JSON thô')).toBeInTheDocument();
  });

  it('bấm nút mở JSON thô gọi onOpenDetail với đúng event', () => {
    const onOpenDetail = vi.fn();
    render(<EventLogCard ev={technicalEvent} idx={0} onOpenDetail={onOpenDetail} />);
    fireEvent.click(screen.getByText('Xem JSON thô'));
    expect(onOpenDetail).toHaveBeenCalledWith(technicalEvent);
  });
});
