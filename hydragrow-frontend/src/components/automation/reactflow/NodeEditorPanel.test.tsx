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

  it('Alert action has a real FCM override control, not fake Email/Webhook channel pills', () => {
    const mockOnChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'a1', type: 'action', data: { actions: [{ type: 'alert', level: 'warning', message: 'x' }] } }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    expect(screen.queryByText('Email')).not.toBeInTheDocument();
    expect(screen.queryByText('Webhook')).not.toBeInTheDocument();
    const select = screen.getByLabelText('Gửi thông báo FCM') as HTMLSelectElement;
    fireEvent.change(select, { target: { value: 'always' } });
    expect(mockOnChange).toHaveBeenCalledWith('a1', expect.objectContaining({
      actions: [{ type: 'alert', level: 'warning', title: '', message: 'x', notifyFcm: true }],
    }));
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

  it('renders the Config read editor and updates configKey/saveToVariable via registry dropdown', () => {
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

    const combo = screen.getByLabelText('Config key') as HTMLSelectElement;
    expect(combo.tagName).toBe('SELECT');
    const opts = Array.from(combo.querySelectorAll('option')).map((o) => o.value);
    expect(opts).toContain('delay_between_a_and_b_sec');
    expect(opts).not.toContain('dose_max_ml');

    fireEvent.change(combo, { target: { value: 'ph_target' } });
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

  it('has no device group selector on the config read panel', () => {
    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'cfg-1', type: 'config', data: { variant: 'read' } }}
        onChange={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    expect(screen.queryByLabelText('Thiết bị / nhóm')).not.toBeInTheDocument();
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
    expect(screen.queryByLabelText('Máy trạng thái')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Kích hoạt khi')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Thời lượng tối thiểu trong giai đoạn')).not.toBeInTheDocument();
    expect(screen.queryByText('Chu trình vệ sinh CIP')).not.toBeInTheDocument();
    expect(
      screen.getByText(/Flow này sẽ chạy khi Condition bên dưới đúng/),
    ).toBeInTheDocument();
  });

  it('Trigger Webhook panel shows endpoint and points to the Webhook & Chain panel (no fictional auth/mode fields)', () => {
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
    expect(screen.getByDisplayValue('(được cấp khi lưu Flow lần đầu)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sao chép' })).toBeInTheDocument();
    expect(screen.queryByLabelText('Xác thực')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Chế độ xử lý')).not.toBeInTheDocument();
    expect(
      screen.getByText(/Cấu hình chi tiết ánh xạ trường ở panel Webhook & Chain phía dưới/),
    ).toBeInTheDocument();
  });

  it('renders the real ConditionGroupEditor for a condition_group node — can edit an existing condition in place', () => {
    const mockOnChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'cg1',
          type: 'condition_group',
          data: {
            conditions: [
              { op: 'and', children: [{ sensor: 'ec', operator: '>', value: 1.8 }] },
            ],
          },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    // AND/OR segmented control from the real editor.
    expect(screen.getAllByRole('button', { name: /AND — tất cả đúng/ })[0]).toBeInTheDocument();
    // The existing condition's VALUE field can be edited in place (not just
    // removed and re-added) — this is exactly what the old Chip UI could not do.
    const valueInput = screen.getByLabelText('Giá trị') as HTMLInputElement;
    expect(valueInput.value).toBe('1.8');
    fireEvent.change(valueInput, { target: { value: '2.1' } });
    expect(mockOnChange).toHaveBeenCalledWith('cg1', expect.objectContaining({
      conditions: [{ op: 'and', children: [{ sensor: 'ec', operator: '>', value: 2.1 }] }],
    }));
  });

  it('exposes the real time-window fields (mode + windowSec) for a plain condition node', () => {
    const mockOnChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="alert"
        node={{
          id: 'tw1',
          type: 'condition',
          data: { type: 'time-window', conditions: [{ sensor: 'ec', operator: '>', value: 1.8 }] },
        }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText('Chế độ đọc'), { target: { value: 'mean' } });
    expect(mockOnChange).toHaveBeenCalledWith('tw1', expect.objectContaining({
      conditions: [{ sensor: 'ec', operator: '>', value: 1.8, mode: 'mean', windowSec: 900 }],
    }));
  });

  it('supports adding a nested AND/OR sub-group — the old UI could not do this at all', () => {
    const mockOnChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'cg2', type: 'condition_group', data: { conditions: [{ op: 'and', children: [] }] } }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );
    fireEvent.click(screen.getAllByRole('button', { name: '+ Thêm nhóm con (AND/OR)' })[0]);
    expect(mockOnChange).toHaveBeenCalledWith('cg2', expect.objectContaining({
      conditions: [{ op: 'and', children: [{ op: 'and', children: [] }] }],
    }));
  });

  it('no longer writes the dead groupOp/applyWindow/debounceContinuous fields', () => {
    const mockOnChange = vi.fn();
    render(
      <NodeEditorPanel
        kind="alert"
        node={{ id: 'cg3', type: 'condition_group', data: { conditions: [{ op: 'and', children: [] }] } }}
        onChange={mockOnChange}
        onClose={vi.fn()}
      />,
    );
    expect(screen.queryByLabelText('Áp dụng trong')).not.toBeInTheDocument();
    expect(screen.queryByText(/Chống nhiễu/)).not.toBeInTheDocument();
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


