// src/components/logs/MetadataRenderers.tsx

const formatMetadataLabel = (key: string) => {
  const labels: Record<string, string> = {
    event_type: 'Loại',
    source: 'Nguồn phát sinh',
    message: 'Thông điệp chi tiết',
    skip_reason: 'Lý do bỏ qua',
    cycle_id: 'Mã chu kỳ',
    alert_type: 'Mã cảnh báo',
    retry_count: 'Số lần thử',
    limit_value: 'Ngưỡng giới hạn',
    threshold_before: 'Ngưỡng trước',
    threshold_after: 'Ngưỡng sau',
    parameter: 'Tham số',
    old_value: 'Giá trị cũ',
    new_value: 'Giá trị mới',
    tank_a_low: 'Bình dinh dưỡng A cạn',
    tank_b_low: 'Bình dinh dưỡng B cạn',
    tank_ph_down_low: 'Bình pH Down cạn',
    tank_ph_up_low: 'Bình pH Up cạn',
  };
  return labels[key] ?? key.replace(/_/g, ' ');
};

const formatMetadataValue = (value: any): string => {
  if (value == null) return '';
  if (typeof value === 'boolean') return value ? 'Có' : 'Không';
  if (typeof value === 'number') return Number.isInteger(value) ? String(value) : value.toFixed(2);
  if (typeof value === 'string') return value;
  return JSON.stringify(value);
};

export const DosingMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const cycleMeta = meta.pre != null ? meta : (meta.dosing_report ?? meta.dosing_data ?? meta);
  const dose = cycleMeta.dose ?? cycleMeta;

  const doseRows: { label: string; value: string; accent?: string }[] = [];
  if (dose.pump_a_ml != null && dose.pump_a_ml > 0)
    doseRows.push({ label: 'Dinh dưỡng A:', value: `${Number(dose.pump_a_ml).toFixed(1)} ml`, accent: 'text-orange-600 font-bold' });
  if (dose.pump_b_ml != null && dose.pump_b_ml > 0)
    doseRows.push({ label: 'Dinh dưỡng B:', value: `${Number(dose.pump_b_ml).toFixed(1)} ml`, accent: 'text-orange-600 font-bold' });
  if (dose.ph_up_ml != null && dose.ph_up_ml > 0)
    doseRows.push({ label: 'Thuốc pH Up:', value: `${Number(dose.ph_up_ml).toFixed(1)} ml`, accent: 'text-purple-700 font-bold' });
  if (dose.ph_down_ml != null && dose.ph_down_ml > 0)
    doseRows.push({ label: 'Thuốc pH Down:', value: `${Number(dose.ph_down_ml).toFixed(1)} ml`, accent: 'text-red-700 font-bold' });

  if (doseRows.length === 0) return null;

  return (
    <div className="mt-3 text-xs">
      <div className="bg-emerald-50/80 border border-emerald-100 rounded-xl px-3 py-2">
        <div className="text-[9px] font-black text-emerald-700/75 mb-1.5 uppercase tracking-wider">
          Khẩu phần châm thực tế
        </div>
        <div className="flex flex-col gap-1.5">
          {doseRows.map((r) => (
            <div key={r.label} className="flex items-center justify-between border-b border-emerald-100/50 last:border-transparent pb-1 last:pb-0">
              <span className="text-emerald-800/80 text-[11px] font-medium">{r.label}</span>
              <span className={`${r.accent ?? 'text-emerald-900'} text-[11px]`}>{r.value}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export const GenericMetadata = ({ meta, title = 'Thông số kỹ thuật' }: { meta: any; title?: string }) => {
  if (!meta) return null;
  const rows = Object.entries(meta)
    .filter(([key, value]) => value != null && value !== '' && key !== 'interaction_matrix' && key !== 'kalman_confidence')
    .map(([key, value]) => ({
      label: formatMetadataLabel(key),
      value: formatMetadataValue(value),
      accent: key === 'event_type' || key === 'cycle_id' ? 'text-indigo-700 font-mono font-bold' : undefined,
    }));

  if (rows.length === 0) return null;

  return (
    <div className="mt-3 flex flex-col gap-1.5 text-xs font-medium bg-emerald-50/80 border border-emerald-100 rounded-xl px-3 py-2.5">
      <div className="text-[9px] font-black text-emerald-700/75 mb-0.5 uppercase tracking-wider">{title}</div>
      {rows.map((r) => (
        <div key={r.label} className="flex items-center justify-between gap-3 border-b border-white/5 last:border-transparent pb-1 last:pb-0">
          <span className="text-emerald-800/80 text-[11px] capitalize">{r.label}</span>
          <span className={`${r.accent ?? 'text-emerald-900'} text-[11px] text-right break-all`}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};

export const MetadataRenderer = ({ metadata }: { metadata?: Record<string, any> }) => {
  if (!metadata) return null;
  switch (metadata.event_type) {
    case 'DosingCycleComplete':
      return <DosingMetadata meta={metadata} />;
    default:
      return <GenericMetadata meta={metadata} />;
  }
};
