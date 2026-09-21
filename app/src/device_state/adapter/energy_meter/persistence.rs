use crate::core::{id::ExternalId, time::DateTime};

use crate::core::timeseries::DataPoint;

use super::{EnergyMeterTarget, EnergyReading};

#[derive(Clone)]
pub struct EnergyReadingRepository {
    pool: sqlx::PgPool,
}

impl EnergyReadingRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn add_yearly_energy_reading(&self, reading: EnergyReading, timestamp: DateTime) -> anyhow::Result<i64> {
        let (type_, item) = database_identity(&reading)?;
        let value = match &reading {
            EnergyReading::Heating(_, value)
            | EnergyReading::ColdWater(_, value)
            | EnergyReading::HotWater(_, value) => *value,
        };

        let rec = sqlx::query!(
            r#"INSERT INTO ENERGY_READING (TYPE, NAME, VALUE, TIMESTAMP)
                VALUES ($1, $2, $3, $4)
                RETURNING id"#,
            &type_,
            &item,
            value,
            timestamp.into_db(),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(rec.id)
    }

    pub async fn get_latest_total_readings_ids(&self) -> anyhow::Result<Vec<i64>> {
        let rows = sqlx::query!(
            r#"SELECT DISTINCT ON (type, name) 
                id as "id!",
                type as "reading_type!", 
                name as "name!",
                value as "value!",
                timestamp as "timestamp!"
                FROM energy_reading_total
                ORDER BY type, name, timestamp DESC"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|row| match try_into_reading(&row.reading_type, &row.name, row.value) {
                Ok(_) => Some(row.id),
                Err(e) => {
                    tracing::warn!(
                        "Invalid energy_reading_total row with id {}, type {}, name {}, ignoring: {}",
                        row.id,
                        row.reading_type,
                        row.name,
                        e
                    );
                    None
                }
            })
            .collect())
    }

    pub async fn get_total_reading_by_id(&self, id: i64) -> anyhow::Result<DataPoint<EnergyReading>> {
        let row = sqlx::query!(
            r#"SELECT DISTINCT ON (type, name) 
                type as "reading_type!", 
                name as "name!",
                value as "value!",
                timestamp as "timestamp!"
                FROM energy_reading_total
                WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => match try_into_reading(&row.reading_type, &row.name, row.value) {
                Ok(reading) => Ok(DataPoint::new(reading, row.timestamp.into())),
                Err(e) => {
                    tracing::warn!(
                        "Invalid energy_reading_total row with id {}, type {}, name {}: {}",
                        id,
                        row.reading_type,
                        row.name,
                        e
                    );
                    Err(e)
                }
            },
            None => anyhow::bail!("No energy reading found with id {}", id),
        }
    }
}

fn database_identity(reading: &EnergyReading) -> anyhow::Result<(String, String)> {
    let target = reading.target();
    let variant_name = target.ext_id().variant_name().to_string();
    let Some((reading_type, item)) = variant_name.split_once("::") else {
        anyhow::bail!("Invalid EnergyMeterTarget external ID: {variant_name}");
    };

    Ok((reading_type.to_string(), item.to_string()))
}

fn try_into_reading(type_: &str, name: &str, value: f64) -> anyhow::Result<EnergyReading> {
    let target = EnergyMeterTarget::try_from(ExternalId::new("energy_meter_target", format!("{type_}::{name}")))?;

    Ok(match target {
        EnergyMeterTarget::Heating(item) => EnergyReading::Heating(item, value),
        EnergyMeterTarget::ColdWater(item) => EnergyReading::ColdWater(item, value),
        EnergyMeterTarget::HotWater(item) => EnergyReading::HotWater(item, value),
    })
}

#[cfg(test)]
mod tests {
    use super::database_identity;
    use crate::frontends::energy_meter::{EnergyReading, Faucet, Radiator};

    #[test]
    fn target_database_identity_preserves_existing_storage_ids() {
        assert_eq!(
            database_identity(&EnergyReading::Heating(Radiator::Bedroom, 12.5)).ok(),
            Some(("heating".to_string(), "bedroom".to_string()))
        );
        assert_eq!(
            database_identity(&EnergyReading::ColdWater(Faucet::Kitchen, 12.5)).ok(),
            Some(("cold_water".to_string(), "kitchen".to_string()))
        );
        assert_eq!(
            database_identity(&EnergyReading::HotWater(Faucet::Kitchen, 12.5)).ok(),
            Some(("hot_water".to_string(), "kitchen".to_string()))
        );
    }

    use crate::t;

    use super::*;

    #[sqlx::test(migrations = "../migrations")]
    async fn latest_total_reading_ids_ignore_unsupported_reading_type(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let repo = EnergyReadingRepository::new(pool);

        sqlx::query!(
            r#"INSERT INTO energy_reading (type, name, value, timestamp) VALUES ($1, $2, $3, $4)"#,
            "removed_type",
            "legacy_meter",
            1.0,
            t!(now).into_db(),
        )
        .execute(&repo.pool)
        .await?;

        let ids = repo.get_latest_total_readings_ids().await?;

        assert!(ids.is_empty());

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_total_reading_by_id_warns_and_errors_for_unsupported_name(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let repo = EnergyReadingRepository::new(pool);

        let rec = sqlx::query!(
            r#"INSERT INTO energy_reading (type, name, value, timestamp) VALUES ($1, $2, $3, $4) RETURNING id"#,
            "heating",
            "removed_radiator",
            1.0,
            t!(now).into_db(),
        )
        .fetch_one(&repo.pool)
        .await?;

        let result = repo.get_total_reading_by_id(rec.id).await;

        assert!(result.is_err());

        Ok(())
    }
}
