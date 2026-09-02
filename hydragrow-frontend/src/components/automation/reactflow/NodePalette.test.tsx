import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { NodePalette } from './NodePalette';

describe('NodePalette', () => {
  it('renders a "+ Condition Group" button that calls onAddNode with condition_group', () => {
    const onAddNode = vi.fn();
    render(<NodePalette onAddNode={onAddNode} />);
    fireEvent.click(screen.getByText('+ Condition Group'));
    expect(onAddNode).toHaveBeenCalledWith('condition_group');
  });

  it('calls onAddNode with the right type for each button', () => {
    const onAddNode = vi.fn();
    render(<NodePalette onAddNode={onAddNode} />);
    fireEvent.click(screen.getByText('+ Condition'));
    fireEvent.click(screen.getByText('+ Action'));
    expect(onAddNode).toHaveBeenNthCalledWith(1, 'condition');
    expect(onAddNode).toHaveBeenNthCalledWith(2, 'action');
  });
});
