import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { VariableCombobox } from './VariableCombobox';

describe('VariableCombobox', () => {
  it('renders the current value and offers available variables as datalist suggestions', () => {
    render(
      <VariableCombobox
        id="cmp-1"
        ariaLabel="Biến so sánh"
        value="ph"
        availableVariables={['ph', 'ec', 'ph_target_now']}
        onChange={vi.fn()}
      />,
    );

    const input = screen.getByLabelText('Biến so sánh') as HTMLInputElement;
    expect(input.value).toBe('ph');
    expect(input).toHaveAttribute('list', 'cmp-1-vars');

    const options = document.querySelectorAll('#cmp-1-vars option');
    expect(Array.from(options).map((o) => o.getAttribute('value'))).toEqual([
      'ph',
      'ec',
      'ph_target_now',
    ]);
  });

  it('calls onChange with the typed text, letting the caller decide literal vs variable', () => {
    const onChange = vi.fn();
    render(
      <VariableCombobox
        id="cmp-2"
        ariaLabel="Giá trị"
        value=""
        availableVariables={['ph_target_now']}
        onChange={onChange}
      />,
    );

    fireEvent.change(screen.getByLabelText('Giá trị'), { target: { value: 'ph_target_now' } });
    expect(onChange).toHaveBeenCalledWith('ph_target_now');
  });

  it('applies a placeholder when provided', () => {
    render(
      <VariableCombobox
        id="cmp-3"
        ariaLabel="Giá trị ghi đè"
        value=""
        availableVariables={[]}
        onChange={vi.fn()}
        placeholder="vd: 1.8 hoặc ph_target_now"
      />,
    );
    expect(screen.getByPlaceholderText('vd: 1.8 hoặc ph_target_now')).toBeInTheDocument();
  });
});
