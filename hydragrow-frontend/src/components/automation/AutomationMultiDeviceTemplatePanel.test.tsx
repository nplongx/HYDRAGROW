// @vitest-environment jsdom
import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { AutomationMultiDeviceTemplatePanel } from './AutomationMultiDeviceTemplatePanel';
import type { OwnedDevice } from '../../types/models';

describe('AutomationMultiDeviceTemplatePanel', () => {
  it('renders multi-device template UI and blocks apply', () => {
    const devices = [
      { device_id: 'd1', label: 'Device 1' },
      { device_id: 'd2', label: 'Device 2' }
    ] as OwnedDevice[];

    render(<AutomationMultiDeviceTemplatePanel devices={devices} currentFlowName="Test Flow" />);

    expect(screen.getByText('Áp Flow template cho nhiều thiết bị')).toBeInTheDocument();

    // Rows
    expect(screen.getByText('Device 1')).toBeInTheDocument();
    expect(screen.getByText('Device 2')).toBeInTheDocument();

    // Sync helper
    expect(screen.getByText(/local overrides are preserved/i)).toBeInTheDocument();

    // CTA disabled
    const applyBtn = screen.getByRole('button', { name: /Áp dụng cho 0 thiết bị đã chọn/i });
    expect(applyBtn).toBeDisabled();
    expect(screen.getByText(/Không có API hỗ trợ/i)).toBeInTheDocument();
  });
});
