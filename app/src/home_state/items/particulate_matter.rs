use r#macro::{EnumVariants, Id};

use crate::{
    core::{
        math::DataFrameStatsExt as _, time::DateTimeRange, timeseries::interpolate::LinearInterpolator,
        unit::MicrogramsPerCubicMeter,
    },
    home_state::calc::{DerivedStateProvider, StateCalculationContext},
    t,
};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum ParticulateMatter {
    LivingRoomPM25,
    BedroomPM25,
}

pub struct ParticulateMatterStateProvider;

impl DerivedStateProvider<ParticulateMatter, MicrogramsPerCubicMeter> for ParticulateMatterStateProvider {
    fn calculate_current(
        &self,
        id: ParticulateMatter,
        ctx: &StateCalculationContext,
    ) -> Option<MicrogramsPerCubicMeter> {
        use crate::device_state::ParticulateMatter as DeviceParticulateMatter;

        let time_to_consider = t!(60 minutes ago);

        let history = ctx.device_states(
            match id {
                ParticulateMatter::LivingRoomPM25 => DeviceParticulateMatter::LivingRoomPM25,
                ParticulateMatter::BedroomPM25 => DeviceParticulateMatter::BedroomAirPurifierPM25,
            },
            time_to_consider,
        )?;

        //TODO check if that can be simplified. Last value interpolator really needed?
        //Does it even make sense with LastSeen?
        //Flow: get device state for range (which can start earlier), constrain hard to range, then calc mean
        //Maybe ref for a full range, then retain to copy exact
        //Also start should not be now-x but last datapoint-x ???
        let value = history
            .retain_range(&DateTimeRange::since(time_to_consider), LinearInterpolator, LinearInterpolator)
            .non_empty()?
            .weighted_aged_mean(t!(20 minutes), LinearInterpolator);

        Some(MicrogramsPerCubicMeter(value))
    }
}
