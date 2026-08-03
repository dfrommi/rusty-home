# Observability Reference

Where rusty-home's telemetry lives and how it's structured.

**When querying Grafana based on this reference**, load the relevant gcx skill first:

- Querying metrics/logs/traces → `debug-with-grafana`
- Investigating alerts → `investigate-alert`
- Working with dashboards → `create-dashboard` or `manage-dashboards`

If not installed: `gcx agent skills install --dir .agents create-dashboard debug-with-grafana investigate-alert manage-dashboards`

## Architecture

Two independent pipelines, plus Z2M's direct Loki driver:

```
                          ┌──▶ home-tempo (all traces, 72h)
                          │
rusty-home ──OTLP/gRPC──▶ otel-collector ──▶ Grafana Cloud ──▶ grafanacloud-traces (sampled, 30d)
  (host.docker.internal:4317)   │                            grafanacloud-logs
                                │                            grafanacloud-prom
                                └── docker_stats receiver ▶ container_* metrics

DeviceStateEvent ──▶ DeviceMetricsAdapter ──▶ VictoriaRepository ──POST──▶ VictoriaMetrics (5y)
HomeStateEvent   ──▶ HomeMetricsAdapter   ──▶ VictoriaRepository ──POST──▶   "prometheus" container
                                                                             bf2qorruxhon4d

zigbee2mqtt ──Docker Loki driver──▶ grafanacloud-logs (direct, with custom label pipeline)
```

| Component | What | Where |
| --- | --- | --- |
| **rusty-home** | OTLP traces/logs/metrics | `lib/infrastructure/src/monitoring/mod.rs` |
| | VictoriaMetrics push of device/home-state metrics | `app/src/observability/mod.rs` |
| **otel-collector** | Receives OTLP, forks traces, emits `container_*` from Docker socket | `../homelab/infrastructure/otel-collector/config.yaml` |
| **VictoriaMetrics** | Container named "prometheus" but runs `victoriametrics/victoria-metrics` with `-retentionPeriod=5y` | `../homelab/smart-home/compose.yaml` |
| **home-tempo** | Local Tempo, all traces, 72h retention | `../homelab/infrastructure/tempo/tempo.yaml` |
| **grafana-pdc** | Bridges local datasources into Grafana Cloud | `../homelab/infrastructure/compose.yaml` |
| **zigbee2mqtt** | Logs directly to Loki via Docker driver with pipeline parsing `topic`, `device_id`, `level`, `scope_name` — NOT via OTLP | `../homelab/smart-home/compose.yaml` |

**Tail sampling** (cloud traces only): always keep `execute_command*` / `calculate_home_state*` spans; rate-limit `plan_for_home` to 1 per 2min; 10% of rest. **Always use home-tempo for debugging** — you'll miss 59/60 planning loops in cloud.

---

## Datasource Inventory

