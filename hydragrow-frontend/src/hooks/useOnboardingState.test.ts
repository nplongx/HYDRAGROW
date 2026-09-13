import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { useOnboardingState, ONBOARDING_STORAGE_KEY } from './useOnboardingState';

describe('useOnboardingState', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('khởi tạo với trạng thái mặc định khi localStorage trống', () => {
    const { result } = renderHook(() => useOnboardingState());

    expect(result.current.completedSteps).toEqual([]);
    expect(result.current.dismissed).toBe(false);
    expect(result.current.shouldShowOnboarding).toBe(true);
    expect(result.current.firstDevicePaired).toBe(false);
  });

  it('đọc trạng thái hợp lệ đã lưu từ localStorage', () => {
    localStorage.setItem(
      ONBOARDING_STORAGE_KEY,
      JSON.stringify({
        completedSteps: ['welcome'],
        dismissed: false,
        firstDevicePaired: true,
        firstDataReceived: false,
        firstSeasonCreated: false,
      })
    );

    const { result } = renderHook(() => useOnboardingState());

    expect(result.current.completedSteps).toEqual(['welcome']);
    expect(result.current.firstDevicePaired).toBe(true);
    expect(result.current.shouldShowOnboarding).toBe(true);
  });

  it('hoàn thành một bước cập nhật completedSteps và lưu vào localStorage', () => {
    const { result } = renderHook(() => useOnboardingState());

    act(() => {
      result.current.completeStep('welcome');
    });

    expect(result.current.completedSteps).toEqual(['welcome']);
    expect(result.current.isStepComplete('welcome')).toBe(true);
    expect(result.current.isStepComplete('pair_device')).toBe(false);

    const stored = JSON.parse(localStorage.getItem(ONBOARDING_STORAGE_KEY) || '{}');
    expect(stored.completedSteps).toContain('welcome');
  });

  it('tự động đánh dấu cờ khi hoàn thành các bước thiết bị, dữ liệu, mùa vụ', () => {
    const { result } = renderHook(() => useOnboardingState());

    act(() => {
      result.current.completeStep('pair_device');
      result.current.completeStep('first_data');
      result.current.completeStep('first_season');
    });

    expect(result.current.firstDevicePaired).toBe(true);
    expect(result.current.firstDataReceived).toBe(true);
    expect(result.current.firstSeasonCreated).toBe(true);
  });

  it('không nhân đôi bước nếu gọi completeStep lặp lại', () => {
    const { result } = renderHook(() => useOnboardingState());

    act(() => {
      result.current.completeStep('welcome');
      result.current.completeStep('welcome');
    });

    expect(result.current.completedSteps).toHaveLength(1);
    expect(result.current.completedSteps).toEqual(['welcome']);
  });

  it('dismiss() đánh dấu dismissed=true và ẩn onboarding', () => {
    const { result } = renderHook(() => useOnboardingState());

    expect(result.current.shouldShowOnboarding).toBe(true);

    act(() => {
      result.current.dismiss();
    });

    expect(result.current.dismissed).toBe(true);
    expect(result.current.shouldShowOnboarding).toBe(false);

    const stored = JSON.parse(localStorage.getItem(ONBOARDING_STORAGE_KEY) || '{}');
    expect(stored.dismissed).toBe(true);
  });

  it('shouldShowOnboarding là false khi hoàn thành đủ 4 bước', () => {
    const { result } = renderHook(() => useOnboardingState());

    act(() => {
      result.current.completeStep('welcome');
      result.current.completeStep('pair_device');
      result.current.completeStep('first_data');
      result.current.completeStep('first_season');
    });

    expect(result.current.completedSteps).toHaveLength(4);
    expect(result.current.shouldShowOnboarding).toBe(false);
  });

  it('reset() đưa trạng thái về ban đầu', () => {
    const { result } = renderHook(() => useOnboardingState());

    act(() => {
      result.current.completeStep('welcome');
      result.current.dismiss();
    });

    expect(result.current.dismissed).toBe(true);

    act(() => {
      result.current.reset();
    });

    expect(result.current.completedSteps).toEqual([]);
    expect(result.current.dismissed).toBe(false);
    expect(result.current.shouldShowOnboarding).toBe(true);
  });
});
