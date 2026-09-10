import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { SeasonHistoryList } from './SeasonHistoryList';
import type { CropSeason } from '../../types/models';

const DAY = 86400000;

const completedSeason: CropSeason = {
  id: 's1',
  device_id: 'd1',
  name: 'Vụ 1',
  plant_type: 'Xà lách',
  description: null,
  start_time: new Date(Date.now() - 40 * DAY).toISOString(),
  end_time: new Date(Date.now() - 10 * DAY).toISOString(),
  status: 'completed',
};

const activeSeason: CropSeason = {
  id: 's2',
  device_id: 'd1',
  name: 'Vụ 2',
  plant_type: 'Cải thảo',
  description: null,
  start_time: new Date(Date.now() - 7 * DAY).toISOString(),
  end_time: null,
  status: 'active',
};

describe('SeasonHistoryList — thời lượng & chevron', () => {
  it('hiển thị số ngày của mùa vụ đã hoàn thành', () => {
    render(<SeasonHistoryList seasons={[completedSeason]} />);
    expect(screen.getByText(/30 ngày/)).toBeInTheDocument();
  });

  it('hiển thị số ngày của mùa vụ đang chạy và có chevron khi onSelect được truyền', () => {
    const onSelect = vi.fn();
    render(<SeasonHistoryList seasons={[activeSeason]} onSelect={onSelect} />);
    expect(screen.getByText(/7 ngày/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Vụ 2/ }));
    expect(onSelect).toHaveBeenCalledWith(activeSeason);
  });

  it('không hiển thị chevron khi không có onSelect', () => {
    render(<SeasonHistoryList seasons={[activeSeason]} />);
    const btn = screen.getByRole('button', { name: /Vụ 2/ });
    expect(btn.className).toContain('cursor-default');
  });
});