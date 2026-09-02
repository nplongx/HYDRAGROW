import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { Activity } from 'lucide-react';
import { SensorBentoCard } from './SensorBentoCard';

describe('SensorBentoCard', () => {
  it('renders correctly with sky theme', () => {
    const { container } = render(
      <SensorBentoCard
        title="Dinh dưỡng EC"
        value={1.2}
        unit="ppm"
        icon={Activity}
        theme="sky"
        statusLabel="Tốt"
        statusTone="good"
      />
    );

    expect(screen.getByText('Dinh dưỡng EC')).toBeInTheDocument();
    expect(screen.getByText('1.2')).toBeInTheDocument();
    expect(screen.getByText('ppm')).toBeInTheDocument();

    const iconWrapper = container.querySelector('.text-sky-700.bg-sky-50.border-sky-100');
    expect(iconWrapper).toBeInTheDocument();
  });
});
