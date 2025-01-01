pub trait TypedItem {
    fn item_name(&self) -> &'static str;
    fn type_name(&self) -> &'static str;
}

pub trait ValueObject {
    type ValueType;
}
