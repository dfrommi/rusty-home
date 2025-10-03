use crate::core::{
    HomeApi,
    time::{DateTime, DateTimeRange},
    timeseries::{
        DataFrame, DataPoint,
        interpolate::{Estimatable, algo},
    },
    unit::KiloWattHours,
};
use crate::port::{DataFrameAccess, DataPointAccess};
use r#macro::{EnumVariants, Id, mockable};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TotalEnergyConsumption {
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

impl Estimatable for TotalEnergyConsumption {
    fn interpolate(&self, at: DateTime, df: &DataFrame<KiloWattHours>) -> Option<KiloWattHours> {
        algo::linear(at, df)
    }
}

impl DataPointAccess<TotalEnergyConsumption> for TotalEnergyConsumption {
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<KiloWattHours>> {
        api.current_data_point(self).await
    }
}

impl DataFrameAccess<TotalEnergyConsumption> for TotalEnergyConsumption {
    #[mockable]
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<KiloWattHours>> {
        api.get_data_frame(self, range).await
    }
}
