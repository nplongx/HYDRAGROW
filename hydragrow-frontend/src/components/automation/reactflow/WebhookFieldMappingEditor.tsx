import type { WebhookTriggerConfig, WebhookFieldMapping } from '../../../lib/automation/ir';

export interface WebhookFieldMappingEditorProps {
  config: WebhookTriggerConfig;
  onChange: (config: WebhookTriggerConfig) => void;
}

export function WebhookFieldMappingEditor({ config, onChange }: WebhookFieldMappingEditorProps) {
  const mode = config.mode ?? 'flow';
  const mappings = config.fieldMappings ?? [];

  const setMode = (nextMode: 'flow' | 'direct') => {
    onChange({
      ...config,
      mode: nextMode,
    });
  };

  const updateMapping = (index: number, updated: WebhookFieldMapping) => {
    const next = [...mappings];
    next[index] = updated;
    onChange({
      ...config,
      fieldMappings: next,
    });
  };

  const removeMapping = (index: number) => {
    const next = mappings.filter((_, i) => i !== index);
    onChange({
      ...config,
      fieldMappings: next,
    });
  };

  const addMapping = () => {
    onChange({
      ...config,
      fieldMappings: [...mappings, { bodyPath: '', targetField: '' }],
    });
  };

  return (
    <div className="space-y-3">
      <div>
        <label className="mb-1 block text-xs font-medium text-primary-deep">Chế độ Webhook</label>
        <div className="flex gap-2">
          <label className="flex flex-1 cursor-pointer items-center justify-center rounded border border-line p-1.5 text-xs">
            <input
              type="radio"
              name="webhookMode"
              value="flow"
              checked={mode === 'flow'}
              onChange={() => setMode('flow')}
              className="mr-1.5 text-primary"
            />
            <span>Chạy qua Flow</span>
          </label>
          <label className="flex flex-1 cursor-pointer items-center justify-center rounded border border-line p-1.5 text-xs">
            <input
              type="radio"
              name="webhookMode"
              value="direct"
              checked={mode === 'direct'}
              onChange={() => setMode('direct')}
              className="mr-1.5 text-primary"
            />
            <span>Gọi lệnh trực tiếp</span>
          </label>
        </div>
      </div>

      <div>
        <div className="mb-1.5 flex items-center justify-between">
          <label className="text-xs font-medium text-primary-deep">Ánh xạ trường (Field Mappings)</label>
          <button
            type="button"
            className="text-xs font-medium text-status hover:text-text-muted"
            onClick={addMapping}
          >
            + Thêm ánh xạ
          </button>
        </div>

        {mappings.length === 0 ? (
          <p className="text-xs text-text-muted/60 italic">Chưa có ánh xạ nào. Mặc định sẽ copy nguyên JSON body.</p>
        ) : (
          <div className="space-y-2">
            {mappings.map((m, idx) => (
              <div key={idx} className="flex items-center gap-1.5 rounded border border-line bg-pill p-1.5">
                <input
                  type="text"
                  placeholder="bodyPath (vd: data.ph)"
                  className="ui-input flex-1 !p-1 text-xs"
                  value={m.bodyPath}
                  onChange={(e) => updateMapping(idx, { ...m, bodyPath: e.target.value })}
                />
                <span className="text-xs text-primary">→</span>
                <input
                  type="text"
                  placeholder="target (vd: ph)"
                  className="ui-input flex-1 !p-1 text-xs"
                  value={m.targetField}
                  onChange={(e) => updateMapping(idx, { ...m, targetField: e.target.value })}
                />
                <button
                  type="button"
                  className="text-xs font-bold text-error hover:text-error px-1"
                  onClick={() => removeMapping(idx)}
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
