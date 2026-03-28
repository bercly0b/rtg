#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    pub phone_attempts: usize,
    pub code_attempts: usize,
    pub password_attempts: usize,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            phone_attempts: 3,
            code_attempts: 3,
            password_attempts: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthCodeToken(pub String);

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignInOutcome {
    Authorized,
    PasswordRequired,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthBackendError {
    InvalidPhone,
    InvalidCode,
    WrongPassword,
    Timeout,
    FloodWait { seconds: u32 },
    Transient { code: &'static str, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuidedAuthOutcome {
    Authenticated,
    ExitWithGuidance,
}
