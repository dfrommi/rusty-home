use std::fmt::Display;

use r#macro::{EnumVariants, Id, trace_state};
use serde::{Deserialize, Serialize};

use crate::core::{
    HomeApi,
    time::DateTimeRange,
    timeseries::{
        DataFrame, DataPoint,
        interpolate::{self, Estimatable},
    },
};
use crate::port::{DataFrameAccess, DataPointAccess};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum FanActivity {
    LivingRoomCeilingFan,
    BedroomCeilingFan,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum FanAirflow {
    Off,
    Forward(FanSpeed),
    Reverse(FanSpeed),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum FanSpeed {
    Silent,
    Low,
    Medium,
    High,
    Turbo,
}

impl Estimatable for FanActivity {
    fn interpolate(&self, at: crate::core::time::DateTime, df: &DataFrame<FanAirflow>) -> Option<FanAirflow> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataPointAccess<FanActivity> for FanActivity {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<FanAirflow>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<FanActivity> for FanActivity {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<FanAirflow>> {
        api.get_data_frame(self, range).await
    }
}

impl Display for FanAirflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FanAirflow::Off => write!(f, "off"),
            FanAirflow::Forward(FanSpeed::Silent) => write!(f, "silent (fwd)"),
            FanAirflow::Forward(FanSpeed::Low) => write!(f, "low (fwd)"),
            FanAirflow::Forward(FanSpeed::Medium) => write!(f, "medium (fwd)"),
            FanAirflow::Forward(FanSpeed::High) => write!(f, "high (fwd)"),
            FanAirflow::Forward(FanSpeed::Turbo) => write!(f, "turbo (fwd)"),
            FanAirflow::Reverse(FanSpeed::Silent) => write!(f, "silent (rev)"),
            FanAirflow::Reverse(FanSpeed::Low) => write!(f, "low (rev)"),
            FanAirflow::Reverse(FanSpeed::Medium) => write!(f, "medium (rev)"),
            FanAirflow::Reverse(FanSpeed::High) => write!(f, "high (rev)"),
            FanAirflow::Reverse(FanSpeed::Turbo) => write!(f, "turbo (rev)"),
        }
    }
}

impl From<&FanAirflow> for f64 {
    fn from(value: &FanAirflow) -> Self {
        match value {
            FanAirflow::Off => 0.0,
            FanAirflow::Forward(FanSpeed::Silent) => 1.0,
            FanAirflow::Forward(FanSpeed::Low) => 2.0,
            FanAirflow::Forward(FanSpeed::Medium) => 3.0,
            FanAirflow::Forward(FanSpeed::High) => 4.0,
            FanAirflow::Forward(FanSpeed::Turbo) => 5.0,
            FanAirflow::Reverse(FanSpeed::Silent) => -1.0,
            FanAirflow::Reverse(FanSpeed::Low) => -2.0,
            FanAirflow::Reverse(FanSpeed::Medium) => -3.0,
            FanAirflow::Reverse(FanSpeed::High) => -4.0,
            FanAirflow::Reverse(FanSpeed::Turbo) => -5.0,
        }
    }
}

impl From<f64> for FanAirflow {
    fn from(value: f64) -> Self {
        if value < -4.0 {
            FanAirflow::Reverse(FanSpeed::Turbo)
        } else if value < -3.0 {
            FanAirflow::Reverse(FanSpeed::High)
        } else if value < -2.0 {
            FanAirflow::Reverse(FanSpeed::Medium)
        } else if value < -1.0 {
            FanAirflow::Reverse(FanSpeed::Low)
        } else if value < 0.0 {
            FanAirflow::Reverse(FanSpeed::Silent)
        } else if value > 4.0 {
            FanAirflow::Forward(FanSpeed::Turbo)
        } else if value > 3.0 {
            FanAirflow::Forward(FanSpeed::High)
        } else if value > 2.0 {
            FanAirflow::Forward(FanSpeed::Medium)
        } else if value > 1.0 {
            FanAirflow::Forward(FanSpeed::Low)
        } else if value > 0.0 {
            FanAirflow::Forward(FanSpeed::Silent)
        } else {
            FanAirflow::Off
        }
    }
}
