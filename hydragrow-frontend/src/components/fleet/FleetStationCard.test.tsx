import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { FleetStationCard, formatRelativeTime } from './FleetStationCard';

describe('FleetStationCard', () => {
  const mockDevice = {
    device_id: 'esp32_01',
    label: 'Trạm Thủy Canh 1',
    is_online: true,
    last_seen: new Date(Date.now() - 5 * 60 * 1000).toISOString(),
    firmware_version: 'v1.4.2',
  };

  const mockSummary = {
    crop: 'Xà lách',
    ec_latest: 1.8,
    ph_latest: 6.0,
    warning_count: 0,
  };

  it('renders device name and status correctly', () => {
    const handleSelect = vi.fn();
    render(
      <FleetStationCard
        device={mockDevice}
        summary={mockSummary}
        onSelect={handleSelect}
      />
    );

    expect(screen.getByText('Trạm Thủy Canh 1')).toBeInTheDocument();
    expect(screen.getByText('esp32_01')).toBeInTheDocument();
    expect(screen.getByText('🌱 Xà lách')).toBeInTheDocument();
    expect(screen.getByText('Trực tuyến')).toBeInTheDocument();
    expect(screen.getByText('FW: v1.4.2')).toBeInTheDocument();
  });

  it('shows warning badge when warning_count > 0', () => {
    const handleSelect = vi.fn();
    render(
      <FleetStationCard
        device={mockDevice}
        summary={{ ...mockSummary, warning_count: 3 }}
        onSelect={handleSelect}
      />
    );

    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('shows EC and pH values from summary with proper formatting', () => {
    const handleSelect = vi.fn();
    render(
      <FleetStationCard
        device={mockDevice}
        summary={{ ...mockSummary, ec_latest: 2.1, ph_latest: 5.8 }}
        onSelect={handleSelect}
      />
    );

    expect(screen.getByText('2.1')).toBeInTheDocument();
    expect(screen.getByText('5.8')).toBeInTheDocument();
  });

  it('displays placeholder dashes when telemetry is null', () => {
    const handleSelect = vi.fn();
    render(
      <FleetStationCard
        device={mockDevice}
        summary={{ crop: null, ec_latest: null, ph_latest: null, warning_count: 0 }}
        onSelect={handleSelect}
      />
    );

    const dashes = screen.getAllByText('—');
    expect(dashes.length).toBeGreaterThanOrEqual(2);
  });

  it('calls onSelect with device_id on click', () => {
    const handleSelect = vi.fn();
    render(
      <FleetStationCard
        device={mockDevice}
        summary={mockSummary}
        onSelect={handleSelect}
      />
    );

    const card = screen.getByRole('button');
    fireEvent.click(card);
    expect(handleSelect).toHaveBeenCalledWith('esp32_01');
  });

  it('has accessible aria-label describing status, warnings, and telemetry', () => {
    const handleSelect = vi.fn();
    render(
      <FleetStationCard
        device={mockDevice}
        summary={{ ...mockSummary, warning_count: 2 }}
        onSelect={handleSelect}
      />
    );

    const card = screen.getByRole('button');
    expect(card).toHaveAttribute(
      'aria-label',
      expect.stringContaining('Trạm Trạm Thủy Canh 1: Đang hoạt động, 2 cảnh báo, EC 1.8, pH 6.0')
    );
  });

  it('formats relative time accurately', () => {
    expect(formatRelativeTime(undefined)).toBe('Chưa có dữ liệu');
    const justNow = new Date().toISOString();
    expect(formatRelativeTime(justNow)).toBe('Vừa xong');
    const tenMinsAgo = new Date(Date.now() - 10 * 60 * 1000).toISOString();
    expect(formatRelativeTime(tenMinsAgo)).toBe('10 phút trước');
    const threeHoursAgo = new Date(Date.now() - 3 * 3600 * 1000).toISOString();
    expect(formatRelativeTime(threeHoursAgo)).toBe('3 giờ trước');
    const twoDaysAgo = new Date(Date.now() - 2 * 86400 * 1000).toISOString();
    expect(formatRelativeTime(twoDaysAgo)).toBe('2 ngày trước');
  });
});
