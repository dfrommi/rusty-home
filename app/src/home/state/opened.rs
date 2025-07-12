use crate::core::HomeApi;
use crate::core::time::{DateTime, DateTimeRange};
use crate::t;
use anyhow::Result;
use r#macro::{Id, mockable};

use crate::core::timeseries::{
    DataFrame, DataPoint, TimeSeries,
    interpolate::{Estimatable, algo},
};

use super::{DataPointAccess, TimeSeriesAccess};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
pub enum Opened {
    LivingRoomWindowOrDoor,
    BedroomWindow,
    KitchenWindow,
    RoomOfRequirementsWindow,
}

pub mod raw {
    use r#macro::Id;

    #[derive(Debug, Clone, Hash, Eq, PartialEq, Id)]
    pub enum Opened {
        KitchenWindow,
        BedroomWindow,
        LivingRoomWindowLeft,
        LivingRoomWindowRight,
        LivingRoomWindowSide,
        LivingRoomBalconyDoor,
        RoomOfRequirementsWindowLeft,
        RoomOfRequirementsWindowRight,
        RoomOfRequirementsWindowSide,
    }
}

impl Opened {
    fn api_items(&self) -> Vec<raw::Opened> {
        match self {
            Opened::LivingRoomWindowOrDoor => vec![
                raw::Opened::LivingRoomWindowLeft,
                raw::Opened::LivingRoomWindowRight,
                raw::Opened::LivingRoomWindowSide,
                raw::Opened::LivingRoomBalconyDoor,
            ],
            Opened::BedroomWindow => vec![raw::Opened::BedroomWindow],
            Opened::KitchenWindow => vec![raw::Opened::KitchenWindow],
            Opened::RoomOfRequirementsWindow => vec![
                raw::Opened::RoomOfRequirementsWindowLeft,
                raw::Opened::RoomOfRequirementsWindowRight,
                raw::Opened::RoomOfRequirementsWindowSide,
            ],
        }
    }
}

impl DataPointAccess<Opened> for Opened {
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        any_of(api, self.api_items()).await
    }
}

async fn any_of(api: &HomeApi, opened_states: Vec<raw::Opened>) -> anyhow::Result<DataPoint<bool>> {
    let futures: Vec<_> = opened_states.iter().map(|o| o.current_data_point(api)).collect();
    let res: Result<Vec<_>, _> = futures::future::try_join_all(futures).await;

    match res {
        Ok(values) => {
            let timestamp = values.iter().map(|v| v.timestamp).max().unwrap_or(t!(now));
            let value = values.iter().any(|v| v.value);

            Ok(DataPoint { value, timestamp })
        }
        Err(e) => Err(e),
    }
}

impl TimeSeriesAccess<Opened> for Opened {
    #[mockable]
    async fn series(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<TimeSeries<Opened>> {
        let api_items = self.api_items();
        let context: raw::Opened = api_items[0].clone();

        let futures = api_items
            .into_iter()
            .map(|item| {
                let range = range.clone();
                async move { item.series(range, api).await }
            })
            .collect::<Vec<_>>();

        let all_ts = futures::future::try_join_all(futures).await?;
        let merged = TimeSeries::reduce(context, all_ts, |&a, &b| a || b)?;

        //from API-opened into this opened type
        Ok(merged.map(self.clone(), |dp| dp.value))
    }
}

impl Estimatable for raw::Opened {
    fn interpolate(&self, at: DateTime, df: &DataFrame<bool>) -> Option<bool> {
        algo::last_seen(at, df)
    }
}

impl Estimatable for Opened {
    fn interpolate(&self, at: DateTime, df: &DataFrame<bool>) -> Option<bool> {
        algo::last_seen(at, df)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::HomeApi;

    use super::*;

    #[tokio::test]
    async fn test_any_of_some_opened() {
        let mut api = HomeApi::for_testing();
        api.with_fixed_current_dp(raw::Opened::LivingRoomWindowLeft, true, t!(now));
        api.with_fixed_current_dp(raw::Opened::LivingRoomWindowRight, false, t!(now));
        api.with_fixed_current_dp(raw::Opened::LivingRoomWindowSide, true, t!(now));
        api.with_fixed_current_dp(raw::Opened::LivingRoomBalconyDoor, false, t!(now));

        let result = Opened::LivingRoomWindowOrDoor.current_data_point(&api).await;

        assert!(result.unwrap().value);
    }

    #[tokio::test]
    async fn test_any_of_all_closed() {
        let mut api = HomeApi::for_testing();
        api.with_fixed_current_dp(raw::Opened::LivingRoomWindowLeft, false, t!(now));
        api.with_fixed_current_dp(raw::Opened::LivingRoomWindowRight, false, t!(now));
        api.with_fixed_current_dp(raw::Opened::LivingRoomWindowSide, false, t!(now));
        api.with_fixed_current_dp(raw::Opened::LivingRoomBalconyDoor, false, t!(now));

        let result = Opened::LivingRoomWindowOrDoor.current_data_point(&api).await;

        assert!(!result.unwrap().value);
    }
}
