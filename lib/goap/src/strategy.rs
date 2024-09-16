use pathfinding::directed::dijkstra::{build_path, dijkstra_all};
use std::{collections::HashMap, hash::Hash};

use super::{graph::ActionGraph, Effects, Preconditions};

pub trait PlanningStrategy<S> {
    type R;
    fn plan(&self, current_state: &S) -> Self::R;
}

pub trait PlanningStrategyResult<S> {
    fn targets(&self) -> Vec<&S>;
    fn path_to(&self, target: &S) -> Vec<S>;
}

pub fn dijkstra<'a, S, A>(
    graph: &'a ActionGraph<'a, A, S>,
) -> impl PlanningStrategy<S, R = impl PlanningStrategyResult<S>> + 'a
where
    S: Eq + Hash + Clone,
    A: Preconditions<S> + Effects<S>,
{
    DijkstaStrategy { graph }
}

struct DijkstaStrategy<'a, S, A>
where
    A: Preconditions<S> + Effects<S>,
{
    graph: &'a ActionGraph<'a, A, S>,
}

pub struct DijkstaStrategyResult<S> {
    start: S,
    result: HashMap<S, (S, usize)>,
}

impl<'a, S, A> PlanningStrategy<S> for DijkstaStrategy<'a, S, A>
where
    S: Eq + Hash + Clone,
    A: Preconditions<S> + Effects<S>,
{
    type R = DijkstaStrategyResult<S>;

    fn plan(&self, current_state: &S) -> Self::R {
        let path = dijkstra_all(current_state, |current| {
            self.graph.next_states_and_costs(current)
        });

        DijkstaStrategyResult {
            start: current_state.clone(),
            result: path,
        }
    }
}

impl<S> PlanningStrategyResult<S> for DijkstaStrategyResult<S>
where
    S: Eq + Hash + Clone,
{
    fn targets(&self) -> Vec<&S> {
        let mut all_targets: Vec<(&S, usize)> =
            self.result.iter().map(|(s, (_, c))| (s, *c)).collect();
        all_targets.push((&self.start, 0));

        all_targets.sort_by(|(_, ca), (_, cb)| ca.cmp(cb));

        all_targets
            .iter()
            .map(move |(s, _)| *s)
            .collect::<Vec<&S>>()
    }

    fn path_to(&self, target: &S) -> Vec<S> {
        build_path(target, &self.result)
    }
}
