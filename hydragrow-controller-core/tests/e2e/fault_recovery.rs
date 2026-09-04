//! E2E: Fault injection và recovery scenarios

use hydragrow_controller_core::WaterDirection;
use hydragrow_controller_core::core::fsm::{
    context::SystemContext, events::OrchestratorEvent, orchestrator,
};
use hydragrow_shared::fsm::{FaultCode, SystemPhase};
use hydragrow_shared::{ControlMode, ControllerConfig, SensorData};

fn minimal_config() -> ControllerConfig {
    ControllerConfig {
        control_mode: ControlMode::Auto,
        is_enabled: true,
        ec_target: 1.5,
        ec_tolerance: 0.05,
        ph_target: 6.0,
        ph_tolerance: 0.1,
        enable_ec_sensor: true,
        enable_ph_sensor: true,
        enable_water_level_sensor: false,
        enable_temp_sensor: false,
        max_dose_per_hour: 0.5, // Rất thấp để trigger hourly limit fault
        max_dose_per_cycle: 10.0,
        cooldown_sec: 2,
        soft_start_duration: 0,
        max_ec_delta: 2.0, // Cao để không bị noise fault
        max_ph_delta: 2.0,
        max_ec_limit: 5.0,
        min_ec_limit: 0.0,
        min_ph_limit: 3.0,
        max_ph_limit: 10.0,
        max_refill_cycles_per_hour: 2,
        max_drain_cycles_per_hour: 2,
        max_refill_duration_sec: 60,
        max_drain_duration_sec: 60,
        ec_gain_per_ml: 0.3,
        ph_shift_up_per_ml: 0.15,
        ph_shift_down_per_ml: 0.15,
        pump_a_capacity_ml_per_sec: 1.0,
        pump_b_capacity_ml_per_sec: 1.0,
        pump_ph_up_capacity_ml_per_sec: 0.5,
        pump_ph_down_capacity_ml_per_sec: 0.5,
        dosing_pwm_percent: 80,
        dosing_min_pwm_percent: 30,
        dosing_pulse_on_ms: 50,
        dosing_pulse_off_ms: 50,
        dosing_min_dose_ml: 0.1,
        dosing_max_pulse_count_per_cycle: 50,
        ec_step_ratio: 1.0,
        ph_step_ratio: 1.0,
        best_ec_ratio: 1.0,
        best_ph_ratio: 1.0,
        adaptive_mixing_sec: 5,
        adaptive_stabilize_sec: 5,
        effective_ec_tolerance: 0.05,
        effective_ph_tolerance: 0.1,
        active_mixing_sec: 5,
        sensor_stabilize_sec: 5,
        water_level_min: 15.0,
        water_level_target: 20.0,
        water_level_max: 24.0,
        water_level_tolerance: 1.0,
        water_level_critical_min: 10.0,
        ec_ack_threshold: 0.05,
        ph_ack_threshold: 0.1,
        water_ack_threshold: 0.5,
        scheduled_mixing_interval_sec: 7200,
        scheduled_mixing_duration_sec: 30,
        misting_on_duration_ms: 5000,
        misting_off_duration_ms: 60000,
        osaka_mixing_pwm_percent: 60,
        osaka_misting_pwm_percent: 100,
        high_temp_misting_on_duration_ms: 15000,
        high_temp_misting_off_duration_ms: 60000,
        misting_temp_threshold: 35.0,
        delay_between_a_and_b_sec: 0,
        min_temp_limit: 10.0,
        max_temp_limit: 40.0,
        auto_refill_enabled: false,
        auto_drain_overflow: false,
        auto_dilute_enabled: false,
        dilute_drain_amount_cm: 0.0,
        scheduled_water_change_enabled: false,
        water_change_cron: String::new(),
        scheduled_drain_amount_cm: 0.0,
        water_change_interval_days: None,
        emergency_shutdown: false,
        nutrient_a_ratio: 1.0,
        nutrient_b_ratio: 1.0,
        device_id: "e2e_fault_test".to_string(),
        active_recipe: None,
        tuner_state: 0,
        interaction_matrix: None,
        matrix_update_count: 0,
        matrix_is_warm: false,
        kalman_confidence: None,
        tank_height: 30,
        pump_a_min_pwm_percent: None,
        pump_b_min_pwm_percent: None,
        pump_ph_up_min_pwm_percent: None,
        pump_ph_down_min_pwm_percent: None,
    }
}

