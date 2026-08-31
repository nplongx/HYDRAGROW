def rewrite(path, search, replace):
    with open(path, "r") as f:
        content = f.read()
    content = content.replace(search, replace)
    with open(path, "w") as f:
        f.write(content)

rewrite("hydragrow-simulator/src/faults/injector.rs",
"""            match fault {
                FaultEventKind::PumpStuckOn { pump } => {
                    if pump == "PUMP_A" {
                        hw.pump_a.on = true;
                    }
                }
                FaultEventKind::PumpStuckOff { pump } => {
                    if pump == "PUMP_A" {
                        hw.pump_a.on = false;
                    }
                }
                _ => {} // Sensor faults handled separately
            }""",
"""            match fault {
                FaultEventKind::PumpStuckOn { pump } if pump == "PUMP_A" => {
                    hw.pump_a.on = true;
                }
                FaultEventKind::PumpStuckOff { pump } if pump == "PUMP_A" => {
                    hw.pump_a.on = false;
                }
                _ => {} // Sensor faults handled separately
            }""")
