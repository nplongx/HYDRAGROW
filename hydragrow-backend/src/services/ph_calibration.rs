use serde::{Deserialize, Serialize};
use thiserror::Error;

const PH_EPSILON: f64 = 1e-3;
const MIN_ABS_SLOPE: f64 = 1e-3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationMode {
    TwoPoint,
    ThreePoint,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CalibrationSample {
    pub ph: f64,
    pub voltage_mv: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhCalibrationResult {
    /// Điện áp chuẩn tại pH=7 theo format firmware (`ph_v7`).
    pub ph_v7: f32,
    /// Điện áp chuẩn tại pH=4 theo format firmware (`ph_v4`).
    pub ph_v4: f32,
    /// Hệ số góc của mô hình pH = slope * V + intercept.
    pub slope: f32,
    /// Hệ số chặn của mô hình pH = slope * V + intercept.
    pub intercept: f32,
    /// Hệ số xác định (0..1, có thể âm nếu fit tệ).
    pub r2: f32,
    /// Sai số tuyệt đối lớn nhất trên các điểm dùng để fit.
    pub max_abs_error: f32,
}

#[derive(Debug, Error, PartialEq)]
pub enum PhCalibrationError {
    #[error("missing required calibration point pH {0}")]
    MissingPoint(i32),
    #[error("invalid slope {0:.6}; probe may be reversed or unstable")]
    InvalidSlope(f64),
    #[error("degenerate dataset; cannot fit linear model")]
    DegenerateDataset,
}

pub fn calibrate_ph(
    mode: CalibrationMode,
    samples: &[CalibrationSample],
) -> Result<PhCalibrationResult, PhCalibrationError> {
    let fit_points: Vec<CalibrationSample> = match mode {
        CalibrationMode::TwoPoint => vec![
            find_sample(samples, 7.0).ok_or(PhCalibrationError::MissingPoint(7))?,
            find_sample(samples, 4.0).ok_or(PhCalibrationError::MissingPoint(4))?,
        ],
        CalibrationMode::ThreePoint => vec![
            find_sample(samples, 7.0).ok_or(PhCalibrationError::MissingPoint(7))?,
            find_sample(samples, 4.0).ok_or(PhCalibrationError::MissingPoint(4))?,
            find_sample(samples, 10.0).ok_or(PhCalibrationError::MissingPoint(10))?,
        ],
    };

    let (slope, intercept) = match mode {
        CalibrationMode::TwoPoint => fit_two_point(&fit_points[0], &fit_points[1])?,
        CalibrationMode::ThreePoint => fit_least_squares(&fit_points)?,
    };

    validate_slope(slope)?;

    let ph_v7 = (7.0 - intercept) / slope;
    let ph_v4 = (4.0 - intercept) / slope;

    let (r2, max_abs_error) = quality_metrics(slope, intercept, &fit_points);

    Ok(PhCalibrationResult {
        ph_v7: ph_v7 as f32,
        ph_v4: ph_v4 as f32,
        slope: slope as f32,
        intercept: intercept as f32,
        r2: r2 as f32,
        max_abs_error: max_abs_error as f32,
    })
}

fn find_sample(samples: &[CalibrationSample], target_ph: f64) -> Option<CalibrationSample> {
    samples
        .iter()
        .copied()
        .find(|s| (s.ph - target_ph).abs() < PH_EPSILON)
}

fn fit_two_point(
    p1: &CalibrationSample,
    p2: &CalibrationSample,
) -> Result<(f64, f64), PhCalibrationError> {
    let dx = p2.voltage_mv - p1.voltage_mv;
    if dx.abs() < f64::EPSILON {
        return Err(PhCalibrationError::DegenerateDataset);
    }

    let slope = (p2.ph - p1.ph) / dx;
    let intercept = p1.ph - slope * p1.voltage_mv;
    Ok((slope, intercept))
}

fn fit_least_squares(points: &[CalibrationSample]) -> Result<(f64, f64), PhCalibrationError> {
    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|p| p.voltage_mv).sum();
    let sum_y: f64 = points.iter().map(|p| p.ph).sum();
    let mean_x = sum_x / n;
    let mean_y = sum_y / n;

    let ss_xx: f64 = points
        .iter()
        .map(|p| {
            let dx = p.voltage_mv - mean_x;
            dx * dx
        })
        .sum();

    if ss_xx.abs() < f64::EPSILON {
        return Err(PhCalibrationError::DegenerateDataset);
    }

    let ss_xy: f64 = points
        .iter()
        .map(|p| (p.voltage_mv - mean_x) * (p.ph - mean_y))
        .sum();

    let slope = ss_xy / ss_xx;
    let intercept = mean_y - slope * mean_x;
    Ok((slope, intercept))
}

fn validate_slope(slope: f64) -> Result<(), PhCalibrationError> {
    if slope >= 0.0 || slope.abs() < MIN_ABS_SLOPE {
        return Err(PhCalibrationError::InvalidSlope(slope));
    }

    Ok(())
}

fn quality_metrics(slope: f64, intercept: f64, points: &[CalibrationSample]) -> (f64, f64) {
    let mean_y = points.iter().map(|p| p.ph).sum::<f64>() / points.len() as f64;

    let (ss_res, max_abs_error) = points.iter().fold((0.0_f64, 0.0_f64), |(sse, maxe), p| {
        let y_hat = slope * p.voltage_mv + intercept;
        let err = p.ph - y_hat;
        (sse + err * err, maxe.max(err.abs()))
    });

    let ss_tot: f64 = points
        .iter()
        .map(|p| {
            let dy = p.ph - mean_y;
            dy * dy
        })
        .sum();

    let r2 = if ss_tot.abs() < f64::EPSILON {
        1.0
    } else {
        1.0 - (ss_res / ss_tot)
    };

    (r2, max_abs_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32, tol: f32) {
        assert!(
            (actual - expected).abs() <= tol,
            "actual={actual}, expected={expected}, tol={tol}"
        );
    }

    #[test]
    fn ideal_data_three_point_returns_expected_calibration() {
        let samples = vec![
            CalibrationSample {
                ph: 7.0,
                voltage_mv: 1650.0,
            },
            CalibrationSample {
                ph: 4.0,
                voltage_mv: 1846.4,
            },
            CalibrationSample {
                ph: 10.0,
                voltage_mv: 1453.6,
            },
        ];

        let result = calibrate_ph(CalibrationMode::ThreePoint, &samples).unwrap();

        assert_close(result.ph_v7, 1650.0, 1e-2);
        assert_close(result.ph_v4, 1846.4, 1e-2);
        assert_close(result.r2, 1.0, 1e-6);
        assert_close(result.max_abs_error, 0.0, 1e-6);
    }

    #[test]
    fn lightly_noisy_data_three_point_keeps_good_metrics() {
        let samples = vec![
            CalibrationSample {
                ph: 7.0,
                voltage_mv: 1650.8,
            },
            CalibrationSample {
                ph: 4.0,
                voltage_mv: 1844.9,
            },
            CalibrationSample {
                ph: 10.0,
                voltage_mv: 1455.3,
            },
        ];

        let result = calibrate_ph(CalibrationMode::ThreePoint, &samples).unwrap();

        assert!(result.r2 > 0.999);
        assert!(result.max_abs_error < 0.05);
        assert!(result.ph_v4 > result.ph_v7);
    }

    #[test]
    fn outlier_point_degrades_quality_metrics() {
        let samples = vec![
            CalibrationSample {
                ph: 7.0,
                voltage_mv: 1650.0,
            },
            CalibrationSample {
                ph: 4.0,
                voltage_mv: 1846.4,
            },
            CalibrationSample {
                ph: 10.0,
                voltage_mv: 1600.0,
            },
        ];

        let result = calibrate_ph(CalibrationMode::ThreePoint, &samples).unwrap();

        assert!(result.r2 < 0.95);
        assert!(result.max_abs_error > 1.0);
    }

    #[test]
    fn invalid_small_or_reversed_slope_returns_error() {
        let too_small = vec![
            CalibrationSample {
                ph: 7.0,
                voltage_mv: 1650.0,
            },
            CalibrationSample {
                ph: 4.0,
                voltage_mv: 7650.0,
            },
        ];

        let err_small = calibrate_ph(CalibrationMode::TwoPoint, &too_small).unwrap_err();
        assert!(matches!(err_small, PhCalibrationError::InvalidSlope(_)));

        let reversed = vec![
            CalibrationSample {
                ph: 7.0,
                voltage_mv: 1650.0,
            },
            CalibrationSample {
                ph: 4.0,
                voltage_mv: 1500.0,
            },
        ];

        let err_reversed = calibrate_ph(CalibrationMode::TwoPoint, &reversed).unwrap_err();
        assert!(matches!(err_reversed, PhCalibrationError::InvalidSlope(_)));
    }
}