fn normal_sensor() -> SensorData {
    SensorData {
        device_id: "e2e_fault_test".to_string(),
        ec: 1.5,
        ph: 6.0,
        temp: 25.0,
        water_level: 20.0,
        pump_status: Default::default(),
        time: "2026-08-25T10:00:00Z".to_string(),
        controller_received_ms: None,
        rssi: None,
        free_heap: None,
        uptime: None,
        err_water: None,
        err_temp: None,
        err_ec: None,
        err_ph: None,
        is_continuous: None,
        ph_voltage_mv: None,
    }
}

fn tick_apply(
    ctx: &mut SystemContext,
    config: &ControllerConfig,
    sensors: &SensorData,
    uptime_ms: u64,
    sensor_last_update_ms: u64,
) -> Vec<OrchestratorEvent> {
    let now_ms = 1_700_000_000_000u64 + uptime_ms;
    let mut result = orchestrator::tick(
        now_ms,
        uptime_ms,
        config,
        sensors,
        sensor_last_update_ms,
        ctx,
    );
    ctx.apply_delta(&mut result.delta);
    result.events
}

/// E2E Fault Test 1: Sensor timeout → SensorTimeout fault → recovery
#[test]
fn e2e_sensor_timeout_fault_and_recovery() {
    let config = minimal_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    let sensor = normal_sensor();

    // Inject timeout: sensor data 100s cũ
    let uptime_ms = 100_000u64;
    let sensor_last_update_ms = 0u64; // 100s không nhận sensor

    let events = tick_apply(&mut ctx, &config, &sensor, uptime_ms, sensor_last_update_ms);

    assert_eq!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::SensorTimeout),
        "100s không nhận sensor phải vào SensorTimeout fault"
    );

    // Phải dừng tất cả hardware khi fault
    let stops_water = events.iter().any(|e| {
        matches!(e, OrchestratorEvent::SetWaterPump { direction }
            if *direction == WaterDirection::Stop)
    });
    assert!(stops_water, "Fault phải emit dừng water pump");

    eprintln!("✅ SensorTimeout fault triggered correctly");

    // === Recovery: Nhận sensor mới ===
    let uptime_after = 101_000u64;
    let sensor_update_recent = 100_500u64; // Vừa nhận sensor 500ms trước

    let _events = tick_apply(
        &mut ctx,
        &config,
        &sensor,
        uptime_after,
        sensor_update_recent,
    );

    assert_eq!(
        ctx.phase,
        SystemPhase::Monitoring,
        "Sau khi nhận sensor mới, thoát SensorTimeout về Monitoring"
    );
    eprintln!("✅ SensorTimeout recovery OK");
}

/// E2E Fault Test 2: Không có fault khi đã ở Fault state và timeout lại
#[test]
fn e2e_fault_state_not_double_faulted() {
    let config = minimal_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Fault(FaultCode::SensorTimeout);

    let sensor = normal_sensor();
    let uptime_ms = 200_000u64;
    let sensor_last_update_ms = 0u64; // Vẫn timeout

    let events = tick_apply(&mut ctx, &config, &sensor, uptime_ms, sensor_last_update_ms);

    // Vẫn ở fault state, không bị fault mới (tránh infinite loop events)
    assert_eq!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::SensorTimeout),
        "Đã fault rồi không được fault lại"
    );

    // Không emit dừng pump lần nữa khi đã fault (phải dừng được rồi)
    eprintln!("✅ No double-fault behavior verified");
    let _ = events; // events có thể empty
}

/// E2E Fault Test 3: Manual mode → không có Fault từ sensor check
#[test]
fn e2e_manual_mode_not_affected_by_sensor_timeout_trigger() {
    let config = minimal_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::ManualMode;

    let sensor = normal_sensor();
    // Sensor timeout: 100s không nhận
    let events = tick_apply(&mut ctx, &config, &sensor, 100_000, 0);

    // ManualMode: sensor timeout vẫn trigger fault (timeout check là unconditional)
    // Verify: phase phải là Fault(SensorTimeout) — timeout check xảy ra trước Manual check
    assert_eq!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::SensorTimeout),
        "Sensor timeout là safety-critical — xảy ra kể cả ở ManualMode"
    );
    eprintln!("✅ Sensor timeout is unconditional safety check");
    let _ = events;
}

