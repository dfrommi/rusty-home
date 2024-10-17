use dehumidify::Dehumidify;
use enum_dispatch::enum_dispatch;
use goap::{Effects, Preconditions};

use anyhow::Result;
use heat::Heat;
use request_closing_window::RequestClosingWindow;

use super::HomeState;

pub mod dehumidify;
pub mod heat;
pub mod request_closing_window;

#[enum_dispatch]
pub trait Action: Preconditions<HomeState> + Effects<HomeState> {
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn is_running(&self) -> bool;
    async fn is_enabled(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[enum_dispatch(Action, Preconditions<HomeState>, Effects<HomeState>)]
pub enum HomeAction {
    Dehumidify(Dehumidify),
    RequestClosingWindow(RequestClosingWindow),
    Heat(Heat),
}

//traits of other crates not working with enum_dispatch
impl Preconditions<HomeState> for HomeAction {
    fn is_fulfilled(&self, other: &HomeState) -> bool {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.is_fulfilled(other),
            HomeAction::RequestClosingWindow(requst_closing_window) => {
                requst_closing_window.is_fulfilled(other)
            }
            HomeAction::Heat(heat) => heat.is_fulfilled(other),
        }
    }
}

//traits of other crates not working with enum_dispatch
impl Effects<HomeState> for HomeAction {
    fn apply_to(&self, state: &HomeState) -> HomeState {
        match self {
            HomeAction::Dehumidify(dehumidify) => dehumidify.apply_to(state),
            HomeAction::RequestClosingWindow(requst_closing_window) => {
                requst_closing_window.apply_to(state)
            }
            HomeAction::Heat(heat) => heat.apply_to(state),
        }
    }
}
