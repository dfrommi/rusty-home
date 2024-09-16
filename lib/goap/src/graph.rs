use std::marker::PhantomData;

use super::{Effects, Preconditions};

pub struct ActionGraph<'a, A, S> {
    actions: &'a [A],
    _marker: std::marker::PhantomData<S>,
}

//TODO caching?
impl<'a, A, S> ActionGraph<'a, A, S>
where
    S: Eq,
    A: Preconditions<S> + Effects<S>,
{
    pub fn new(actions: &'a [A]) -> Self {
        ActionGraph {
            actions,
            _marker: PhantomData,
        }
    }

    pub fn action(&self, from: &S, to: &S) -> Option<&'a A> {
        self.next_actions_and_states(from)
            .iter()
            .find(|(_, s)| s == to)
            .map(|(a, _)| *a)
    }

    pub fn to_actions(&self, path: &[S]) -> Vec<&'a A> {
        if path.len() < 2 {
            return vec![];
        }

        path.iter()
            .as_slice()
            .windows(2)
            .map(|w| {
                self.action(&w[0], &w[1])
                    .expect("Internal error: transition not found") //TODO
            })
            .collect()
    }

    pub fn next_states_and_costs(&self, current_state: &S) -> Vec<(S, usize)> {
        self.next_actions_and_states(current_state)
            .into_iter()
            .map(|(_, s)| (s, 1))
            .collect::<Vec<(S, usize)>>()
    }

    fn next_actions_and_states(&self, current_state: &S) -> Vec<(&'a A, S)> {
        self.actions
            .iter()
            .filter(|a| a.is_fulfilled(current_state))
            .map(|a| (a, a.apply_to(current_state)))
            .filter(|(_, s)| s != current_state)
            .collect()
    }
}
