use crate::core::HomeApi;
use crate::core::time::{DateTime, DateTimeRange};
use crate::port::{DataFrameAccess, TimeSeriesAccess as _};
use crate::t;
use anyhow::Result;
use r#macro::{EnumVariants, Id, trace_state};

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
    BathroomThermostat,
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

impl DataPointAccess<bool> for Opened {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<bool> for Opened {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<bool>> {
        api.get_data_frame(self, range).await
    }
}

impl DataPointAccess<bool> for OpenedArea {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        let opened_items = self.api_items();
        let futures: Vec<_> = opened_items.iter().map(|o| o.current_data_point(api)).collect();
        let values: Vec<_> = futures::future::try_join_all(futures).await?;

        Ok(any_of(values))
    }
}

fn any_of(opened_dps: Vec<DataPoint<bool>>) -> DataPoint<bool> {
    let timestamp = opened_dps.iter().map(|v| v.timestamp).max().unwrap_or(t!(now));
    let value = opened_dps.iter().any(|v| v.value);

    DataPoint { value, timestamp }
}

impl DataFrameAccess<bool> for OpenedArea {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> Result<DataFrame<bool>> {
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
        let merged_ts = TimeSeries::reduce(context, all_ts, |a, b| a || b)?;
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
    use super::*;

    #[tokio::test]
    async fn test_any_of_some_opened() {
        let res = any_of(vec![
            DataPoint::new(false, t!(5 minutes ago)),
            DataPoint::new(true, t!(3 minutes ago)),
            DataPoint::new(false, t!(1 minutes ago)),
        ]);

        assert!(res.value);
    }

    #[tokio::test]
    async fn test_any_of_all_closed() {
        let res = any_of(vec![
            DataPoint::new(false, t!(5 minutes ago)),
            DataPoint::new(false, t!(3 minutes ago)),
            DataPoint::new(false, t!(1 minutes ago)),
        ]);

        assert!(!res.value);
    }
}
