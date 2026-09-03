import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { WaterLevelGauge } from './WaterLevelGauge';

describe('WaterLevelGauge', () => {
  it('hiển thị nhãn Min, Mục tiêu, Max với giá trị đúng', () => {
    render(<WaterLevelGauge min={20} target={80} max={90} />);
    expect(screen.getByText(/Min 20/)).toBeInTheDocument();
    expect(screen.getByText(/Mục tiêu 80/)).toBeInTheDocument();
    expect(screen.getByText(/Max 90/)).toBeInTheDocument();
  });

  it('kẹp vị trí thanh mục tiêu trong khoảng 0-100% kể cả khi target ngoài range', () => {
    render(<WaterLevelGauge min={20} target={150} max={90} />);
    const fill = screen.getByTestId('water-level-gauge-fill');
    expect(fill).toHaveStyle({ width: '100%' });
  });

  it('không vỡ khi min === max', () => {
    render(<WaterLevelGauge min={50} target={50} max={50} />);
    expect(screen.getByText(/Min 50/)).toBeInTheDocument();
  });
});
