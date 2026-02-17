use super::events::ConnectivityStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    running: bool,
    connectivity_status: ConnectivityStatus,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            running: true,
            connectivity_status: ConnectivityStatus::Connecting,
        }
    }
}

impl ShellState {
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn connectivity_status(&self) -> ConnectivityStatus {
        self.connectivity_status
    }

    pub fn set_connectivity_status(&mut self, status: ConnectivityStatus) {
        self.connectivity_status = status;
    }
}
