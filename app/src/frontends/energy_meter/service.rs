use std::{collections::HashSet, fmt};

use anyhow::{Context as _, ensure};
use infrastructure::{EventEmitter, SQLiteDatabase};

use crate::core::{id::ExternalId, time::DateTime, timeseries::DataPoint};

use super::{
    EnergyMeterTarget, EnergyReading,
    persistence_sqlite::{RawEnergyReading, SQLiteEnergyReadingRepository},
};
#[derive(Debug)]
pub(super) struct InvalidEnergyReading(&'static str);

impl fmt::Display for InvalidEnergyReading {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for InvalidEnergyReading {}

pub struct EnergyReadingService {
    sqlite_repo: SQLiteEnergyReadingRepository,
    sqlite_db: SQLiteDatabase,
    events: EventEmitter<DataPoint<EnergyReading>>,
}

impl EnergyReadingService {
    pub fn new(
        sqlite_repo: SQLiteEnergyReadingRepository,
        sqlite_db: SQLiteDatabase,
        events: EventEmitter<DataPoint<EnergyReading>>,
    ) -> Self {
        Self {
            sqlite_repo,
            sqlite_db,
            events,
        }
    }

    pub async fn add_reading(&self, reading: EnergyReading, timestamp: DateTime) -> anyhow::Result<()> {
        let EnergyReading::Heating(_, value) = reading;
        let target = reading.target();
        let id = target.ext_id();
        let (reading_type, name) = id
            .variant_name()
            .split_once("::")
            .context("Invalid EnergyMeterTarget ID")?;
        let row = RawEnergyReading {
            reading_type: reading_type.to_string(),
            name: name.to_string(),
            value,
            timestamp,
            year_end: false,
        };

        if value < 0.0 {
            return Err(InvalidEnergyReading("Energy reading cannot be negative").into());
        }

        let history = {
            let connection = self.sqlite_db.for_reading()?;
            self.sqlite_repo.get_all(&connection)?
        };
        validate_reading(&row, &history)?;

        let rows = {
            let connection = self.sqlite_db.for_writing()?;
            self.sqlite_repo
                .insert(&connection, &row)
                .context("Could not save energy reading to SQLite")?;
            self.sqlite_repo.get_all(&connection)?
        };
        self.events.send(total_reading(&row, &rows)?);
        Ok(())
    }

    pub fn publish_initial_readings(&self) -> anyhow::Result<()> {
        let rows = {
            let connection = self.sqlite_db.for_reading()?;
            self.sqlite_repo.get_all(&connection)?
        };
        let mut seen = HashSet::new();
        for row in &rows {
            if seen.insert((&row.reading_type, &row.name)) {
                match total_reading(row, &rows) {
                    Ok(reading) => self.events.send(reading),
                    Err(error) => {
                        tracing::warn!(reading_type = %row.reading_type, meter = %row.name, %error, "Skipping energy reading")
                    }
                }
            }
        }
        Ok(())
    }
}

fn validate_reading(row: &RawEnergyReading, history: &[RawEnergyReading]) -> Result<(), InvalidEnergyReading> {
    let previous = history.iter().find(|previous| {
        previous.timestamp < row.timestamp && previous.reading_type == row.reading_type && previous.name == row.name
    });
    if let Some(previous) = previous
        && !previous.year_end
        && row.value < previous.value
    {
        return Err(InvalidEnergyReading(
            "Energy reading cannot be smaller than the previous reading unless it was a year-end reading",
        ));
    }

    Ok(())
}

fn total_reading(row: &RawEnergyReading, history: &[RawEnergyReading]) -> anyhow::Result<DataPoint<EnergyReading>> {
    let target = EnergyMeterTarget::try_from(ExternalId::new(
        "energy_meter_target",
        format!("{}::{}", row.reading_type, row.name),
    ))?;
    let earlier_years: f64 = history
        .iter()
        .filter(|previous| {
            previous.year_end
                && previous.timestamp < row.timestamp
                && previous.reading_type == row.reading_type
                && previous.name == row.name
        })
        .map(|previous| previous.value)
        .sum();
    let total = row.value + earlier_years;
    ensure!(total.is_finite(), "Cumulative energy reading must be finite");
    let EnergyMeterTarget::Heating(radiator) = target;
    Ok(DataPoint::new(EnergyReading::Heating(radiator, total), row.timestamp))
}
