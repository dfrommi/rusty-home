use crate::core::time::{DateTime, FIXED_NOW};

use crate::{
    core::timeseries::DataPoint,
    home::state::{AutomaticTemperatureIncrease, HomeStateValueType},
    port::DataPointAccess,
};

use super::{infrastructure, runtime};

fn get_state_at<T>(iso: &str, item: T) -> DataPoint<T::ValueType>
where
    T: DataPointAccess<T> + HomeStateValueType + Clone,
{
    let fake_now = DateTime::from_iso(iso).unwrap();

    runtime().block_on(FIXED_NOW.scope(fake_now, async {
        let api = &infrastructure().api();
        item.current_data_point(api).await.unwrap()
    }))
}

mod automatic_temp_increase {
    use crate::{core::time::DateTimeRange, port::DataFrameAccess, t};

    use super::*;

    #[test]
    fn not_enough_temperature_measurements() {
        let dp = get_state_at("2025-01-21T18:06:24.086+01:00", AutomaticTemperatureIncrease::LivingRoom);

        assert!(dp.value);
    }

    // #[tokio::test]
    // async fn sampled_time_series() {
    //     let api = infrastructure().api();
    //
    //     //let start = DateTime::from_iso("2025-01-21T12:06:24.086+01:00").unwrap();
    //     //let end = start + t!(60 hours);
    //     let start = DateTime::from_iso("2025-01-22T08:40:16.949+01:00").unwrap();
    //     let end = start + t!(1 hours);
    //     let df = AutomaticTemperatureIncrease::LivingRoom
    //         .get_data_frame(DateTimeRange::new(start, end), &api)
    //         .await
    //         .unwrap();
    //
    //     for dp in df.iter() {
    //         println!("{dp:?}");
    //     }
    // }
}
