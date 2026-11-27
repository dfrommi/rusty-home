use std::sync::Arc;

use actix_web::web::{self, Json};
use actix_web::{HttpResponse, error::ErrorInternalServerError};
use serde::Deserialize;

use crate::t;

use super::{EnergyReading, Radiator, service::EnergyReadingService};

pub fn new_actix_web_scope(service: Arc<EnergyReadingService>) -> actix_web::Scope {
    web::scope("/api/energy/readings")
        .route("/heating", web::put().to(handle_heating_reading))
        .app_data(web::Data::from(service))
}

#[derive(Debug, Deserialize)]
struct HeatingReadingDTO {
    label: String,
    value: String,
}

async fn handle_heating_reading(
    service: web::Data<EnergyReadingService>,
    Json(dto): Json<HeatingReadingDTO>,
) -> Result<HttpResponse, actix_web::Error> {
    let radiator = match dto.label.as_str() {
        "Wohnzimmer (groß)" => Radiator::LivingRoomBig,
        "Wohnzimmer (klein)" => Radiator::LivingRoomSmall,
        "Room of Requirements" => Radiator::RoomOfRequirements,
        "Küche" => Radiator::Kitchen,
        "Schlafzimmer" => Radiator::Bedroom,
        "Bad" => Radiator::Bathroom,
        _ => return Ok(HttpResponse::BadRequest().finish()),
    };

    let value = match dto.value.parse::<f64>() {
        Ok(v) if v.is_finite() => v,
        _ => return Ok(HttpResponse::BadRequest().finish()),
    };

    let reading = EnergyReading::Heating(radiator, value);

    tracing::info!("Received reading {:?}", reading);

    service
        .add_reading(reading, t!(now))
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use actix_web::{App, http::StatusCode, test};
    use anyhow::Context as _;
    use infrastructure::EventBus;
    use serde_json::json;

    use crate::{
        core::time::DateTime,
        frontends::energy_meter::{
            EnergyMeter, EnergyReading, Radiator,
            tests::{seed_heating_readings, test_database},
        },
    };

    #[sqlx::test(migrations = "../migrations")]
    async fn http_reading_is_stored_as_yearly_and_emitted_as_cumulative_reading(
        pool: sqlx::PgPool,
    ) -> anyhow::Result<()> {
        let (_file, db) = test_database()?;
        let bus = EventBus::new(8);
        let mut readings = bus.subscribe();
        let meter = EnergyMeter::new(pool.clone(), db.clone(), bus.emitter())?;
        seed_heating_readings(
            &db,
            &[
                ("bedroom", 100.0, "2023-12-31T23:00:00Z", true),
                ("bedroom", 200.0, "2024-12-31T23:00:00Z", true),
                ("bedroom", 50.0, "2024-06-01T12:00:00Z", false),
                ("kitchen", 900.0, "2024-12-31T23:00:00Z", true),
                ("bedroom", 300.0, "2025-01-10T12:34:56.123456Z", true),
                ("bedroom", 400.0, "2025-12-31T23:00:00Z", true),
            ],
        )?;
        let app = test::init_service(App::new().service(meter.new_web_service())).await;
        let timestamp = DateTime::from_static_iso("2025-01-10T12:34:56.123456Z");
        let request = test::TestRequest::put()
            .uri("/api/energy/readings/heating")
            .set_json(json!({ "label": "Schlafzimmer", "value": "12.5" }))
            .to_request();

        let response = timestamp.eval_timeshifted(test::call_service(&app, request)).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let stored = sqlx::query!(
            r#"SELECT value, year_end FROM energy_reading
                WHERE type = 'heating' AND name = 'bedroom' AND timestamp = $1"#,
            timestamp.into_db(),
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(stored.value, 12.5);
        assert!(!stored.year_end);

        let sqlite_row: (String, String, f64, i64, bool) = db.for_reading()?.query_row(
            "SELECT type, name, value, timestamp_us, year_end FROM energy_reading ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )?;
        assert_eq!(
            sqlite_row,
            ("heating".into(), "bedroom".into(), 12.5, timestamp.unix_micros(), false)
        );

        let total = tokio::time::timeout(std::time::Duration::from_secs(1), readings.recv())
            .await?
            .context("energy meter source closed")?;
        let EnergyReading::Heating(radiator, value) = total.value;
        assert_eq!(radiator, Radiator::Bedroom);
        assert_eq!(value, 312.5);
        assert_eq!(total.timestamp, timestamp);

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn database_failure_returns_error_without_emitting_a_reading(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let (_file, db) = test_database()?;
        let bus = EventBus::new(8);
        let mut readings = bus.subscribe();
        let meter = EnergyMeter::new(pool.clone(), db.clone(), bus.emitter())?;
        let app = test::init_service(App::new().service(meter.new_web_service())).await;
        let request = || {
            test::TestRequest::put()
                .uri("/api/energy/readings/heating")
                .set_json(json!({ "label": "Schlafzimmer", "value": "12.5" }))
                .to_request()
        };

        db.for_writing()?.execute_batch(
            "CREATE TRIGGER reject_insert BEFORE INSERT ON energy_reading BEGIN SELECT RAISE(ABORT, 'test'); END;",
        )?;
        let response = test::call_service(&app, request()).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let postgres_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM energy_reading")
            .fetch_one(&pool)
            .await?;
        assert_eq!(postgres_rows, 1);

        db.for_writing()?.execute_batch("DROP TRIGGER reject_insert;")?;
        pool.close().await;
        let response = test::call_service(&app, request()).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let sqlite_rows: i64 = db
            .for_reading()?
            .query_row("SELECT count(*) FROM energy_reading", [], |row| row.get(0))?;
        assert_eq!(sqlite_rows, 0);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), readings.recv())
                .await
                .is_err()
        );
        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn non_finite_readings_are_rejected_before_either_write(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let (_file, db) = test_database()?;
        let bus = EventBus::new(8);
        let meter = EnergyMeter::new(pool.clone(), db.clone(), bus.emitter())?;
        let app = test::init_service(App::new().service(meter.new_web_service())).await;
        for value in ["NaN", "inf", "-inf"] {
            let request = test::TestRequest::put()
                .uri("/api/energy/readings/heating")
                .set_json(json!({ "label": "Schlafzimmer", "value": value }))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        let postgres_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM energy_reading")
            .fetch_one(&pool)
            .await?;
        let sqlite_rows: i64 = db
            .for_reading()?
            .query_row("SELECT count(*) FROM energy_reading", [], |row| row.get(0))?;
        assert_eq!((postgres_rows, sqlite_rows), (0, 0));
        Ok(())
    }
}
