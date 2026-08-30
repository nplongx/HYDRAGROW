import { describe, expect, it, vi } from 'vitest';
import { render } from '@testing-library/react';

const hydrateWorkspaceMock = vi.fn();
vi.mock('./blockly/hydrateIr', () => ({ hydrateWorkspace: hydrateWorkspaceMock }));
vi.mock('./blockly/blocks', () => ({ registerHydragrowBlocks: vi.fn() }));
vi.mock('./blockly/extractIr', () => ({
  extractConditions: vi.fn(() => []),
  extractActions: vi.fn(() => []),
}));
vi.mock('blockly/blocks', () => ({}));

const fakeWorkspace = {
  addChangeListener: vi.fn(),
  removeChangeListener: vi.fn(),
  dispose: vi.fn(),
};
vi.mock('blockly/core', async () => {
  const actual = await vi.importActual<typeof import('blockly/core')>('blockly/core');
  return { ...actual, inject: vi.fn(() => fakeWorkspace) };
});

// import sau các vi.mock để mock có hiệu lực trước khi module thật được load
const { BlockLogicEditor } = await import('./BlockLogicEditor');

describe('BlockLogicEditor', () => {
  it('hydrates the workspace when initialConditions/initialActions are given', () => {
    const initialConditions = [{ sensor: 'ph' as const, operator: '>' as const, value: 7.5 }];
    const initialActions = [{ type: 'alert' as const, level: 'warning' as const, message: 'pH cao' }];
    render(
      <BlockLogicEditor
        kind="alert"
        onChange={() => {}}
        initialConditions={initialConditions}
        initialActions={initialActions}
      />,
    );
    expect(hydrateWorkspaceMock).toHaveBeenCalledWith(fakeWorkspace, initialConditions, initialActions);
  });

  it('does not hydrate when there is nothing to restore (fresh automation)', () => {
    hydrateWorkspaceMock.mockClear();
    render(<BlockLogicEditor kind="alert" onChange={() => {}} />);
    expect(hydrateWorkspaceMock).not.toHaveBeenCalled();
  });
});
