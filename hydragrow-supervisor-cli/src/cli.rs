use clap::{Parser, Subcommand};
use hydragrow_supervisor_query::{
    CropTargetQuery, DosingHistoryQuery, FsmEventsQuery, HealthTopicsQuery, HestiaSnapshotQuery,
    MAX_DOSING_HISTORY_WINDOW_MINUTES, MAX_FSM_EVENTS_COUNT, MAX_SENSOR_HISTORY_MINUTES,
    SensorHistoryQuery, SupervisorQuery,
};

#[derive(Parser)]
#[command(
    name = "supervisor-cli",
    about = "Read-only investigation CLI for HYDRAGROW devices"
)]
pub struct Cli {
    #[arg(long, env = "SUPERVISOR_CLI_BACKEND_URL")]
    pub backend_url: String,
    #[arg(long, env = "SUPERVISOR_CLI_API_KEY")]
    pub api_key: String,
    /// Print compact single-line JSON instead of pretty-printed.
    #[arg(long, default_value_t = false)]
    pub compact: bool,
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    #[allow(dead_code)]
    pub fn parse_from<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        <Self as Parser>::parse_from(args)
    }
}

#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum Command {
    SensorHistory {
        #[arg(long)]
        device_id: String,
        #[arg(long, default_value_t = MAX_SENSOR_HISTORY_MINUTES)]
        minutes: u32,
    },
    DosingHistory {
        #[arg(long)]
        device_id: String,
        #[arg(long, default_value_t = MAX_DOSING_HISTORY_WINDOW_MINUTES)]
        minutes: u32,
    },
    FsmEvents {
        #[arg(long)]
        device_id: String,
        #[arg(long, default_value_t = MAX_FSM_EVENTS_COUNT as u32)]
        limit: u32,
    },
    HealthTopics {
        #[arg(long)]
        device_id: String,
    },
    HestiaSnapshot {
        #[arg(long)]
        device_id: String,
    },
    CropTarget {
        #[arg(long)]
        device_id: String,
    },
}

impl From<Command> for SupervisorQuery {
    fn from(cmd: Command) -> Self {
        match cmd {
            Command::SensorHistory { device_id, minutes } => {
                SupervisorQuery::SensorHistory(SensorHistoryQuery { device_id, minutes })
            }
            Command::DosingHistory { device_id, minutes } => {
                SupervisorQuery::DosingHistory(DosingHistoryQuery { device_id, minutes })
            }
            Command::FsmEvents { device_id, limit } => {
                SupervisorQuery::FsmEvents(FsmEventsQuery { device_id, limit })
            }
            Command::HealthTopics { device_id } => {
                SupervisorQuery::HealthTopics(HealthTopicsQuery { device_id })
            }
            Command::HestiaSnapshot { device_id } => {
                SupervisorQuery::HestiaSnapshot(HestiaSnapshotQuery { device_id })
            }
            Command::CropTarget { device_id } => {
                SupervisorQuery::CropTarget(CropTargetQuery { device_id })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_supervisor_query::SupervisorQuery;

    #[test]
    fn sensor_history_command_converts_with_given_minutes() {
        let cmd = Command::SensorHistory {
            device_id: "d1".to_string(),
            minutes: 15,
        };
        let query: SupervisorQuery = cmd.into();
        match query {
            SupervisorQuery::SensorHistory(q) => {
                assert_eq!(q.device_id, "d1");
                assert_eq!(q.minutes, 15);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn health_topics_command_converts() {
        let cmd = Command::HealthTopics {
            device_id: "d1".to_string(),
        };
        let query: SupervisorQuery = cmd.into();
        assert_eq!(query.tag(), "health_topics");
        assert_eq!(query.device_id(), "d1");
    }

    #[test]
    fn cli_parses_sensor_history_subcommand_from_args() {
        let cli = Cli::parse_from([
            "supervisor-cli",
            "--backend-url",
            "https://example.com",
            "--api-key",
            "svc_test",
            "sensor-history",
            "--device-id",
            "d1",
            "--minutes",
            "20",
        ]);
        match cli.command {
            Command::SensorHistory { device_id, minutes } => {
                assert_eq!(device_id, "d1");
                assert_eq!(minutes, 20);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn cli_applies_default_minutes_when_omitted() {
        let cli = Cli::parse_from([
            "supervisor-cli",
            "--backend-url",
            "https://example.com",
            "--api-key",
            "svc_test",
            "sensor-history",
            "--device-id",
            "d1",
        ]);
        match cli.command {
            Command::SensorHistory { minutes, .. } => {
                assert_eq!(
                    minutes,
                    hydragrow_supervisor_query::MAX_SENSOR_HISTORY_MINUTES
                )
            }
            _ => panic!("wrong variant"),
        }
    }
}
