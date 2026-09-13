import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { SeasonStageChecklist } from './SeasonStageChecklist';

describe('SeasonStageChecklist', () => {
  const items = [
    { name: 'Nảy mầm', status: 'done' as const },
    { name: 'Sinh trưởng', status: 'current' as const },
    { name: 'Ra hoa', status: 'upcoming' as const },
  ];

  it('render đủ tên 3 giai đoạn', () => {
    render(<SeasonStageChecklist items={items} remainingDaysCount={12} />);
    expect(screen.getByText('Nảy mầm')).toBeInTheDocument();
    expect(screen.getByText('Sinh trưởng')).toBeInTheDocument();
    expect(screen.getByText('Ra hoa')).toBeInTheDocument();
  });

  it('hiện số ngày còn lại — open loop có lối ra rõ ràng (zeigarnik-effect: "always provide a clear path to completion")', () => {
    render(<SeasonStageChecklist items={items} remainingDaysCount={12} />);
    expect(screen.getByText(/Còn 12 ngày/)).toBeInTheDocument();
  });

  it('giai đoạn done có icon check, current được nhấn mạnh', () => {
    render(<SeasonStageChecklist items={items} remainingDaysCount={12} />);
    const doneItem = screen.getByText('Nảy mầm').closest('li');
    const currentItem = screen.getByText('Sinh trưởng').closest('li');
    expect(doneItem?.className).toContain('text-faint');
    expect(currentItem?.className).toContain('text-primary-deep');
  });
});
