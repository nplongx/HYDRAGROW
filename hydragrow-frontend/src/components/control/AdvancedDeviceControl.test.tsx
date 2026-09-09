import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Droplets } from 'lucide-react';
import { AdvancedDeviceControl } from './AdvancedDeviceControl';
import { useDeviceStore } from '../../store/useDeviceStore';

vi.mock('../../hooks/useDeviceControl', () => ({
  useDeviceControl: () => ({
    togglePump: vi.fn(),
    setPwm: vi.fn(),
    forceOn: vi.fn(),
    processingPumpIds: {},
    commandStatus: { PH_UP: 'sending' },
  }),
}));

describe('AdvancedDeviceControl', () => {
  beforeEach(() => {
    useDeviceStore.setState({ pwmPreferences: {}, savePwmPreference: vi.fn() } as any);
  });

  it('hiển thị banner khoá chéo khi có lockedByPumpId', () => {
    render(
      <AdvancedDeviceControl
        deviceId="dev-1"
        pumpId="PH_DOWN"
        title="Bơm pH Down"
        icon={Droplets}
        currentStatus={false}
        canSendCommands={true}
        isEmergency={false}
        isAutoMode={false}
        colorTheme="fuchsia"
        lockedByPumpId="PH_UP"
        lockedByPumpLabel="Bơm pH Up"
      />,
    );
    expect(screen.getByText(/Đã khoá vì Bơm pH Up đang chạy/)).toBeInTheDocument();
  });

  it('hiển thị StatusPill theo commandStatus từ hook', () => {
    render(
      <AdvancedDeviceControl
        deviceId="dev-1"
        pumpId="PH_UP"
        title="Bơm pH Up"
        icon={Droplets}
        currentStatus={true}
        canSendCommands={true}
        isEmergency={false}
        isAutoMode={false}
        colorTheme="fuchsia"
      />,
    );
    expect(screen.getByText('Đang gửi…')).toBeInTheDocument();
  });

  it('hiển thị %PWM ngay trên card khi allowPwm và đang bật, không cần mở Tùy chỉnh kỹ thuật', () => {
    useDeviceStore.setState({ pwmPreferences: { PUMP_A: 72 }, savePwmPreference: vi.fn() } as any);
    render(
      <AdvancedDeviceControl
        deviceId="dev-1"
        pumpId="PUMP_A"
        title="Bơm phân A"
        icon={Droplets}
        currentStatus={true}
        allowPwm={true}
        canSendCommands={true}
        isEmergency={false}
        isAutoMode={false}
        colorTheme="orange"
      />,
    );
    expect(screen.getByText(/72%/)).toBeInTheDocument();
  });
});
