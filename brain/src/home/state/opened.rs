use support::{
    t,
    time::{DateTime, DateTimeRange},
    DataPoint, ValueObject,
};

use crate::support::timeseries::{
    interpolate::{algo, Estimatable},
    TimeSeries,
};

use super::{DataPointAccess, TimeSeriesAccess};

#[derive(Debug, Clone)]
pub enum Opened {
    LivingRoomWindowOrDoor,
    BedroomWindow,
    KitchenWindow,
    RoomOfRequirementsWindow,
}

impl ValueObject for Opened {
    type ValueType = bool;
}

impl Opened {
    fn api_items(&self) -> Vec<api::state::Opened> {
        match self {
            Opened::LivingRoomWindowOrDoor => vec![
                api::state::Opened::LivingRoomWindowLeft,
                api::state::Opened::LivingRoomWindowRight,
                api::state::Opened::LivingRoomWindowSide,
                api::state::Opened::LivingRoomBalconyDoor,
            ],
            Opened::BedroomWindow => vec![api::state::Opened::BedroomWindow],
            Opened::KitchenWindow => vec![api::state::Opened::KitchenWindow],
            Opened::RoomOfRequirementsWindow => vec![
                api::state::Opened::RoomOfRequirementsWindowLeft,
                api::state::Opened::RoomOfRequirementsWindowRight,
                api::state::Opened::RoomOfRequirementsWindowSide,
            ],
        }
    }
}

impl<T> DataPointAccess<Opened> for T
where
    T: DataPointAccess<api::state::Opened>,
{
    async fn current_data_point(&self, item: Opened) -> anyhow::Result<DataPoint<bool>> {
        any_of(self, item.api_items()).await
    }
}

async fn any_of(
    api: &impl DataPointAccess<api::state::Opened>,
    opened_states: Vec<api::state::Opened>,
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
    T: TimeSeriesAccess<api::state::Opened>,
{
    async fn series(
        &self,
        item: Opened,
        range: DateTimeRange,
    ) -> anyhow::Result<TimeSeries<Opened>> {
        let api_items = item.api_items();
        let context: api::state::Opened = api_items[0].clone();

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

impl Estimatable for api::state::Opened {
    type Type = bool;

    fn interpolate(&self, at: DateTime, df: &support::DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}

impl Estimatable for Opened {
    type Type = bool;

    fn interpolate(&self, at: DateTime, df: &support::DataFrame<Self::Type>) -> Option<Self::Type> {
        algo::last_seen(at, df)
    }
}

#[cfg(test)]
mod tests {
    use support::time::DateTime;

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

    impl DataPointAccess<api::state::Opened> for FakeAccess {
        async fn current_data_point(
            &self,
            item: api::state::Opened,
        ) -> anyhow::Result<DataPoint<bool>> {
            Ok(DataPoint {
                value: match item {
                    api::state::Opened::LivingRoomWindowLeft => self.left,
                    api::state::Opened::LivingRoomWindowRight => self.right,
                    api::state::Opened::LivingRoomWindowSide => self.side,
                    api::state::Opened::LivingRoomBalconyDoor => self.balcony,
                    _ => panic!("Unexpected item {:?}", item),
                },
                timestamp: DateTime::now(),
            })
        }
    }
}
