import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { NodePalette } from './NodePalette';

describe('NodePalette', () => {
  it('exposes all expected capabilities', () => {
    render(<NodePalette onAddNode={vi.fn()} />);

    expect(screen.getByText('TRIGGER')).toBeInTheDocument();
    expect(screen.getByText('+ Sensor')).toBeInTheDocument();
    expect(screen.getByText('+ FSM giai đoạn')).toBeInTheDocument();
    expect(screen.getByText('+ Cron (lịch)')).toBeInTheDocument();
    expect(screen.getByText('+ Webhook')).toBeInTheDocument();

    expect(screen.getByText('CONDITION')).toBeInTheDocument();
    expect(screen.getByText('+ Condition')).toBeInTheDocument();
    expect(screen.getByText('+ Condition Group (AND/OR)')).toBeInTheDocument();
    expect(screen.getByText('+ Time-window (mean/min/max)')).toBeInTheDocument();

    expect(screen.getByText('DELAY')).toBeInTheDocument();
    expect(screen.getByText('+ Delay')).toBeInTheDocument();

    expect(screen.getByText('ACTION')).toBeInTheDocument();
    expect(screen.getByText('+ Alert')).toBeInTheDocument();
    expect(screen.getByText('+ Dose / Water / Emergency stop')).toBeInTheDocument();
    expect(screen.getByText('+ Advance stage / End season')).toBeInTheDocument();
    expect(screen.getByText('+ Chain — chạy Flow khác')).toBeInTheDocument();
  });

  it('calls onUpdateTrigger when trigger buttons are clicked', () => {
    const onUpdateTrigger = vi.fn();
    render(<NodePalette onAddNode={vi.fn()} onUpdateTrigger={onUpdateTrigger} />);

    fireEvent.click(screen.getByText('+ Sensor'));
    expect(onUpdateTrigger).toHaveBeenCalledWith('sensor');

    fireEvent.click(screen.getByText('+ FSM giai đoạn'));
    expect(onUpdateTrigger).toHaveBeenCalledWith('fsm');

    fireEvent.click(screen.getByText(/\+ Cron \(lịch\)/));
    expect(onUpdateTrigger).toHaveBeenCalledWith('cron');

    fireEvent.click(screen.getByText(/\+ Webhook/));
    expect(onUpdateTrigger).toHaveBeenCalledWith('webhook');
  });

  it('calls onAddNode with proper type and variant', () => {
    const onAddNode = vi.fn();
    render(<NodePalette onAddNode={onAddNode} />);

    fireEvent.click(screen.getByText('+ Chain — chạy Flow khác'));
    expect(onAddNode).toHaveBeenCalledWith('action', 'chain');

    fireEvent.click(screen.getByText('+ Delay'));
    expect(onAddNode).toHaveBeenCalledWith('action', 'delay');

    fireEvent.click(screen.getByText('+ Time-window (mean/min/max)'));
    expect(onAddNode).toHaveBeenCalledWith('condition', 'time-window');
  });

  it('exposes the CONFIG section', () => {
    render(<NodePalette onAddNode={vi.fn()} />);
    expect(screen.getByText('CONFIG')).toBeInTheDocument();
    expect(screen.getByText('+ Đọc cấu hình')).toBeInTheDocument();
    expect(screen.getByText('+ Ghi đè cấu hình')).toBeInTheDocument();
  });

  it('calls onAddNode with config type and read/overwrite variant', () => {
    const onAddNode = vi.fn();
    render(<NodePalette onAddNode={onAddNode} />);

    fireEvent.click(screen.getByText('+ Đọc cấu hình'));
    expect(onAddNode).toHaveBeenCalledWith('config', 'read');

    fireEvent.click(screen.getByText('+ Ghi đè cấu hình'));
    expect(onAddNode).toHaveBeenCalledWith('config', 'overwrite');
  });
});
