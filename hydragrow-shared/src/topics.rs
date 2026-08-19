#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAgitechTopic<'a> {
    pub device_id: &'a str,
    pub suffix: &'a str,
}

pub struct MqttTopics;

impl MqttTopics {
    pub fn controller_config(device_id: &str) -> String {
        format!("AGITECH/{}/controller/config", device_id)
    }
    pub fn controller_recipe(device_id: &str) -> String {
        format!("AGITECH/{}/controller/recipe", device_id)
    }
    pub fn recipe_events(device_id: &str) -> String {
        format!("AGITECH/{}/recipe/events", device_id)
    }
    pub fn controller_command(device_id: &str) -> String {
        format!("AGITECH/{}/controller/command", device_id)
    }
    pub fn sensors(device_id: &str) -> String {
        format!("AGITECH/{}/sensors", device_id)
    }
    pub fn fsm_state(device_id: &str) -> String {
        format!("AGITECH/{}/fsm/state", device_id)
    }
    pub fn fsm_events(device_id: &str) -> String {
        format!("AGITECH/{}/fsm/events", device_id)
    }
    pub fn recipe_set(device_id: &str) -> String {
        format!("AGITECH/{}/recipe/set", device_id)
    }
    pub fn recipe_clear(device_id: &str) -> String {
        format!("AGITECH/{}/recipe/clear", device_id)
    }
    pub fn recipe_status(device_id: &str) -> String {
        format!("AGITECH/{}/recipe/status", device_id)
    }
    pub fn dosing_report(device_id: &str) -> String {
        format!("AGITECH/{}/dosing_report", device_id)
    }
    pub fn controller_status(device_id: &str) -> String {
        format!("AGITECH/{}/controller/status", device_id)
    }
    pub fn device_status(device_id: &str) -> String {
        format!("AGITECH/{}/status", device_id)
    }
    pub fn sensors_config(device_id: &str) -> String {
        format!("AGITECH/{}/sensors/config", device_id)
    }
    pub fn system_log(device_id: &str) -> String {
        format!("AGITECH/{}/system_log", device_id)
    }
    pub fn parse(topic: &str) -> Option<(String, String)> {
        let rest = topic.strip_prefix("AGITECH/")?;
        let slash = rest.find('/')?;
        Some((rest[..slash].to_string(), rest[slash..].to_string()))
    }
}

pub const AGITECH_PREFIX: &str = "AGITECH";
pub fn topic_system_log(device_id: &str) -> String {
    MqttTopics::system_log(device_id)
}
pub fn topic_sensors(device_id: &str) -> String {
    MqttTopics::sensors(device_id)
}
pub fn topic_status(device_id: &str) -> String {
    MqttTopics::device_status(device_id)
}
pub fn topic_controller_command(device_id: &str) -> String {
    MqttTopics::controller_command(device_id)
}
pub fn topic_controller_config(device_id: &str) -> String {
    MqttTopics::controller_config(device_id)
}
pub fn topic_controller_recipe(device_id: &str) -> String {
    MqttTopics::controller_recipe(device_id)
}
pub fn topic_recipe_events(device_id: &str) -> String {
    MqttTopics::recipe_events(device_id)
}
pub fn topic_sensor_status(device_id: &str) -> String {
    format!("AGITECH/{}/sensor/status", device_id)
}
pub fn topic_controller_status(device_id: &str) -> String {
    MqttTopics::controller_status(device_id)
}
pub fn topic_sensor_command(device_id: &str) -> String {
    format!("AGITECH/{}/sensor/command", device_id)
}
pub fn topic_fsm_state(device_id: &str) -> String {
    MqttTopics::fsm_state(device_id)
}
pub fn topic_fsm_events(device_id: &str) -> String {
    MqttTopics::fsm_events(device_id)
}
pub fn topic_recipe_set(device_id: &str) -> String {
    MqttTopics::recipe_set(device_id)
}
pub fn topic_recipe_clear(device_id: &str) -> String {
    MqttTopics::recipe_clear(device_id)
}
pub fn topic_recipe_status(device_id: &str) -> String {
    MqttTopics::recipe_status(device_id)
}
pub fn topic_calibration(device_id: &str) -> String {
    format!("AGITECH/{}/calibration", device_id)
}
pub fn topic_dosing_report(device_id: &str) -> String {
    MqttTopics::dosing_report(device_id)
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

pub fn topic_fsm_transition(device_id: &str) -> String {
    format!("AGITECH/{}/fsm/transition", device_id)
}

pub fn topic_dosing_cycle(device_id: &str) -> String {
    format!("AGITECH/{}/dosing_cycle", device_id)
}

pub fn topic_water_cycle(device_id: &str) -> String {
    format!("AGITECH/{}/water_cycle", device_id)
}

pub fn topic_health_snapshot(device_id: &str) -> String {
    format!("AGITECH/{}/controller/status", device_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_topics_use_recipe_suffixes() {
        let device_id = "device-01";

        assert_eq!(
            MqttTopics::recipe_set(device_id),
            "AGITECH/device-01/recipe/set"
        );
        assert_eq!(
            MqttTopics::recipe_clear(device_id),
            "AGITECH/device-01/recipe/clear"
        );
        assert_eq!(
            MqttTopics::recipe_status(device_id),
            "AGITECH/device-01/recipe/status"
        );
        assert_eq!(
            MqttTopics::recipe_events(device_id),
            "AGITECH/device-01/recipe/events"
        );
    }

    #[test]
    fn recipe_topic_wrappers_match_mqtt_topics() {
        let device_id = "device-01";

        assert_eq!(
            topic_recipe_set(device_id),
            MqttTopics::recipe_set(device_id)
        );
        assert_eq!(
            topic_recipe_clear(device_id),
            MqttTopics::recipe_clear(device_id)
        );
        assert_eq!(
            topic_recipe_status(device_id),
            MqttTopics::recipe_status(device_id)
        );
        assert_eq!(
            topic_recipe_events(device_id),
            MqttTopics::recipe_events(device_id)
        );
    }

    #[test]
    fn parse_agitech_topic_accepts_recipe_topics() {
        let topic = topic_recipe_events("device-01");
        let parsed = parse_agitech_topic(&topic).unwrap();

        assert_eq!(parsed.device_id, "device-01");
        assert_eq!(parsed.suffix, "recipe/events");
    }
}
