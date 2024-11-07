mod dehumidify;
mod heating;
mod keep_user_override;
mod request_closing_window;

use std::fmt::Debug;
use std::fmt::Display;

use anyhow::Result;
use api::command::Command;
use api::command::CommandTarget;
use enum_dispatch::enum_dispatch;

pub use dehumidify::Dehumidify;
pub use heating::*;
pub use keep_user_override::KeepUserOverride;
pub use request_closing_window::RequestClosingWindow;

#[derive(Debug, Clone, derive_more::Display)]
#[enum_dispatch(Action)]
pub enum HomeAction {
    Dehumidify(Dehumidify),
    RequestClosingWindow(RequestClosingWindow),
    NoHeatingDuringVentilation(NoHeatingDuringVentilation),
    NoHeatingDuringAutomaticTemperatureIncrease(NoHeatingDuringAutomaticTemperatureIncrease),
    KeepUserOverride(KeepUserOverride),
    ExtendHeatingUntilSleeping(ExtendHeatingUntilSleeping),
    DeferHeatingUntilVentilationDone(DeferHeatingUntilVentilationDone),
}

#[enum_dispatch]
pub trait Action: Debug + Display {
    //action should be started based on current state
    async fn preconditions_fulfilled(&self) -> Result<bool>;

    //action was just triggered or effect of action is fulfilled based on current state
    async fn is_running(&self) -> Result<bool>;

    fn start_command(&self) -> Option<Command>;

    fn stop_command(&self) -> Option<Command>;

    fn controls_target(&self) -> Option<CommandTarget> {
        let start_target = self.start_command().map(|c| CommandTarget::from(&c));
        let stop_target = self.stop_command().map(|c| CommandTarget::from(&c));

        match (start_target, stop_target) {
            (Some(start), Some(stop)) => {
                if start != stop {
                    tracing::error!(
                        "Action {} controls different devices in start and stop commands. Falling back to start command",
                        self
                    );
                }

                Some(start)
            }
            (Some(start), None) => Some(start),
            (None, Some(stop)) => Some(stop),
            (None, None) => None,
        }
    }
}
