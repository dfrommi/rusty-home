use infrastructure::EventEmitter;
use r#macro::{EnumVariants, Id};

use crate::core::timeseries::DataPoint;

mod http_server;
mod persistence;

use persistence::EnergyReadingRepository;

#[derive(Debug, Clone)]
pub struct EnergyMeter;

impl EnergyMeter {
    pub fn new_web_service(pool: sqlx::PgPool, tx: EventEmitter<DataPoint<EnergyReading>>) -> actix_web::Scope {
        http_server::new_actix_web_scope(EnergyReadingRepository::new(pool), tx)
    }

    pub async fn publish_initial_readings(
        pool: sqlx::PgPool,
        tx: &EventEmitter<DataPoint<EnergyReading>>,
    ) -> anyhow::Result<()> {
        let repo = EnergyReadingRepository::new(pool);
        for id in repo.get_latest_total_readings_ids().await? {
            match repo.get_total_reading_by_id(id).await {
                Ok(reading) => tx.send(reading),
                Err(e) => tracing::error!("Error loading Energy Reading with id {}: {:?}", id, e),
            }
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum EnergyMeterTarget {
    Heating(Radiator),
}

#[derive(Debug, Clone)]
pub enum EnergyReading {
    Heating(Radiator, f64),
}

impl EnergyReading {
    pub fn target(&self) -> EnergyMeterTarget {
        match self {
            Self::Heating(item, _) => EnergyMeterTarget::Heating(*item),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum Radiator {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::HashMap;

    use infrastructure::EventBus;

    use crate::core::time::DateTime;

    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn startup_publishes_latest_totals_with_original_timestamps_without_adding_readings(
        pool: sqlx::PgPool,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO energy_reading (type, name, value, timestamp, year_end) VALUES
                ('heating', 'bedroom', 100, '2024-12-31T23:00:00Z', true),
                ('heating', 'bedroom', 12.5, '2025-01-10T12:00:00Z', false),
                ('heating', 'kitchen', 900, '2024-12-31T23:00:00Z', true),
                ('heating', 'kitchen', 50, '2025-01-11T12:00:00Z', false)"#,
        )
        .execute(&pool)
        .await?;

        let bus = EventBus::new(8);
        let mut readings = bus.subscribe();
        EnergyMeter::publish_initial_readings(pool.clone(), &bus.emitter()).await?;

        let mut totals = HashMap::new();
        for _ in 0..2 {
            let reading = readings.recv().await.expect("energy meter source closed");
            totals.insert(reading.value.target(), reading);
        }
        for (radiator, expected, timestamp) in [
            (Radiator::Bedroom, 112.5, "2025-01-10T12:00:00Z"),
            (Radiator::Kitchen, 950.0, "2025-01-11T12:00:00Z"),
        ] {
            let reading = totals
                .get(&EnergyMeterTarget::Heating(radiator))
                .expect("missing initial total");
            match reading.value {
                EnergyReading::Heating(_, value) => assert_eq!(value, expected),
            }
            assert_eq!(reading.timestamp, DateTime::from_static_iso(timestamp));
        }

        let count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM energy_reading"#)
            .fetch_one(&pool)
            .await?;
        assert_eq!(count, 4);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), readings.recv())
                .await
                .is_err()
        );

        Ok(())
    }
}
