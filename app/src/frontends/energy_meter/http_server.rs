use actix_web::web::{self, Json};
use actix_web::{HttpResponse, error::ErrorInternalServerError};
use infrastructure::EventEmitter;
use serde::Deserialize;

use crate::{core::timeseries::DataPoint, t};

use super::{EnergyReading, Radiator, persistence::EnergyReadingRepository};

type EnergyReadingSender = EventEmitter<DataPoint<EnergyReading>>;

pub fn new_actix_web_scope(repo: EnergyReadingRepository, events: EnergyReadingSender) -> actix_web::Scope {
    web::scope("/api/energy/readings")
        .route("/heating", web::put().to(handle_heating_reading))
        .app_data(web::Data::new(repo))
        .app_data(web::Data::new(events))
}

#[derive(Debug, Deserialize)]
struct HeatingReadingDTO {
    label: String,
    value: String,
}

async fn handle_heating_reading(
    repo: web::Data<EnergyReadingRepository>,
    sender: web::Data<EnergyReadingSender>,
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
        Ok(v) => v,
        Err(_) => return Ok(HttpResponse::BadRequest().finish()),
    };

    let reading = EnergyReading::Heating(radiator, value);

    tracing::info!("Received reading {:?}", reading);

    let id = repo
        .add_yearly_energy_reading(reading, t!(now))
        .await
        .map_err(ErrorInternalServerError)?;
    let total = repo
        .get_total_reading_by_id(id)
        .await
        .map_err(ErrorInternalServerError)?;
    sender.send(total);

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
        frontends::energy_meter::{EnergyMeter, EnergyReading, Radiator},
    };

    #[sqlx::test(migrations = "../migrations")]
    async fn http_reading_is_stored_as_yearly_and_emitted_as_cumulative_reading(
        pool: sqlx::PgPool,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO energy_reading (type, name, value, timestamp, year_end) VALUES
                ('heating', 'bedroom', 100, '2023-12-31T23:00:00Z', true),
                ('heating', 'bedroom', 200, '2024-12-31T23:00:00Z', true),
                ('heating', 'bedroom', 50, '2024-06-01T12:00:00Z', false),
                ('heating', 'kitchen', 900, '2024-12-31T23:00:00Z', true),
                ('heating', 'bedroom', 400, '2025-12-31T23:00:00Z', true)"#,
        )
        .execute(&pool)
        .await?;

        let bus = EventBus::new(8);
        let mut readings = bus.subscribe();
        let app =
            test::init_service(App::new().service(EnergyMeter::new_web_service(pool.clone(), bus.emitter()))).await;
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

        let total = readings.recv().await.context("energy meter source closed")?;
        let EnergyReading::Heating(radiator, value) = total.value;
        assert_eq!(radiator, Radiator::Bedroom);
        assert_eq!(value, 312.5);
        assert_eq!(total.timestamp, timestamp);

        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn database_failure_returns_error_without_emitting_a_reading(pool: sqlx::PgPool) {
        let bus = EventBus::new(8);
        let mut readings = bus.subscribe();
        let app =
            test::init_service(App::new().service(EnergyMeter::new_web_service(pool.clone(), bus.emitter()))).await;
        pool.close().await;

        let request = test::TestRequest::put()
            .uri("/api/energy/readings/heating")
            .set_json(json!({ "label": "Schlafzimmer", "value": "12.5" }))
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), readings.recv())
                .await
                .is_err()
        );
    }
}
