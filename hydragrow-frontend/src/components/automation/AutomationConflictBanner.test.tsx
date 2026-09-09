import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { AutomationConflictBanner } from './AutomationConflictBanner';
import type { ScheduleConflict } from '../../lib/automation/scheduleConflicts';
import type { UserScript } from '../../types/automation';

const script = (id: string, name: string): UserScript => ({
  id,
  device_id: 'dev-1',
  kind: 'action_command',
  name,
  source: '',
  enabled: true,
  ir_json: null,
  created_at: '',
  updated_at: '',
});

describe('AutomationConflictBanner', () => {
  it('không render gì khi không có xung đột', () => {
    const { container } = render(<AutomationConflictBanner conflicts={[]} onViewDetail={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('hiển thị tên 2 flow và thiết bị xung đột đầu tiên', () => {
    const conflicts: ScheduleConflict[] = [
      {
        flowA: script('a', 'Tưới buổi sáng'),
        flowB: script('b', 'Xả bồn định kỳ'),
        sharedPumps: ['WATER_PUMP_IN'],
        cronExpression: '0 0 6 * * *',
      },
    ];
    render(<AutomationConflictBanner conflicts={conflicts} onViewDetail={vi.fn()} />);
    expect(screen.getByText(/Tưới buổi sáng/)).toBeInTheDocument();
    expect(screen.getByText(/Xả bồn định kỳ/)).toBeInTheDocument();
  });

  it('hiện +N khi có nhiều hơn 1 xung đột', () => {
    const conflicts: ScheduleConflict[] = [
      {
        flowA: script('a', 'Flow A'),
        flowB: script('b', 'Flow B'),
        sharedPumps: ['PUMP_A'],
        cronExpression: '0 0 6 * * *',
      },
      {
        flowA: script('c', 'Flow C'),
        flowB: script('d', 'Flow D'),
        sharedPumps: ['PH_UP'],
        cronExpression: '0 0 7 * * *',
      },
    ];
    render(<AutomationConflictBanner conflicts={conflicts} onViewDetail={vi.fn()} />);
    expect(screen.getByText('+1 xung đột khác')).toBeInTheDocument();
  });

  it('gọi onViewDetail với flowA khi bấm Xem chi tiết', () => {
    const onViewDetail = vi.fn();
    const flowA = script('a', 'Tưới buổi sáng');
    const conflicts: ScheduleConflict[] = [
      {
        flowA,
        flowB: script('b', 'Xả bồn định kỳ'),
        sharedPumps: ['WATER_PUMP_IN'],
        cronExpression: '0 0 6 * * *',
      },
    ];
    render(<AutomationConflictBanner conflicts={conflicts} onViewDetail={onViewDetail} />);
    fireEvent.click(screen.getByText('Xem chi tiết'));
    expect(onViewDetail).toHaveBeenCalledWith(flowA);
  });
});
