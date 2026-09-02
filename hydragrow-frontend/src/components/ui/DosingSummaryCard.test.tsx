import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { DosingSummaryCard } from './DosingSummaryCard';

describe('DosingSummaryCard', () => {
  it('hiển thị số lần châm và giờ lần cuối', () => {
    const ts = new Date('2026-09-02T08:40:00').getTime();
    render(<DosingSummaryCard totalCount={3} lastDosedAt={ts} />);
    expect(screen.getByText('Châm dinh dưỡng hôm nay')).toBeInTheDocument();
    expect(screen.getByText(/3 lần/)).toBeInTheDocument();
    expect(screen.getByText(/08:40/)).toBeInTheDocument();
  });

  it('hiển thị trạng thái chưa có lần châm nào khi lastDosedAt là null', () => {
    render(<DosingSummaryCard totalCount={0} lastDosedAt={null} />);
    expect(screen.getByText(/Chưa ghi nhận lần châm nào hôm nay/)).toBeInTheDocument();
  });
});
