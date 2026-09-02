import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ConditionGroupEditor } from './ConditionGroupEditor';
import type { ConditionGroup } from '../../../lib/automation/ir';

const FIELDS = ['ph', 'ec', 'temp', 'water_level'] as const;

describe('ConditionGroupEditor', () => {
  it('renders the Figma frame-03 example: OR subgroup + AND leaf at root', () => {
    const group: ConditionGroup = {
      op: 'and',
      children: [
        { op: 'or', children: [
          { sensor: 'ph', operator: '<', value: 5.5 },
          { sensor: 'ph', operator: '>', value: 7.5 },
        ]},
        { sensor: 'ec', operator: '>', value: 3.0 },
      ],
    };
    render(<ConditionGroupEditor group={group} fields={FIELDS} onChange={vi.fn()} isRoot />);
    expect(screen.getAllByDisplayValue('ph')).toHaveLength(2);
    expect(screen.getByDisplayValue('ec')).toBeInTheDocument();
  });

  it('toggling root op from AND to OR calls onChange with updated op, same children', () => {
    const group: ConditionGroup = {
      op: 'and',
      children: [{ sensor: 'ph', operator: '>', value: 7.5 }],
    };
    const onChange = vi.fn();
    render(<ConditionGroupEditor group={group} fields={FIELDS} onChange={onChange} isRoot />);
    fireEvent.click(screen.getByRole('button', { name: 'OR' }));
    expect(onChange).toHaveBeenCalledWith({ ...group, op: 'or' });
  });

  it('"+ Thêm điều kiện" appends a new leaf to children', () => {
    const group: ConditionGroup = { op: 'and', children: [] };
    const onChange = vi.fn();
    render(<ConditionGroupEditor group={group} fields={FIELDS} onChange={onChange} isRoot />);
    fireEvent.click(screen.getByText('+ Thêm điều kiện'));
    expect(onChange).toHaveBeenCalledWith({
      op: 'and',
      children: [{ sensor: 'ph', operator: '>', value: 0 }],
    });
  });

  it('"+ Thêm nhóm con (AND/OR)" appends an empty nested group', () => {
    const group: ConditionGroup = { op: 'and', children: [] };
    const onChange = vi.fn();
    render(<ConditionGroupEditor group={group} fields={FIELDS} onChange={onChange} isRoot />);
    fireEvent.click(screen.getByText('+ Thêm nhóm con (AND/OR)'));
    expect(onChange).toHaveBeenCalledWith({
      op: 'and',
      children: [{ op: 'and', children: [] }],
    });
  });

  it('removing a child calls onChange without that child', () => {
    const group: ConditionGroup = {
      op: 'and',
      children: [
        { sensor: 'ph', operator: '>', value: 7.5 },
        { sensor: 'ec', operator: '<', value: 1.2 },
      ],
    };
    const onChange = vi.fn();
    render(<ConditionGroupEditor group={group} fields={FIELDS} onChange={onChange} isRoot />);
    fireEvent.click(screen.getAllByText('✕')[0]);
    expect(onChange).toHaveBeenCalledWith({
      op: 'and',
      children: [{ sensor: 'ec', operator: '<', value: 1.2 }],
    });
  });
});
