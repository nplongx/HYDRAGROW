use crate::models::config::{DosingCalibration, SafetyConfig};

#[derive(Debug, PartialEq, Eq)]
pub enum SafetyDataError {
    Missing,
    Database,
    Invalid,
}

impl SafetyDataError {
    pub fn reason_code(&self) -> &'static str {
        match self {
            Self::Missing => "SAFETY_DATA_MISSING",
            Self::Database => "SAFETY_DATA_DB_ERROR",
            Self::Invalid => "SAFETY_DATA_INVALID",
        }
    }
}

pub fn classify_safety_config(
    result: Result<Option<SafetyConfig>, sqlx::Error>,
) -> Result<SafetyConfig, SafetyDataError> {
    let config = result
        .map_err(|_| SafetyDataError::Database)?
        .ok_or(SafetyDataError::Missing)?;
    if !config.max_dose_per_cycle.is_finite()
        || config.max_dose_per_cycle <= 0.0
        || !config.max_dose_per_hour.is_finite()
        || config.max_dose_per_hour <= 0.0
        || config.cooldown_sec < 0
    {
        return Err(SafetyDataError::Invalid);
    }
    Ok(config)
}

pub fn classify_calibration(
    result: Result<Option<DosingCalibration>, sqlx::Error>,
) -> Result<DosingCalibration, &'static str> {
    let calibration = result
        .map_err(|_| "DOSING_CALIBRATION_DB_ERROR")?
        .ok_or("DOSING_CALIBRATION_MISSING")?;
    let capacities = [
        calibration.pump_a_capacity_ml_per_sec,
        calibration.pump_b_capacity_ml_per_sec,
        calibration.pump_ph_up_capacity_ml_per_sec,
        calibration.pump_ph_down_capacity_ml_per_sec,
    ];
    if capacities.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err("DOSING_CALIBRATION_INVALID");
    }
    Ok(calibration)
}

pub async fn load_safety_config(
    pool: &sqlx::PgPool,
    device_id: &str,
) -> Result<SafetyConfig, SafetyDataError> {
    classify_safety_config(
        sqlx::query_as::<_, SafetyConfig>("SELECT * FROM safety_config WHERE device_id = $1")
            .bind(device_id)
            .fetch_optional(pool)
            .await,
    )
}

pub async fn load_dosing_calibration(
    pool: &sqlx::PgPool,
    device_id: &str,
) -> Result<DosingCalibration, SafetyDataError> {
    classify_calibration(
        sqlx::query_as::<_, DosingCalibration>(
            "SELECT * FROM dosing_calibration WHERE device_id = $1",
        )
        .bind(device_id)
        .fetch_optional(pool)
        .await,
    )
    .map_err(|reason| match reason {
        "DOSING_CALIBRATION_MISSING" => SafetyDataError::Missing,
        "DOSING_CALIBRATION_INVALID" => SafetyDataError::Invalid,
        _ => SafetyDataError::Database,
    })
}

pub fn classify_history(
    result: Result<Vec<(u64, f32)>, sqlx::Error>,
) -> Result<Vec<(u64, f32)>, &'static str> {
    result.map_err(|_| "DOSING_HISTORY_DB_ERROR")
}

pub fn classify_last_dose(
    result: Result<Option<u64>, sqlx::Error>,
) -> Result<Option<u64>, &'static str> {
    result.map_err(|_| "LAST_DOSE_DB_ERROR")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn safety() -> SafetyConfig {
        SafetyConfig {
            device_id: "d1".into(),
            last_updated: Utc::now(),
            ..Default::default()
        }
    }

    fn calibration() -> DosingCalibration {
        DosingCalibration {
            device_id: "d1".into(),
            last_calibrated: Utc::now(),
            ..Default::default()
        }
    }

    #[test]
    fn safety_config_classification_distinguishes_missing_db_error_and_invalid() {
        assert_eq!(
            classify_safety_config(Ok(None)).unwrap_err(),
            SafetyDataError::Missing
        );
        assert_eq!(
            classify_safety_config(Err(sqlx::Error::RowNotFound)).unwrap_err(),
            SafetyDataError::Database
        );
        let mut invalid = safety();
        invalid.max_dose_per_cycle = 0.0;
        assert_eq!(
            classify_safety_config(Ok(Some(invalid))).unwrap_err(),
            SafetyDataError::Invalid
        );
        assert!(classify_safety_config(Ok(Some(safety()))).is_ok());
    }

    #[test]
    fn calibration_classification_distinguishes_missing_db_error_and_invalid() {
        assert!(matches!(
            classify_calibration(Ok(None)),
            Err("DOSING_CALIBRATION_MISSING")
        ));
        assert!(matches!(
            classify_calibration(Err(sqlx::Error::RowNotFound)),
            Err("DOSING_CALIBRATION_DB_ERROR")
        ));
        let mut invalid = calibration();
        invalid.pump_a_capacity_ml_per_sec = 0.0;
        assert!(matches!(
            classify_calibration(Ok(Some(invalid))),
            Err("DOSING_CALIBRATION_INVALID")
        ));
        assert!(classify_calibration(Ok(Some(calibration()))).is_ok());
    }

    #[test]
    fn empty_history_and_no_previous_dose_are_available_but_query_errors_are_not() {
        assert_eq!(classify_history(Ok(vec![])).unwrap(), vec![]);
        assert_eq!(
            classify_history(Err(sqlx::Error::RowNotFound)).unwrap_err(),
            "DOSING_HISTORY_DB_ERROR"
        );
        assert_eq!(classify_last_dose(Ok(None)).unwrap(), None);
        assert_eq!(
            classify_last_dose(Err(sqlx::Error::RowNotFound)).unwrap_err(),
            "LAST_DOSE_DB_ERROR"
        );
    }

    #[test]
    fn reason_codes_are_stable_machine_values() {
        assert_eq!(
            SafetyDataError::Missing.reason_code(),
            "SAFETY_DATA_MISSING"
        );
        assert_eq!(
            SafetyDataError::Database.reason_code(),
            "SAFETY_DATA_DB_ERROR"
        );
        assert_eq!(
            SafetyDataError::Invalid.reason_code(),
            "SAFETY_DATA_INVALID"
        );
    }
}
