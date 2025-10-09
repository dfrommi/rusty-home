use crate::core::HomeApi;
use crate::core::time::{DateTime, DateTimeRange};
use crate::port::{DataFrameAccess, TimeSeriesAccess as _};
use crate::t;
use anyhow::Result;
use r#macro::{EnumVariants, Id, mockable, trace_state};

use crate::core::timeseries::{
    DataFrame, DataPoint, TimeSeries,
    interpolate::{Estimatable, algo},
};

use super::DataPointAccess;

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum OpenedArea {
    LivingRoomWindowOrDoor,
    BedroomWindow,
    KitchenWindow,
    RoomOfRequirementsWindow,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, EnumVariants, Id)]
pub enum Opened {
    KitchenWindow,
    KitchenRadiatorThermostat,
    BedroomWindow,
    BedroomRadiatorThermostat,
    LivingRoomWindowLeft,
    LivingRoomWindowRight,
    LivingRoomWindowSide,
    LivingRoomBalconyDoor,
    LivingRoomRadiatorThermostatSmall,
    LivingRoomRadiatorThermostatBig,
    RoomOfRequirementsWindowLeft,
    RoomOfRequirementsWindowRight,
    RoomOfRequirementsWindowSide,
    RoomOfRequirementsThermostat,
}

impl OpenedArea {
    fn api_items(&self) -> Vec<Opened> {
        match self {
            OpenedArea::LivingRoomWindowOrDoor => vec![
                Opened::LivingRoomWindowLeft,
                Opened::LivingRoomWindowRight,
                Opened::LivingRoomWindowSide,
                Opened::LivingRoomBalconyDoor,
            ],
            OpenedArea::BedroomWindow => vec![Opened::BedroomWindow],
            OpenedArea::KitchenWindow => vec![Opened::KitchenWindow],
            OpenedArea::RoomOfRequirementsWindow => vec![
                Opened::RoomOfRequirementsWindowLeft,
                Opened::RoomOfRequirementsWindowRight,
                Opened::RoomOfRequirementsWindowSide,
            ],
        }
    }
}

impl DataPointAccess<Opened> for Opened {
    #[trace_state]
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<Opened> for Opened {
    #[mockable]
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<bool>> {
        api.get_data_frame(self, range).await
    }
}

impl DataPointAccess<OpenedArea> for OpenedArea {
    #[trace_state]
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        any_of(api, self.api_items()).await
    }
}

async fn any_of(api: &HomeApi, opened_states: Vec<Opened>) -> anyhow::Result<DataPoint<bool>> {
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

impl DataFrameAccess<OpenedArea> for OpenedArea {
    #[mockable]
    async fn get_data_frame(
        &self,
        range: DateTimeRange,
        api: &HomeApi,
    ) -> Result<DataFrame<<OpenedArea as crate::core::ValueObject>::ValueType>> {
        let api_items = self.api_items();
        let context: Opened = api_items[0].clone();

        let futures = api_items
            .into_iter()
            .map(|item| {
                let range = range.clone();
                async move { item.series(range, api).await }
            })
            .collect::<Vec<_>>();

        let all_ts = futures::future::try_join_all(futures).await?;

        //TODO work more directly on DataFrame
        let merged_ts = TimeSeries::reduce(context, all_ts, |&a, &b| a || b)?;
        let merged_df = merged_ts.inner();

        //from API-opened into this opened type
        Ok(merged_df.map(|dp| dp.value))
    }
}

impl Estimatable for Opened {
    fn interpolate(&self, at: DateTime, df: &DataFrame<bool>) -> Option<bool> {
        algo::last_seen(at, df)
    }
}

impl Estimatable for OpenedArea {
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
        api.with_fixed_current_dp(Opened::LivingRoomWindowLeft, true, t!(now));
        api.with_fixed_current_dp(Opened::LivingRoomWindowRight, false, t!(now));
        api.with_fixed_current_dp(Opened::LivingRoomWindowSide, true, t!(now));
        api.with_fixed_current_dp(Opened::LivingRoomBalconyDoor, false, t!(now));

        let result = OpenedArea::LivingRoomWindowOrDoor.current_data_point(&api).await;

        assert!(result.unwrap().value);
    }

    #[tokio::test]
    async fn test_any_of_all_closed() {
        let mut api = HomeApi::for_testing();
        api.with_fixed_current_dp(Opened::LivingRoomWindowLeft, false, t!(now));
        api.with_fixed_current_dp(Opened::LivingRoomWindowRight, false, t!(now));
        api.with_fixed_current_dp(Opened::LivingRoomWindowSide, false, t!(now));
        api.with_fixed_current_dp(Opened::LivingRoomBalconyDoor, false, t!(now));

        let result = OpenedArea::LivingRoomWindowOrDoor.current_data_point(&api).await;

        assert!(!result.unwrap().value);
    }
}
