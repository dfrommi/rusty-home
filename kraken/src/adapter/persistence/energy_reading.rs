#![allow(dead_code)]

use chrono::{DateTime, Utc};

use super::BackendApi;

#[derive(Debug, Clone)]
pub enum EnergyReading {
    Heating(Radiator, f64),
    ColdWater(Faucet, f64),
    HotWater(Faucet, f64),
}

#[derive(Debug, Clone)]
pub enum EnergyReadingType {
    Heating,
    ColdWater,
    HotWater,
}

#[derive(Debug, Clone)]
pub enum Radiator {
    LivingRoomBig,
    LivingRoomSmall,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

#[derive(Debug, Clone)]
pub enum Faucet {
    Kitchen,
    Bathroom,
}

pub trait EnergyReadingRepository {
    async fn add_energy_reading(
        &self,
        reading: EnergyReading,
        timestamp: DateTime<Utc>,
    ) -> anyhow::Result<()>;
}

impl EnergyReadingRepository for BackendApi {
    async fn add_energy_reading(
        &self,
        reading: EnergyReading,
        timestamp: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        //TODO derive automatically from enum
        let (type_, item, value) = match reading {
            EnergyReading::Heating(item, value) => (
                "heating",
                match item {
                    Radiator::LivingRoomBig => "living_room_big",
                    Radiator::LivingRoomSmall => "living_room_small",
                    Radiator::Bedroom => "bedroom",
                    Radiator::Kitchen => "kitchen",
                    Radiator::RoomOfRequirements => "room_of_requirements",
                    Radiator::Bathroom => "bathroom",
                },
                value,
            ),
            EnergyReading::ColdWater(item, value) => (
                "cold_water",
                match item {
                    Faucet::Kitchen => "kitchen",
                    Faucet::Bathroom => "bathroom",
                },
                value,
            ),
            EnergyReading::HotWater(item, value) => (
                "hot_water",
                match item {
                    Faucet::Kitchen => "kitchen",
                    Faucet::Bathroom => "bathroom",
                },
                value,
            ),
        };

        sqlx::query!(
            r#"INSERT INTO ENERGY_READING (TYPE, NAME, VALUE, TIMESTAMP)
                VALUES ($1, $2, $3, $4)"#,
            type_,
            item,
            value,
            timestamp,
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}
