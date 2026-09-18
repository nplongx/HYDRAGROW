//! Virtual controller lifecycle around the existing controller-core FSM.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerLifecycle {
    Boot,
    Connecting,
    Connected,
    Running,
    Degraded,
    Offline,
}

pub struct VirtualController {
    pub lifecycle: ControllerLifecycle,
    pub boot_id: u64,
}

impl Default for VirtualController {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualController {
    pub fn new() -> Self {
        Self {
            lifecycle: ControllerLifecycle::Boot,
            boot_id: 1,
        }
    }

    pub fn connecting(&mut self) {
        self.lifecycle = ControllerLifecycle::Connecting;
    }
    pub fn connect(&mut self) {
        self.lifecycle = ControllerLifecycle::Connected;
    }
    pub fn running(&mut self) {
        self.lifecycle = ControllerLifecycle::Running;
    }
    pub fn degrade(&mut self) {
        self.lifecycle = ControllerLifecycle::Degraded;
    }
    pub fn disconnect(&mut self) {
        self.lifecycle = ControllerLifecycle::Offline;
    }

    pub fn restart(&mut self) {
        self.boot_id = self.boot_id.saturating_add(1);
        self.lifecycle = ControllerLifecycle::Boot;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restart_changes_boot_identity_and_resets_state() {
        let mut c = VirtualController::new();
        c.running();
        c.restart();
        assert_eq!(c.boot_id, 2);
        assert_eq!(c.lifecycle, ControllerLifecycle::Boot);
    }
}
