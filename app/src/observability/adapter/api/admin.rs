use std::{collections::HashSet, sync::Arc};

use actix_web::{
    Error, HttpResponse,
    web::{self, Query},
};
use serde::Deserialize;

use crate::{
    core::{
        id::ExternalId,
        time::{DateTime, DateTimeRange},
    },
    device_state::{DeviceStateClient, DeviceStateId},
    home_state::{HomeStateClient, HomeStateId},
    observability::{
        adapter::{
            MetricsAdapter, device_metrics::DeviceMetricsAdapter, home_metrics::HomeMetricsAdapter,
            repository::VictoriaRepository,
        },
        domain::{Metric, MetricId},
    },
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

    #[serde(flatten)]
    range: BackfillTimeRangeQuery,
}

#[derive(Debug, Clone, Deserialize)]
struct BackfillTimeRangeQuery {
    start: Option<DateTime>,
    end: Option<DateTime>,
}

impl BackfillTimeRangeQuery {
    fn to_range(&self) -> DateTimeRange {
        //TODO find solution for const timestamps / check at compile time?
        let absolute_min = DateTime::from_iso("2023-10-01T12:00:00+02:00").expect("Invalid ISO datetime in backfill");
        let now = t!(now);

        let min_dt = self.start.unwrap_or(absolute_min).max(absolute_min);
        let max_dt = self.end.unwrap_or(now).min(now);

        DateTimeRange::new(min_dt, max_dt)
    }
}

impl BackfillQuery {
    fn split(csv: &Option<String>) -> Vec<String> {
        csv.iter()
            .flat_map(|n| n.split(','))
            .map(|s| s.trim().to_string())
            .collect()
    }

    fn contains(list: &[String], s: ExternalId) -> bool {
        list.iter().any(|n| s.to_string().starts_with(n))
    }

    fn matching_variants<T>(&self, items: Vec<T>) -> Vec<T>
    where
        for<'a> &'a T: Into<ExternalId>,
    {
        let names = Self::split(&self.name);
        let excluded_names = Self::split(&self.exclude);

        items
            .into_iter()
            .filter(|s| self.all || Self::contains(&names, s.into()))
            .filter(|s| !Self::contains(&excluded_names, s.into()))
            .collect()
    }
}

pub fn routes(
    repo: Arc<VictoriaRepository>,
    device_client: Arc<DeviceStateClient>,
    home_state_client: Arc<HomeStateClient>,
) -> actix_web::Scope {
    web::scope("/metrics")
        .route("/home/names", web::get().to(home_state_names_handler))
        .route("/home/backfill", web::get().to(backfill_handler_home))
        .route("/device/names", web::get().to(device_state_names_handler))
        .route("/device/backfill", web::get().to(backfill_handler_device))
        .app_data(web::Data::from(repo))
        .app_data(web::Data::from(device_client))
        .app_data(web::Data::from(home_state_client))
}

async fn backfill_handler_device(
    repo: web::Data<VictoriaRepository>,
    client: web::Data<DeviceStateClient>,
    query: Query<BackfillQuery>,
) -> Result<HttpResponse, Error> {
    let full_range = query.range.to_range();
    tracing::info!("Backfilling device state metrics for range {}", full_range);

    let variants = query.matching_variants(DeviceStateId::variants());

    let mut batch = MetricsBackfillWriter::new(20000, repo.into_inner());

    for range in full_range.chunked(t!(30 days)) {
        tracing::info!("Processing range {}", range);

        let data = client.get_all_data_points_in_range(range.clone()).await.map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Error fetching device state data points from DeviceStateClient: {}",
                e
            ))
        })?;

        for dt in range.step_by(t!(30 seconds)) {
            for (id, df) in data.iter() {
                if !variants.contains(id) {
                    continue;
                }

                let dp = match df.prev_or_at(dt) {
                    Some(dp) => dp.clone().at(dt),
                    None => continue,
                };

                for metric in DeviceMetricsAdapter.to_metrics(dp) {
                    batch.push(metric).await.map_err(|e| {
                        actix_web::error::ErrorInternalServerError(format!(
                            "Error buffering metrics for VictoriaMetrics: {}",
                            e
                        ))
                    })?;
                }
            }
        }

        batch.flush().await.map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Error flushing metrics batch to VictoriaMetrics: {}",
                e
            ))
        })?;
    }

    Ok(HttpResponse::NoContent().finish())
}

async fn backfill_handler_home(
    repo: web::Data<VictoriaRepository>,
    client: web::Data<HomeStateClient>,
    query: Query<BackfillQuery>,
) -> Result<HttpResponse, Error> {
    let full_range = query.range.to_range();
    tracing::info!("Backfilling home state metrics for range {}", full_range);

    let variants = query.matching_variants(HomeStateId::variants());

    let mut batch = MetricsBackfillWriter::new(20000, repo.into_inner());
    let mut snapshot_iter = client.snapshot_iter(full_range);

    while let Some(snapshot) = snapshot_iter.next().await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Error fetching home state snapshot from HomeStateClient: {}",
            e
        ))
    })? {
        for id in variants.iter() {
            if let Some(dp) = snapshot.get(*id) {
                for metric in HomeMetricsAdapter.to_metrics(dp.clone()).into_iter() {
                    batch.push(metric).await.map_err(|e| {
                        actix_web::error::ErrorInternalServerError(format!(
                            "Error buffering metrics for VictoriaMetrics: {}",
                            e
                        ))
                    })?;
                }
            }
        }
    }

    batch.flush().await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Error flushing metrics batch to VictoriaMetrics: {}", e))
    })?;

    Ok(HttpResponse::NoContent().finish())
}

async fn home_state_names_handler() -> Result<HttpResponse, Error> {
    let variants = HomeStateId::variants();
    let items: Vec<String> = variants.iter().map(|s| s.ext_id().to_string()).collect();

    Ok(HttpResponse::Ok().body(items.join("\n")))
}

async fn device_state_names_handler() -> Result<HttpResponse, Error> {
    let variants = DeviceStateId::variants();
    let items: Vec<String> = variants.iter().map(|s| s.ext_id().to_string()).collect();

    Ok(HttpResponse::Ok().body(items.join("\n")))
}

struct MetricsBackfillWriter {
    repo: Arc<VictoriaRepository>,
    buffer: Vec<Metric>,
    deleted: HashSet<MetricId>,
    capacity: usize,
}

impl MetricsBackfillWriter {
    fn new(capacity: usize, repo: Arc<VictoriaRepository>) -> Self {
        Self {
            repo,
            buffer: Vec::with_capacity(capacity),
            deleted: HashSet::new(),
            capacity,
        }
    }

    async fn push(&mut self, metric: Metric) -> anyhow::Result<()> {
        self.delete_if_needed(metric.id.clone()).await?;

        self.buffer.push(metric);

        if self.buffer.len() >= self.capacity {
            self.flush().await?
        }

        Ok(())
    }

    async fn flush(&mut self) -> anyhow::Result<()> {
        if !self.buffer.is_empty() {
            self.repo.push(&self.buffer).await?;
            self.buffer.clear();
        }

        Ok(())
    }

    async fn delete_if_needed(&mut self, metric_id: MetricId) -> anyhow::Result<()> {
        if self.deleted.insert(metric_id.clone()) {
            tracing::info!("Deleting existing data for metric: {}", metric_id);
            self.repo.delete_series(metric_id).await?;
        }

        Ok(())
    }
}
