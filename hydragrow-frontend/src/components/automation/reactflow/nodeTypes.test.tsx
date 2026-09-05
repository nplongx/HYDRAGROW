import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { ReactFlowProvider } from '@xyflow/react';
import { AUTOMATION_NODE_TYPES } from './nodeTypes';

function renderConfigNode(data: Record<string, unknown>) {
  const ConfigNode = AUTOMATION_NODE_TYPES.config;
  return render(
    <ReactFlowProvider>
      <ConfigNode data={data} />
    </ReactFlowProvider>,
  );
}

describe('ConfigNode', () => {
  it('renders a read-variant badge and configKey/saveToVariable summary', () => {
    renderConfigNode({ variant: 'read', configKey: 'ph_target', saveToVariable: 'ph_target_now' });
    expect(screen.getByText('CONFIG · ĐỌC')).toBeInTheDocument();
    expect(screen.getByText(/ph_target → ph_target_now/)).toBeInTheDocument();
  });

  it('renders an overwrite-variant badge and configKey/value summary', () => {
    renderConfigNode({ variant: 'overwrite', configKey: 'ec_target', overrideValue: '1.8' });
    expect(screen.getByText('CONFIG · GHI ĐÈ')).toBeInTheDocument();
    expect(screen.getByText(/ec_target → 1.8/)).toBeInTheDocument();
  });

  it('falls back to a placeholder summary when not yet configured', () => {
    renderConfigNode({ variant: 'read' });
    expect(screen.getByText('Chưa cấu hình')).toBeInTheDocument();
  });
});
