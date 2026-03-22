# Observability

Exports device and home state as Prometheus metrics to VictoriaMetrics. Provides Grafana CSV endpoints.

## Non-obvious metric conversions

- **Range metrics** (HeatingDemandLimit, SetPoint): generate `_min`/`_max` suffixed metrics
- **Temporal metrics** (TemperatureChange): generate per-window metrics (`_1m`, `_10m`, `_15m`, `_1h`)
- **Enum metrics** (TargetHeatingMode): one metric per variant with 0.0/1.0 values

## Buffering

Metrics are batched before push: max 500 metrics or 15s flush interval, whichever comes first. All metric timestamps are normalized to "now" when buffered, ensuring consistent timing even if state events arrived out-of-order.
