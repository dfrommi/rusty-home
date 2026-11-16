mod correlation_id;
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

use tracing_subscriber::reload;

pub use correlation_id::CorrelationId;
pub use trace::TraceContext;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

//KNOWN ISSUES:
// - EnvFilter on layer-level looses log-statements! Try again in a later version by adding statements in between planning
// - OpenTelemetry log appender doesn't contain trace-id and attributes from the span. Issue and PR open
//     https://github.com/open-telemetry/opentelemetry-rust/pull/2438

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

type FilterReloader = Box<dyn Fn(EnvFilter) -> anyhow::Result<()> + Send + Sync>;
pub struct MonitoringControl {
    normal_logs: EnvFilter,
    normal_traces: Option<EnvFilter>,
    log_reloaders: Vec<FilterReloader>,
    trace_reloader: Option<FilterReloader>,
}

pub struct BootstrapMonitoring<'a> {
    monitoring: &'a MonitoringControl,
    restored: bool,
}
impl MonitoringConfig {
    pub fn init(&self) -> anyhow::Result<MonitoringControl> {
        let resource = Resource::builder()
            .with_attribute(KeyValue::new("service.namespace", "smart-home"))
            .with_attribute(KeyValue::new("service.name", self.service_name.clone()))
            .build();

        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::default());

        let normal_logs: EnvFilter = self.logs.clone().try_into()?;
        let mut log_reloaders: Vec<FilterReloader> = Vec::new();
        let (normal_traces, trace_reloader) = if let Some(otlp_config) = &self.otlp {
            let (fmt_filter, fmt_handle) = reload::Layer::new(normal_logs.clone());
            let fmt_layer = tracing_subscriber::fmt::layer().with_filter(fmt_filter);
            log_reloaders.push(Box::new(move |filter| {
                fmt_handle
                    .reload(filter)
                    .map_err(|error| anyhow::anyhow!(error.to_string()))
            }));
            //TODO check 0.32 release when available: are tracing-field added to log events?
            let logger_provider = init_logs(resource.clone(), otlp_config.url.clone())?;
            let (logging_filter, logging_handle) = reload::Layer::new(normal_logs.clone());
            let logging_layer = OpenTelemetryTracingBridge::new(&logger_provider).with_filter(logging_filter);
            log_reloaders.push(Box::new(move |filter| {
                logging_handle
                    .reload(filter)
                    .map_err(|error| anyhow::anyhow!(error.to_string()))
            }));
            let tracer_provider = init_traces(resource.clone(), otlp_config.url.clone())?;
            let tracer = tracer_provider.tracer(self.app_name.to_owned());
            let normal_traces: EnvFilter = self.traces.clone().try_into()?;
            let (tracing_filter, tracing_handle) = reload::Layer::new(normal_traces.clone());
            let tracing_layer = OpenTelemetryLayer::new(tracer).with_filter(tracing_filter);
            let metrics = init_metrics(resource.clone(), otlp_config.url.clone())?;
            opentelemetry::global::set_meter_provider(metrics);

            tracing_subscriber::registry()
                .with(tracing_layer)
                .with(logging_layer)
                .with(fmt_layer)
                .init();

            let trace_reloader: FilterReloader = Box::new(move |filter| {
                tracing_handle
                    .reload(filter)
                    .map_err(|error| anyhow::anyhow!(error.to_string()))
            });

            (Some(normal_traces), Some(trace_reloader))
        } else {
            let (logging_filter, logging_handle) = reload::Layer::new(normal_logs.clone());
            let fmt_layer = tracing_subscriber::fmt::layer();
            tracing_subscriber::registry()
                .with(fmt_layer)
                .with(logging_filter)
                .init();

            log_reloaders.push(Box::new(move |filter| {
                logging_handle
                    .reload(filter)
                    .map_err(|error| anyhow::anyhow!(error.to_string()))
            }));
            (None, None)
        };

        Ok(MonitoringControl {
            normal_logs,
            normal_traces,
            log_reloaders,
            trace_reloader,
        })
    }
}

impl MonitoringControl {
    pub fn enter_bootstrap_mode(&self) -> anyhow::Result<BootstrapMonitoring<'_>> {
        for reload in &self.log_reloaders {
            reload(EnvFilter::new("warn"))?;
        }

        if let Some(reload) = &self.trace_reloader {
            reload(EnvFilter::new("off"))?;
        }

        Ok(BootstrapMonitoring {
            monitoring: self,
            restored: false,
        })
    }

    fn restore_normal_mode(&self) -> anyhow::Result<()> {
        for reload in &self.log_reloaders {
            reload(self.normal_logs.clone())?;
        }

        if let (Some(reload), Some(normal_traces)) = (&self.trace_reloader, &self.normal_traces) {
            reload(normal_traces.clone())?;
        }

        Ok(())
    }
}

impl BootstrapMonitoring<'_> {
    pub fn finish(mut self) -> anyhow::Result<()> {
        let result = self.monitoring.restore_normal_mode();
        if result.is_ok() {
            self.restored = true;
        }
        result
    }
}

impl Drop for BootstrapMonitoring<'_> {
    fn drop(&mut self) {
        if !self.restored {
            let _ = self.monitoring.restore_normal_mode();
        }
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
