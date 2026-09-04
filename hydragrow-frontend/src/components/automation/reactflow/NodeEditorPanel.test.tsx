import { render, screen } from '@testing-library/react';
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
});
