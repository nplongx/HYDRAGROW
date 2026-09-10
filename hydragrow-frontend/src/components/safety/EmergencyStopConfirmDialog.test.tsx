import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { EmergencyStopConfirmDialog } from './EmergencyStopConfirmDialog';

describe('EmergencyStopConfirmDialog', () => {
  it('liệt kê đúng các thiết bị đang chạy', () => {
    render(
      <EmergencyStopConfirmDialog
        open={true}
        runningPumps={{ pump_a: true, ph_up: true, mist_valve: false }}
        runningPwm={{ pump_a: 75, ph_up: 50 }}
        onCancel={vi.fn()}
        onConfirm={vi.fn()}
        isSubmitting={false}
      />,
    );
    expect(screen.getByText(/Dinh dưỡng A/)).toBeInTheDocument();
    expect(screen.getByText('Dinh dưỡng A · 75%')).toBeInTheDocument();
    expect(screen.getByText('pH Up · 50%')).toBeInTheDocument();
    expect(screen.queryByText('Phun sương')).not.toBeInTheDocument();
  });

  it('hiển thị tiêu đề SẼ DỪNG NGAY', () => {
    render(
      <EmergencyStopConfirmDialog
        open={true}
        runningPumps={{ pump_a: true }}
        onCancel={vi.fn()}
        onConfirm={vi.fn()}
        isSubmitting={false}
      />,
    );
    expect(screen.getByText(/SẼ DỪNG NGAY/)).toBeInTheDocument();
  });

  it('gọi onConfirm khi bấm nút xác nhận', () => {
    const onConfirm = vi.fn();
    render(
      <EmergencyStopConfirmDialog
        open={true}
        runningPumps={{ pump_a: true }}
        onCancel={vi.fn()}
        onConfirm={onConfirm}
        isSubmitting={false}
      />,
    );
    fireEvent.click(screen.getByText('Xác nhận dừng khẩn cấp'));
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it('gọi onCancel khi bấm Huỷ, không gọi onConfirm', () => {
    const onCancel = vi.fn();
    const onConfirm = vi.fn();
    render(
      <EmergencyStopConfirmDialog
        open={true}
        runningPumps={{ pump_a: true }}
        onCancel={onCancel}
        onConfirm={onConfirm}
        isSubmitting={false}
      />,
    );
    fireEvent.click(screen.getByText('Huỷ'));
    expect(onCancel).toHaveBeenCalledOnce();
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('không render gì khi open=false', () => {
    const { container } = render(
      <EmergencyStopConfirmDialog
        open={false}
        runningPumps={{ pump_a: true }}
        onCancel={vi.fn()}
        onConfirm={vi.fn()}
        isSubmitting={false}
      />,
    );
    expect(container).toBeEmptyDOMElement();
  });
});
