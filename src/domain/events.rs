#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Tick,
    QuitRequested,
    InputKey(KeyInput),
    ConnectivityChanged(ConnectivityStatus),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityStatus {
    Connected,
    Connecting,
    Disconnected,
}

impl ConnectivityStatus {
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Connecting => "connecting",
            Self::Disconnected => "disconnected",
        }
    }
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
