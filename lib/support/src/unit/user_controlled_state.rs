use std::{f64, fmt::Display};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum UserControlledState {
    System,
    User,
}

impl UserControlledState {
    pub fn is_user_controlled(self) -> bool {
        Self::User == self
    }
}

impl From<&UserControlledState> for f64 {
    fn from(value: &UserControlledState) -> Self {
        match value {
            UserControlledState::User => 1.0,
            UserControlledState::System => 0.0,
        }
    }
}

impl From<f64> for UserControlledState {
    fn from(value: f64) -> Self {
        if value > 0.0 {
            Self::System
        } else {
            Self::User
        }
    }
}

impl Display for UserControlledState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserControlledState::User => write!(f, "user-controlled"),
            UserControlledState::System => write!(f, "system-controlled"),
        }
    }
}
