use api::{
    command::{
        db::schema::{DbCommandState, DbCommandType, DbDevice, DbThingCommand, DbThingCommandRow},
        Command, CommandExecution, CommandTarget,
    },
    get_tag_id,
    state::{db::DbChannelId, ChannelId},
};
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgRow, PgPool, Row};

pub use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct DataPoint<V> {
    pub value: V,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug)]
pub struct HomeApi {
    db_pool: PgPool,
}

impl HomeApi {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn get_latest<'a, C: ChannelId>(&self, id: &'a C) -> Result<DataPoint<C::ValueType>>
    where
        &'a C: Into<DbChannelId>,
        C::ValueType: From<f64>,
    {
        let tag_id = get_tag_id(&self.db_pool, id.into(), false).await?;

        //TODO rewrite to max query
        let rec: Option<PgRow> = sqlx::query(
            "SELECT value, timestamp
            FROM THING_VALUES
            WHERE TAG_ID = $1
            ORDER BY timestamp DESC
            LIMIT 1",
        )
        .bind(tag_id)
        .fetch_optional(&self.db_pool)
        .await?;

        match rec {
            Some(r) => Ok(DataPoint {
                value: r.get::<f64, _>("value").into(),
                timestamp: r.get("timestamp"),
            }),
            None => Err(Error::NotFound),
        }
    }

    pub async fn get_covering<'a, C: ChannelId>(
        &self,
        id: &'a C,
        start: DateTime<Utc>,
    ) -> Result<Vec<DataPoint<C::ValueType>>>
    where
        &'a C: Into<DbChannelId>,
        C::ValueType: From<f64>,
    {
        let tags_id = get_tag_id(&self.db_pool, id.into(), false).await?;

        //TODO rewrite to max query
        let rec = sqlx::query(
            "(SELECT value, timestamp
              FROM THING_VALUES
              WHERE TAG_ID = $1
              AND timestamp > $2)
            UNION ALL
            (SELECT value, timestamp
              FROM THING_VALUES
              WHERE TAG_ID = $1
              AND timestamp <= $2
              ORDER BY timestamp DESC
              LIMIT 1)",
        )
        .bind(tags_id)
        .bind(start)
        .fetch_all(&self.db_pool)
        .await?;

        let dps: Vec<DataPoint<C::ValueType>> = rec
            .into_iter()
            .map(|row| DataPoint {
                value: C::ValueType::from(row.get("value")),
                timestamp: row.get("timestamp"),
            })
            .collect();

        Ok(dps)
    }

    pub async fn execute_command(&self, command: &Command) -> Result<()> {
        let data: DbThingCommand = command.into();

        sqlx::query( "INSERT INTO THING_COMMANDS (TYPE, POSITION, PAYLOAD, TIMESTAMP, STATUS) VALUES ($1, $2, $3, $4, $5)")
            .bind(data.command_type)
            .bind(data.position)
            .bind(data.payload)
            .bind(chrono::Utc::now())
            .bind(DbCommandState::Pending)
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }

    pub async fn get_latest_command(
        &self,
        target: &CommandTarget,
    ) -> Result<Option<CommandExecution>> {
        let (command_type, device): (DbCommandType, DbDevice) = target.into();

        let row: Option<DbThingCommandRow> = sqlx::query_as(
            "SELECT *
            from THING_COMMANDS
            where type = $1
              and position = $2
           order by timestamp desc
           limit 1",
        )
        .bind(command_type)
        .bind(device)
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(match row {
            Some(row) => Option::Some(row.try_into()?),
            None => Option::None,
        })
    }
}
