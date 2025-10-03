use r#macro::{EnumVariants, Id, mockable};

use crate::core::{
    HomeApi,
    time::{DateTime, DateTimeRange},
    timeseries::{
        DataFrame, DataPoint,
        interpolate::{self, Estimatable},
    },
};
use crate::port::{DataFrameAccess, DataPointAccess};

use super::Watt;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum CurrentPowerUsage {
    Fridge,
    Dehumidifier,
    AppleTv,
    Tv,
    AirPurifier,
    CouchLight,
    Dishwasher,
    Kettle,
    WashingMachine,
    Nuc,
    DslModem,
    InternetGateway,
    NetworkSwitch,
    KitchenMultiPlug,
    CouchPlug,
    RoomOfRequirementsDesk,
    InfraredHeater,
}

impl Estimatable for CurrentPowerUsage {
    fn interpolate(&self, at: DateTime, df: &DataFrame<Watt>) -> Option<Watt> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataPointAccess<CurrentPowerUsage> for CurrentPowerUsage {
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<Watt>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<CurrentPowerUsage> for CurrentPowerUsage {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<Watt>> {
        api.get_data_frame(self, range).await
    }
}
