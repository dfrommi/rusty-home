use crate::core::time::DateTimeRange;
use crate::core::timeseries::DataFrame;
use crate::core::timeseries::interpolate::{self, Estimatable};
use crate::core::unit::DegreeCelsius;
use crate::home::state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::port::DataFrameAccess;
use crate::t;
use crate::{core::timeseries::DataPoint, home::state::Temperature};
use r#macro::{EnumVariants, Id, trace_state};

use super::{DataPointAccess, OpenedArea, sampled_data_frame};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum ColdAirComingIn {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

pub struct ColdAirComingInStateProvider;

impl DerivedStateProvider<ColdAirComingIn, bool> for ColdAirComingInStateProvider {
    fn calculate_current(&self, id: ColdAirComingIn, ctx: &StateCalculationContext) -> Option<DataPoint<bool>> {
        let outside_temp = ctx.get(Temperature::Outside)?;

        if outside_temp.value > DegreeCelsius(22.0) {
            tracing::trace!("No cold air coming in, temperature outside is too high");
            return Some(DataPoint::new(false, outside_temp.timestamp));
        }

        let window_opened = match id {
            ColdAirComingIn::LivingRoom => ctx.get(OpenedArea::LivingRoomWindowOrDoor)?,
            ColdAirComingIn::Bedroom => ctx.get(OpenedArea::BedroomWindow)?,
            ColdAirComingIn::Kitchen => ctx.get(OpenedArea::KitchenWindow)?,
            ColdAirComingIn::RoomOfRequirements => ctx.get(OpenedArea::RoomOfRequirementsWindow)?,
        };

        let message = if window_opened.value {
            "Cold air coming in, because it's cold outside and window is open"
        } else {
            "No cold air coming in, because window is closed"
        };
        tracing::trace!("{}", message);
        Some(DataPoint::new(window_opened.value, window_opened.timestamp))
    }
}

impl DataPointAccess<bool> for ColdAirComingIn {
    #[trace_state]
    async fn current_data_point(&self, api: &crate::core::HomeApi) -> anyhow::Result<DataPoint<bool>> {
        let outside_temp = Temperature::Outside.current_data_point(api).await?;

        if outside_temp.value > DegreeCelsius(22.0) {
            tracing::trace!("No cold air coming in, temperature outside is too high");
            return Ok(DataPoint::new(false, outside_temp.timestamp));
        }

        let window_opened = match self {
            ColdAirComingIn::LivingRoom => OpenedArea::LivingRoomWindowOrDoor.current_data_point(api).await,
            ColdAirComingIn::Bedroom => OpenedArea::BedroomWindow.current_data_point(api).await,
            ColdAirComingIn::Kitchen => OpenedArea::KitchenWindow.current_data_point(api).await,
            ColdAirComingIn::RoomOfRequirements => OpenedArea::RoomOfRequirementsWindow.current_data_point(api).await,
        }?;

        let message = if window_opened.value {
            "Cold air coming in, because it's cold outside and window is open"
        } else {
            "No cold air coming in, because window is closed"
        };
        tracing::trace!("{}", message);
        Ok(DataPoint::new(window_opened.value, window_opened.timestamp))
    }
}

impl Estimatable for ColdAirComingIn {
    fn interpolate(&self, at: crate::core::time::DateTime, df: &DataFrame<Self::ValueType>) -> Option<Self::ValueType> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataFrameAccess<bool> for ColdAirComingIn {
    async fn get_data_frame(
        &self,
        range: DateTimeRange,
        api: &crate::core::HomeApi,
    ) -> anyhow::Result<DataFrame<bool>> {
        sampled_data_frame(self, range, t!(30 seconds), api).await
    }
}
