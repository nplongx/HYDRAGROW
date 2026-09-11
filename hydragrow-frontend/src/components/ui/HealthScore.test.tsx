import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { HealthScore } from './HealthScore';

describe('HealthScore', () => {
  it('hiển thị điểm và nhãn', () => {
    render(<HealthScore score={92} label="Sức khoẻ vườn" />);
    expect(screen.getByText('92')).toBeInTheDocument();
    expect(screen.getByText('/100')).toBeInTheDocument();
    expect(screen.getByText('Sức khoẻ vườn')).toBeInTheDocument();
  });

  it('score ≥ 80 → tone good (bg-success)', () => {
    const { container } = render(<HealthScore score={85} />);
    expect(container.querySelector('span[aria-hidden="true"].bg-success')).not.toBeNull();
  });

  it('score 50–79 → tone attention (bg-warn-deep)', () => {
    const { container } = render(<HealthScore score={60} />);
    expect(container.querySelector('span[aria-hidden="true"].bg-warn-deep')).not.toBeNull();
  });

  it('score < 50 → tone critical (bg-error)', () => {
    const { container } = render(<HealthScore score={30} />);
    expect(container.querySelector('span[aria-hidden="true"].bg-error')).not.toBeNull();
  });

  it('không có điểm → offline', () => {
    const { container } = render(<HealthScore score={null} />);
    expect(container.querySelector('span[aria-hidden="true"].bg-line')).not.toBeNull();
    expect(screen.getByText('--')).toBeInTheDocument();
  });
});