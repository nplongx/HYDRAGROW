import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DosingHourlyChart } from './DosingHourlyChart';

const buckets = {
  pump_a_ml: new Array(24).fill(0).map((_, h) => (h === 9 ? 5 : 0)),
  pump_b_ml: new Array(24).fill(0),
  ph_up_ml: new Array(24).fill(0).map((_, h) => (h === 14 ? 2 : 0)),
  ph_down_ml: new Array(24).fill(0),
};

describe('DosingHourlyChart', () => {
  it('render đủ 4 mục chú giải với nhãn tiếng Việt', () => {
    render(<DosingHourlyChart bucketsByPump={buckets} />);
    expect(screen.getByText('Phân A')).toBeInTheDocument();
    expect(screen.getByText('Phân B')).toBeInTheDocument();
    expect(screen.getByText('pH Up')).toBeInTheDocument();
    expect(screen.getByText('pH Down')).toBeInTheDocument();
  });

  it('có bảng text alternative cho screen reader (sr-only)', () => {
    render(<DosingHourlyChart bucketsByPump={buckets} />);
    const table = screen.getByRole('table', { name: /Lượng châm theo giờ/ });
    expect(table).toBeInTheDocument();
    expect(table.className).toContain('sr-only');
  });

  it('mỗi cột giờ có nhãn accessible mô tả đủ 4 giá trị', () => {
    render(<DosingHourlyChart bucketsByPump={buckets} />);
    expect(
      screen.getByLabelText(/Giờ 9: Phân A 5\.0ml, Phân B 0\.0ml, pH Up 0\.0ml, pH Down 0\.0ml/),
    ).toBeInTheDocument();
  });

  it('nhãn trục giờ hiển thị mốc 0/6/12/18', () => {
    render(<DosingHourlyChart bucketsByPump={buckets} />);
    expect(screen.getByText('0h')).toBeInTheDocument();
    expect(screen.getByText('6h')).toBeInTheDocument();
    expect(screen.getByText('12h')).toBeInTheDocument();
    expect(screen.getByText('18h')).toBeInTheDocument();
  });
});
