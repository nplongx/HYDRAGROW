import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { NodeEditorPanel } from './NodeEditorPanel';

describe('NodeEditorPanel', () => {
  it('adds a condition and reports the update via onChange', () => {
    const onChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: '2', type: 'condition', data: { conditions: [] } }}
        onChange={onChange}
        onClose={() => {}}
      />,
    );
    fireEvent.click(screen.getByText('+ Thêm điều kiện'));
    expect(onChange).toHaveBeenCalledWith(
      '2',
      expect.objectContaining({ conditions: [{ sensor: 'ph', operator: '>', value: 0 }] }),
    );
  });

  it('renders advance_stage fields for recipe_override action nodes', () => {
    render(
      <NodeEditorPanel
        kind="recipe_override"
        node={{ id: '3', type: 'action', data: { actions: [] } }}
        onChange={() => {}}
        onClose={() => {}}
      />,
    );
    expect(screen.getByText('Action — Recipe')).toBeInTheDocument();
  });

  it('renders dose/water/emergency-stop action picker for action_command nodes', () => {
    render(
      <NodeEditorPanel
        kind="action_command"
        node={{ id: '3', type: 'action', data: { actions: [] } }}
        onChange={() => {}}
        onClose={() => {}}
      />,
    );
    expect(screen.getByLabelText('Loại hành động')).toBeInTheDocument();
  });

  it('emits a dose action with the right shape', () => {
    const onChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="action_command"
        node={{ id: '3', type: 'action', data: { actions: [{ type: 'dose', pump: 'PUMP_B', doseMl: 12, pwm: 100 }] } }}
        onChange={onChange}
        onClose={() => {}}
      />,
    );
    fireEvent.change(screen.getByLabelText('PWM (%)'), { target: { value: '80' } });
    expect(onChange).toHaveBeenLastCalledWith(
      '3',
      expect.objectContaining({ actions: [{ type: 'dose', pump: 'PUMP_B', doseMl: 12, pwm: 80 }] }),
    );
  });

  it('offers end_season as an action type for recipe_override nodes', () => {
    render(
      <NodeEditorPanel
        kind="recipe_override"
        node={{ id: '3', type: 'action', data: { actions: [] } }}
        onChange={() => {}}
        onClose={() => {}}
      />,
    );
    const select = screen.getByLabelText('Loại hành động') as HTMLSelectElement;
    const optionValues = Array.from(select.options).map((o) => o.value);
    expect(optionValues).toEqual(['advance_stage', 'end_season']);
  });
});
