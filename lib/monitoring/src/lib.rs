use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_opentelemetry::OpenTelemetryLayer;

use tracing_subscriber::layer::SubscriberExt;

use opentelemetry::trace::TraceError;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::LogError;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::{trace as sdktrace, Resource};
use std::error::Error;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

pub struct Monitoring {}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MonitoringConfig {
    pub service_name: String,
    pub app_name: String,
    pub fmt: FmtConfig,
    pub otlp: Option<OtlpConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct OtlpConfig {
    pub otlp_url: Option<String>,

    pub default_level: String,
    pub filters: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FmtConfig {
    pub default_level: String,
    pub filters: Vec<String>,
}

impl Monitoring {
    pub fn init(config: &MonitoringConfig) -> Result<Self, Box<dyn Error>> {
        let resource = Resource::new(vec![KeyValue::new(
            "service.name",
            config.service_name.clone(),
        )]);

        let fmt_filter = EnvFilter::builder()
            .with_default_directive(config.fmt.default_level.parse()?)
            .parse(config.fmt.filters.join(","))?;
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_thread_names(true)
            .with_filter(fmt_filter);
        let subscriber = tracing_subscriber::registry().with(fmt_layer);

        if let Some(otlp_config) = &config.otlp {
            opentelemetry::global::set_text_map_propagator(TraceContextPropagator::default());

            let logger_provider = init_logs(resource.clone(), otlp_config.otlp_url.clone())?;
            let tracer_provider = init_traces(resource.clone(), otlp_config.otlp_url.clone())?;

            let logging_layer = OpenTelemetryTracingBridge::new(&logger_provider);
            let filter_otel = EnvFilter::builder()
                .with_default_directive(otlp_config.default_level.parse()?)
                .parse(otlp_config.filters.join(","))?;
            let logging_layer = logging_layer.with_filter(filter_otel);

            let tracer = tracer_provider.tracer(config.app_name.to_owned());
            let tracing_layer = OpenTelemetryLayer::new(tracer);

            subscriber.with(tracing_layer).with(logging_layer).init();
        } else {
            subscriber.init();
        }

        Ok(Self {})
    }
}

#[tracing::instrument(fields(hello = "world"))]
fn do_stuff() {
    tracing::info!("Doing stuff");
}

fn init_traces(
    resource: Resource,
    url: Option<String>,
) -> Result<sdktrace::TracerProvider, TraceError> {
    match url {
        Some(url) => {
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(url)
                .build()?;
            Ok(sdktrace::TracerProvider::builder()
                .with_resource(resource)
                .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
                .build())
        }
        None => Ok(sdktrace::TracerProvider::builder()
            .with_resource(resource)
            .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
            .build()),
    }
}

/*
fn init_metrics(
    resource: Resource,
) -> Result<opentelemetry_sdk::metrics::SdkMeterProvider, MetricError> {
    let exporter = MetricExporter::builder().with_tonic().build()?;
    let reader = PeriodicReader::builder(exporter).build();

    Ok(SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(RESOURCE.clone())
        .build())
}
*/

fn init_logs(
    resource: Resource,
    url: Option<String>,
) -> Result<opentelemetry_sdk::logs::LoggerProvider, LogError> {
    match url {
        Some(url) => {
            let exporter = opentelemetry_otlp::LogExporter::builder()
                .with_tonic()
                .with_endpoint(url)
                .build()?;

            Ok(LoggerProvider::builder()
                .with_resource(resource)
                .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
                .build())
        }
        None => Ok(LoggerProvider::builder()
            .with_resource(resource)
            .with_simple_exporter(opentelemetry_stdout::LogExporter::default())
            .build()),
    }
}
