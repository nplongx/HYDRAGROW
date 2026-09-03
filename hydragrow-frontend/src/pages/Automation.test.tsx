// @vitest-environment jsdom
import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import Automation from './Automation';

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(), // Deprecated
    removeListener: vi.fn(), // Deprecated
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
}
window.ResizeObserver = ResizeObserver;

const queryClient = new QueryClient();

const Wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={queryClient}>
    {children}
  </QueryClientProvider>
);

describe('Automation page overview-state', () => {
  it('empty state renders Chưa có Flow nào', () => {
    render(<Automation scripts={[]} />, { wrapper: Wrapper });
    expect(screen.getByText('Chưa có Flow nào')).toBeInTheDocument();
  });

  it('a saved alert node shows its kind badge', () => {
    const alertScript = { id: 's1', name: 'Alert Flow', kind: 'alert', enabled: true, ir_json: { conditions: [], actions: [] } } as any;
    render(<Automation scripts={[alertScript]} />, { wrapper: Wrapper });
    expect(screen.getByText('Alert')).toBeInTheDocument();
  });

  it('a disabled Flow renders muted styling', () => {
    const disabledScript = { id: 's2', name: 'Disabled Flow', kind: 'alert', enabled: false, ir_json: { conditions: [], actions: [] } } as any;
    render(<Automation scripts={[disabledScript]} />, { wrapper: Wrapper });
    expect(screen.getByText('Đã tắt')).toBeInTheDocument();
    const nodeText = screen.getByText('Disabled Flow');
    const summaryDiv = nodeText.closest('div.w-52');
    expect(summaryDiv).toHaveClass('opacity-60');
  });

  it('trigger badge prefers CRON or WEBHOOK when configured', () => {
     const cronScript = {
       id: 's3',
       name: 'Cron Flow',
       kind: 'action_command',
       enabled: true,
       ir_json: {
         trigger: { type: 'cron', cron: '0 * * * *' },
         conditions: [],
         actions: []
       }
     } as any;

     const webhookScript = {
       id: 's4',
       name: 'Webhook Flow',
       kind: 'action_command',
       enabled: true,
       ir_json: {
         trigger: { type: 'webhook', bodyPath: '', targetField: '' },
         conditions: [],
         actions: []
       }
     } as any;

     render(<Automation scripts={[cronScript, webhookScript]} />, { wrapper: Wrapper });
     expect(screen.getByText('CRON')).toBeInTheDocument();
     expect(screen.getByText('WEBHOOK')).toBeInTheDocument();
  });
});
