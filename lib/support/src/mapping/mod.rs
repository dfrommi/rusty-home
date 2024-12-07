pub trait TypedItem {
    fn item_name(&self) -> &'static str;
    fn type_name(&self) -> &'static str;
}
