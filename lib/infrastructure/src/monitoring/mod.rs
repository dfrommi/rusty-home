pub mod meter;
mod trace;

use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_opentelemetry::OpenTelemetryLayer;

use tracing_subscriber::layer::SubscriberExt;

use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{ExporterBuildError, WithExportConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use std::error::Error;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

pub use trace::TraceContext;

//KNOWN ISSUES:
// - EnvFilter on layer-level looses log-statements! Try again in a later version by adding statements in between planning
// - OpenTelemetry log appender doesn't contain trace-id and attributes from the span. Issue and PR open

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MonitoringConfig {
    pub service_name: String,
    pub app_name: String,
    pub logs: EnvFilterConfig,
    pub traces: EnvFilterConfig,
    pub otlp: Option<OtlpConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EnvFilterConfig {
    pub default_level: String,
    pub filters: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct OtlpConfig {
    pub url: Option<String>,
}

impl TryInto<EnvFilter> for EnvFilterConfig {
    type Error = tracing_subscriber::filter::ParseError;

    fn try_into(self) -> Result<EnvFilter, Self::Error> {
        EnvFilter::builder()
            .with_default_directive(self.default_level.parse()?)
            .parse(self.filters.join(","))
    }
}

impl MonitoringConfig {
    pub fn init(&self) -> Result<(), Box<dyn Error>> {
        let resource = Resource::builder()
            .with_attribute(KeyValue::new("service.name", self.service_name.clone()))
            .with_attribute(KeyValue::new("app.name", self.app_name.clone()))
            .build();

        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::default());

        if let Some(otlp_config) = &self.otlp {
            let fmt_filter: EnvFilter = self.logs.clone().try_into()?;
            let fmt_layer = tracing_subscriber::fmt::layer().with_filter(fmt_filter);

            let logger_provider = init_logs(resource.clone(), otlp_config.url.clone())?;
            let logging_filter: EnvFilter = self.logs.clone().try_into()?;
            let logging_layer = OpenTelemetryTracingBridge::new(&logger_provider).with_filter(logging_filter);

            let tracer_provider = init_traces(resource.clone(), otlp_config.url.clone())?;
            let tracer = tracer_provider.tracer(self.app_name.to_owned());
            let tracing_filter: EnvFilter = self.traces.clone().try_into()?;
            let tracing_layer = OpenTelemetryLayer::new(tracer).with_filter(tracing_filter);

            let metrics = init_metrics(resource.clone(), otlp_config.url.clone())?;
            opentelemetry::global::set_meter_provider(metrics);

            tracing_subscriber::registry()
                .with(tracing_layer)
                .with(logging_layer)
                .with(fmt_layer)
                .init();
        } else {
            let logging_filter: EnvFilter = self.logs.clone().try_into()?;
            let fmt_layer = tracing_subscriber::fmt::layer();
            tracing_subscriber::registry()
                .with(fmt_layer)
                .with(logging_filter)
                .init();
        }

        Ok(())
    }
}

fn init_traces(resource: Resource, url: Option<String>) -> Result<SdkTracerProvider, ExporterBuildError> {
    match url {
        Some(url) => {
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(url)
                .build()?;
            Ok(SdkTracerProvider::builder()
                .with_resource(resource)
                .with_batch_exporter(exporter)
                .build())
        }
        None => Ok(SdkTracerProvider::builder()
            .with_resource(resource)
            .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
            .build()),
    }
}

fn init_metrics(
    resource: Resource,
    url: Option<String>,
) -> Result<opentelemetry_sdk::metrics::SdkMeterProvider, ExporterBuildError> {
    match url {
        Some(url) => {
            let exporter = opentelemetry_otlp::MetricExporter::builder()
                .with_tonic()
                .with_endpoint(url)
                .build()?;
            let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter)
                .with_interval(std::time::Duration::from_secs(15))
                .build();

            Ok(opentelemetry_sdk::metrics::SdkMeterProvider::builder()
                .with_reader(reader)
                .with_resource(resource)
                .build())
        }
        None => {
            let reader =
                opentelemetry_sdk::metrics::PeriodicReader::builder(opentelemetry_stdout::MetricExporter::default())
                    .with_interval(std::time::Duration::from_secs(5))
                    .build();

            Ok(opentelemetry_sdk::metrics::SdkMeterProvider::builder()
                .with_reader(reader)
                .with_resource(resource)
                .build())
        }
    }
}

fn init_logs(resource: Resource, url: Option<String>) -> Result<SdkLoggerProvider, ExporterBuildError> {
    match url {
        Some(url) => {
            let exporter = opentelemetry_otlp::LogExporter::builder()
                .with_tonic()
                .with_endpoint(url)
                .build()?;

            Ok(SdkLoggerProvider::builder()
                .with_resource(resource)
                .with_batch_exporter(exporter)
                .build())
        }
        None => Ok(SdkLoggerProvider::builder()
            .with_resource(resource)
            .with_simple_exporter(opentelemetry_stdout::LogExporter::default())
            .build()),
    }
}
