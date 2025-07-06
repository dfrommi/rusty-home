use crate::core::time::{DateTime, DateTimeRange};
use crate::t;
use support::ValueObject;

use crate::core::timeseries::{
    DataFrame, DataPoint, TimeSeries,
    interpolate::{Estimatable, algo},
};

use super::{DataPointAccess, TimeSeriesAccess};

#[derive(Debug, Clone)]
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

impl ValueObject for Opened {
    type ValueType = bool;
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

impl<T> DataPointAccess<Opened> for T
where
    T: DataPointAccess<raw::Opened>,
{
    async fn current_data_point(&self, item: Opened) -> anyhow::Result<DataPoint<bool>> {
        any_of(self, item.api_items()).await
    }
}

async fn any_of(
    api: &impl DataPointAccess<raw::Opened>,
    opened_states: Vec<raw::Opened>,
) -> anyhow::Result<DataPoint<bool>> {
    let futures: Vec<_> = opened_states
        .into_iter()
        .map(|o| api.current_data_point(o))
        .collect();
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

impl<T> TimeSeriesAccess<Opened> for T
where
    T: TimeSeriesAccess<raw::Opened>,
{
    async fn series(
        &self,
        item: Opened,
        range: DateTimeRange,
    ) -> anyhow::Result<TimeSeries<Opened>> {
        let api_items = item.api_items();
        let context: raw::Opened = api_items[0].clone();

        let futures = api_items
            .into_iter()
            .map(|item| self.series(item, range.clone()))
            .collect::<Vec<_>>();

        let all_ts = futures::future::try_join_all(futures).await?;
        let merged = TimeSeries::reduce(context, all_ts, |&a, &b| a || b)?;

        //from API-opened into this opened type
        Ok(merged.map(item, |dp| dp.value))
    }
}

impl Estimatable for raw::Opened {
    type Type = bool;

    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}

impl Estimatable for Opened {
    type Type = bool;

    fn interpolate(&self, at: DateTime, df: &DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::time::DateTime;

    use super::*;

    #[tokio::test]
    async fn test_any_of_some_opened() {
        let api = FakeAccess {
            left: true,
            right: false,
            side: true,
            balcony: false,
        };

        let result = api.current_data_point(Opened::LivingRoomWindowOrDoor).await;

        assert!(result.unwrap().value);
    }

    #[tokio::test]
    async fn test_any_of_all_closed() {
        let api = FakeAccess {
            left: false,
            right: false,
            side: false,
            balcony: false,
        };

        let result = api.current_data_point(Opened::LivingRoomWindowOrDoor).await;

        assert!(!result.unwrap().value);
    }

    struct FakeAccess {
        left: bool,
        right: bool,
        side: bool,
        balcony: bool,
    }

    impl DataPointAccess<raw::Opened> for FakeAccess {
        async fn current_data_point(&self, item: raw::Opened) -> anyhow::Result<DataPoint<bool>> {
            Ok(DataPoint {
                value: match item {
                    raw::Opened::LivingRoomWindowLeft => self.left,
                    raw::Opened::LivingRoomWindowRight => self.right,
                    raw::Opened::LivingRoomWindowSide => self.side,
                    raw::Opened::LivingRoomBalconyDoor => self.balcony,
                    _ => panic!("Unexpected item {:?}", item),
                },
                timestamp: DateTime::now(),
            })
        }
    }
}
