use std::hash::Hash;

use super::strategy::{PlanningStrategy, PlanningStrategyResult};
use super::PlanningResult;
use super::{eval::StateError, graph::ActionGraph, strategy::dijkstra, Effects, Preconditions};

pub fn plan<'a, S, A>(
    current_state: &S,
    actions: &'a [A],
    state_error: &impl StateError<S>,
) -> PlanningResult<'a, S, A>
where
    S: Eq + Hash + Clone,
    A: Preconditions<S> + Effects<S>,
{
    let graph = ActionGraph::new(actions);
    let strategy = dijkstra(&graph);
    let result = strategy.plan(current_state);

    let best_target = result
        .targets()
        .into_iter()
        .min_by(|a, b| state_error.cmp(a, b))
        .expect("Error. No best match found");

    let best_path = result.path_to(best_target);
    let best_actions = graph.to_actions(&best_path);

    let (next_actions, future_actions) = best_actions
        .iter()
        .partition(|a| a.is_fulfilled(current_state));

    PlanningResult {
        next_actions,
        future_actions,
        final_state: best_target.clone(),
    }
}

#[cfg(test)]
mod tests {
    use crate::eval::MissedGoalsError;

    use super::*;

    #[derive(PartialEq, Eq, Hash, Clone, Debug)]
    struct HomeState {
        warm: bool,
        active_cooling: bool,
        good_room_climate: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    enum HomeAction {
        Heat,
        Off,
        FilterAir,
    }

    #[derive(Debug)]
    enum HomeGoal {
        Comfortable,
    }

    #[test]
    fn should_work() {
        let current_state = HomeState {
            warm: false,
            active_cooling: true,
            good_room_climate: false,
        };

        let actions = vec![HomeAction::Heat, HomeAction::Off, HomeAction::FilterAir];
        let goals = vec![HomeGoal::Comfortable];

        let state_error = MissedGoalsError::new(&goals);

        let before = std::time::Instant::now();

        let result = plan(&current_state, &actions, &state_error);

        println!("Planning took {:?}", before.elapsed());

        assert_eq!(2, result.next_actions.len());
        assert!(result.next_actions.contains(&&HomeAction::Off));
        assert!(result.next_actions.contains(&&HomeAction::FilterAir));

        assert_eq!(1, result.future_actions.len());
        assert!(result.future_actions.contains(&&HomeAction::Heat));

        assert_eq!(
            result.final_state,
            HomeState {
                warm: true,
                active_cooling: false,
                good_room_climate: true
            }
        );
    }

    impl Preconditions<HomeState> for HomeAction {
        fn is_fulfilled(&self, state: &HomeState) -> bool {
            match self {
                HomeAction::Heat => !state.warm && !state.active_cooling,
                HomeAction::Off => state.active_cooling,
                HomeAction::FilterAir => !state.good_room_climate,
            }
        }
    }

    impl Effects<HomeState> for HomeAction {
        fn apply_to(&self, state: &HomeState) -> HomeState {
            match self {
                HomeAction::Heat => HomeState {
                    warm: true,
                    ..state.clone()
                },
                HomeAction::Off => HomeState {
                    active_cooling: false,
                    ..state.clone()
                },
                HomeAction::FilterAir => HomeState {
                    good_room_climate: true,
                    ..state.clone()
                },
            }
        }
    }

    impl Preconditions<HomeState> for HomeGoal {
        fn is_fulfilled(&self, state: &HomeState) -> bool {
            match self {
                HomeGoal::Comfortable => state.warm && state.good_room_climate,
            }
        }
    }
}
