use crate::home::state::StateValue;

pub trait ValueObject {
    type ValueType: Clone;

    fn as_state_value(value: Self::ValueType) -> StateValue;
    fn project_state_value(&self, value: StateValue) -> Option<Self::ValueType>;
}
