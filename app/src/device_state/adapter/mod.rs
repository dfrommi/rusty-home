pub mod db;
pub mod energy_meter;
pub mod homeassistant;
pub mod internal;
pub mod tado;
pub mod tasmota;
pub mod z2m;

use crate::{
    core::timeseries::DataPoint,
    device_state::{DeviceAvailability, DeviceStateValue},
};

#[derive(Debug, Clone, derive_more::From)]
pub enum IncomingData {
    StateValue(DataPoint<DeviceStateValue>),
    ItemAvailability(DeviceAvailability),
}

pub trait IncomingDataSource {
    async fn recv_multi(&mut self) -> Option<Vec<IncomingData>>;
}
