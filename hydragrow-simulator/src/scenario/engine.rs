use super::format::{FaultEventKind, Scenario};

pub struct ScenarioEngine {
    scenario: Scenario,
    next_fault: usize,
}

impl ScenarioEngine {
    pub fn new(mut scenario: Scenario) -> Self {
        scenario.faults.sort_by_key(|fault| fault.at_ms);
        Self {
            scenario,
            next_fault: 0,
        }
    }

    pub fn activate_between(&mut self, previous_ms: u64, current_ms: u64) -> Vec<FaultEventKind> {
        let mut out = Vec::new();
        while let Some(fault) = self.scenario.faults.get(self.next_fault) {
            if fault.at_ms <= previous_ms {
                self.next_fault += 1;
                continue;
            }
            if fault.at_ms > current_ms {
                break;
            }
            out.push(fault.kind.clone());
            self.next_fault += 1;
        }
        out
    }
}
