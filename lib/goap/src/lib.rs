pub mod eval;
mod graph;
mod planner;
mod strategy;

pub use planner::plan;

pub trait Preconditions<S> {
    fn is_fulfilled(&self, other: &S) -> bool;
}

pub trait Effects<S> {
    fn apply_to(&self, state: &S) -> S;
}

#[derive(Debug)]
pub struct PlanningResult<'a, S, A> {
    pub next_actions: Vec<&'a A>,
    pub future_actions: Vec<&'a A>,
    pub final_state: S,
}
