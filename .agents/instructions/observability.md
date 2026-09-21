# Observability

Exports device and home state as Prometheus metrics to VictoriaMetrics. Command execution rows are emitted as structured OTLP logs and queried directly from Loki.

## Non-obvious metric conversions

In `app/src/observability/adapter/home_metrics.rs` (`HomeMetricsAdapter::to_metrics`):

- **Range metrics** (HeatingDemandLimit, SetPoint): generate `_min`/`_max` suffixed metrics
- **Temporal metrics** (TemperatureChange): generate per-window metrics (`_1m`, `_10m`, `_15m`, `_1h`)
- **Enum metrics** (TargetHeatingMode): one metric per variant (as label) with 0.0/1.0 values
- **Scaled enums** (TargetHeatingAdjustment): enum variants map to numeric scale factors (MustIncrease=2.0 … MustOff=-4.0)

## Buffering

Metrics are batched before push in `app/src/observability/mod.rs`: max 500 metrics or 15s flush interval, whichever comes first. All metric timestamps are normalized to "now" when buffered, ensuring consistent timing even if state events arrived out-of-order.

## Cross-reference

See `.agents/observability.md` for the telemetry pipelines, Grafana datasource inventory, and query patterns.
