import { fireEvent, render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { NodeEditorPanel } from './NodeEditorPanel';

describe('NodeEditorPanel Chain Action', () => {
  it('renders chain action editor', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'action-1', type: 'action', data: { type: 'chain' } }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />
    );

    // The chain is edited using NextFlowSelector now which is in the FlowDetailDrawer
    // But the NodeEditorPanel for "chain" should at least explain it or provide UI

    expect(screen.getByText(/Hành động — Kích hoạt Flow khác/)).toBeInTheDocument();
    expect(screen.getByText(/Để chọn Flow cần kích hoạt/)).toBeInTheDocument();
  });

  it('selects preset daily 7am and triggers onChange with 6-field cronExpression', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'trigger', type: 'trigger', data: { kind: 'cron' } }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />
    );

    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'daily_7am' } });

    expect(mockOnChange).toHaveBeenCalledWith('trigger', {
      kind: 'cron',
      expression: '0 0 7 * * *',
      trigger: {
        type: 'cron',
        cronExpression: '0 0 7 * * *',
        timezone: 'Asia/Ho_Chi_Minh',
      },
    });
  });
});
