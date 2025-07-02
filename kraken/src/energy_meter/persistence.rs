use support::{DataPoint, time::DateTime};

use crate::Database;

use super::{EnergyReading, Faucet, Radiator};

impl Database {
    pub async fn add_yearly_energy_reading(
        &self,
        reading: EnergyReading,
        timestamp: DateTime,
    ) -> anyhow::Result<()> {
        //TODO derive automatically from enum
        let (type_, item, value): (&str, &str, f64) = match reading {
            EnergyReading::Heating(item, value) => ("heating", item.into(), value),
            EnergyReading::ColdWater(item, value) => ("cold_water", item.into(), value),
            EnergyReading::HotWater(item, value) => ("hot_water", item.into(), value),
        };

        sqlx::query!(
            r#"INSERT INTO ENERGY_READING (TYPE, NAME, VALUE, TIMESTAMP)
                VALUES ($1, $2, $3, $4)"#,
            type_,
            item,
            value,
            timestamp.into_db(),
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    pub async fn get_latest_total_readings(&self) -> anyhow::Result<Vec<DataPoint<EnergyReading>>> {
        let rows = sqlx::query!(
            r#"SELECT DISTINCT ON (type, name) 
                type as "reading_type!", 
                name as "name!",
                value as "value!",
                timestamp as "timestamp!"
                FROM energy_reading_total
                ORDER BY type, name, timestamp DESC"#
        )
        .fetch_all(&self.db_pool)
        .await?;

        let mut readings: Vec<DataPoint<EnergyReading>> = vec![];

        for row in rows {
            let reading = try_into_reading(&row.reading_type, &row.name, row.value)?;
            readings.push(DataPoint::new(reading, row.timestamp.into()));
        }

        Ok(readings)
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
        .fetch_all(&self.db_pool)
        .await?;

        let mut readings: Vec<i64> = vec![];

        for row in rows {
            readings.push(row.id);
        }

        Ok(readings)
    }

    pub async fn get_total_reading_by_id(
        &self,
        id: i64,
    ) -> anyhow::Result<DataPoint<EnergyReading>> {
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
        .fetch_optional(&self.db_pool)
        .await?;

        match row {
            Some(row) => Ok(DataPoint::new(
                try_into_reading(&row.reading_type, &row.name, row.value)?,
                row.timestamp.into(),
            )),
            None => anyhow::bail!("No energy reading found with id {}", id),
        }
    }
}

fn try_into_reading(type_: &str, name: &str, value: f64) -> anyhow::Result<EnergyReading> {
    match type_ {
        "heating" => Ok(EnergyReading::Heating(name.try_into()?, value)),
        "cold_water" => Ok(EnergyReading::ColdWater(name.try_into().unwrap(), value)),
        "hot_water" => Ok(EnergyReading::HotWater(name.try_into().unwrap(), value)),
        _ => Err(anyhow::anyhow!(
            "Received unsupported energy reading type {}",
            type_
        )),
    }
}

//TODO macro

impl From<Radiator> for &'static str {
    fn from(val: Radiator) -> Self {
        match val {
            Radiator::LivingRoomBig => "living_room_big",
            Radiator::LivingRoomSmall => "living_room_small",
            Radiator::Bedroom => "bedroom",
            Radiator::Kitchen => "kitchen",
            Radiator::RoomOfRequirements => "room_of_requirements",
            Radiator::Bathroom => "bathroom",
        }
    }
}

impl From<Faucet> for &'static str {
    fn from(val: Faucet) -> Self {
        match val {
            Faucet::Kitchen => "kitchen",
            Faucet::Bathroom => "bathroom",
        }
    }
}

impl TryInto<Radiator> for &str {
    type Error = anyhow::Error;

    fn try_into(self) -> anyhow::Result<Radiator> {
        match self {
            "living_room_big" => Ok(Radiator::LivingRoomBig),
            "living_room_small" => Ok(Radiator::LivingRoomSmall),
            "bedroom" => Ok(Radiator::Bedroom),
            "kitchen" => Ok(Radiator::Kitchen),
            "room_of_requirements" => Ok(Radiator::RoomOfRequirements),
            "bathroom" => Ok(Radiator::Bathroom),
            _ => Err(anyhow::anyhow!("Error parsing Radiator from {}", self)),
        }
    }
}

impl TryInto<Faucet> for &str {
    type Error = anyhow::Error;

    fn try_into(self) -> anyhow::Result<Faucet> {
        match self {
            "kitchen" => Ok(Faucet::Kitchen),
            "bathroom" => Ok(Faucet::Bathroom),
            _ => Err(anyhow::anyhow!("Error parsing Faucet from {}", self)),
        }
    }
}
