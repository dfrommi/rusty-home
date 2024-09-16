use std::cmp::Ordering;

use super::Preconditions;

pub trait StateError<S> {
    fn cmp(&self, lhs: &S, rhs: &S) -> Ordering;
}

pub struct MissedGoalsError<'a, G> {
    goals: &'a [G],
}

impl<'a, G> MissedGoalsError<'a, G> {
    pub fn new(goals: &'a [G]) -> Self {
        Self { goals }
    }
}

impl<'a, S, G: Preconditions<S>> StateError<S> for MissedGoalsError<'a, G> {
    fn cmp(&self, lhs: &S, rhs: &S) -> Ordering {
        count_missed_goals(lhs, self.goals).cmp(&count_missed_goals(rhs, self.goals))
    }
}

fn count_missed_goals<S, G: Preconditions<S>>(state: &S, goals: &[G]) -> usize {
    goals.iter().filter(|g| !g.is_fulfilled(state)).count()
}