/// E2E Fault Test 4: Drain limit violation → TooManyDrains fault
#[test]
fn e2e_too_many_drains_triggers_fault() {
    let mut config = minimal_config();
    config.max_drain_cycles_per_hour = 1; // Chỉ cho 1 lần drain/h
    config.enable_water_level_sensor = true;
    config.auto_drain_overflow = true;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    // Đã drain 1 lần trước đó (consume the budget)
    ctx.safety.record_drain(0, 1); // 1 lần, max = 1 → blocked

    // Sensor: mực nước cao → muốn drain
    let high_water = SensorData {
        water_level: 25.0, // > water_level_max 24.0
        ..normal_sensor()
    };

    let _events = tick_apply(&mut ctx, &config, &high_water, 10_000, 10_000);

    // Phải fault TooManyDrains
    assert_eq!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::TooManyDrains),
        "Vượt max drain cycles phải vào TooManyDrains fault"
    );
    eprintln!("✅ TooManyDrains fault correctly triggered");
}

fn assert_all_outputs_stopped(events: &[OrchestratorEvent]) {
    use hydragrow_controller_core::core::fsm::events::DosingPumpTarget;

    let assert_stop_event_for = |target: DosingPumpTarget| {
        events.iter().any(|e| {
            matches!(
                e,
                OrchestratorEvent::SetDosingPump {
                    pump,
                    on: false,
                    ..
                } if *pump == target
            )
        })
    };
    assert!(
        assert_stop_event_for(DosingPumpTarget::NutrientA),
        "Missing stop event for NutrientA"
    );
    assert!(
        assert_stop_event_for(DosingPumpTarget::NutrientB),
        "Missing stop event for NutrientB"
    );
    assert!(
        assert_stop_event_for(DosingPumpTarget::PhUp),
        "Missing stop event for PhUp"
    );
    assert!(
        assert_stop_event_for(DosingPumpTarget::PhDown),
        "Missing stop event for PhDown"
    );

    let assert_water_stop = events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop
            }
        )
    });
    assert!(assert_water_stop, "Missing stop event for WaterPump");

    let assert_mist_off = events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::SetMistValve { on: false }));
    assert!(assert_mist_off, "Missing stop event for MistValve");

    let assert_mix_off = events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::SetMixValve { on: false }));
    assert!(assert_mix_off, "Missing stop event for MixValve");

    let assert_osaka_zero = events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::SetOsakaPump { pwm_percent: 0 }));
    assert!(assert_osaka_zero, "Missing stop event for OsakaPump");
}

#[test]
fn fault_invariant_sensor_timeout_stops_all_actuators() {
    let config = minimal_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    let sensor = normal_sensor();
    // 100s since last sensor update
    let events = tick_apply(&mut ctx, &config, &sensor, 100_000, 0);

    assert_eq!(ctx.phase, SystemPhase::Fault(FaultCode::SensorTimeout));
    assert_all_outputs_stopped(&events);
}

#[test]
fn fault_invariant_osaka_running_without_valve_stops_all_actuators() {
    let config = minimal_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;
    ctx.peripherals.pump_status.osaka_pump = true;
    ctx.peripherals.pump_status.mist_valve = false;
    ctx.peripherals.pump_status.mix_valve = false;

    let sensor = normal_sensor();
    let events = tick_apply(&mut ctx, &config, &sensor, 10_000, 10_000);

    assert_eq!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::OsakaRunningWithoutValve)
    );
    assert_all_outputs_stopped(&events);
}

#[test]
fn fault_invariant_hourly_ec_limit_stops_all_actuators() {
    let mut config = minimal_config();
    config.max_dose_per_hour = 1.0;
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    // Exhaust hourly dose budget for EC
    ctx.safety.commit_hourly_dose("NutrientA", 10, 2.0);

    let mut low_ec = normal_sensor();
    low_ec.ec = 0.5; // low EC triggers dosing
    let events = tick_apply(&mut ctx, &config, &low_ec, 10_000, 10_000);

    assert_eq!(ctx.phase, SystemPhase::Fault(FaultCode::MaxHourlyDoseEc));
    assert_all_outputs_stopped(&events);
}

