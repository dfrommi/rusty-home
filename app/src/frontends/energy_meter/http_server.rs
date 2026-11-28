use std::sync::Arc;

use actix_web::web::{self, Json};
use actix_web::{
    HttpResponse,
    error::{ErrorBadRequest, ErrorInternalServerError},
};
use serde::Deserialize;

use crate::t;

use super::{
    EnergyReading, Radiator,
    service::{EnergyReadingService, InvalidEnergyReading},
};

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

    service.add_reading(reading, t!(now)).await.map_err(|error| {
        if error.downcast_ref::<InvalidEnergyReading>().is_some() {
            ErrorBadRequest(error)
        } else {
            ErrorInternalServerError(error)
        }
    })?;

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

    #[actix_web::test]
    async fn http_reading_is_stored_and_emitted_as_cumulative_reading() -> anyhow::Result<()> {
        let (_file, db) = test_database()?;
        let bus = EventBus::new(8);
        let mut readings = bus.subscribe();
        let meter = EnergyMeter::new(db.clone(), bus.emitter())?;
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

        let stored: (f64, bool) = db.for_reading()?.query_row(
            "SELECT value, year_end FROM energy_reading WHERE type = 'heating' AND name = 'bedroom' AND timestamp_us = ?1 ORDER BY id DESC LIMIT 1",
            [timestamp.unix_micros()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(stored, (12.5, false));

        let total = tokio::time::timeout(std::time::Duration::from_secs(1), readings.recv())
            .await?
            .context("energy meter source closed")?;
        let EnergyReading::Heating(radiator, value) = total.value;
        assert_eq!(radiator, Radiator::Bedroom);
        assert_eq!(value, 312.5);
        assert_eq!(total.timestamp, timestamp);

        Ok(())
    }

    #[actix_web::test]
    async fn readings_cannot_decrease_unless_previous_reading_was_year_end() -> anyhow::Result<()> {
        let (_file, db) = test_database()?;
        let bus = EventBus::new(8);
        let meter = EnergyMeter::new(db.clone(), bus.emitter())?;
        let app = test::init_service(App::new().service(meter.new_web_service())).await;
        seed_heating_readings(
            &db,
            &[
                ("bedroom", 100.0, "2025-01-01T00:00:00Z", true),
                ("kitchen", 100.0, "2025-01-01T00:00:00Z", false),
                ("bathroom", 100.0, "2025-01-01T00:00:00Z", false),
            ],
        )?;
        let request = |label: &str, value: &str| {
            test::TestRequest::put()
                .uri("/api/energy/readings/heating")
                .set_json(json!({ "label": label, "value": value }))
                .to_request()
        };

        let reset_time = DateTime::from_static_iso("2025-01-02T00:00:00Z");
        let response = reset_time
            .eval_timeshifted(test::call_service(&app, request("Schlafzimmer", "10")))
            .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let after_reset_time = DateTime::from_static_iso("2025-01-03T00:00:00Z");
        let response = after_reset_time
            .eval_timeshifted(test::call_service(&app, request("Schlafzimmer", "9")))
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let response = after_reset_time
            .eval_timeshifted(test::call_service(&app, request("Küche", "99")))
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let response = after_reset_time
            .eval_timeshifted(test::call_service(&app, request("Bad", "100")))
            .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let sqlite_rows: i64 = db
            .for_reading()?
            .query_row("SELECT count(*) FROM energy_reading", [], |row| row.get(0))?;
        assert_eq!(sqlite_rows, 5);

        Ok(())
    }

    #[actix_web::test]
    async fn sqlite_failure_returns_error_without_emitting_a_reading() -> anyhow::Result<()> {
        let (_file, db) = test_database()?;
        let bus = EventBus::new(8);
        let mut readings = bus.subscribe();
        let meter = EnergyMeter::new(db.clone(), bus.emitter())?;
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
        let rows: i64 = db
            .for_reading()?
            .query_row("SELECT count(*) FROM energy_reading", [], |row| row.get(0))?;
        assert_eq!(rows, 0);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), readings.recv())
                .await
                .is_err()
        );
        Ok(())
    }

    #[actix_web::test]
    async fn invalid_values_are_rejected_before_writing() -> anyhow::Result<()> {
        let (_file, db) = test_database()?;
        let bus = EventBus::new(8);
        let meter = EnergyMeter::new(db.clone(), bus.emitter())?;
        let app = test::init_service(App::new().service(meter.new_web_service())).await;
        for value in ["NaN", "inf", "-inf", "-1"] {
            let request = test::TestRequest::put()
                .uri("/api/energy/readings/heating")
                .set_json(json!({ "label": "Schlafzimmer", "value": value }))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        let rows: i64 = db
            .for_reading()?
            .query_row("SELECT count(*) FROM energy_reading", [], |row| row.get(0))?;
        assert_eq!(rows, 0);
        Ok(())
    }
}
