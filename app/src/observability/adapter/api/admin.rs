use std::sync::Arc;

use actix_web::{
    Error, HttpResponse,
    web::{self, Query},
};
use serde::Deserialize;

use crate::{
    core::time::{DateTime, DateTimeRange},
    home_state::HomeStateId,
    observability::{adapter::repository::VictoriaRepository, domain::Metric},
    t,
};

#[derive(Debug, Clone, Deserialize)]
struct BackfillQuery {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    all: bool,
    #[serde(default)]
    exclude: Option<String>,

    start: Option<DateTime>,
    end: Option<DateTime>,
}

impl BackfillQuery {
    fn split(csv: &Option<String>) -> Vec<String> {
        csv.iter()
            .flat_map(|n| n.split(','))
            .map(|s| s.trim().to_string())
            .collect()
    }

    fn contains(list: &[String], s: &HomeStateId) -> bool {
        list.iter().any(|n| s.ext_id().to_string().starts_with(n))
    }

    fn matching_variants(&self) -> Vec<HomeStateId> {
        let variants = HomeStateId::variants();
        let names = Self::split(&self.name);
        let excluded_names = Self::split(&self.exclude);

        variants
            .into_iter()
            .filter(|s| self.all || Self::contains(&names, s))
            .filter(|s| !Self::contains(&excluded_names, s))
            .collect()
    }
}

pub fn routes(repo: Arc<VictoriaRepository>) -> actix_web::Scope {
    web::scope("/metrics")
        .route("/backfill", web::get().to(backfill_handler))
        .route("/names", web::get().to(items_handler))
        .app_data(web::Data::from(repo))
}

async fn backfill_handler(
    repo: web::Data<VictoriaRepository>,
    query: Query<BackfillQuery>,
) -> Result<HttpResponse, Error> {
    //Date where data collection started
    let absolute_min_dt = DateTime::from_iso("2023-10-01T12:00:00+02:00")
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Error parsing min datetime: {}", e)))?;

    const BATCH_SIZE: usize = 20000;
    let buffer: Vec<Metric> = Vec::with_capacity(BATCH_SIZE);

    let min_dt = query.start.unwrap_or(absolute_min_dt).max(absolute_min_dt);
    let max_dt = query.end.unwrap_or(t!(now)).min(t!(now));

    let variants = query.matching_variants();
    let variants_names = variants.iter().map(|s| s.ext_id().to_string()).collect::<Vec<_>>();

    if variants.is_empty() {
        return Ok(HttpResponse::BadRequest().body("No matching home state variants found"));
    }

    let full_range = DateTimeRange::new(min_dt, max_dt);
    let rate = t!(15 seconds);

    tracing::info!(
        "Backfilling metrics for range {} in {} steps for items {}",
        full_range,
        rate,
        variants_names.join(", ")
    );

    //Delete existing data
    // TODO handle diffrent naming patterns
    // for state in &variants {
    //     let id = MetricId::from(&state.ext_id());
    //     tracing::info!("Deleting existing data for metric: {}", id);
    //
    //     repo.delete_series(id.clone()).await.map_err(|e| {
    //         actix_web::error::ErrorInternalServerError(format!(
    //             "Error deleting existing metrics {} from VictoriaMetrics: {}",
    //             id, e
    //         ))
    //     })?;
    // }

    for range in full_range.chunked(t!(30 days)) {
        tracing::info!("Processing range {}", range);

        //TODO fix for StateSnapshot
        //     for state in &variants {
        //         //This is expected for states that were added later
        //         let Ok(frame) = state.get_data_frame(range.clone(), &api).await else {
        //             tracing::debug!("No data frame found for state {} in range {}, skipping", state.ext_id(), range);
        //             continue;
        //         };
        //
        //         for dt in range.step_by(rate.clone()) {
        //             let dp_at = frame.prev_or_at(dt);
        //
        //             if let Some(dp) = dp_at {
        //                 let dp = DataPoint::new(dp.value.clone(), dt);
        //                 buffer.push(dp.into());
        //
        //                 if buffer.len() >= BATCH_SIZE {
        //                     ctx.repo.push(&buffer).await.map_err(|e| {
        //                         actix_web::error::ErrorInternalServerError(format!(
        //                             "Error pushing metrics to VictoriaMetrics: {}",
        //                             e
        //                         ))
        //                     })?;
        //
        //                     buffer.clear();
        //                 }
        //             }
        //         }
        //     }
    }

    repo.push(&buffer).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Error pushing metrics to VictoriaMetrics: {}", e))
    })?;

    Ok(HttpResponse::Ok().body(variants_names.join("\n")))
}

async fn items_handler() -> Result<HttpResponse, Error> {
    let variants = HomeStateId::variants();
    let items: Vec<String> = variants.iter().map(|s| s.ext_id().to_string()).collect();

    Ok(HttpResponse::Ok().body(items.join("\n")))
}
