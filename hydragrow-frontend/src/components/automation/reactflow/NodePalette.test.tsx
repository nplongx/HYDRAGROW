import { render, screen } from '@testing-library/react';
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
});
