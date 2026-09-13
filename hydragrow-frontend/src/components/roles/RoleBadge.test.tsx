import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { RoleBadge } from './RoleBadge';

describe('RoleBadge', () => {
  it('renders correct Vietnamese role name and semantic color for admin role', () => {
    render(<RoleBadge role="admin" data-testid="role-badge" />);
    const badge = screen.getByTestId('role-badge');

    expect(badge).toHaveTextContent('Quản trị viên');
    expect(badge).toHaveClass('bg-amber-100');
    expect(badge).toHaveClass('text-amber-800');
  });

  it('renders correct Vietnamese role name and semantic color for operator role', () => {
    render(<RoleBadge role="operator" data-testid="role-badge" />);
    const badge = screen.getByTestId('role-badge');

    expect(badge).toHaveTextContent('Vận hành viên');
    expect(badge).toHaveClass('bg-pill');
    expect(badge).toHaveClass('text-status');
  });

  it('renders correct Vietnamese role name and semantic color for viewer role', () => {
    render(<RoleBadge role="viewer" data-testid="role-badge" />);
    const badge = screen.getByTestId('role-badge');

    expect(badge).toHaveTextContent('Người xem');
    expect(badge).toHaveClass('bg-surface-muted');
    expect(badge).toHaveClass('text-text-muted');
  });

  it('applies correct sizing classes for sm and md variants', () => {
    const { rerender } = render(<RoleBadge role="operator" size="sm" data-testid="badge-size" />);
    let badge = screen.getByTestId('badge-size');
    expect(badge).toHaveClass('text-[11px]');
    expect(badge).toHaveClass('px-2');

    rerender(<RoleBadge role="operator" size="md" data-testid="badge-size" />);
    badge = screen.getByTestId('badge-size');
    expect(badge).toHaveClass('text-xs');
    expect(badge).toHaveClass('px-2.5');
  });

  it('applies custom className', () => {
    render(<RoleBadge role="admin" className="custom-test-class" data-testid="badge-custom" />);
    const badge = screen.getByTestId('badge-custom');
    expect(badge).toHaveClass('custom-test-class');
  });
});
