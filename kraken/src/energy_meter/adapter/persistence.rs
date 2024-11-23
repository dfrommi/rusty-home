use support::time::DateTime;

use crate::Database;

use super::{EnergyReading, EnergyReadingRepository, Faucet, Radiator};

impl EnergyReadingRepository for Database {
    async fn add_energy_reading(
        &self,
        reading: EnergyReading,
        timestamp: DateTime,
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
            timestamp.into_db(),
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}
