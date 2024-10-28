use anyhow::Result;
use api::state::Presence;
use chrono::Duration;
use support::{
    ext::ToOk,
    time::{elapsed_since, in_time_range},
};

use crate::{adapter::persistence::DataPoint, home_api};

use super::DataPointAccess;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResidentState {
    Home,
    Away,
    Sleeping,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Resident {
    Dennis,
    Sabine,
}

//TODO maybe combination via Baysian to detect resident state
impl DataPointAccess<ResidentState> for Resident {
    async fn current_data_point(&self) -> Result<DataPoint<ResidentState>> {
        let (at_home, in_bed) = match self {
            Resident::Dennis => (Presence::AtHomeDennis, Presence::BedDennis),
            Resident::Sabine => (Presence::AtHomeSabine, Presence::BedSabine),
        };

        let at_home = home_api().get_latest(&at_home).await?;

        if !at_home.value {
            return Ok(DataPoint {
                value: ResidentState::Away,
                timestamp: at_home.timestamp,
            });
        }

        let in_bed = home_api().get_latest(&in_bed).await?;

        //in bed or not more than 5 minutes out of bed
        //TODO and if blinds are closed
        let sleeping = in_time_range(in_bed.timestamp, (21, 0), (3, 0))?
            && (elapsed_since(in_bed.timestamp) < Duration::minutes(5) || in_bed.value);

        DataPoint {
            value: if sleeping {
                ResidentState::Sleeping
            } else {
                ResidentState::Home
            },
            timestamp: std::cmp::max(at_home.timestamp, in_bed.timestamp),
        }
        .to_ok()
    }
}
