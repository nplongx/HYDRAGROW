#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAgitechTopic<'a> {
    pub device_id: &'a str,
    pub suffix: &'a str,
}

pub const AGITECH_PREFIX: &str = "AGITECH";

#[inline]
fn build_topic(device_id: &str, suffix: &str) -> String {
    format!("{AGITECH_PREFIX}/{device_id}/{suffix}")
}

pub fn topic_system_log(device_id: &str) -> String {
    build_topic(device_id, "system_log")
}

pub fn topic_sensors(device_id: &str) -> String {
    build_topic(device_id, "sensors")
}

pub fn topic_status(device_id: &str) -> String {
    build_topic(device_id, "status")
}

pub fn topic_controller_command(device_id: &str) -> String {
    build_topic(device_id, "controller/command")
}

pub fn topic_controller_config(device_id: &str) -> String {
    build_topic(device_id, "controller/config")
}

pub fn topic_sensor_status(device_id: &str) -> String {
    build_topic(device_id, "sensor/status")
}

pub fn topic_controller_status(device_id: &str) -> String {
    build_topic(device_id, "controller/status")
}

pub fn topic_sensor_command(device_id: &str) -> String {
    build_topic(device_id, "sensor/command")
}

pub fn parse_agitech_topic(topic: &str) -> Option<ParsedAgitechTopic<'_>> {
    let mut parts = topic.splitn(3, '/');
    let prefix = parts.next()?;
    let device_id = parts.next()?;
    let suffix = parts.next()?;

    if prefix != AGITECH_PREFIX || device_id.is_empty() || suffix.is_empty() {
        return None;
    }

    Some(ParsedAgitechTopic { device_id, suffix })
}

pub fn topic_fsm_state(device_id: &str) -> String {
    build_topic(device_id, "fsm/state")
}

pub fn topic_fsm_events(device_id: &str) -> String {
    build_topic(device_id, "fsm/events")
}

pub fn topic_calibration(device_id: &str) -> String {
    build_topic(device_id, "calibration")
}

pub fn topic_dosing_report(device_id: &str) -> String {
    build_topic(device_id, "dosing_report")
}
