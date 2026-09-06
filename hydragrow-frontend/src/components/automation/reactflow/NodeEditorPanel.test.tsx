import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { NodeEditorPanel } from './NodeEditorPanel';

describe('NodeEditorPanel', () => {
  it('renders chain action editor', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'action-1', type: 'action', data: { type: 'chain' } }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />
    );

    expect(screen.getByText(/Hành động — Kích hoạt Flow khác/)).toBeInTheDocument();
    expect(screen.getByText(/Chạy tiếp Flow khác/)).toBeInTheDocument();
    expect(screen.getByLabelText('Flow tiếp theo')).toBeInTheDocument();
    expect(screen.getByLabelText('Độ trễ trước khi chạy')).toBeInTheDocument();
  });

  it('selects preset daily 7am and triggers onChange with 6-field cronExpression', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'trigger', type: 'trigger', data: { kind: 'cron' } }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />
    );

    const select = screen.getByRole('combobox', { name: 'Lịch dựng sẵn' });
    fireEvent.change(select, { target: { value: 'daily_7am' } });

    expect(mockOnChange).toHaveBeenCalledWith('trigger', {
      kind: 'cron',
      expression: '0 0 7 * * *',
      trigger: {
        type: 'cron',
        cronExpression: '0 0 7 * * *',
        timezone: 'Asia/Ho_Chi_Minh',
      },
    });
  });

  it('shows a sky TRIGGER badge for the selected trigger tab', () => {
    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'trigger', type: 'trigger', data: { kind: 'cron' } }}
        onChange={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByText('TRIGGER · CRON')).toBeInTheDocument();
  });

  it('updates condition node data with proper conditions array and summary', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'cond-1',
          type: 'condition',
          data: {
            conditions: [{ sensor: 'ph', operator: '>', value: 7.5 }],
            summary: 'ph > 7.5',
          },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />
    );

    const valueInput = screen.getByLabelText('Giá trị');
    fireEvent.change(valueInput, { target: { value: '8.2' } });

    expect(mockOnChange).toHaveBeenCalledWith('cond-1', {
      conditions: [{ sensor: 'ph', operator: '>', value: 8.2 }],
      summary: 'ph > 8.2',
    });
  });

  it('updates action node data with proper actions array and summary', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'action-1',
          type: 'action',
          data: {
            actions: [{ type: 'alert', level: 'info', title: '', message: 'Initial' }],
            summary: 'alert (info): Initial',
          },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />
    );

    const messageInput = screen.getByLabelText('Message');
    fireEvent.change(messageInput, { target: { value: 'Updated message' } });

    expect(mockOnChange).toHaveBeenCalledWith('action-1', {
      actions: [
        {
          type: 'alert',
          level: 'info',
          title: '',
          message: 'Updated message',
        },
      ],
      summary: 'alert (info): Updated message',
    });
  });

  it('updates action_command action node data with proper actions array and summary', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="action_command"
        node={{
          id: 'action-cmd-1',
          type: 'action',
          data: {
            actions: [{ type: 'dose', pump: 'PUMP_A', doseMl: 5, pwm: 100 }],
            summary: 'dose 5ml (PUMP_A)',
          },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />
    );

    const doseInput = screen.getByLabelText(/Liều \(ml\)/);
    fireEvent.change(doseInput, { target: { value: '15' } });

    expect(mockOnChange).toHaveBeenCalledWith('action-cmd-1', {
      actions: [
        {
          type: 'dose',
          pump: 'PUMP_A',
          doseMl: 15,
          pwm: 100,
        },
      ],
      summary: 'dose 15ml (PUMP_A)',
    });
  });

  it('shows an ACTION badge on the alert action panel', () => {
    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'a1', type: 'action', data: { actions: [] } }}
        onChange={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByText('ACTION · ALERT')).toBeInTheDocument();
  });
});

