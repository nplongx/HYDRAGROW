import React from 'react';
import { Target, Waves, FlaskConical, ShieldAlert, Activity, CalendarClock } from 'lucide-react';
import { AccordionSection } from '../../components/ui/AccordionSection';
import { SubCard } from '../../components/ui/SubCard';
import { InputGroup } from '../../components/ui/InputGroup';
import { Switch } from '../../components/ui/Switch';
import { parse_cron_safe } from '../../../gleam_core/build/dev/javascript/gleam_core/settings/cron.mjs';

type InputEvent = React.ChangeEvent<HTMLInputElement | HTMLSelectElement>;

const VisualCronPicker = ({ value, onChange, label, desc }: {
  value: string; onChange: (val: string) => void; label: string; desc?: string;
}) => {
  const schedule = parse_cron_safe(value || "0 0 8 * * *");
  const minuteStr = String(schedule.minute).padStart(2, '0');
  const hourStr = String(schedule.hour).padStart(2, '0');
  const timeStr = `${hourStr}:${minuteStr}`;
  const isEveryDay = schedule.is_every_day;
  const selectedDays: string[] = schedule.days_str ? schedule.days_str.split(',') : [];

  const handleTimeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    if (!val) return;
    const [h, m] = val.split(':');
    const dow = selectedDays.length === 0 || isEveryDay ? '*' : selectedDays.join(',');
    onChange(`0 ${parseInt(m, 10)} ${parseInt(h, 10)} * * ${dow}`);
  };

  const toggleDay = (dayVal: string) => {
    let newDays = [...selectedDays];
    if (newDays.includes(dayVal)) newDays = newDays.filter(d => d !== dayVal);
    else newDays.push(dayVal);
    const newDow = newDays.length === 0 ? '*' : newDays.join(',');
    onChange(`0 ${parseInt(minuteStr, 10)} ${parseInt(hourStr, 10)} * * ${newDow}`);
  };

  const setEveryDay = () => onChange(`0 ${parseInt(minuteStr, 10)} ${parseInt(hourStr, 10)} * * *`);

  const daysOfWeek = [
    { val: 'MON', label: 'T2' }, { val: 'TUE', label: 'T3' }, { val: 'WED', label: 'T4' },
    { val: 'THU', label: 'T5' }, { val: 'FRI', label: 'T6' }, { val: 'SAT', label: 'T7' }, { val: 'SUN', label: 'CN' },
  ];

  return (
    <div className="space-y-4 bg-white/85 border border-emerald-100 p-5 rounded-xl w-full">
      <div>
        <label className="text-sm font-medium text-emerald-950 flex items-center gap-2">
          <CalendarClock size={16} className="text-emerald-800/80" /> {label}
        </label>
        {desc && <p className="text-xs text-emerald-700/75 mt-1">{desc}</p>}
      </div>
      <div className="flex flex-col md:flex-row md:items-center gap-6">
        <div className="bg-white px-4 py-2 rounded-lg border border-emerald-100 flex-shrink-0">
          <input
            type="time"
            value={timeStr}
            onChange={handleTimeChange}
            className="bg-transparent text-emerald-950 text-xl font-medium outline-none text-center cursor-pointer [color-scheme:dark]"
          />
        </div>
        <div className="flex-1 space-y-3">
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={setEveryDay}
              className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${isEveryDay ? 'bg-sky-600 text-white' : 'bg-emerald-100 text-emerald-800/80 hover:bg-emerald-200'}`}
            >
              Hằng ngày
            </button>
            <span className="text-xs text-emerald-700/75">hoặc chọn ngày:</span>
          </div>
          <div className="flex flex-wrap gap-2">
            {daysOfWeek.map(day => {
              const isSelected = !isEveryDay && selectedDays.includes(day.val);
              return (
                <button
                  key={day.val}
                  type="button"
                  onClick={() => toggleDay(day.val)}
                  className={`w-9 h-9 rounded-full text-xs font-medium transition-colors flex items-center justify-center border ${isSelected ? 'bg-sky-500/20 border-sky-500 text-sky-700' : 'bg-emerald-50 border-emerald-200 text-emerald-800/80 hover:border-emerald-400 hover:text-emerald-950'}`}
                >
                  {day.label}
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
};

export interface ThresholdsSectionProps {
  openSection: string | null;
  onToggleSection: (id: string) => void;
  /* eslint-disable-next-line @typescript-eslint/no-explicit-any */
  config: any;
  /* eslint-disable-next-line @typescript-eslint/no-explicit-any */
  setConfig: React.Dispatch<React.SetStateAction<any>>;
  isAdvancedMode: boolean;
  dosingValidationErrors: Record<string, string>;
  wizardStep: number;
  calibrationPoints: number[];
  activePoint: number;
  isCalibrationBlocked: boolean;
  isCapturingPoint: boolean;
  countdown: number;
  capturedPoints: Record<number, { voltage: number; confidence: number; capturedAt: string }>;
  calibrationSummary: {
    ph_v7: number | null;
    ph_v4: number | null;
    reliability: number;
  };
  handleCapturePoint: () => void;
  goToNextPoint: () => void;
  handleFinishAndSaveCalibration: () => void;
}

export const ThresholdsSection: React.FC<ThresholdsSectionProps> = ({
  openSection,
  onToggleSection,
  config,
  setConfig,
  isAdvancedMode,
  dosingValidationErrors,
  wizardStep,
  calibrationPoints,
  activePoint,
  isCalibrationBlocked,
  isCapturingPoint,
  countdown,
  capturedPoints,
  calibrationSummary,
  handleCapturePoint,
  goToNextPoint,
  handleFinishAndSaveCalibration,
}) => {
  return (
    <div className="space-y-4">
      {/* GROWTH */}
      <AccordionSection
        id="growth"
        title="Ngưỡng mục tiêu"
        icon={Target}
        isOpen={openSection === 'growth'}
        onToggle={() => onToggleSection('growth')}
      >
        <SubCard title="Dinh dưỡng (EC) & pH">
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <InputGroup
              label="EC mục tiêu"
              step="0.1"
              value={config.ec_target}
              onChange={(e: InputEvent) => setConfig({ ...config, ec_target: e.target.value })}
            />
            <InputGroup
              label="Sai số EC (±)"
              step="0.05"
              value={config.ec_tolerance}
              onChange={(e: InputEvent) => setConfig({ ...config, ec_tolerance: e.target.value })}
            />
            <InputGroup
              label="pH mục tiêu"
              step="0.1"
              value={config.ph_target}
              onChange={(e: InputEvent) => setConfig({ ...config, ph_target: e.target.value })}
            />
            <InputGroup
              label="Sai số pH (±)"
              step="0.05"
              value={config.ph_tolerance}
              onChange={(e: InputEvent) => setConfig({ ...config, ph_tolerance: e.target.value })}
            />
          </div>
        </SubCard>
        <SubCard title="Nhiệt độ & Phun sương" className="mt-4">
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <div className="sm:col-span-2 mt-2">
              <InputGroup
                label="Kích hoạt sương mạnh khi > (°C)"
                step="0.5"
                value={config.misting_temp_threshold}
                onChange={(e: InputEvent) => setConfig({ ...config, misting_temp_threshold: e.target.value })}
              />
            </div>
            <div className="sm:col-span-2 lg:col-span-4 pt-3 pb-1 border-t border-emerald-100">
              <span className="text-xs font-semibold text-emerald-800/80 uppercase tracking-wider">Thời tiết bình thường</span>
            </div>
            <InputGroup
              label="Phun sương (ms)"
              step="1000"
              value={config.misting_on_duration_ms}
              onChange={(e: InputEvent) => setConfig({ ...config, misting_on_duration_ms: e.target.value })}
            />
            <InputGroup
              label="Nghỉ (ms)"
              step="1000"
              value={config.misting_off_duration_ms}
              onChange={(e: InputEvent) => setConfig({ ...config, misting_off_duration_ms: e.target.value })}
            />
            <div className="hidden lg:block lg:col-span-2"></div>
            <div className="sm:col-span-2 lg:col-span-4 pt-3 pb-1 border-t border-emerald-100">
              <span className="text-xs font-semibold text-emerald-800/80 uppercase tracking-wider">Nắng nóng</span>
            </div>
            <InputGroup
              label="Phun sương (ms)"
              step="1000"
              value={config.high_temp_misting_on_duration_ms}
              onChange={(e: InputEvent) => setConfig({ ...config, high_temp_misting_on_duration_ms: e.target.value })}
            />
            <InputGroup
              label="Nghỉ (ms)"
              step="1000"
              value={config.high_temp_misting_off_duration_ms}
              onChange={(e: InputEvent) => setConfig({ ...config, high_temp_misting_off_duration_ms: e.target.value })}
            />
          </div>
        </SubCard>
      </AccordionSection>

      {/* WATER */}
      <AccordionSection
        id="water"
        title="Quản lý nước"
        icon={Waves}
        isOpen={openSection === 'water'}
        onToggle={() => onToggleSection('water')}
      >
        <SubCard title="Mực nước bồn">
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            <InputGroup label="Chiều cao bồn (cm)" value={config.tank_height} onChange={(e: InputEvent) => setConfig({ ...config, tank_height: e.target.value })} />
            <InputGroup label="Mực nước mục tiêu (cm)" value={config.water_level_target} onChange={(e: InputEvent) => setConfig({ ...config, water_level_target: e.target.value })} />
            <InputGroup label="Sai số (cm)" value={config.water_level_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, water_level_tolerance: e.target.value })} />
            <InputGroup label="Tối thiểu (cm)" value={config.water_level_min} onChange={(e: InputEvent) => setConfig({ ...config, water_level_min: e.target.value })} />
            <InputGroup label="Báo tràn (cm)" value={config.water_level_max} onChange={(e: InputEvent) => setConfig({ ...config, water_level_max: e.target.value })} />
          </div>
        </SubCard>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mt-4">
          <SubCard title="Bơm & Xả" className="h-full">
            <div className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="flex items-center justify-between p-3 bg-white/80 rounded-lg border border-emerald-100">
                  <span className="text-sm text-emerald-900 font-medium">Tự động cấp nước</span>
                  <Switch isOn={config.auto_refill_enabled} onClick={(val) => setConfig({ ...config, auto_refill_enabled: val })} />
                </div>
                <div className="flex items-center justify-between p-3 bg-white/80 rounded-lg border border-emerald-100">
                  <span className="text-sm text-emerald-900 font-medium">Tự động xả tràn</span>
                  <Switch isOn={config.auto_drain_overflow} onClick={(val) => setConfig({ ...config, auto_drain_overflow: val })} />
                </div>
              </div>
              <div className="pt-3 border-t border-emerald-100">
                <div className="flex items-center justify-between mb-3">
                  <span className="text-sm text-emerald-900 font-medium">Tự động pha loãng khi quá EC</span>
                  <Switch isOn={config.auto_dilute_enabled} onClick={(val) => setConfig({ ...config, auto_dilute_enabled: val })} />
                </div>
                {config.auto_dilute_enabled && (
                  <InputGroup label="Lượng xả pha loãng (cm)" step="0.5" value={config.dilute_drain_amount_cm} onChange={(e: InputEvent) => setConfig({ ...config, dilute_drain_amount_cm: e.target.value })} />
                )}
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 pt-3 border-t border-emerald-100">
                <InputGroup label="T.Gian Bơm Max (s)" value={config.max_refill_duration_sec} onChange={(e: InputEvent) => setConfig({ ...config, max_refill_duration_sec: e.target.value })} />
                <InputGroup label="T.Gian Xả Max (s)" value={config.max_drain_duration_sec} onChange={(e: InputEvent) => setConfig({ ...config, max_drain_duration_sec: e.target.value })} />
                <InputGroup label="Nước Timeout (s)" value={config.water_ack_threshold} onChange={(e: InputEvent) => setConfig({ ...config, water_ack_threshold: e.target.value })} />
              </div>
            </div>
          </SubCard>
          <SubCard title="Thay nước định kỳ" className="h-full">
            <div className="flex items-center justify-between mb-4 p-3 bg-white/80 rounded-lg border border-emerald-100">
              <span className="text-sm text-emerald-900 font-medium">Bật lịch xả nước</span>
              <Switch isOn={config.scheduled_water_change_enabled} onClick={(val) => setConfig({ ...config, scheduled_water_change_enabled: val })} />
            </div>
            {config.scheduled_water_change_enabled && (
              <div className="space-y-4">
                <VisualCronPicker label="Lịch xả tự động" value={config.water_change_cron} onChange={(val) => setConfig({ ...config, water_change_cron: val })} />
                <InputGroup label="Lượng xả (cm)" value={config.scheduled_drain_amount_cm} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_drain_amount_cm: e.target.value })} />
              </div>
            )}
          </SubCard>
        </div>
        <SubCard title="Cảm biến hoạt động" className="mt-4">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div className="flex items-center justify-between p-3 bg-white/80 rounded-lg border border-emerald-100">
              <span className="text-sm text-emerald-900 font-medium">Cảm biến EC</span>
              <Switch isOn={config.enable_ec_sensor ?? true} onClick={(val) => setConfig({ ...config, enable_ec_sensor: val })} />
            </div>
            <div className="flex items-center justify-between p-3 bg-white/80 rounded-lg border border-emerald-100">
              <span className="text-sm text-emerald-900 font-medium">Cảm biến pH</span>
              <Switch isOn={config.enable_ph_sensor ?? true} onClick={(val) => setConfig({ ...config, enable_ph_sensor: val })} />
            </div>
            <div className="flex items-center justify-between p-3 bg-white/80 rounded-lg border border-emerald-100">
              <span className="text-sm text-emerald-900 font-medium">Cảm biến Mực nước</span>
              <Switch isOn={config.enable_water_level_sensor ?? true} onClick={(val) => setConfig({ ...config, enable_water_level_sensor: val })} />
            </div>
            <div className="flex items-center justify-between p-3 bg-white/80 rounded-lg border border-emerald-100">
              <span className="text-sm text-emerald-900 font-medium">Cảm biến Nhiệt độ</span>
              <Switch isOn={config.enable_temp_sensor ?? true} onClick={(val) => setConfig({ ...config, enable_temp_sensor: val })} />
            </div>
          </div>
        </SubCard>
      </AccordionSection>

      {/* DOSING */}
      <AccordionSection
        id="dosing"
        title="Máy châm phân"
        icon={FlaskConical}
        isOpen={openSection === 'dosing'}
        onToggle={() => onToggleSection('dosing')}
      >
        <SubCard title="Lưu lượng bơm châm (ml/s)">
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <InputGroup label="Bơm Phân A (ml/s)" step="0.1" value={config.pump_a_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_a_capacity_ml_per_sec: e.target.value })} errorText={dosingValidationErrors.pump_a_capacity_ml_per_sec} />
            <InputGroup label="Bơm Phân B (ml/s)" step="0.1" value={config.pump_b_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_b_capacity_ml_per_sec: e.target.value })} errorText={dosingValidationErrors.pump_b_capacity_ml_per_sec} />
            <InputGroup label="Bơm pH UP (ml/s)" step="0.1" value={config.pump_ph_up_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_ph_up_capacity_ml_per_sec: e.target.value })} errorText={dosingValidationErrors.pump_ph_up_capacity_ml_per_sec} />
            <InputGroup label="Bơm pH DOWN (ml/s)" step="0.1" value={config.pump_ph_down_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_ph_down_capacity_ml_per_sec: e.target.value })} errorText={dosingValidationErrors.pump_ph_down_capacity_ml_per_sec} />
          </div>
        </SubCard>

        {isAdvancedMode && (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 my-4">
            <SubCard title="Thông số tính toán & Tỷ lệ bước (Per-pump)">
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                <InputGroup label="Max Dose / Chu kỳ (ml)" value={config.max_dose_per_cycle} onChange={(e: InputEvent) => setConfig({ ...config, max_dose_per_cycle: e.target.value })} />
                <InputGroup label="Độ trễ bơm A & B (s)" value={config.delay_between_a_and_b_sec} onChange={(e: InputEvent) => setConfig({ ...config, delay_between_a_and_b_sec: e.target.value })} />
                <InputGroup label="EC tăng / ml" step="0.01" value={config.ec_gain_per_ml} onChange={(e: InputEvent) => setConfig({ ...config, ec_gain_per_ml: e.target.value })} />
                <InputGroup label="pH tăng / ml" step="0.01" value={config.ph_shift_up_per_ml} onChange={(e: InputEvent) => setConfig({ ...config, ph_shift_up_per_ml: e.target.value })} />
                <InputGroup label="pH giảm / ml" step="0.01" value={config.ph_shift_down_per_ml} onChange={(e: InputEvent) => setConfig({ ...config, ph_shift_down_per_ml: e.target.value })} />
                <InputGroup label="Tỷ lệ bước EC A" step="0.05" value={config.ec_a_step_ratio ?? config.ec_step_ratio} onChange={(e: InputEvent) => setConfig({ ...config, ec_a_step_ratio: e.target.value })} />
                <InputGroup label="Tỷ lệ bước EC B" step="0.05" value={config.ec_b_step_ratio ?? config.ec_step_ratio} onChange={(e: InputEvent) => setConfig({ ...config, ec_b_step_ratio: e.target.value })} />
                <InputGroup label="Tỷ lệ bước pH UP" step="0.05" value={config.ph_up_step_ratio ?? config.ph_step_ratio} onChange={(e: InputEvent) => setConfig({ ...config, ph_up_step_ratio: e.target.value })} />
                <InputGroup label="Tỷ lệ bước pH DOWN" step="0.05" value={config.ph_down_step_ratio ?? config.ph_step_ratio} onChange={(e: InputEvent) => setConfig({ ...config, ph_down_step_ratio: e.target.value })} />
                <InputGroup label="EC Ack Threshold (s)" value={config.ec_ack_threshold} onChange={(e: InputEvent) => setConfig({ ...config, ec_ack_threshold: e.target.value })} />
                <InputGroup label="pH Ack Threshold (s)" value={config.ph_ack_threshold} onChange={(e: InputEvent) => setConfig({ ...config, ph_ack_threshold: e.target.value })} />
              </div>
            </SubCard>

            <SubCard title="Công suất PWM & Khởi động">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <InputGroup label="Bơm châm chung (%)" value={config.dosing_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, dosing_pwm_percent: e.target.value })} errorText={dosingValidationErrors.dosing_pwm_percent} />
                <InputGroup label="Bơm trộn (%)" value={config.osaka_mixing_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, osaka_mixing_pwm_percent: e.target.value })} />
                <InputGroup label="Bơm sương (%)" value={config.osaka_misting_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, osaka_misting_pwm_percent: e.target.value })} />
                <InputGroup label="Khởi động mềm (ms)" value={config.soft_start_duration} onChange={(e: InputEvent) => setConfig({ ...config, soft_start_duration: e.target.value })} />
                <div className="sm:col-span-2 pt-2 pb-1 border-t border-emerald-100">
                  <span className="text-xs font-semibold text-emerald-700/75 uppercase">PWM Tối thiểu từng bơm (%)</span>
                </div>
                <InputGroup label="Min PWM Phân A (%)" value={config.pump_a_min_pwm_percent ?? config.dosing_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, pump_a_min_pwm_percent: e.target.value })} />
                <InputGroup label="Min PWM Phân B (%)" value={config.pump_b_min_pwm_percent ?? config.dosing_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, pump_b_min_pwm_percent: e.target.value })} />
                <InputGroup label="Min PWM pH UP (%)" value={config.pump_ph_up_min_pwm_percent ?? config.dosing_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, pump_ph_up_min_pwm_percent: e.target.value })} />
                <InputGroup label="Min PWM pH DOWN (%)" value={config.pump_ph_down_min_pwm_percent ?? config.dosing_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, pump_ph_down_min_pwm_percent: e.target.value })} />
              </div>
            </SubCard>

            <SubCard title="Cấu hình xung (Pulse) & Nhịp châm" className="lg:col-span-2">
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
                <InputGroup label="PWM tối thiểu chung (%)" value={config.dosing_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, dosing_min_pwm_percent: e.target.value })} errorText={dosingValidationErrors.dosing_min_pwm_percent} />
                <InputGroup label="Mức kích hoạt nhịp (ml)" value={config.dosing_min_dose_ml} onChange={(e: InputEvent) => setConfig({ ...config, dosing_min_dose_ml: e.target.value })} />
                <InputGroup label="Xung Bật (ms)" value={config.dosing_pulse_on_ms} onChange={(e: InputEvent) => setConfig({ ...config, dosing_pulse_on_ms: e.target.value })} />
                <InputGroup label="Xung Tắt (ms)" value={config.dosing_pulse_off_ms} onChange={(e: InputEvent) => setConfig({ ...config, dosing_pulse_off_ms: e.target.value })} />
                <InputGroup label="Max xung / chu kỳ" value={config.dosing_max_pulse_count_per_cycle} onChange={(e: InputEvent) => setConfig({ ...config, dosing_max_pulse_count_per_cycle: e.target.value })} />
              </div>
            </SubCard>
          </div>
        )}
        <SubCard title="Khuấy trộn" className="mt-4">
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <InputGroup label="Chu kỳ khuấy (s)" value={config.scheduled_mixing_interval_sec} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_mixing_interval_sec: e.target.value })} />
            <InputGroup label="Thời gian khuấy (s)" value={config.scheduled_mixing_duration_sec} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_mixing_duration_sec: e.target.value })} />
            <InputGroup label="Khuấy sau châm (s)" value={config.active_mixing_sec} onChange={(e: InputEvent) => setConfig({ ...config, active_mixing_sec: e.target.value })} />
            <InputGroup label="Thời gian ổn định cảm biến (s)" value={config.sensor_stabilize_sec} onChange={(e: InputEvent) => setConfig({ ...config, sensor_stabilize_sec: e.target.value })} />
          </div>
        </SubCard>
      </AccordionSection>

      {/* SAFETY */}
      {isAdvancedMode && (
        <AccordionSection
          id="safety"
          title="An toàn"
          icon={ShieldAlert}
          isOpen={openSection === 'safety'}
          onToggle={() => onToggleSection('safety')}
        >
          <SubCard title="Ngưỡng cảnh báo & Giới hạn vận hành">
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
              <InputGroup label="Nhiệt độ thấp (°C)" value={config.min_temp_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_temp_limit: e.target.value })} />
              <InputGroup label="Nhiệt độ cao (°C)" value={config.max_temp_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_temp_limit: e.target.value })} />
              <InputGroup label="EC thấp" value={config.min_ec_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_ec_limit: e.target.value })} />
              <InputGroup label="EC cao" value={config.max_ec_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_ec_limit: e.target.value })} />
              <InputGroup label="pH thấp" value={config.min_ph_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_ph_limit: e.target.value })} />
              <InputGroup label="pH cao" value={config.max_ph_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_ph_limit: e.target.value })} />

              <InputGroup label="EC thay đổi tối đa / chu kỳ" step="0.1" value={config.max_ec_delta} onChange={(e: InputEvent) => setConfig({ ...config, max_ec_delta: e.target.value })} />
              <InputGroup label="pH thay đổi tối đa / chu kỳ" step="0.1" value={config.max_ph_delta} onChange={(e: InputEvent) => setConfig({ ...config, max_ph_delta: e.target.value })} />
              <InputGroup label="Max Dose / Giờ (ml)" value={config.max_dose_per_hour} onChange={(e: InputEvent) => setConfig({ ...config, max_dose_per_hour: e.target.value })} />
              <InputGroup label="Thời gian Cooldown (s)" value={config.cooldown_sec} onChange={(e: InputEvent) => setConfig({ ...config, cooldown_sec: e.target.value })} />
              <InputGroup label="Nước tối thiểu ngắt khẩn (cm)" value={config.water_level_critical_min} onChange={(e: InputEvent) => setConfig({ ...config, water_level_critical_min: e.target.value })} />
              <InputGroup label="Max chu kỳ bơm / giờ" value={config.max_refill_cycles_per_hour} onChange={(e: InputEvent) => setConfig({ ...config, max_refill_cycles_per_hour: e.target.value })} />
              <InputGroup label="Max chu kỳ xả / giờ" value={config.max_drain_cycles_per_hour} onChange={(e: InputEvent) => setConfig({ ...config, max_drain_cycles_per_hour: e.target.value })} />
            </div>
          </SubCard>
        </AccordionSection>
      )}

      {/* CALIBRATION / SENSOR */}
      <AccordionSection
        id="sensor"
        title="Cảm biến & Hiệu chuẩn"
        icon={Activity}
        isOpen={openSection === 'sensor'}
        onToggle={() => onToggleSection('sensor')}
      >
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <SubCard title="Hiệu chuẩn pH" className="h-full">
            <div className="space-y-4">
              {isCalibrationBlocked && (
                <div className="p-3 rounded-lg border border-red-500/30 bg-red-500/10 text-red-400 text-xs flex items-center gap-2">
                  <ShieldAlert size={16} />
                  Cảm biến ngoại tuyến hoặc lỗi hệ thống
                </div>
              )}
              {wizardStep < calibrationPoints.length ? (
                <div className="p-5 rounded-xl bg-white border border-emerald-100 shadow-inner">
                  <p className="text-xs text-sky-700 font-bold tracking-wider mb-1">
                    BƯỚC {wizardStep + 1}/{calibrationPoints.length}
                  </p>
                  <p className="text-sm text-emerald-950 mb-4">
                    Nhúng vào dung dịch <span className="font-bold text-emerald-800">pH {activePoint}</span>
                  </p>
                  <div className="flex items-center gap-3">
                    <button
                      type="button"
                      onClick={handleCapturePoint}
                      disabled={isCalibrationBlocked || isCapturingPoint}
                      className="px-4 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-sm font-medium disabled:opacity-50 transition-all"
                    >
                      {isCapturingPoint ? 'ĐANG ĐO...' : 'BẮT ĐẦU ĐO'}
                    </button>
                    {isCapturingPoint && (
                      <span className="text-sm font-mono text-emerald-900 bg-white px-3 py-1.5 rounded-md">
                        {countdown}s
                      </span>
                    )}
                    {capturedPoints[activePoint] && !isCapturingPoint && (
                      <button
                        type="button"
                        onClick={goToNextPoint}
                        className="px-4 py-2 rounded-lg bg-emerald-100 hover:bg-emerald-200 text-emerald-950 text-sm font-medium transition-all"
                      >
                        TIẾP THEO
                      </button>
                    )}
                  </div>
                </div>
              ) : (
                <div className="p-5 rounded-xl bg-white border border-emerald-100 shadow-inner space-y-4">
                  <div className="grid grid-cols-2 gap-2 text-center">
                    <div className="p-2 bg-white rounded-lg border border-emerald-100">
                      <p className="text-[10px] text-emerald-700/75 mb-0.5">V7</p>
                      <p className="text-sm font-mono text-emerald-950">{calibrationSummary.ph_v7}V</p>
                    </div>
                    <div className="p-2 bg-white rounded-lg border border-emerald-100">
                      <p className="text-[10px] text-emerald-700/75 mb-0.5">V4</p>
                      <p className="text-sm font-mono text-emerald-950">{calibrationSummary.ph_v4}V</p>
                    </div>
                    <div className="col-span-2 p-2 bg-white rounded-lg border border-emerald-100">
                      <p className="text-[10px] text-emerald-700/75 mb-0.5">Độ tin cậy</p>
                      <p className={`text-sm font-mono ${calibrationSummary.reliability >= 80 ? 'text-green-600' : 'text-yellow-600'}`}>
                        {calibrationSummary.reliability}%
                      </p>
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={handleFinishAndSaveCalibration}
                    className="w-full py-2.5 bg-sky-600 hover:bg-sky-500 text-white font-medium rounded-lg transition-all text-sm"
                  >
                    XÁC NHẬN & LƯU HIỆU CHUẨN
                  </button>
                </div>
              )}
            </div>
          </SubCard>

          {isAdvancedMode && (
            <SubCard title="Thông số hiệu chuẩn Cảm biến (Nâng cao)" className="h-full">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <InputGroup label="EC Factor" value={config.ec_factor} onChange={(e: InputEvent) => setConfig({ ...config, ec_factor: e.target.value })} />
                <InputGroup label="EC Offset" value={config.ec_offset} onChange={(e: InputEvent) => setConfig({ ...config, ec_offset: e.target.value })} />
                <InputGroup label="Temp Offset (°C)" value={config.temp_offset} onChange={(e: InputEvent) => setConfig({ ...config, temp_offset: e.target.value })} />
                <InputGroup label="Temp Beta" value={config.temp_compensation_beta} onChange={(e: InputEvent) => setConfig({ ...config, temp_compensation_beta: e.target.value })} />
                <InputGroup label="Tần suất gửi MQTT (ms)" value={config.publish_interval} onChange={(e: InputEvent) => setConfig({ ...config, publish_interval: e.target.value })} />
                <InputGroup label="Cửa sổ lọc TB (M.A.)" value={config.moving_average_window} onChange={(e: InputEvent) => setConfig({ ...config, moving_average_window: e.target.value })} />
              </div>
            </SubCard>
          )}
        </div>
      </AccordionSection>
    </div>
  );
};
