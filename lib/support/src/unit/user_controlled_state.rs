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

impl Display for UserControlledState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserControlledState::User => write!(f, "user-controlled"),
            UserControlledState::System => write!(f, "system-controlled"),
        }
    }
}
