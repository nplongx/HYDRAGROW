// src/components/logs/CycleEventCard.test.tsx
import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CycleEventCard } from './CycleEventCard';
import type { SystemEvent } from './EventLogCard';

const events: SystemEvent[] = [
  { id: 1, device_id: 'd1', level: 'info', category: 'dosing', title: 'Bắt đầu châm EC', message: '', timestamp: 1000, metadata: { cycle_id: 'cyc-9' } },
  { id: 2, device_id: 'd1', level: 'success', category: 'dosing', title: 'Hoàn tất châm EC', message: 'Đã bơm 6ml', timestamp: 2000, metadata: { cycle_id: 'cyc-9' } },
  { id: 3, device_id: 'd1', level: 'info', category: 'water', title: 'Trộn tuần hoàn', message: '', timestamp: 3000, metadata: { cycle_id: 'cyc-9' } },
];

describe('CycleEventCard', () => {
  it('hiển thị số bước và tiêu đề của bước đầu tiên', () => {
    render(<CycleEventCard cycleId="cyc-9" events={events} onOpenDetail={vi.fn()} />);
    expect(screen.getAllByText('Bắt đầu châm EC')[0]).toBeInTheDocument();
    expect(screen.getByText(/3 bước/)).toBeInTheDocument();
    expect(screen.getByText('cyc-9')).toBeInTheDocument();
  });

  it('liệt kê đủ 3 bước theo đúng thứ tự thời gian', () => {
    render(<CycleEventCard cycleId="cyc-9" events={[...events].reverse()} onOpenDetail={vi.fn()} />);
    const items = screen.getAllByRole('listitem');
    expect(items).toHaveLength(3);
    expect(items[0]).toHaveTextContent('Bắt đầu châm EC');
    expect(items[2]).toHaveTextContent('Trộn tuần hoàn');
  });

  it('bấm vào 1 bước gọi onOpenDetail với đúng event', () => {
    const onOpenDetail = vi.fn();
    render(<CycleEventCard cycleId="cyc-9" events={events} onOpenDetail={onOpenDetail} />);
    fireEvent.click(screen.getByText('Hoàn tất châm EC'));
    expect(onOpenDetail).toHaveBeenCalledWith(events[1]);
  });
});
