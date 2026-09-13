// src/components/logs/HealthSummaryBar.test.tsx
import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { HealthSummaryBar } from './HealthSummaryBar';

describe('HealthSummaryBar', () => {
  it('hiển thị 0 khi chưa có summary', () => {
    render(<HealthSummaryBar mode="important" onModeChange={vi.fn()} search="" onSearchChange={vi.fn()} />);
    expect(screen.getByText(/0 lần châm EC/)).toBeInTheDocument();
  });

  it('hiển thị đúng số liệu summary truyền vào', () => {
    render(
      <HealthSummaryBar
        summary={{
          window_seconds: 3600,
          ec_dosing_count: 4,
          ph_dosing_count: 2,
          water_operation_count: 1,
          warning_count: 3,
          critical_count: 1,
          latest_ph_dosing_at: null,
        }}
        mode="important"
        onModeChange={vi.fn()}
        search=""
        onSearchChange={vi.fn()}
      />,
    );
    expect(screen.getByText(/4 lần châm EC/)).toBeInTheDocument();
    expect(screen.getByText(/3 cảnh báo · 1 nghiêm trọng/)).toBeInTheDocument();
  });

  it('gõ vào ô tìm kiếm gọi onSearchChange', () => {
    const onSearchChange = vi.fn();
    render(<HealthSummaryBar mode="important" onModeChange={vi.fn()} search="" onSearchChange={onSearchChange} />);
    fireEvent.change(screen.getByLabelText('Tìm kiếm nhật ký'), { target: { value: 'châm ec' } });
    expect(onSearchChange).toHaveBeenCalledWith('châm ec');
  });

  it('bật toggle gọi onModeChange với all_technical', () => {
    const onModeChange = vi.fn();
    render(<HealthSummaryBar mode="important" onModeChange={onModeChange} search="" onSearchChange={vi.fn()} />);
    fireEvent.click(screen.getByRole('switch'));
    expect(onModeChange).toHaveBeenCalledWith('all_technical');
  });

  it('không hiện nút xoá khi search rỗng', () => {
    render(<HealthSummaryBar mode="important" onModeChange={vi.fn()} search="" onSearchChange={vi.fn()} />);
    expect(screen.queryByLabelText('Xoá tìm kiếm')).not.toBeInTheDocument();
  });

  it('hiện nút xoá + số kết quả khi có search và resultCount', () => {
    render(
      <HealthSummaryBar
        mode="important"
        onModeChange={vi.fn()}
        search="bơm"
        onSearchChange={vi.fn()}
        resultCount={7}
      />,
    );
    expect(screen.getByLabelText('Xoá tìm kiếm')).toBeInTheDocument();
    expect(screen.getByText('7 kết quả')).toBeInTheDocument();
  });

  it('bấm nút xoá gọi onSearchChange("")', () => {
    const onSearchChange = vi.fn();
    render(
      <HealthSummaryBar mode="important" onModeChange={vi.fn()} search="bơm" onSearchChange={onSearchChange} resultCount={0} />,
    );
    fireEvent.click(screen.getByLabelText('Xoá tìm kiếm'));
    expect(onSearchChange).toHaveBeenCalledWith('');
  });
});