describe('NodeEditorPanel — Config nodes', () => {
  it('shows a safe-read note on the config read panel', () => {
    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'cfg1', type: 'config', data: { variant: 'read' } }}
        onChange={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    expect(
      screen.getByText('Chỉ đọc — không thay đổi trạng thái thiết bị'),
    ).toBeInTheDocument();
  });

  it('renders the Config·Read editor and updates configKey/saveToVariable', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'cfg-1',
          type: 'config',
          data: { variant: 'read', configKey: '', saveToVariable: '' },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText('Config — Đọc')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Config key'), { target: { value: 'ph_target' } });
    expect(mockOnChange).toHaveBeenCalledWith('cfg-1', {
      variant: 'read',
      configKey: 'ph_target',
      saveToVariable: '',
    });

    fireEvent.change(screen.getByLabelText('Lưu vào biến'), { target: { value: 'ph_target_now' } });
    expect(mockOnChange).toHaveBeenCalledWith('cfg-1', {
      variant: 'read',
      configKey: '',
      saveToVariable: 'ph_target_now',
    });
  });

  it('renders the Config·Overwrite editor with rollback toggle and restore mode', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'cfg-2',
          type: 'config',
          data: {
            variant: 'overwrite',
            configKey: 'ec_target',
            overrideValue: '1.8',
            applyWhen: 'previous_condition_true',
            readOriginalBeforeWrite: false,
            restoreMode: 'on_condition_false',
          },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText('Config — Ghi đè')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Đọc giá trị gốc trước khi ghi (rollback an toàn)'));
    expect(mockOnChange).toHaveBeenCalledWith('cfg-2', {
      variant: 'overwrite',
      configKey: 'ec_target',
      overrideValue: '1.8',
      applyWhen: 'previous_condition_true',
      readOriginalBeforeWrite: true,
      restoreMode: 'on_condition_false',
    });
  });

  it('lets the overwrite value reference a context variable via the combobox', () => {
    const mockOnChange = vi.fn();

    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'cfg-3',
          type: 'config',
          data: { variant: 'overwrite', configKey: 'ec_target', overrideValue: '' },
        }}
        nodes={[
          { id: 'trigger', type: 'trigger', data: { kind: 'sensor' } },
          { id: 'cfg-3', type: 'config', data: { variant: 'overwrite' } },
        ]}
        edges={[{ id: 'e1', source: 'trigger', target: 'cfg-3' }]}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    const overrideInput = screen.getByLabelText('Giá trị ghi đè') as HTMLInputElement;
    const options = Array.from(
      document.querySelectorAll(`#${overrideInput.getAttribute('list')} option`),
    ).map((o) => o.getAttribute('value'));
    expect(options).toEqual(['ec', 'ph', 'temp', 'water_level']);
  });
});

describe('NodeEditorPanel — Alert template preview', () => {
  it('renders a preview substituting {{time}} and any in-scope variable, leaving unknown tokens visible', () => {
    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'action-1',
          type: 'action',
          data: {
            actions: [
              { type: 'alert', level: 'warning', title: '', message: 'EC: {{ec}} lúc {{time}}, x={{unknown_var}}' },
            ],
            summary: 'alert (warning): ...',
          },
        }}
        nodes={[{ id: 'trigger', type: 'trigger', data: { kind: 'sensor' } }]}
        edges={[]}
        onChange={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText(/EC: ⟨ec⟩ lúc ⟨time⟩, x=\{\{unknown_var\}\}/)).toBeInTheDocument();
    expect(screen.getByText(/Biến chưa xác định.*unknown_var/)).toBeInTheDocument();
  });

  it('clicking a variable chip appends {{name}} to the message', () => {
    const mockOnChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'action-1',
          type: 'action',
          data: {
            actions: [{ type: 'alert', level: 'info', title: '', message: 'Giá trị: ' }],
            summary: '...',
          },
        }}
        nodes={[{ id: 'trigger', type: 'trigger', data: { kind: 'sensor' } }]}
        edges={[]}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'ec' }));
    expect(mockOnChange).toHaveBeenCalledWith('action-1', {
      actions: [{ type: 'alert', level: 'info', title: '', message: 'Giá trị: {{ec}}' }],
      summary: 'alert (info): Giá trị: {{ec}}',
    });
  });
});

