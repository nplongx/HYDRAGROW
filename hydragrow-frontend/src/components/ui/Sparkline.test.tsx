import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Sparkline } from './Sparkline';

describe('Sparkline', () => {
  it('render svg với đúng số điểm dữ liệu', () => {
    const { container } = render(<Sparkline values={[1, 3, 2, 5, 4]} label="RSSI WiFi" />);
    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute('role', 'img');
    expect(svg).toHaveAttribute('aria-label', expect.stringContaining('RSSI WiFi'));
  });

  it('dưới 2 điểm dữ liệu → không render svg (tránh vẽ đường vô nghĩa)', () => {
    const { container } = render(<Sparkline values={[1]} label="RSSI WiFi" />);
    expect(container.querySelector('svg')).not.toBeInTheDocument();
  });
});
