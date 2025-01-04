use std::collections::HashMap;

use opentelemetry::trace::TraceContextExt;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug, Clone)]
pub struct TraceContext {
    trace_id: String,
    span_id: String,
    traceparent: String,
}

impl TraceContext {
    pub fn current() -> Option<Self> {
        let ctx: opentelemetry::Context = Span::current().context();
        let span = ctx.span();
        let span_context = span.span_context();

        if span_context.is_valid() {
            let mut headers: HashMap<String, String> = HashMap::new();
            opentelemetry::global::get_text_map_propagator(|propagator| {
                propagator.inject_context(&ctx, &mut headers)
            });
            let traceparent = headers.get("traceparent").cloned();

            Some(Self {
                trace_id: span_context.trace_id().to_string(),
                span_id: span_context.span_id().to_string(),
                traceparent: traceparent.unwrap_or_default(),
            })
        } else {
            None
        }
    }

    pub fn from_correlation_id(correlation_id: &str) -> Self {
        let mut ctx: HashMap<String, String> = HashMap::new();
        ctx.insert("traceparent".to_string(), correlation_id.to_string());

        let otel_ctx =
            opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&ctx));
        let otel_span = otel_ctx.span();
        let span_context = otel_span.span_context();

        Self {
            trace_id: span_context.trace_id().to_string(),
            span_id: span_context.span_id().to_string(),
            traceparent: correlation_id.to_string(),
        }
    }

    pub fn current_correlation_id() -> Option<String> {
        Self::current().map(|c| c.correlation_id().to_owned())
    }

    pub fn correlation_id(&self) -> &str {
        &self.traceparent
    }

    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    pub fn span_id(&self) -> &str {
        &self.span_id
    }
}
