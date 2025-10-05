use anyhow::Result;
use r#macro::Id;
use std::convert::TryFrom;

mod core {
    pub mod id {
        use std::borrow::Cow;
        use std::fmt::{self, Display, Formatter};

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct ExternalId {
            type_name: Cow<'static, str>,
            variant_name: Cow<'static, str>,
        }

        impl ExternalId {
            pub const fn new_static(type_name: &'static str, variant_name: &'static str) -> Self {
                Self {
                    type_name: Cow::Borrowed(type_name),
                    variant_name: Cow::Borrowed(variant_name),
                }
            }

            pub fn new(type_name: impl Into<String>, variant_name: impl Into<String>) -> Self {
                Self {
                    type_name: Cow::Owned(type_name.into()),
                    variant_name: Cow::Owned(variant_name.into()),
                }
            }

            pub fn type_name(&self) -> &str {
                &self.type_name
            }

            pub fn variant_name(&self) -> &str {
                &self.variant_name
            }
        }

        impl Display for ExternalId {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}::{}", self.type_name, self.variant_name)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Id)]
enum SimpleItem {
    Alpha,
    Beta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Id)]
enum Parameter {
    Kitchen,
    Bedroom,
    LivingRoom,
}

#[derive(Debug, Clone, PartialEq, Eq, Id)]
enum CompositeEnum {
    None,
    Pair(Parameter, Parameter),
    Named { left: Parameter, right: Parameter },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Id)]
struct TupleStruct(Parameter, Parameter);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Id)]
struct NamedStruct {
    primary: Parameter,
    secondary: Parameter,
}

#[test]
fn unit_enum_supports_static_ids_and_try_from() -> Result<()> {
    let ext = SimpleItem::Alpha.ext_id();
    assert_eq!(ext.type_name(), "simple_item");
    assert_eq!(ext.variant_name(), "alpha");

    let restored = SimpleItem::try_from(ext.clone())?;
    assert_eq!(restored, SimpleItem::Alpha);

    assert!(SimpleItem::try_from(core::id::ExternalId::new("simple_item", "unknown")).is_err());

    Ok(())
}

#[test]
fn tuple_enum_variant_concatenates_parameter_names() {
    let ext = CompositeEnum::Pair(Parameter::Kitchen, Parameter::Bedroom).ext_id();
    assert_eq!(ext.type_name(), "composite_enum");
    assert_eq!(ext.variant_name(), "pair::kitchen::bedroom");

    assert!(CompositeEnum::try_from(ext).is_err());
}

#[test]
fn named_enum_variant_concatenates_in_field_order() {
    let ext = CompositeEnum::Named {
        left: Parameter::Bedroom,
        right: Parameter::LivingRoom,
    }
    .ext_id();
    assert_eq!(ext.type_name(), "composite_enum");
    assert_eq!(ext.variant_name(), "named::bedroom::living_room");

    assert!(CompositeEnum::try_from(ext).is_err());
}

#[test]
fn tuple_struct_concatenates_parameter_names() {
    let ext = TupleStruct(Parameter::Kitchen, Parameter::LivingRoom).ext_id();
    assert_eq!(ext.type_name(), "tuple_struct");
    assert_eq!(ext.variant_name(), "kitchen::living_room");

    // No TryFrom is generated for structs; compile-time coverage is handled in trybuild tests.
}

#[test]
fn named_struct_concatenates_parameter_names() {
    let ext = NamedStruct {
        primary: Parameter::Bedroom,
        secondary: Parameter::Kitchen,
    }
    .ext_id();
    assert_eq!(ext.type_name(), "named_struct");
    assert_eq!(ext.variant_name(), "bedroom::kitchen");
}
