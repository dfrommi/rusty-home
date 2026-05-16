use crate::core::domain::Room;
use crate::core::math::DataFrameStatsExt;
use crate::core::time::DateTimeRange;
use crate::core::timeseries::interpolate::LinearInterpolator;
use crate::core::unit::{DegreeCelsius, Percent};
use crate::home_state::RelativeHumidity;
use crate::home_state::Temperature;
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::t;
use r#macro::{EnumVariants, Id};

use super::dewpoint::DewPoint;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, EnumVariants, Id)]
pub enum RiskOfMould {
    Bathroom,
    Bedroom,
}

pub struct RiskOfMouldStateProvider;

impl DerivedStateProvider<RiskOfMould, bool> for RiskOfMouldStateProvider {
    fn calculate_current(&self, id: RiskOfMould, ctx: &StateCalculationContext) -> Option<bool> {
        match id {
            RiskOfMould::Bathroom => Self::calculate_bathroom(ctx),
            RiskOfMould::Bedroom => Self::calculate_bedroom(ctx),
        }
    }
}

impl RiskOfMouldStateProvider {
    fn calculate_bathroom(ctx: &StateCalculationContext) -> Option<bool> {
        let humidity = ctx.get(RelativeHumidity::Room(Room::Bathroom))?;

        if humidity.value < Percent(70.0) {
            tracing::trace!("Humidity of shower-sensor is below 70%, no risk of mould");
            return Some(false);
        }

        let this_dp = ctx.get(DewPoint::BathroomShower)?;
        let ref_dp = Self::get_reference_dewpoint(RiskOfMould::Bathroom, ctx)?;

        let risk = this_dp.value.0 - ref_dp.0 > 3.0;

        tracing::trace!(
            "Risk is {}. Dewpoint is {}, reference dewpoint is {}, threshold is 3.0",
            if risk { "high" } else { "low" },
            this_dp.value,
            ref_dp
        );

        Some(risk)
    }

    // Bedroom mould risk via dewpoint margin against the estimated corner
    // surface temperature.
    //
    // `margin = T_surface_corner − DewPoint_room`
    //
    // - `margin ≤ 0 K` → condensation on the wall
    // - `margin ≈ 3 K` → ~80 % surface RH (DIN 4108-2 mould threshold)
    // - `margin ≥ 5 K` → conservative safe zone (~70 % surface RH)
    //
    // TODO future improvements:
    // - Use a longer averaging window in summer / shorter in winter.
    // - Consider [`crate::home_state::TemperatureChange`] of the bedroom as
    //   a leading indicator (falling room temp pulls the corner colder with
    //   a small lag, so risk grows before the margin actually crosses).
    // - Once `f_Rsi` is calibrated, tighten the threshold from 3.0 K toward
    //   the more conservative 4.0-5.0 K range.
    fn calculate_bedroom(ctx: &StateCalculationContext) -> Option<bool> {
        const MARGIN_THRESHOLD_K: f64 = 3.0;

        let window = t!(3 hours ago);
        let corner_temps = ctx.all_since(Temperature::BedroomCorner, window)?;
        let bedroom_dewpoints = ctx.all_since(DewPoint::Room(Room::Bedroom), window)?;

        let mean_corner = corner_temps.weighted_aged_mean(t!(1 hours).to_half_life(), LinearInterpolator);
        let mean_dewpoint = bedroom_dewpoints.weighted_aged_mean(t!(1 hours).to_half_life(), LinearInterpolator);
        let mean_margin = mean_corner - mean_dewpoint;

        let risk = mean_margin < MARGIN_THRESHOLD_K;

        tracing::trace!(
            "Bedroom mould risk is {}. Corner temp mean: {:.2} °C, bedroom dewpoint mean: {:.2} °C, margin: {:.2} K, threshold: {:.1} K",
            if risk { "high" } else { "low" },
            mean_corner,
            mean_dewpoint,
            mean_margin,
            MARGIN_THRESHOLD_K
        );

        Some(risk)
    }

    fn get_reference_dewpoint(id: RiskOfMould, ctx: &StateCalculationContext) -> Option<DegreeCelsius> {
        let ref_dewpoints = match id {
            RiskOfMould::Bathroom => vec![
                DewPoint::Room(Room::LivingRoom),
                DewPoint::Room(Room::RoomOfRequirements),
            ],
            RiskOfMould::Bedroom => return None,
        };

        let mut ref_sum: f64 = 0.0;
        let ref_len = ref_dewpoints.len();
        for ref_dp in ref_dewpoints.into_iter() {
            let range = DateTimeRange::new(t!(3 hours ago), t!(now));
            let df = ctx.all_since(ref_dp, *range.start())?;
            ref_sum += df.weighted_aged_mean(t!(2 hours), LinearInterpolator);
        }

        let ref_mean = ref_sum / ref_len as f64;

        Some(DegreeCelsius(ref_mean))
    }
}
