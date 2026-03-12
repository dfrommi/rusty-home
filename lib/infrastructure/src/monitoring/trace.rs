use opentelemetry::{KeyValue, trace::TraceContextExt};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::CorrelationId;

#[derive(Debug, Clone)]
pub struct TraceContext {
    span: tracing::Span,
}

impl TraceContext {
    pub fn current() -> Self {
        Self { span: Span::current() }
    }

    pub fn for_span(span: &tracing::Span) -> Self {
        Self { span: span.clone() }
    }

    pub fn make_parent_of(&self, span: &tracing::Span) {
        let _ = span.set_parent(self.otel_ctx());
    }

    pub fn correlation_id(&self) -> Option<CorrelationId> {
        CorrelationId::from_context(&self.otel_ctx())
    }

    pub fn trace_id(&self) -> String {
        self.otel_ctx().span().span_context().trace_id().to_string()
    }

    pub fn span_id(&self) -> String {
        self.otel_ctx().span().span_context().span_id().to_string()
    }

    pub fn set_span_name(&self, name: String) {
        self.otel_ctx().span().update_name(name);
    }

    pub fn set_error(&self, error: impl Into<String>) {
        self.otel_ctx().span().set_status(opentelemetry::trace::Status::Error {
            description: error.into().into(),
        });
    }

    pub fn record(&self, key: impl Into<String>, value: impl Into<String>) {
        self.otel_ctx()
            .span()
            .set_attribute(KeyValue::new(key.into(), value.into()));
    }

    pub fn record_json(&self, key: impl Into<String>, value: &serde_json::Value) {
        if let Ok(value_str) = serde_json::to_string_pretty(&value) {
            self.record(key, value_str);
        }
    }

    fn otel_ctx(&self) -> opentelemetry::Context {
        self.span.context()
    }
}
