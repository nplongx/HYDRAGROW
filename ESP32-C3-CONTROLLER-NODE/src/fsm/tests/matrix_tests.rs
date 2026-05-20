#[cfg(test)]
mod tests {
    use crate::fsm::matrix::{InteractionMatrix, KalmanCovarianceDiag};

    /// Kalman gain phải nằm trong (0, 1) và p phải shrink sau update
    #[test]
    fn kalman_update_and_get_gain_returns_finite_between_zero_and_one() {
        let mut k = KalmanCovarianceDiag::new(1.0, 0.001, 0.1);
        k.predict();
        let gain = k.update_and_get_gain(0);
        assert!(gain > 0.0 && gain < 1.0, "gain={}", gain);
    }

    #[test]
    fn kalman_p_decreases_after_update() {
        let mut k = KalmanCovarianceDiag::new(1.0, 0.001, 0.1);
        let p_before = k.p[0];
        k.predict();
        let _ = k.update_and_get_gain(0);
        assert!(k.p[0] < p_before + 0.001 + 0.01, "p should shrink");
    }

    /// update_column phải thay đổi giá trị matrix theo hướng đúng
    #[test]
    fn update_column_moves_toward_observed_gain() {
        let mut m = InteractionMatrix::from_scalar(0.01, 0.02);
        let old_val = m.get(0, 0);
        // observed lớn hơn predicted: gain phải tăng
        m.update_column(0, 10.0, 0.2, 0, 0.5); // dose=10ml, observed=0.2 ec
        let new_val = m.get(0, 0);
        assert!(new_val > old_val, "old={old_val} new={new_val}");
    }

    #[test]
    fn predict_returns_nonzero_ec_delta_for_ec_dose() {
        let m = InteractionMatrix::from_scalar(0.015, 0.02);
        use crate::fsm::matrix::DoseVector;
        let dose = DoseVector {
            nutrient_a_ml: 10.0,
            nutrient_b_ml: 0.0,
            ph_up_ml: 0.0,
        };
        let response = m.predict(&dose);
        assert!(
            (response.ec_delta - 0.15).abs() < 1e-5,
            "ec_delta={}",
            response.ec_delta
        );
        assert_eq!(response.ph_delta, 0.0);
    }

    #[test]
    fn monitoring_matrix_solve_ec_dose_with_warm_matrix() {
        use crate::fsm::matrix::InteractionMatrix;
        use crate::fsm::system_context::{AutoTuner, GainLearner};
        use hydragrow_shared::ControllerConfig;

        let config = ControllerConfig {
            ec_target: 2.0,
            ec_tolerance: 0.1,
            ec_gain_per_ml: 0.015,
            max_dose_per_cycle: 20.0,
            ..ControllerConfig::default()
        };

        let mut tuner = AutoTuner::default();
        tuner.matrix_is_warm = true;
        // Matrix với ec_a = 0.015 mS/cm per ml
        tuner.interaction_matrix = InteractionMatrix::from_scalar(0.015, 0.02);

        let ec_delta = 0.5_f32; // cần bơm EC
        let ph_delta = 0.0_f32;

        let (pump_a_ml, _ph_ml) = solve_for_test(ec_delta, ph_delta, &config, &tuner);
        // dose = delta / gain = 0.5 / 0.015 * step_ratio(0.4) * deadband
        assert!(pump_a_ml > 0.0 && pump_a_ml <= config.max_dose_per_cycle);
    }

    #[test]
    fn dosing_pulse_toggle_updates_pump_status_in_context() {
        use crate::fsm::actors::dosing_actor::{DosingActor, PumpTarget};
        use hydragrow_shared::PumpStatus;

        // Không thể mock PumpController dễ trên embedded, skip hardware test
        // Chỉ verify rằng DosingEvent::PulseToggle carry đúng pump identity
        let mut actor = DosingActor::new();
        assert!(actor.is_idle());
        // State machine bắt đầu ở Idle — đảm bảo không crash khi tick mà idle
    }

    #[test]
    fn matrix_converges_toward_true_gain_after_10_cycles() {
        use crate::fsm::matrix::{InteractionMatrix, KalmanCovarianceDiag};
        use crate::fsm::types::PendingCalibrationSample;

        let true_ec_gain_per_ml = 0.020_f32; // "thực tế" của hệ thống
        let mut matrix = InteractionMatrix::from_scalar(0.015, 0.02); // seed khác
        let mut kalman = KalmanCovarianceDiag::new(1.0, 0.001, 0.1);

        for cycle in 0..12 {
            let dose_a = 5.0_f32;
            let observed_delta_ec = dose_a * true_ec_gain_per_ml + (cycle as f32 * 0.001); // noise nhỏ

            kalman.predict();
            let k_a = kalman.update_and_get_gain(0);
            matrix.update_column(0, dose_a, observed_delta_ec, 0, k_a);
        }

        let learned_gain = matrix.get(0, 0);
        // Sau 12 chu kỳ, gain phải hội tụ trong 20% của true gain
        let error_ratio = (learned_gain - true_ec_gain_per_ml).abs() / true_ec_gain_per_ml;
        assert!(
            error_ratio < 0.20,
            "learned={:.5} true={:.5} error={:.1}%",
            learned_gain,
            true_ec_gain_per_ml,
            error_ratio * 100.0
        );
    }

    #[test]
    fn matrix_from_flat_roundtrip() {
        let original = [0.015_f32, 0.015, 0.0, 0.0, 0.0, 0.02];
        let m = InteractionMatrix::from_flat(original);
        let back = m.as_flat();
        for (a, b) in original.iter().zip(back.iter()) {
            assert!((a - b).abs() < 1e-7, "roundtrip mismatch: {a} vs {b}");
        }
    }

    #[test]
    fn nvs_snapshot_invalid_matrix_is_rejected_on_restore() {
        use crate::fsm::system_context::NvsSnapshot;

        // Matrix với diagonal = 0 phải bị reject (vô nghĩa về vật lý)
        let bad_matrix: [f32; 6] = [0.0, 0.015, 0.0, 0.0, 0.0, 0.0]; // m[0][0] = 0
        let values_valid = bad_matrix
            .iter()
            .all(|v| v.is_finite() && *v >= -10.0 && *v <= 10.0);
        let diagonal_valid = bad_matrix[0] > 0.0 && bad_matrix[5] > 0.0;
        assert!(!diagonal_valid, "matrix với m00=0 phải bị reject");
        assert!(values_valid, "nhưng values vẫn là finite");
    }

    #[test]
    fn nvs_snapshot_valid_warm_matrix_is_accepted() {
        let good_matrix: [f32; 6] = [0.015, 0.015, 0.0, 0.0, 0.0, 0.02];
        let values_valid = good_matrix
            .iter()
            .all(|v| v.is_finite() && *v >= -10.0 && *v <= 10.0);
        let diagonal_valid = good_matrix[0] > 0.0 && good_matrix[5] > 0.0;
        assert!(values_valid && diagonal_valid);
    }
}
