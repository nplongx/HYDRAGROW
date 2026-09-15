import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import { Operations } from './Operations';

vi.mock('./ControlPanel', () => ({
  default: ({ variant }: { variant?: string }) => (
    <div data-testid="control-panel">ControlPanel Mock variant={variant}</div>
  ),
}));

describe('Operations Page', () => {
  it('renders Điều khiển / Tự động hóa surface nav links, defaulting to Điều khiển', () => {
    render(
      <MemoryRouter initialEntries={['/operations']}>
        <Operations />
      </MemoryRouter>
    );
    expect(screen.getByRole('link', { name: /điều khiển/i })).toHaveAttribute('href', '/operations');
    expect(screen.getByRole('link', { name: /tự động hóa/i })).toHaveAttribute('href', '/automation');
    expect(screen.getByRole('link', { name: /điều khiển/i })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByTestId('control-panel')).toBeInTheDocument();
  });

  it('always shows ControlPanel and never embeds Automation', () => {
    render(
      <MemoryRouter initialEntries={['/automation']}>
        <Operations />
      </MemoryRouter>
    );
    expect(screen.getByRole('link', { name: /tự động hóa/i })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByTestId('control-panel')).toBeInTheDocument();
    expect(screen.queryByTestId('automation-page')).not.toBeInTheDocument();
  });
});
