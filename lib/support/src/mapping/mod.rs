use std::{borrow::Cow, fmt::Display};

pub trait ValueObject {
    type ValueType;
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct InternalId {
    type_: &'static str,
    name: &'static str,
}

impl InternalId {
    pub const fn new(type_: &'static str, name: &'static str) -> Self {
        Self { type_, name }
    }

    pub fn int_type(&self) -> &str {
        self.type_
    }

    pub fn int_name(&self) -> &str {
        self.name
    }
}

impl Display for InternalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.type_, self.name)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExternalId {
    #[serde(rename = "type")]
    type_: Cow<'static, str>,
    name: Cow<'static, str>,
}

impl ExternalId {
    pub const fn new_static(type_: &'static str, name: &'static str) -> Self {
        Self {
            type_: Cow::Borrowed(type_),
            name: Cow::Borrowed(name),
        }
    }

    pub fn new(type_: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            type_: Cow::Owned(type_.into()),
            name: Cow::Owned(name.into()),
        }
    }

    pub fn ext_type(&self) -> &str {
        &self.type_
    }

    pub fn ext_name(&self) -> &str {
        &self.name
    }
}

impl Display for ExternalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.type_, self.name)
    }
}