describe('NodeEditorPanel — Detailed Node Mockup Configurations', () => {
  it('renders Trigger Sensor fields: source probe, interval with unit, filter, and fallback toggle', () => {
    const mockOnChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 't1', type: 'trigger', data: { kind: 'sensor' } }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText('TRIGGER · SENSOR')).toBeInTheDocument();
    expect(screen.getByText('pH (thời gian thực)')).toBeInTheDocument();
    expect(screen.getByLabelText('Cảm biến nguồn')).toBeInTheDocument();
    expect(screen.getByLabelText('Chu kỳ đọc')).toBeInTheDocument();
    expect(screen.getByText('giây')).toBeInTheDocument();
    expect(screen.getByLabelText('Lọc nhiễu tín hiệu')).toBeInTheDocument();
    expect(screen.getByLabelText('Dùng giá trị gần nhất khi mất kết nối')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Chu kỳ đọc'), { target: { value: '45' } });
    expect(mockOnChange).toHaveBeenCalledWith('t1', expect.objectContaining({ kind: 'sensor', intervalSec: 45 }));
  });

  it('renders Trigger FSM fields: state machine, trigger timing, stages pills, and min duration', () => {
    const mockOnChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="recipe_override"
        node={{ id: 't2', type: 'trigger', data: { kind: 'fsm' } }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText('TRIGGER · FSM')).toBeInTheDocument();
    expect(screen.getByText('Giai đoạn canh tác (FSM)')).toBeInTheDocument();
    expect(screen.getByLabelText('Máy trạng thái')).toBeInTheDocument();
    expect(screen.getByLabelText('Kích hoạt khi')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Ra hoa/ })).toBeInTheDocument();
    expect(screen.getByLabelText('Thời lượng tối thiểu trong giai đoạn')).toBeInTheDocument();
    expect(screen.getByText('ngày')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Sinh trưởng/ }));
    expect(mockOnChange).toHaveBeenCalledWith('t2', expect.objectContaining({ kind: 'fsm', stage: 'Sinh trưởng' }));
  });

  it('renders Trigger Webhook fields: endpoint with copy button, mode, auth, and payload tags', () => {
    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 't3', type: 'trigger', data: { kind: 'webhook' } }}
        onChange={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText('TRIGGER · WEBHOOK')).toBeInTheDocument();
    expect(screen.getByText('Nhận dữ liệu từ bên ngoài')).toBeInTheDocument();
    expect(screen.getByDisplayValue('/hooks/f-2201')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sao chép' })).toBeInTheDocument();
    expect(screen.getByLabelText('Chế độ xử lý')).toBeInTheDocument();
    expect(screen.getByLabelText('Xác thực')).toBeInTheDocument();
    expect(screen.getByText('ec_out:flow')).toBeInTheDocument();
  });

  it('renders Condition Group fields: group operator, condition chips, and NOT toggle', () => {
    const mockOnChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'cg1',
          type: 'condition_group',
          data: {
            isGroup: true,
            groupOp: 'and',
            conditions: [
              { sensor: 'ec', operator: '>', value: 1.8 },
              { sensor: 'temp', operator: '<', value: 26 },
            ],
          },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText('Tất cả đều đúng')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /AND/ })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText('ec > 1.8')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '+ Thêm điều kiện' })).toBeInTheDocument();
    expect(screen.getByLabelText('Đảo ngược kết quả (NOT)')).toBeInTheDocument();
  });

  it('renders Action Dose/Water with pump, volume, PWM, and safety limit toggle', () => {
    const mockOnChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="action_command"
        node={{
          id: 'act-dose',
          type: 'action',
          data: {
            pump: 'PUMP_A',
            volumeMl: 12,
            pwm: 60,
            safetyLimit: true,
          },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText('ACTION · DOSE/WATER')).toBeInTheDocument();
    expect(screen.getByText('Định lượng dinh dưỡng A')).toBeInTheDocument();
    expect(screen.getByLabelText('Bơm')).toBeInTheDocument();
    expect(screen.getByLabelText(/Liều \(ml\)/)).toHaveValue(12);
    expect(screen.getByLabelText(/Công suất PWM/)).toHaveValue(60);
    expect(screen.getByLabelText('Giới hạn an toàn: tối đa 3 lần / giờ')).toBeChecked();
  });
});


