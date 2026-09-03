import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { WebhookFieldMappingEditor } from '../WebhookFieldMappingEditor';
import type { WebhookTriggerConfig } from '../../../../lib/automation/ir';

describe('WebhookFieldMappingEditor', () => {
  const defaultConfig: WebhookTriggerConfig = {
    type: 'webhook',
    mode: 'flow',
    fieldMappings: [
      { bodyPath: 'external.sensor.ph', targetField: 'ph' }
    ]
  };

  it('renders mode radios and existing field mappings', () => {
    const onChange = vi.fn();
    render(<WebhookFieldMappingEditor config={defaultConfig} onChange={onChange} />);

    expect(screen.getByText('Chạy qua Flow')).toBeInTheDocument();
    expect(screen.getByText('Gọi lệnh trực tiếp')).toBeInTheDocument();
    expect(screen.getByDisplayValue('external.sensor.ph')).toBeInTheDocument();
    expect(screen.getByDisplayValue('ph')).toBeInTheDocument();
  });

  it('triggers onChange when switching webhook mode', () => {
    const onChange = vi.fn();
    render(<WebhookFieldMappingEditor config={defaultConfig} onChange={onChange} />);

    fireEvent.click(screen.getByLabelText('Gọi lệnh trực tiếp'));

    expect(onChange).toHaveBeenCalledWith({
      ...defaultConfig,
      mode: 'direct'
    });
  });

  it('allows adding and removing field mappings', () => {
    const onChange = vi.fn();
    render(<WebhookFieldMappingEditor config={defaultConfig} onChange={onChange} />);

    fireEvent.click(screen.getByText('+ Thêm ánh xạ'));

    expect(onChange).toHaveBeenCalledWith({
      ...defaultConfig,
      fieldMappings: [
        { bodyPath: 'external.sensor.ph', targetField: 'ph' },
        { bodyPath: '', targetField: '' }
      ]
    });
  });
});
