mod current;
mod series;
mod total;

use api::state::HeatingDemand;
pub use current::current_power;
pub use series::{heating_series_aggregated_sum, outside_temperature_series};
pub use total::total_power;

pub use current::current_heating;
pub use total::total_heating;

const EURO_PER_KWH: f64 = 0.349;

fn heating_factor(item: &HeatingDemand) -> f64 {
    match item {
        HeatingDemand::LivingRoom => 1.728 + 0.501,
        HeatingDemand::Bedroom => 1.401,
        HeatingDemand::RoomOfRequirements => 1.193,
        HeatingDemand::Kitchen => 1.485,
        HeatingDemand::Bathroom => 0.496,
    }
}
