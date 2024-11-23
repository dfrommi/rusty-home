use api::state::ChannelValue;
use support::{t, DataPoint};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::core::StateCollector;

use super::{AddEnergyReadingUseCase, EnergyReading, EnergyReadingRepository};

#[derive(Clone)]
pub struct EnergyMeterService<R> {
    repo: R,
    tx: Sender<DataPoint<ChannelValue>>,
}

pub struct EnergyMeterStateCollector<R> {
    repo: R,
    rx: Receiver<DataPoint<ChannelValue>>,
}

impl<R> EnergyMeterService<R> {
    pub fn new(repo: R, tx: Sender<DataPoint<ChannelValue>>) -> Self {
        Self { repo, tx }
    }
}

impl<R> EnergyMeterStateCollector<R> {
    pub fn new(repo: R, rx: Receiver<DataPoint<ChannelValue>>) -> Self {
        Self { repo, rx }
    }
}

//PORT IN

impl<R> AddEnergyReadingUseCase for EnergyMeterService<R>
where
    R: EnergyReadingRepository + Send + Clone + Sync,
{
    async fn add_energy_reading(&self, reading: EnergyReading) -> anyhow::Result<()> {
        self.repo.add_energy_reading(reading, t!(now)).await?;
        todo!()
        //self.tx.send(DataPoint::new(reading.value, t!(now))); //TODO sum of values
        //Ok(())
    }
}

impl<R> StateCollector for EnergyMeterStateCollector<R>
where
    R: EnergyReadingRepository,
{
    async fn get_current_state(&self) -> anyhow::Result<Vec<DataPoint<ChannelValue>>> {
        todo!()
        //self.repo.get_current_state().await
    }

    async fn recv(&mut self) -> anyhow::Result<DataPoint<ChannelValue>> {
        loop {
            match self.rx.recv().await {
                Some(dp) => return Ok(dp),
                None => {
                    tracing::error!("Error receiving energy reading");
                    tokio::task::yield_now().await
                }
            }
        }
    }
}