| UID | Type | What's in it |
| --- | --- | --- |
| `bf2qorruxhon4d` | prometheus (VictoriaMetrics) | **Smart home metrics** — `device_*` and home-state metrics, 5y retention. Push-based (no `up` metric). |
| `grafanacloud-prom` | prometheus | **Infra + OTLP metrics** — `container_*`, `command_executed_total`, `home_state_calculation`, `z2m_state`, `traces_spanmetrics_*`. Free-tier retention. |
| `grafanacloud-logs` | loki | **Application logs** — see [Log Reference](#log-reference) below for label values and query patterns. |
| `df85ynnsf0cu8a` | home-tempo | **All traces, 72h** — use this for debugging. |
| `grafanacloud-traces` | tempo | **Tail-sampled traces, ~30d** — 1/60 `plan_for_home`, all `execute_command*`. |
| `f82fffd9-…` | infinity | Smart Home — used in dashboards only, not queryable via gcx. |
| `da9d089e-…` | influxdb | Not used. |

Ignore: `grafanacloud-knowledgegraph`, `grafanacloud-k6`, `grafanacloud-graphite`, `grafanacloud-ngalertmanager`, `grafanacloud-usage*`, `grafanacloud-ml-metrics`, `grafanacloud-alert-state-history`, `grafanacloud-cardinality-management`, `grafanacloud-profiles` — Grafana Cloud Free Tier defaults.

---

## Metric Naming Conventions

Metrics are created by two adapters. Use `gcx metrics labels -d bf2qorruxhon4d -l __name__` to discover the current set.

### Device metrics (`device_*`)

`DeviceMetricsAdapter` in `app/src/observability/adapter/device_metrics.rs` converts every `DeviceStateValue` variant into a metric named `device_{type_name}`.

Labels: `item` (variant name, e.g. `living_room`), `room` (Wohnzimmer/Schlafzimmer/Küche/Bad/Room of Requirements, if applicable), optionally `friendly_name`.

Special cases:

- `device_heating_demand` also emits `device_heating_demand_scaled` (× scaling factor)
- `device_total_radiator_consumption` also emits `_scaled` variant

### Availability metrics

`AvailabilityMetricsAdapter` in `app/src/observability/adapter/availability_metrics.rs` polls `item_availability` every 60s.

| Metric | Type | Labels | Description |
| --- | --- | --- | --- |
| `device_last_seen_seconds` | gauge | `item`, `source` | Seconds since the device was last seen (max of `now - last_seen` and `now - entry_updated`). Emitted for all devices in `item_availability`. |
| `device_offline` | gauge | `item`, `source` | `1` if offline, `0` if online. A device is offline when `marked_offline` is true OR `max(now - last_seen, now - entry_updated) > considered_offline_after`. |

The **Offline Devices** panel on the Smart Home Overview dashboard uses these metrics via PromQL (`device_last_seen_seconds / 86400 and on(item, source) (device_offline == 1)`); the old Infinity/API-backed panel and the `GET /observability/grafana/overview/offline` endpoint were removed.

### Home-state metrics (no prefix)

`HomeMetricsAdapter` in `app/src/observability/adapter/home_metrics.rs` converts `HomeStateValue` variants. Metric name is the type name directly (e.g. `temperature`, `occupancy`, `heating_demand`).

Labels: same `item`/`room` pattern.

Special cases:

- **Ranges** (`HeatingDemandLimit`, `SetPoint`) — split into two metrics with `_min`/`_max` suffix
- **TemperatureChange** — emits 4 metrics: `temperature_change_1m`, `_10m`, `_15m`, `_1h`
- **TargetHeatingMode** — one-hot encoded with `enum_variant` label (`energy_saving`, `comfort`, `sleep`, `ventilation`, `away`, `manual`)
- **TargetHeatingAdjustment** — encoded as numeric: MustOff=-4, MustDecrease=-2, ShouldDecrease=-1, Hold=0, ShouldIncrease=+1, MustIncrease=+2

### Infra metrics (`grafanacloud-prom`)

`container_*` comes from otel-collector's `docker_stats` receiver. `command_executed_total`, `home_state_calculation`, `z2m_state` come from rusty-home's OTLP meter. `traces_spanmetrics_*` are auto-generated by Tempo.

---

## Log Reference

Datasource: `grafanacloud-logs`. Indexed labels (usable in `{...}` stream selector):

| Label | Values | Notes |
| --- | --- | --- |
| `service_name` | `rusty-home`, `zigbee2mqtt` | Primary filter |
| `container_name` | `zigbee2mqtt` | Only set for Z2M (Docker driver). `rusty-home` has no container_name label. |
| `level` | `info`, `debug`, `warn`, `error` | Parsed from log line by Z2M pipeline; set by OTLP for rusty-home |
| `scope_name` | e.g. `z2m:mqtt`, `app::automation::planner` | Module/scope — parsed by Z2M pipeline; set by OTLP for rusty-home |
| `device_id` | e.g. `bathroom/dehumidifier` | **Z2M only** — parsed from topic by pipeline stages |
| `topic` | e.g. `zigbee/bathroom/dehumidifier` | **Z2M only** — MQTT topic |
| `source` | `stdout` | **Z2M only** |

**rusty-home** logs arrive via OTLP and have: `service_name`, `level`, `scope_name`.  
**zigbee2mqtt** logs arrive via Docker Loki driver with pipeline stages that parse `level`, `scope_name`, `topic`, `device_id` from each log line.

Common starting points:

```logql
# rusty-home errors/warnings
{service_name="rusty-home"} |~ "error|Error|ERROR|warn|WARN"

# rusty-home planning changes (logged at INFO when plan differs from previous)
{service_name="rusty-home"} |= "Planning result changed"

# Z2M traffic for a specific device
{service_name="zigbee2mqtt",device_id="bathroom/dehumidifier"}

# Z2M errors only
{service_name="zigbee2mqtt",level="error"}
```

Discover available label values: `gcx logs labels -d grafanacloud-logs -l <label>`

**Pitfall:** `{job=~".+"}` returns nothing — `job` is not an indexed label. Always start with `service_name`.

**Pitfall:** `app::automation::domain` logs are filtered to `warn` and above in rusty-home's config. Rule evaluation debug/trace messages only appear as span events in traces, not in Loki.

---

## Planning Loop Traces

Root span: `plan_for_home` (`app/src/automation/planner/mod.rs:14`), runs every ~18s, ~220ms, ~76 child spans.

```
plan_for_home
├── SetPower[Dehumidifier]              ← resource span (one per CommandTarget)
│   ├── block_automation::bathroom_...  ← rule (HomeAction) — skipped
│   ├── dehumidify::bathroom            ← rule — skipped
│   └── follow_default_setting::...     ← rule WON → execute_command
│       └── execute_command
│           └── should_execute          ← 3 dedup checks
├── SetHeating[LivingRoomBig]
│   └── follow_target_heating_demand::...
│       └── execute_command → should_execute
│           └── "is_reflected: true"    ← deduped: already in desired state
├── ...
└── "Planning result is unchanged" event
```

**Span naming:** Resource spans use `otel.name` override to the `CommandTarget` Display (e.g. `SetPower[Dehumidifier]`). Rule spans use the `HomeAction` Display (e.g. `dehumidify::bathroom`). Rule spans carry `action` and `resource` attributes.

**Span status:**

- `STATUS_CODE_OK` — rule triggered AND command passed all dedup checks and was forwarded to the backend. These are rare and show actual state changes.
- `STATUS_CODE_UNSET` — rule skipped, or triggered but command was deduplicated. The steady state.
- `STATUS_CODE_ERROR` — evaluation error.

**Command deduplication** (`should_execute` in `app/src/automation/planner/processor.rs:132`):

1. Last execution < 30s ago → skip (waiting for state to propagate)
2. Last execution < `min_wait_duration_between_executions` → skip
3. Already reflected in device state → skip ("nothing to do")

All spans carry `code.file.path`, `code.line.number`, `code.module.name`, `busy_ns`, `idle_ns`, `thread.id`, `thread.name`. Span events contain the `tracing::info!`/`debug!` messages — rule evaluation logs at debug/trace level only appear here, not in Loki (because `app::automation::domain` is suppressed to `warn` in the log filter).

---

## Dashboards & Alerts

| Dashboard | UID | Notes |
| --- | --- | --- |
| Smart Home Overview | `be8vajnp7stmoe` | Main overview |
| Energiemonitor | `a358001b-…` | Energy monitoring |
| Energy IQ | `ce84bz10sh88we` | Energy insights |
| GOAP Monitor | `b617ab80-…` | Automation planner |
| State Machine | `d3bb14dc-…` | Device state machine |
| Heizen Details | `e13c9ce3-…` | Heating details |
| Luftfeuchtigkeit | `dfpxq6p` | Humidity |
| Sonoff debug | `dffpp9q` | Sonoff device debug |

| Alert | State | Paused |
| --- | --- | --- |
| `Heating without Request` | inactive | no |
| `Too many Z2M retries` | inactive | **yes** ⚠️ |

---

## Pitfalls

1. **`gcx datasources list --json` omits the `type` field.** Use `-o json` for full output; `--json field1,field2` for compact.

2. **Smart Home datasource is Infinity, not Prometheus.** Don't use `f82fffd9` with `gcx metrics query`.

3. **`is_running` ≠ service health.** It's a per-device on/off state. The TV being off shows `is_running=0`. For service health, check `container_restarts_total`, trace freshness, or log activity.

4. **`gcx` writes hints to stderr.** Use `2>/dev/null` when piping through `jq`/`python3`.

5. **`--json metric,value` can return `null`.** Fall back to `-o table` or `-o json`.

6. **No `up` metric for smart home metrics.** VictoriaMetrics is push-based (POST to `/api/v1/import/prometheus`), not scrape-based.

7. **Always use `home-tempo` for debugging.** Cloud traces are tail-sampled (1/60 planning loops). Home-tempo has everything for 72h.
