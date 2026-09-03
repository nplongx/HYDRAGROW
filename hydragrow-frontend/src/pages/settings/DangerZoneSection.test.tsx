import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { DangerZoneSection } from './DangerZoneSection';

describe('DangerZoneSection', () => {
  it('gọi onFactoryResetClick khi bấm nút khôi phục cài đặt gốc', () => {
    const onFactoryResetClick = vi.fn();
    render(
      <DangerZoneSection
        rebootLoading={false}
        onReboot={() => {}}
        factoryResetConfirm={false}
        onFactoryResetClick={onFactoryResetClick}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /Factory Reset/i }));
    expect(onFactoryResetClick).toHaveBeenCalledOnce();
  });

  it('gọi onReboot khi bấm nút reboot', () => {
    const onReboot = vi.fn();
    render(
      <DangerZoneSection
        rebootLoading={false}
        onReboot={onReboot}
        factoryResetConfirm={false}
        onFactoryResetClick={() => {}}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /Reboot thiết bị/i }));
    expect(onReboot).toHaveBeenCalledOnce();
  });

  it('hiển thị dòng cảnh báo Factory Reset xoá toàn bộ cấu hình', () => {
    render(
      <DangerZoneSection
        rebootLoading={false}
        onReboot={() => {}}
        factoryResetConfirm={false}
        onFactoryResetClick={() => {}}
      />
    );
    expect(screen.getByText(/Factory Reset sẽ xoá toàn bộ cấu hình/i)).toBeInTheDocument();
  });

  it('disable nút reboot khi rebootLoading true', () => {
    render(
      <DangerZoneSection
        rebootLoading={true}
        onReboot={() => {}}
        factoryResetConfirm={false}
        onFactoryResetClick={() => {}}
      />
    );
    expect(screen.getByRole('button', { name: /Đang gửi lệnh/i })).toBeDisabled();
  });

  it('gọi onConfirmFactoryReset khi xác nhận trong hộp thoại cảnh báo', () => {
    const onConfirmFactoryReset = vi.fn();
    render(
      <DangerZoneSection
        rebootLoading={false}
        onReboot={() => {}}
        factoryResetConfirm={true}
        onFactoryResetClick={() => {}}
        onConfirmFactoryReset={onConfirmFactoryReset}
        onCancelFactoryReset={() => {}}
      />
    );
    fireEvent.click(screen.getByText(/Xác Nhận Factory Reset/i));
    expect(onConfirmFactoryReset).toHaveBeenCalledOnce();
  });
});
