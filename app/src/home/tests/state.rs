use crate::core::time::{DateTime, FIXED_NOW};
use crate::core::{HomeApi, ValueObject};

use crate::{core::timeseries::DataPoint, home::state::AutomaticTemperatureIncrease, port::DataPointAccess};

use super::{infrastructure, runtime};

fn get_state_at<T>(iso: &str, item: T) -> DataPoint<T::ValueType>
where
    T: ValueObject + Clone,
    HomeApi: DataPointAccess<T>,
{
    let fake_now = DateTime::from_iso(iso).unwrap();

    runtime().block_on(FIXED_NOW.scope(fake_now, async {
        let api = &infrastructure().api();
        item.current_data_point(api).await.unwrap()
    }))
}

mod automatic_temp_increase {
    use super::*;

    #[test]
    fn not_enough_temperature_measurements() {
        let dp = get_state_at("2025-01-21T18:06:24.086+01:00", AutomaticTemperatureIncrease::LivingRoom);

        assert!(dp.value);
    }
}