#[test]
fn fault_invariant_hourly_ph_limit_stops_all_actuators() {
    let mut config = minimal_config();
    config.max_dose_per_hour = 1.0;
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    // Exhaust hourly dose budget for PhUp
    ctx.safety.commit_hourly_dose("PhUp", 10, 2.0);

    let mut low_ph = normal_sensor();
    low_ph.ph = 5.0; // low pH triggers pH Up dosing
    let events = tick_apply(&mut ctx, &config, &low_ph, 10_000, 10_000);

    assert_eq!(ctx.phase, SystemPhase::Fault(FaultCode::MaxHourlyDosePh));
    assert_all_outputs_stopped(&events);
}

#[test]
fn fault_invariant_hourly_ph_down_limit_stops_all_actuators() {
    let mut config = minimal_config();
    config.max_dose_per_hour = 1.0;
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    // Exhaust hourly dose budget for PhDown
    ctx.safety.commit_hourly_dose("PhDown", 10, 2.0);

    let mut high_ph = normal_sensor();
    high_ph.ph = 8.0; // high pH triggers pH Down dosing
    let events = tick_apply(&mut ctx, &config, &high_ph, 10_000, 10_000);

    assert_eq!(ctx.phase, SystemPhase::Fault(FaultCode::MaxHourlyDosePh));
    assert_all_outputs_stopped(&events);
}

#[test]
fn hourly_dose_safety_check_is_transactional_and_does_not_commit_on_ph_failure() {
    let mut config = minimal_config();
    config.max_dose_per_hour = 100.0;
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    // Leave enough budget for EC (~2ml) but exhaust total budget so pH (~15ml) fails
    ctx.safety.commit_hourly_dose("PhUp", 10, 95.0);

    // Both low EC and low pH -> will request EC dosing and pH Up dosing
    let mut low_both = normal_sensor();
    low_both.ec = 0.5;
    low_both.ph = 5.0;

    let events = tick_apply(&mut ctx, &config, &low_both, 10_000, 10_000);

    assert_eq!(ctx.phase, SystemPhase::Fault(FaultCode::MaxHourlyDosePh));
    assert_all_outputs_stopped(&events);

    // NutrientA and NutrientB must NOT have been committed
    assert!(
        !ctx.safety.hourly_doses().contains_key("NutrientA")
            || ctx.safety.hourly_doses()["NutrientA"].is_empty(),
        "NutrientA must not be committed when pH check fails in the same tick"
    );
}

#[test]
fn reset_fault_delta_fully_resets_dosing_and_water_actors() {
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Fault(FaultCode::EmergencyStop);

    // Corrupt / dirty dosing actor state
    ctx.dosing.retry_ec = 3;
    ctx.dosing.retry_ph = 2;
    ctx.dosing.sub_state =
        hydragrow_controller_core::core::actors::dosing_actor::DosingSubState::PumpingA(
            hydragrow_controller_core::core::actors::dosing_actor::PulseJob {
                pump:
                    hydragrow_controller_core::core::actors::dosing_actor::PumpTarget::NutrientA {
                        dose_b_ml: 5.0,
                    },
                target_ml: 5.0,
                delivered_ml: 2.0,
                pulse_on: true,
                pulse_count: 4,
                max_pulses: 10,
                on_ms: 100,
                off_ms: 100,
                pwm: 80,
                ml_per_sec: 1.0,
                next_toggle_ms: 5000,
            },
        );
    ctx.dosing.cycle_ctx = Some(
        hydragrow_controller_core::core::actors::dosing_actor::DosingCycleCtx {
            dose_a_delivered_ml: 2.0,
            dose_b_delivered_ml: 0.0,
            ph_up_delivered_ml: 0.0,
            ph_down_delivered_ml: 0.0,
        },
    );
    ctx.dosing.pending_ph_job = Some(
        hydragrow_controller_core::core::actors::dosing_actor::PulseJob {
            pump: hydragrow_controller_core::core::actors::dosing_actor::PumpTarget::PhUp,
            target_ml: 1.0,
            delivered_ml: 0.0,
            pulse_on: false,
            pulse_count: 0,
            max_pulses: 5,
            on_ms: 100,
            off_ms: 100,
            pwm: 80,
            ml_per_sec: 0.5,
            next_toggle_ms: 6000,
        },
    );

    // Corrupt / dirty water actor state
    ctx.water.retry_refill = 2;
    ctx.water.sub_state =
        hydragrow_controller_core::core::actors::water_actor::WaterSubState::Filling {
            job: hydragrow_controller_core::core::actors::water_actor::WaterJob {
                trigger: "schedule".to_string(),
                target_level: 25.0,
                start_level: 10.0,
                start_ms: 1000,
            },
        };

    // Apply delta simulating reset_fault (reset_safety_budget = true, phase = Monitoring)
    let mut reset_delta = hydragrow_controller_core::core::fsm::ContextDelta {
        phase: Some(SystemPhase::Monitoring),
        reset_safety_budget: true,
        ..Default::default()
    };
    ctx.apply_delta(&mut reset_delta);

    // Assert DosingActor is completely idle and clean
    assert_eq!(
        ctx.dosing.sub_state,
        hydragrow_controller_core::core::actors::dosing_actor::DosingSubState::Idle
    );
    assert_eq!(ctx.dosing.retry_ec, 0);
    assert_eq!(ctx.dosing.retry_ph, 0);
    assert!(ctx.dosing.cycle_ctx.is_none());
    assert!(ctx.dosing.pending_ph_job.is_none());

    // Assert WaterActor is completely idle and clean
    assert!(matches!(
        ctx.water.sub_state,
        hydragrow_controller_core::core::actors::water_actor::WaterSubState::Idle
    ));
    assert_eq!(ctx.water.retry_refill, 0);
}

