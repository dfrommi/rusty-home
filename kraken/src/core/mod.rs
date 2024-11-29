mod adapter;
mod domain;

pub use domain::collect_states;
pub use domain::execute_commands;
pub use domain::CommandExecutor;
pub use domain::StateCollector;
