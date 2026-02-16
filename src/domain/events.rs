#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Tick,
    QuitRequested,
    InputKey(KeyInput),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
    pub key: String,
    pub ctrl: bool,
}

impl KeyInput {
    pub fn new(key: impl Into<String>, ctrl: bool) -> Self {
        Self {
            key: key.into(),
            ctrl,
        }
    }
}
