use crate::core::math::DataFrameStatsExt;
use crate::core::time::DateTimeRange;
use crate::core::timeseries::interpolate::LinearInterpolator;
use crate::core::unit::{DegreeCelsius, Percent};
use crate::home_state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::t;
use crate::home_state::RelativeHumidity;
use anyhow::Result;
use r#macro::{EnumVariants, Id};

use super::dewpoint::DewPoint;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, EnumVariants, Id)]
pub enum RiskOfMould {
    Bathroom,
}

pub struct RiskOfMouldStateProvider;

impl DerivedStateProvider<RiskOfMould, bool> for RiskOfMouldStateProvider {
    fn calculate_current(&self, id: RiskOfMould, ctx: &StateCalculationContext) -> Option<bool> {
        let humidity = match id {
            RiskOfMould::Bathroom => ctx.get(RelativeHumidity::Bathroom)?,
        };

        if humidity.value < Percent(70.0) {
            tracing::trace!("Humidity of shower-sensor is below 70%, no risk of mould");
            return Some(false);
        }

        let this_dp = match id {
            RiskOfMould::Bathroom => ctx.get(DewPoint::BathroomShower)?,
        };

        let ref_dp = Self::get_reference_dewpoint(id, ctx)?;

        let risk = this_dp.value.0 - ref_dp.0 > 3.0;

        tracing::trace!(
            "Risk is {}. Dewpoint is {}, reference dewpoint is {}, threashold is 3.0",
            if risk { "high" } else { "low" },
            this_dp.value,
            ref_dp
        );

        Some(risk)
    }
}

impl RiskOfMouldStateProvider {
    fn get_reference_dewpoint(id: RiskOfMould, ctx: &StateCalculationContext) -> Option<DegreeCelsius> {
        let ref_dewpoints = match id {
            RiskOfMould::Bathroom => vec![DewPoint::LivingRoom, DewPoint::RoomOfRequirement],
        };

        let mut ref_sum: f64 = 0.0;
        let ref_len = ref_dewpoints.len();
        for ref_dp in ref_dewpoints.into_iter() {
            let range = DateTimeRange::new(t!(3 hours ago), t!(now));
            let df = ctx.all_since(ref_dp.clone(), *range.start())?;
            ref_sum += df.weighted_aged_mean(t!(2 hours), LinearInterpolator);
        }

        let ref_mean = ref_sum / ref_len as f64;

        Some(DegreeCelsius(ref_mean))
    }
}
