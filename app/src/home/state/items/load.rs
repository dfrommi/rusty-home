use crate::core::unit::RawValue;
use crate::home::state::RawVendorValue;
use crate::home::state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::port::{DataFrameAccess, DataPointAccess};
use crate::{
    core::{
        HomeApi,
        time::{DateTime, DateTimeRange},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{Estimatable, algo},
        },
        unit::Percent,
    },
    home::Thermostat,
};
use r#macro::{EnumVariants, Id, trace_state};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Load {
    Thermostat(Thermostat),
}

pub struct LoadStateProvider;

impl DerivedStateProvider<Load, Percent> for LoadStateProvider {
    fn calculate_current(&self, id: Load, ctx: &StateCalculationContext) -> Option<DataPoint<Percent>> {
        match id {
            Load::Thermostat(thermostat) => {
                let raw = ctx.get(RawVendorValue::AllyLoadEstimate(thermostat.clone()))?;
                Some(DataPoint::new(percent_load_for_ally(raw.value), raw.timestamp))
            }
        }
    }
}

impl Estimatable for Load {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Percent>) -> Option<Percent> {
        algo::last_seen(at, df)
    }
}

impl DataPointAccess<Percent> for Load {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<Percent>> {
        match self {
            Load::Thermostat(thermostat) => {
                let raw = RawVendorValue::AllyLoadEstimate(thermostat.clone())
                    .current_data_point(api)
                    .await?;

                Ok(DataPoint::new(percent_load_for_ally(raw.value), raw.timestamp))
            }
        }
    }
}

fn percent_load_for_ally(raw_value: RawValue) -> Percent {
    // Range: discard < -500, max value 3600, below 0 are different levels of zero.
    // -8000 invalid
    // TODO skip lower than -500 instead of mapping to 0?
    Percent(raw_value.0.max(0.0) / 36.0) // 0-3600 to percent
}

impl DataFrameAccess<Percent> for Load {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<Percent>> {
        match self {
            Load::Thermostat(thermostat) => {
                let raw_df = RawVendorValue::AllyLoadEstimate(thermostat.clone())
                    .get_data_frame(range.clone(), api)
                    .await?;

                let percent_df = raw_df.map(|raw_value| percent_load_for_ally(raw_value.value));
                Ok(percent_df)
            }
        }
    }
}
