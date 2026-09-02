import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { EventLegend, EVENT_LEGEND_ENTRIES } from './EventLegend';

describe('EventLegend', () => {
  it('renders one entry per legend item', () => {
    render(<EventLegend />);
    EVENT_LEGEND_ENTRIES.forEach((entry) => {
      expect(screen.getByText(entry.label)).toBeInTheDocument();
    });
  });

  it('includes the merged-technical-event entry using the Neutral token', () => {
    render(<EventLegend />);
    expect(screen.getByText('Kỹ thuật đã gộp')).toBeInTheDocument();
    const neutralEntry = EVENT_LEGEND_ENTRIES.find((e) => e.label === 'Kỹ thuật đã gộp');
    expect(neutralEntry?.swatchClassName).toBe('log-neutral-dot');
  });
});
