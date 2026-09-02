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
    fireEvent.click(screen.getByText(/Factory Reset/i));
    expect(onFactoryResetClick).toHaveBeenCalledOnce();
  });
});
