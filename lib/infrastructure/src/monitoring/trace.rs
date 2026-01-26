use std::collections::HashMap;

use opentelemetry::{propagation::TextMapPropagator, trace::TraceContextExt};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug, Clone)]
pub struct TraceContext {
    otel_ctx: opentelemetry::Context,
}

impl TraceContext {
    pub fn current() -> Option<Self> {
        let ctx: opentelemetry::Context = Span::current().context();
        let span = ctx.span();
        let span_context = span.span_context();

        if span_context.is_valid() {
            Some(Self { otel_ctx: ctx })
        } else {
            None
        }
    }

    pub fn from_correlation_id(correlation_id: &str) -> Self {
        let propagator = TraceContextPropagator::default();

        let mut ctx: HashMap<String, String> = HashMap::new();
        ctx.insert("traceparent".to_string(), correlation_id.to_string());

        let otel_ctx = propagator.extract(&ctx);

        Self { otel_ctx }
    }

    pub fn for_span(span: tracing::Span) -> Self {
        let ctx: opentelemetry::Context = span.context();
        Self { otel_ctx: ctx }
    }

    pub fn continue_from(correlation_id: &Option<String>) {
        if let Some(id) = correlation_id {
            Self::from_correlation_id(id.as_str()).make_parent();
        }
    }

    pub fn make_parent_of(&self, span: &tracing::Span) {
        span.set_parent(self.otel_ctx.clone());
    }

    pub fn make_parent(&self) {
        self.make_parent_of(&tracing::Span::current());
    }

    pub fn current_correlation_id() -> Option<String> {
        Self::current().map(|c| c.correlation_id())
    }

    pub fn correlation_id(&self) -> String {
        let propagator = TraceContextPropagator::default();

        let mut headers: HashMap<String, String> = HashMap::new();
        propagator.inject_context(&self.otel_ctx, &mut headers);

        headers.get("traceparent").cloned().unwrap_or_default().to_string()
    }

    pub fn trace_id(&self) -> String {
        self.otel_ctx.span().span_context().trace_id().to_string()
    }

    pub fn span_id(&self) -> String {
        self.otel_ctx.span().span_context().span_id().to_string()
    }

    pub fn set_current_span_name(name: String) {
        tracing::Span::current().record("otel.name", name.to_string());
    }
}

#[cfg(test)]
mod tests {
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    use super::*;

    #[test]
    fn test_trace_context_from_correlation_id() {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::default());

        let ctx = TraceContext::from_correlation_id("00-4318fb888997822f5d20fc5c5793c0dc-1075ceed63969488-00");

        assert_eq!(ctx.trace_id(), "4318fb888997822f5d20fc5c5793c0dc");
        assert_eq!(ctx.span_id(), "1075ceed63969488");
    }
}
