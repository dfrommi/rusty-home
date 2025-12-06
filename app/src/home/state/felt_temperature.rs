use r#macro::{EnumVariants, Id};

use crate::{
    core::{
        HomeApi,
        math::{Sigmoid, Tanh},
        timeseries::DataPoint,
        unit::{DegreeCelsius, GramPerCubicMeter},
    },
    home::{
        state::{AbsoluteHumidity, Temperature, sampled_data_frame},
        state_registry::{DerivedStateProvider, StateCalculationContext},
    },
    port::{DataFrameAccess, DataPointAccess},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Id, EnumVariants)]
pub enum FeltTemperature {
    BathroomShower,
    LivingRoom,
    Kitchen,
    KitchenOuterWall,
    RoomOfRequirements,
}

pub struct FeltTemperatureStateProvider;

impl DerivedStateProvider<FeltTemperature, DegreeCelsius> for FeltTemperatureStateProvider {
    fn calculate_current(
        &self,
        id: FeltTemperature,
        ctx: &StateCalculationContext,
    ) -> Option<DataPoint<DegreeCelsius>> {
        let temperature_dp = ctx.get(id.temperature())?;
        let abs_humidity_dp = ctx.get(id.abs_humidity())?;

        let felt_temp_value = calculate_felt_temperature(temperature_dp.value, abs_humidity_dp.value);

        Some(DataPoint {
            value: felt_temp_value,
            timestamp: std::cmp::max(temperature_dp.timestamp, abs_humidity_dp.timestamp),
        })
    }
}

impl FeltTemperature {
    fn temperature(&self) -> Temperature {
        match self {
            FeltTemperature::LivingRoom => Temperature::LivingRoom,
            FeltTemperature::BathroomShower => Temperature::BathroomShower,
            FeltTemperature::Kitchen => Temperature::Kitchen,
            FeltTemperature::KitchenOuterWall => Temperature::KitchenOuterWall,
            FeltTemperature::RoomOfRequirements => Temperature::RoomOfRequirements,
        }
    }

    fn abs_humidity(&self) -> AbsoluteHumidity {
        match self {
            FeltTemperature::LivingRoom => AbsoluteHumidity::LivingRoom,
            FeltTemperature::BathroomShower => AbsoluteHumidity::BathroomShower,
            FeltTemperature::Kitchen => AbsoluteHumidity::Kitchen,
            FeltTemperature::KitchenOuterWall => AbsoluteHumidity::KitchenOuterWall,
            FeltTemperature::RoomOfRequirements => AbsoluteHumidity::RoomOfRequirements,
        }
    }
}

fn calculate_felt_temperature(temperature: DegreeCelsius, abs_humidity: GramPerCubicMeter) -> DegreeCelsius {
    // --- Hohe Feuchte Wirkung ---
    let delta_humid = {
        let sigmoid_af = Sigmoid::around(GramPerCubicMeter(10.0), GramPerCubicMeter(4.0)); // ∈ [0, 1]
        let tanh_temp = Tanh::new(DegreeCelsius(21.0), 0.3); // ∈ [-1, 1]
        let max_gain = DegreeCelsius(1.5); // max Korrektur in °C

        let abs_humidity_effect = sigmoid_af.eval(abs_humidity);
        let temp_effect = tanh_temp.eval(temperature);

        temp_effect * abs_humidity_effect.factor() * max_gain
    };

    // --- Trockene Luft Wirkung ---
    let delta_dry = {
        let sigmoid_temp = Sigmoid::around(DegreeCelsius(22.0), DegreeCelsius(4.0)); // ∈ [0, 1]
        let sigmoid_af = Sigmoid::around(GramPerCubicMeter(6.0), GramPerCubicMeter(6.0)); // ∈ [0, 1]
        let max_gain = DegreeCelsius(-0.7); // max Korrektur in °C

        let abs_humidity_effect = sigmoid_af.eval(abs_humidity); // ∈ [0, 1]
        let temp_effect = sigmoid_temp.eval(temperature); // f(T) ∈ [0, 1]

        temp_effect.factor() * abs_humidity_effect.factor() * max_gain
    };

    temperature + delta_humid + delta_dry
}

impl DataPointAccess<DegreeCelsius> for FeltTemperature {
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<DegreeCelsius>> {
        let temp = self.temperature().current_data_point(api).await?;
        let abs_humidity = self.abs_humidity().current_data_point(api).await?;
        let felt_temp_value = calculate_felt_temperature(temp.value, abs_humidity.value);

        Ok(DataPoint {
            value: felt_temp_value,
            timestamp: std::cmp::max(temp.timestamp, abs_humidity.timestamp),
        })
    }
}

impl DataFrameAccess<DegreeCelsius> for FeltTemperature {
    async fn get_data_frame(
        &self,
        range: crate::core::time::DateTimeRange,
        api: &crate::core::HomeApi,
    ) -> anyhow::Result<crate::core::timeseries::DataFrame<DegreeCelsius>> {
        sampled_data_frame(self, range, crate::t!(30 seconds), api).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_felt_temperature_hot_humid() {
        let temp = DegreeCelsius(25.0);
        let abs_humidity = GramPerCubicMeter(12.0);
        let felt_temp = calculate_felt_temperature(temp, abs_humidity);
        assert!(felt_temp > temp);
    }

    #[test]
    fn test_felt_temperature_cold_dry() {
        let temp = DegreeCelsius(18.0);
        let abs_humidity = GramPerCubicMeter(4.0);
        let felt_temp = calculate_felt_temperature(temp, abs_humidity);
        assert!(felt_temp < temp);
    }
}
