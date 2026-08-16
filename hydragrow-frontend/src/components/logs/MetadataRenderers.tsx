const getMetaNumber = (meta: any, keys: string[]): number | undefined => {
  if (!meta) return undefined;
  for (const key of keys) {
    const val = meta[key];
    if (val != null && !isNaN(Number(val))) return Number(val);
  }
  return undefined;
};

const formatMetadataLabel = (key: string) => {
  const labels: Record<string, string> = {
    event_type: 'Loại sự kiện',
    source: 'Nguồn phát sinh',
    message: 'Thông điệp chi tiết',
    skip_reason: 'Lý do bỏ qua',
    cycle_id: 'Mã chu kỳ',
    alert_type: 'Mã cảnh báo',
    retry_count: 'Số lần thử lại',
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
  if (typeof value === 'number') return Number.isInteger(value) ? String(value) : value.toFixed(4);
  if (typeof value === 'string') return value;
  return JSON.stringify(value);
};

export const DosingMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const cycleMeta = meta.pre != null ? meta : (meta.dosing_report ?? meta.dosing_data ?? meta);
  const pre = cycleMeta.pre ?? {};
  const post = cycleMeta.post_stable ?? cycleMeta.post_mixing ?? cycleMeta.post ?? {};
  const dose = cycleMeta.dose ?? cycleMeta;
  const target = cycleMeta.target ?? cycleMeta;

  const sections: { title?: string; rows: { label: string; value: string; accent?: string }[] }[] = [];
  const doseRows: { label: string; value: string; accent?: string }[] = [];

  if (dose.pump_a_ml != null && dose.pump_a_ml > 0) doseRows.push({ label: 'Dinh dưỡng A:', value: `${Number(dose.pump_a_ml).toFixed(1)} ml`, accent: 'text-orange-400 font-bold' });
  if (dose.pump_b_ml != null && dose.pump_b_ml > 0) doseRows.push({ label: 'Dinh dưỡng B:', value: `${Number(dose.pump_b_ml).toFixed(1)} ml`, accent: 'text-orange-400 font-bold' });
  if (dose.ph_up_ml != null && dose.ph_up_ml > 0) doseRows.push({ label: 'Thuốc pH Up:', value: `${Number(dose.ph_up_ml).toFixed(1)} ml`, accent: 'text-purple-700 font-bold' });
  if (dose.ph_down_ml != null && dose.ph_down_ml > 0) doseRows.push({ label: 'Thuốc pH Down:', value: `${Number(dose.ph_down_ml).toFixed(1)} ml`, accent: 'text-red-700 font-bold' });
  if (doseRows.length) sections.push({ title: 'Khẩu phần châm thực tế', rows: doseRows });

  const deltaRows: { label: string; value: string; accent?: string }[] = [];
  const ecBefore = getMetaNumber(pre, ['ec', 'EC', 'start_ec']);
  const ecAfter = getMetaNumber(post, ['ec', 'EC', 'after_ec', 'post_mixing_ec']);
  const phBefore = getMetaNumber(pre, ['ph', 'pH', 'start_ph']);
  const phAfter = getMetaNumber(post, ['ph', 'pH', 'after_ph', 'post_mixing_ph']);

  if (ecBefore != null && ecAfter != null && ecBefore !== 0.0) {
    const diff = ecAfter - ecBefore;
    deltaRows.push({ label: 'Hành trình sai số EC:', value: `${ecBefore.toFixed(2)} → ${ecAfter.toFixed(2)} (${diff >= 0 ? '+' : ''}${diff.toFixed(2)})`, accent: 'text-cyan-700 font-mono font-bold' });
  }
  if (phBefore != null && phAfter != null && phBefore !== 0.0) {
    const diff = phAfter - phBefore;
    deltaRows.push({ label: 'Hành trình sai số pH:', value: `${phBefore.toFixed(2)} → ${phAfter.toFixed(2)} (${diff >= 0 ? '+' : ''}${diff.toFixed(2)})`, accent: 'text-fuchsia-400 font-mono font-bold' });
  }
  if (deltaRows.length) sections.push({ title: 'Biến động cảm biến', rows: deltaRows });

  const targetRows: { label: string; value: string; accent?: string }[] = [];
  const targetEc = getMetaNumber(target, ['ec', 'target_ec']);
  const targetPh = getMetaNumber(target, ['ph', 'target_ph']);

  if (targetEc != null && targetEc > 0) targetRows.push({ label: 'Ngưỡng EC mục tiêu:', value: targetEc.toFixed(2), accent: 'text-cyan-300 font-bold' });
  if (targetPh != null && targetPh > 0) targetRows.push({ label: 'Ngưỡng pH mục tiêu:', value: targetPh.toFixed(2), accent: 'text-fuchsia-300 font-bold' });
  if (cycleMeta.step_ratio_ec != null) targetRows.push({ label: 'AI Kalman Step EC:', value: `${(cycleMeta.step_ratio_ec * 100).toFixed(0)}%`, accent: 'text-teal-400 font-bold' });

  if (targetRows.length) sections.push({ title: 'Mục tiêu & Thuật toán', rows: targetRows });
  if (sections.length === 0) return null;

  return (
    <div className="mt-3 space-y-2 text-xs grid grid-cols-1 sm:grid-cols-2 gap-2">
      {sections.map((sec, idx) => (
        <div key={idx} className="bg-emerald-50/80 border border-emerald-100 rounded-xl px-3 py-2">
          {sec.title && <div className="text-[9px] font-black text-emerald-700/75 mb-1.5 uppercase tracking-wider">{sec.title}</div>}
          <div className="flex flex-col gap-1.5">
            {sec.rows.map(r => (
              <div key={r.label} className="flex items-center justify-between border-b border-white/5 last:border-transparent pb-1 last:pb-0">
                <span className="text-emerald-800/80 text-[11px] font-medium">{r.label}</span>
                <span className={`${r.accent ?? 'text-emerald-900'} text-[11px]`}>{r.value}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
};

export const GenericMetadata = ({ meta, title = 'Thông số kỹ thuật' }: { meta: any; title?: string }) => {
  if (!meta) return null;
  const rows = Object.entries(meta)
    .filter(([, value]) => value != null && value !== '')
    .map(([key, value]) => ({
      label: formatMetadataLabel(key),
      value: formatMetadataValue(value),
      accent: key === 'event_type' || key === 'cycle_id' ? 'text-indigo-700 font-mono font-bold' : undefined,
    }));
  if (rows.length === 0) return null;

  return (
    <div className="mt-3 flex flex-col gap-1.5 text-xs font-medium bg-emerald-50/80 border border-emerald-100 rounded-xl px-3 py-2.5">
      <div className="text-[9px] font-black text-emerald-700/75 mb-0.5 uppercase tracking-wider">{title}</div>
      {rows.map(r => (
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
    case 'DosingCycleComplete': return <DosingMetadata meta={metadata} />;
    default: return <GenericMetadata meta={metadata} />;
  }
};
