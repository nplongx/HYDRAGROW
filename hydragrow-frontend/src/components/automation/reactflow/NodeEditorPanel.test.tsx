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
    expect(screen.getByText('Action — Advance Stage')).toBeInTheDocument();
  });
});
