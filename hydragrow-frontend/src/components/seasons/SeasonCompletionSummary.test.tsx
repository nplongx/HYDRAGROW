import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SeasonCompletionSummary } from './SeasonCompletionSummary';

describe('SeasonCompletionSummary', () => {
  it('hiển thị tên mùa vụ, số ngày trồng, số ảnh nhật ký', () => {
    render(
      <SeasonCompletionSummary
        seasonName="Vụ dâu tây Đông 2026"
        totalDaysGrown={42}
        photoCount={15}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByText('Vụ dâu tây Đông 2026')).toBeInTheDocument();
    expect(screen.getByText(/42 ngày/)).toBeInTheDocument();
    expect(screen.getByText(/15 ảnh/)).toBeInTheDocument();
  });

  it('bấm nút đóng gọi onClose', () => {
    const onClose = vi.fn();
    render(
      <SeasonCompletionSummary seasonName="Vụ A" totalDaysGrown={10} photoCount={0} onClose={onClose} />,
    );
    fireEvent.click(screen.getByText('Bắt đầu mùa vụ tiếp theo'));
    expect(onClose).toHaveBeenCalled();
  });

  it('photoCount=0 → vẫn hiển thị, không báo lỗi (không giả định luôn có ảnh)', () => {
    render(<SeasonCompletionSummary seasonName="Vụ A" totalDaysGrown={10} photoCount={0} onClose={vi.fn()} />);
    expect(screen.getByText(/0 ảnh/)).toBeInTheDocument();
  });
});
