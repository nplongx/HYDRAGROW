export interface AppSettings {
  backend_url: string;
  api_key: string;
  device_id: string;
  control_mode?: 'auto' | 'manual';
  min_ph_limit?: number;
  max_ph_limit?: number;
  min_temp_limit?: number;
  max_temp_limit?: number;
  water_level_min?: number;
  water_level_max?: number;
  water_level_target?: number;
  ph_target?: number;
  ph_tolerance?: number;
  [key: string]: unknown;
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
  ec_a_step_ratio?: number;
  ec_b_step_ratio?: number;
  ph_up_step_ratio?: number;
  ph_down_step_ratio?: number;
  pump_a_capacity_ml_per_sec?: number;
  pump_b_capacity_ml_per_sec?: number;
  pump_ph_up_capacity_ml_per_sec?: number;
  pump_ph_down_capacity_ml_per_sec?: number;
  dosing_pwm_percent: number;
  dosing_min_pwm_percent?: number;
  pump_a_min_pwm_percent?: number;
  pump_b_min_pwm_percent?: number;
  pump_ph_up_min_pwm_percent?: number;
  pump_ph_down_min_pwm_percent?: number;
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

export interface OtaStatus {
  device_id: string;
  current_version: string;
  latest_version: string | null;
  update_available: boolean;
}

export interface WifiCandidate {
  ssid: string;
  password: string;
  priority: number;
}

// --- Types từ hydragrow-shared/src/telemetry/health.rs ---
export interface KalmanConfidence {
  nutrient_a: number;
  nutrient_b: number;
  ph_up: number;
  ph_down: number;
  water_in: number;
  water_out: number;
  osaka_mixing: number;
  misting: number;
}

export interface DeviceHealthSnapshot {
  device_id: string;
  free_heap: number;
  uptime_sec: number;
  rssi: number;
  health_score_percent: number;
  fsm_state_display: string;
  log_drop_count: number;
  firmware_version: string;
  kalman_confidence?: KalmanConfidence;
  matrix_update_count: number;
  matrix_is_warm: boolean;
  hestia?: unknown;
  diagnostics?: {
    health_score_percent?: number;
  };
  timestamp_ms: number;
}

// --- Types từ hydragrow-shared/src/log.rs ---
export type LogLevel = 'info' | 'success' | 'warning' | 'critical';

export type LogCategory =
  | 'system'
  | 'dosing'
  | 'water'
  | 'calibration'
  | 'sensor'
  | 'alert'
  | 'user_action';

export interface SystemEvent {
  id?: string | number;
  device_id: string;
  level: LogLevel;
  category: LogCategory;
  message: string;
  timestamp_ms: number;
  metadata?: Record<string, unknown>;
}

// --- Multi-device types ---
export interface OwnedDevice {
  id: number;
  user_id: number;
  device_id: string;
  label: string | null;
  claimed_at: string;
}

export interface DeviceRecipeStatus {
  device_id: string;
  recipe_id: string | null;
  recipe_name: string | null;
  crop: string | null;
  updated_at: string;
}

export interface BulkApplyResult {
  succeeded: string[];
  failed: { device_id: string; reason: string }[];
}
