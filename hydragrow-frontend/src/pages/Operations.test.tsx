import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import { Operations } from './Operations';

vi.mock('./ControlPanel', () => ({
  default: ({ variant }: { variant?: string }) => (
    <div data-testid="control-panel">ControlPanel Mock variant={variant}</div>
  ),
}));

vi.mock('./Automation', () => ({
  Automation: () => <div data-testid="automation-page">Automation Mock</div>,
}));

describe('Operations Page', () => {
  it('renders both Điều khiển and Tự động hóa tabs, defaulting to Điều khiển', () => {
    render(
      <MemoryRouter>
        <Operations />
      </MemoryRouter>
    );
    expect(screen.getByRole('tab', { name: /điều khiển/i })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tab', { name: /tự động hóa/i })).toHaveAttribute('aria-selected', 'false');
    expect(screen.getByTestId('control-panel')).toBeInTheDocument();
    expect(screen.queryByTestId('automation-page')).not.toBeInTheDocument();
  });

  it('switches to Automation tab on click', () => {
    render(
      <MemoryRouter>
        <Operations />
      </MemoryRouter>
    );
    fireEvent.click(screen.getByRole('tab', { name: /tự động hóa/i }));
    expect(screen.getByRole('tab', { name: /tự động hóa/i })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tab', { name: /điều khiển/i })).toHaveAttribute('aria-selected', 'false');
    expect(screen.getByTestId('automation-page')).toBeInTheDocument();
    expect(screen.queryByTestId('control-panel')).not.toBeInTheDocument();
  });
});