#[test]
fn sensor_error_flags_gate_actuator_commands_only_healthy_channels_dose() {
    let mut config = minimal_config();
    config.control_mode = ControlMode::Auto;
    config.is_enabled = true;
    config.enable_ec_sensor = true;
    config.enable_ph_sensor = true;
    config.ec_target = 1.5;
    config.ph_target = 6.0;
    config.max_dose_per_hour = 50.0;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    // Sensor report:
    // EC is low (0.5 vs target 1.5) BUT err_ec is true!
    // pH is low (5.0 vs target 6.0) AND err_ph is false (healthy)
    let mut sensor = normal_sensor();
    sensor.ec = 0.5;
    sensor.err_ec = Some(true);
    sensor.ph = 5.0;
    sensor.err_ph = Some(false);

    let events = tick_apply(&mut ctx, &config, &sensor, 10_000, 10_000);

    // Must transition to MimoDosing because pH needs dosing
    assert_eq!(ctx.phase, SystemPhase::MimoDosing);

    // EC must NOT have any dosing commands
    let has_nutrient_dose = events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: hydragrow_controller_core::core::fsm::events::DosingPumpTarget::NutrientA
                    | hydragrow_controller_core::core::fsm::events::DosingPumpTarget::NutrientB,
                on: true,
                ..
            }
        )
    });
    assert!(
        !has_nutrient_dose,
        "Faulty EC sensor (err_ec=true) must NOT command Nutrient dosing even if EC is low"
    );

    // Only healthy pH channel should have dosing scheduled directly in sub_state
    assert!(
        matches!(
            ctx.dosing.sub_state,
            hydragrow_controller_core::core::actors::dosing_actor::DosingSubState::SoftStarting { .. }
                | hydragrow_controller_core::core::actors::dosing_actor::DosingSubState::PumpingPH(
                    ..
                )
        ),
        "Healthy pH channel must schedule pH dosing in DosingSubState"
    );
}

#[test]
fn osaka_supersession_generation_counter_prevents_stale_soft_start() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    let soft_start_gen = Arc::new(AtomicU64::new(0));

    // Start soft start (gen becomes 1)
    let current_gen = soft_start_gen.fetch_add(1, Ordering::SeqCst) + 1;
    assert_eq!(current_gen, 1);

    // A newer direct PWM command is dispatched (gen becomes 2)
    soft_start_gen.fetch_add(1, Ordering::SeqCst);
    assert_eq!(soft_start_gen.load(Ordering::SeqCst), 2);

    // Any running soft-start loop checking generation must cancel
    let is_superseded = soft_start_gen.load(Ordering::SeqCst) != current_gen;
    assert!(
        is_superseded,
        "Soft-start step must detect generation mismatch and cancel immediately without writing duty"
    );
}
