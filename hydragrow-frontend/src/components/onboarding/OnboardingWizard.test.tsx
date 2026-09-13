import { render, screen, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { BrowserRouter } from 'react-router-dom';
import { OnboardingWizard } from './OnboardingWizard';
import { useDeviceStore } from '../../store/useDeviceStore';
import { ONBOARDING_STORAGE_KEY } from '../../hooks/useOnboardingState';

const mockNavigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

describe('OnboardingWizard', () => {
  beforeEach(() => {
    localStorage.clear();
    mockNavigate.mockReset();
    useDeviceStore.setState({
      sensorData: null,
      ownedDevices: [],
    });
  });

  const renderWizard = () => {
    return render(
      <BrowserRouter>
        <OnboardingWizard />
      </BrowserRouter>
    );
  };

  it('hiển thị đầy đủ tiêu đề và 4 bước onboarding khi mới truy cập', () => {
    renderWizard();

    expect(screen.getByText('Thiết lập hệ thống HydraGrow')).toBeInTheDocument();
    expect(screen.getByText('0/4 bước')).toBeInTheDocument();
    expect(screen.getByText('Chào mừng đến HydraGrow')).toBeInTheDocument();
    expect(screen.getByText('Kết nối thiết bị')).toBeInTheDocument();
    expect(screen.getByText('Dữ liệu thời gian thực')).toBeInTheDocument();
    expect(screen.getByText('Bắt đầu mùa vụ')).toBeInTheDocument();
  });

  it('bấm "Bắt đầu" ở bước 1 hoàn thành bước Chào mừng và tăng tiến trình', () => {
    renderWizard();

    const startBtn = screen.getByRole('button', { name: 'Bắt đầu' });
    fireEvent.click(startBtn);

    expect(screen.getByText('1/4 bước')).toBeInTheDocument();
  });

  it('bấm "Ghép nối thiết bị" điều hướng tới /pairing', () => {
    renderWizard();

    const pairBtn = screen.getByRole('button', { name: 'Ghép nối thiết bị' });
    fireEvent.click(pairBtn);

    expect(mockNavigate).toHaveBeenCalledWith('/pairing');
  });

  it('bấm "Bỏ qua hướng dẫn" ẩn wizard khỏi màn hình', () => {
    renderWizard();

    const dismissBtns = screen.getAllByRole('button', { name: /Bỏ qua hướng dẫn/i });
    fireEvent.click(dismissBtns[0]);

    expect(screen.queryByTestId('onboarding-wizard')).not.toBeInTheDocument();
    const stored = JSON.parse(localStorage.getItem(ONBOARDING_STORAGE_KEY) || '{}');
    expect(stored.dismissed).toBe(true);
  });

  it('tự động hoàn thành bước 2 khi có ownedDevices trong store', () => {
    act(() => {
      useDeviceStore.setState({
        ownedDevices: [
          { id: 1, user_id: 1, device_id: 'dev-001', label: 'Trạm A', claimed_at: '2026-08-24' },
        ],
      });
    });

    renderWizard();

    expect(screen.getByText('1/4 bước')).toBeInTheDocument();
  });

  it('tự động hoàn thành bước 3 khi có sensorData trong store (Aha moment)', () => {
    act(() => {
      useDeviceStore.setState({
        sensorData: {
          ec: 1.8,
          ph: 6.0,
          water_temperature: 24.5,
          air_temperature: 28.0,
          humidity: 70,
          water_level: 80,
          timestamp: '2026-09-13T10:00:00Z',
        } as any,
      });
    });

    renderWizard();

    expect(screen.getByText('1/4 bước')).toBeInTheDocument();
  });

  it('hiển thị màn hình ăn mừng (Peak-End Rule) khi hoàn thành cả 4 bước', () => {
    localStorage.setItem(
      ONBOARDING_STORAGE_KEY,
      JSON.stringify({
        completedSteps: ['welcome', 'pair_device', 'first_data', 'first_season'],
        dismissed: false,
        firstDevicePaired: true,
        firstDataReceived: true,
        firstSeasonCreated: true,
      })
    );

    renderWizard();

    expect(screen.getByTestId('onboarding-celebration')).toBeInTheDocument();
    expect(screen.getByText('🎉 Hệ thống đã sẵn sàng!')).toBeInTheDocument();
    expect(screen.getByText('Chúc vụ mùa bội thu. Bạn đã hoàn thành tất cả các bước thiết lập ban đầu!')).toBeInTheDocument();

    const finishBtn = screen.getByRole('button', { name: 'Hoàn tất hướng dẫn' });
    fireEvent.click(finishBtn);

    expect(screen.queryByTestId('onboarding-celebration')).not.toBeInTheDocument();
  });
});
