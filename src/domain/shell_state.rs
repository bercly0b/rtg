#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    running: bool,
}

impl Default for ShellState {
    fn default() -> Self {
        Self { running: true }
    }
}

impl ShellState {
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}
