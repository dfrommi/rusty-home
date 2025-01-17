use std::fmt::Display;

pub trait ValueObject {
    type ValueType;
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExternalId {
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
}

impl ExternalId {
    pub fn new(type_: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            type_: type_.into(),
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct InternalId {
    pub type_: String,
    pub name: String,
}

impl InternalId {
    pub fn new(type_: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            type_: type_.into(),
            name: name.into(),
        }
    }
}

impl Display for InternalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.type_, self.name)
    }
}

impl Display for ExternalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.type_, self.name)
    }
}
