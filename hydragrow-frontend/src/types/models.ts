export interface AppSettings {
  backend_url: string;
  api_key: string;
  device_id: string;
  [key: string]: any;
}

export type DeviceState = 'on' | 'off';

export interface PumpStatus {
  pump_a: boolean;
  pump_b: boolean;
  ph_up: boolean;
  ph_down: boolean;
  osaka_pump: boolean;
  mist_valve: boolean;
  mix_valve: boolean;
  water_pump_in: boolean;
  water_pump_out: boolean;
  pump_a_pwm?: number;
  pump_b_pwm?: number;
  ph_up_pwm?: number;
  ph_down_pwm?: number;
  osaka_pwm?: number;
  dosing_pulse_active?: boolean;
  dosing_pulse_count?: number;
}

export interface SensorData extends DeviceHealth {
  device_id: string;
  ec: number;
  ph: number;
  temp: number;
  water_level: number;
  pump_status: PumpStatus;
  time: string;
  rssi?: number;
  free_heap?: number;
  uptime?: number;
  err_water?: boolean;
  err_temp?: boolean;
  err_ph?: boolean;
  err_ec?: boolean;
  is_continuous?: boolean;
  ph_voltage_mv?: number;
}

export interface CropStage {
  name: string;
  duration_sec: number;
  ec_target: number;
  ec_tolerance: number;
  ph_target: number;
  ph_tolerance: number;
  nutrient_a_ratio: number;
  nutrient_b_ratio: number;
  water_level_target: number;
  water_change_interval_days?: number;
  water_change_drain_cm?: number;
  auto_dilute_ec_trigger?: number;
  misting_on_duration_ms: number;
  misting_off_duration_ms: number;
  max_dose_per_cycle_ml?: number;
}

export interface CropRecipe {
  schema_version: number;
  recipe_id: string;
  season_id: string;
  device_id: string;
  revision: number;
  start_time_sec: number;
  current_stage_index: number;
  stages: CropStage[];
}

export interface RecipeTemplate {
  id: string;
  name: string;
  crop: string;
  description?: string;
  stages: CropStage[];
  created_at: string;
}

export interface UnifiedDeviceConfig {
  device_id: string;
  control_mode: 'auto' | 'manual';
  is_enabled: boolean;
  ec_target: number;
  ec_tolerance: number;
  ph_target: number;
  ph_tolerance: number;
  nutrient_a_ratio: number;
  nutrient_b_ratio: number;
  water_level_min: number;
  water_level_target: number;
  water_level_max: number;
  water_level_tolerance: number;
  auto_refill_enabled: boolean;
  auto_drain_overflow: boolean;
  auto_dilute_enabled: boolean;
  dilute_drain_amount_cm: number;
  scheduled_water_change_enabled: boolean;
  water_change_cron: string;
  scheduled_drain_amount_cm: number;
  water_change_interval_days?: number;
  misting_on_duration_ms: number;
  misting_off_duration_ms: number;
  emergency_shutdown: boolean;
  max_ec_limit: number;
  min_ec_limit: number;
  min_ph_limit: number;
  max_ph_limit: number;
  max_ec_delta: number;
  max_ph_delta: number;
  max_dose_per_cycle: number;
  water_level_critical_min: number;
  max_refill_duration_sec: number;
  max_drain_duration_sec: number;
  ec_ack_threshold: number;
  ph_ack_threshold: number;
  water_ack_threshold: number;
  ec_gain_per_ml: number;
  ph_shift_up_per_ml: number;
  ph_shift_down_per_ml: number;
  active_mixing_sec: number;
  sensor_stabilize_sec: number;
  ec_step_ratio: number;
  ph_step_ratio: number;
  dosing_pwm_percent: number;
  osaka_mixing_pwm_percent: number;
  osaka_misting_pwm_percent: number;
  enable_ec_sensor: boolean;
  enable_ph_sensor: boolean;
  enable_water_level_sensor: boolean;
  enable_temp_sensor: boolean;
}

export interface CropSeason {
  id: string;
  device_id: string;
  name: string;
  plant_type: string | null;
  description: string | null;
  start_time: string;
  end_time: string | null;
  status: 'active' | 'completed';
}

export interface DeviceHealth {
  rssi?: number;
  free_heap?: number;
  uptime?: number;
}

export interface StatusPayload {
  is_online: boolean;
  last_seen: string;
}

export interface TankAlert {
  tank_a_low: boolean;
  tank_b_low: boolean;
  tank_ph_down_low: boolean;
  tank_ph_up_low: boolean;
}