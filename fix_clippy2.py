import re

def rewrite(path, search, replace):
    with open(path, "r") as f:
        content = f.read()
    content = content.replace(search, replace)
    with open(path, "w") as f:
        f.write(content)

rewrite("hydragrow-simulator/tests/mqtt_integration.rs",
"""                if let Ok(n) = pub_conn.read(&mut buf) {
                    if n > 0 {
                        let _ = sub_conn.write_all(&buf[..n]);
                        let _ = sub_conn.flush();
                    }
                }""",
"""                if let Ok(n) = pub_conn.read(&mut buf) {
                    #[allow(clippy::collapsible_if)]
                    if n > 0 {
                        let _ = sub_conn.write_all(&buf[..n]);
                        let _ = sub_conn.flush();
                    }
                }""")

rewrite("hydragrow-simulator/src/plant/tank.rs",
"""        let mut config = ControllerConfig::default();
        config.ec_gain_per_ml = 0.5;
        config.pump_a_capacity_ml_per_sec = 2.0;

        let mut hw = VirtualHardwareState::default();
        hw.pump_a = VirtualPump { on: true, pwm: 100 };""",
"""        let config = ControllerConfig {
            ec_gain_per_ml: 0.5,
            pump_a_capacity_ml_per_sec: 2.0,
            ..Default::default()
        };

        let hw = VirtualHardwareState {
            pump_a: VirtualPump { on: true, pwm: 100 },
            ..Default::default()
        };""")

rewrite("hydragrow-simulator/tests/snapshot_dosing.rs",
"""    let mut config = ControllerConfig::default();
    config.ec_gain_per_ml = 0.5;
    config.pump_a_capacity_ml_per_sec = 2.0;""",
"""    let config = ControllerConfig {
        ec_gain_per_ml: 0.5,
        pump_a_capacity_ml_per_sec: 2.0,
        ..Default::default()
    };""")
