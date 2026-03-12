use std::collections::HashMap;

use opentelemetry::{propagation::TextMapPropagator, trace::TraceContextExt};
use opentelemetry_sdk::propagation::TraceContextPropagator;

#[derive(Debug, Clone, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "String", into = "String")]
pub struct CorrelationId {
    id: String,
    trace_id: String,
    span_id: String,
}

impl CorrelationId {
    pub fn parse(correlation_id: impl Into<String>) -> Self {
        let correlation_id = correlation_id.into();
        let propagator = TraceContextPropagator::default();

        let mut ctx: HashMap<String, String> = HashMap::new();
        ctx.insert("traceparent".to_string(), correlation_id.clone());

        let otel_ctx = propagator.extract(&ctx);

        Self {
            id: correlation_id,
            trace_id: otel_ctx.span().span_context().trace_id().to_string(),
            span_id: otel_ctx.span().span_context().span_id().to_string(),
        }
    }

    pub(super) fn from_context(otel_ctx: &opentelemetry::Context) -> Option<Self> {
        let is_valid = otel_ctx.span().span_context().is_valid();
        if !is_valid {
            return None;
        }

        let propagator = TraceContextPropagator::default();

        let mut headers: HashMap<String, String> = HashMap::new();
        propagator.inject_context(otel_ctx, &mut headers);

        Some(Self {
            id: headers.get("traceparent")?.to_string(),
            trace_id: otel_ctx.span().span_context().trace_id().to_string(),
            span_id: otel_ctx.span().span_context().span_id().to_string(),
        })
    }

    pub fn trace_id(&self) -> String {
        self.trace_id.clone()
    }

    pub fn span_id(&self) -> String {
        self.span_id.clone()
    }
}

impl PartialEq for CorrelationId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<CorrelationId> for String {
    fn from(val: CorrelationId) -> Self {
        val.id
    }
}

impl From<String> for CorrelationId {
    fn from(val: String) -> Self {
        Self::parse(val)
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_from_correlation_id() {
        let ctx = CorrelationId::parse("00-4318fb888997822f5d20fc5c5793c0dc-1075ceed63969488-00");

        assert_eq!(ctx.trace_id(), "4318fb888997822f5d20fc5c5793c0dc");
        assert_eq!(ctx.span_id(), "1075ceed63969488");
    }
}
