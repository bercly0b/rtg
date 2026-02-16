#[derive(Debug, Clone)]
pub struct AppState {
    running: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self { running: true }
    }
}

impl AppState {
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}
