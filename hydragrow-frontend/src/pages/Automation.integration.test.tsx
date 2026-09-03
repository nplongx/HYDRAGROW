// @vitest-environment jsdom
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import Automation from './Automation';

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
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
  <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
);

describe('Automation Integration', () => {
  it('allows full navigation path', async () => {
    const mockScripts = [
      { id: '1', name: 'Flow 1', kind: 'action_command', enabled: true, ir_json: { trigger: { type: 'sensor' }, conditions: [], actions: [] } }
    ] as any;

    render(<Automation scripts={mockScripts} />, { wrapper: Wrapper });

    expect(screen.getByText('Flow 1')).toBeInTheDocument();

    // Open flow
    fireEvent.click(screen.getByRole('button', { name: /Flow mới/i }));

    expect(screen.getByRole('heading', { name: 'Flow mới' })).toBeInTheDocument();

    // We have multiple buttons matching /+ Condition/i (one is + Condition, another is + Condition Group).
    // Let's match the exact text.
    fireEvent.click(screen.getByText('+ Condition'));

    // Test panel
    fireEvent.click(screen.getByRole('button', { name: /Chạy thử/i }));
    expect(screen.getByText(/Chạy thử \(Dry Run\)/i)).toBeInTheDocument();
  });
});
