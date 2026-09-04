import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { NodeEditorPanel } from './NodeEditorPanel';

describe('NodeEditorPanel', () => {
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

  it('updates condition node data with proper conditions array and summary', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'cond-1',
          type: 'condition',
          data: {
            conditions: [{ sensor: 'ph', operator: '>', value: 7.5 }],
            summary: 'ph > 7.5',
          },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />
    );

    const valueInput = screen.getByLabelText('Giá trị');
    fireEvent.change(valueInput, { target: { value: '8.2' } });

    expect(mockOnChange).toHaveBeenCalledWith('cond-1', {
      conditions: [{ sensor: 'ph', operator: '>', value: 8.2 }],
      summary: 'ph > 8.2',
    });
  });

  it('updates action node data with proper actions array and summary', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'action-1',
          type: 'action',
          data: {
            actions: [{ type: 'alert', level: 'info', title: '', message: 'Initial' }],
            summary: 'alert (info): Initial',
          },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />
    );

    const messageInput = screen.getByLabelText('Message');
    fireEvent.change(messageInput, { target: { value: 'Updated message' } });

    expect(mockOnChange).toHaveBeenCalledWith('action-1', {
      actions: [
        {
          type: 'alert',
          level: 'info',
          title: '',
          message: 'Updated message',
        },
      ],
      summary: 'alert (info): Updated message',
    });
  });

  it('updates action_command action node data with proper actions array and summary', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="action_command"
        node={{
          id: 'action-cmd-1',
          type: 'action',
          data: {
            actions: [{ type: 'dose', pump: 'PUMP_A', doseMl: 5, pwm: 100 }],
            summary: 'dose 5ml (PUMP_A)',
          },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />
    );

    const doseInput = screen.getByLabelText(/Liều \(ml\)/);
    fireEvent.change(doseInput, { target: { value: '15' } });

    expect(mockOnChange).toHaveBeenCalledWith('action-cmd-1', {
      actions: [
        {
          type: 'dose',
          pump: 'PUMP_A',
          doseMl: 15,
          pwm: 100,
        },
      ],
      summary: 'dose 15ml (PUMP_A)',
    });
  });
});
