use crate::models::PumpStatus;
use std::sync::Mutex;

/// Latest controller status used to reject unsafe manual water commands before
/// they leave the desktop client. The controller remains the authoritative
/// safety layer; this guard prevents an avoidable conflicting request.
#[derive(Default)]
pub struct ValveGuardState {
    latest_status: Mutex<PumpStatus>,
}

impl ValveGuardState {
    pub fn update_status(&self, new_status: PumpStatus) {
        match self.latest_status.lock() {
            Ok(mut status) => *status = new_status,
            Err(_) => eprintln!("Valve guard status lock poisoned; retaining fail-closed state"),
        }
    }

    pub fn check_safety(&self, target_pump: &str, is_on: bool) -> Result<(), String> {
        // Switching a pump off is always permitted, including while degraded.
        if !is_on {
            return Ok(());
        }

        let status = self.latest_status.lock().map_err(|_| {
            "Hệ thống bận, không thể đọc trạng thái van. Đã hủy lệnh để đảm bảo an toàn."
                .to_string()
        })?;

        match target_pump {
            "WATER_PUMP_IN" if status.water_pump_out => Err(
                "⛔ XUNG ĐỘT AN TOÀN: Không thể mở VAN_IN do bơm/van xả đang hoạt động!"
                    .to_string(),
            ),
            "WATER_PUMP_OUT" if status.water_pump_in => {
                Err("⛔ XUNG ĐỘT AN TOÀN: Không thể mở bơm/van xả do VAN_IN đang mở!".to_string())
            }
            "WATER_PUMP_IN" | "WATER_PUMP_OUT" => Ok(()),
            _ => Err(format!("Tên bơm/van không hợp lệ: {target_pump}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_inlet_while_outlet_is_on_is_blocked() {
        let guard = ValveGuardState::default();
        guard.update_status(PumpStatus {
            water_pump_out: true,
            ..Default::default()
        });
        assert!(guard.check_safety("WATER_PUMP_IN", true).is_err());
    }

    #[test]
    fn switching_off_any_water_pump_is_allowed() {
        let guard = ValveGuardState::default();
        guard.update_status(PumpStatus {
            water_pump_in: true,
            water_pump_out: true,
            ..Default::default()
        });
        assert!(guard.check_safety("WATER_PUMP_IN", false).is_ok());
    }

    #[test]
    fn opening_a_water_pump_without_conflict_is_allowed() {
        let guard = ValveGuardState::default();
        assert!(guard.check_safety("WATER_PUMP_IN", true).is_ok());
    }
}
