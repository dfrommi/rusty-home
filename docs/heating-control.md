# Heating Control

Reference for the heating stack. The behavior is **not reconstructable from the code alone**: the TRV's physical behavior, the two-time-scale control loop, and several dead code paths were reverse-engineered in a dedicated investigation (Aug 2026). Read this before touching any heating code.

## TL;DR

Six radiators, each with a Sonoff TRV + a temperature sensor mounted on the radiator surface. The TRV is a **dumb binary valve** — fully open or fully closed, no PID, no in-between. The app is the only controller: it sets *where* the valve flips (setpoint ± accuracy), *how far open* "open" means (valve opening degree = target demand), and *how far closed* "closed" means (always 0 today). The TRV itself just flips open/closed on the injected room temperature — "heat until too warm, then close."

Delivered heat = **duty cycle (emergent, not controlled) × amplitude (the one real knob)**. A binary relay against a high-inertia radiator makes the duty cycle slow and weather-dependent; that is the root of the "unpredictable" behavior.

## Physical setup

Six radiators (`app/src/core/domain/mod.rs: Radiator`): `LivingRoomBig`, `LivingRoomSmall`, `Bedroom`, `Kitchen`, `RoomOfRequirements`, `Bathroom`. Per radiator:

| Thing | Source | Home-state type (`Temperature::*`) |
|---|---|---|
| **Sonoff TRV** (actuator) | Z2M `SonoffThermostat` | — |
| **Surface temp sensor** (mounted on radiator) | Z2M climate sensor `*/temp_sensor_radiator_*` | `Radiator(Radiator)` |
| TRV's own local temp (unreliable) | TRV `local_temperature` | `ThermostatOnDevice(Radiator)` |
| Injected external temp (= room temp) | pushed by `Z2mSensorSyncRunner` | `ThermostatExternalInput(Radiator)` |
| Room temp | Tado (LR/Bed/RoR), Z2M (Kitchen), shower+dehumidifier avg (Bathroom) | `Room(Room)` |

Two radiators share one zone: `LivingRoomBig` + `LivingRoomSmall` both map to `HeatingZone::LivingRoom`.

`Radiator::heating_factor()` (1.728 / 0.501 / 1.401 / 1.485 / 1.193 / 0.496) is **observability only** (`TotalRadiatorConsumption` energy scaling) — not used in control.

## The TRV — the part not visible in code

- The Sonoff TRV is a **binary relay with two settable stops**. It flips between two positions based on `occupied_heating_setpoint` ± `temperature_accuracy` vs `external_temperature_input`:
  - **open** → `valve_opening_degree`
  - **closed** → `100 − valve_closing_degree`
- There is **no PID and no in-between state**. "Opening degree" is a *limit* that defines what "fully open" means, not a commanded position.
- The app therefore has exactly **three knobs**:

| Knob | Wire | Physical meaning |
|---|---|---|
| Switch thresholds | `occupied_heating_setpoint` + `temperature_accuracy` | *when* the relay flips (the `SetPoint` range) |
| On-amplitude | `valve_opening_degree` | *how open* while on (= `TargetHeatingDemand`) |
| Off-amplitude | `valve_closing_degree` | *how closed* while off (always 0 today) |

- The TRV's decision input is the **injected external temp** (room temp, pushed by `Z2mSensorSyncRunner` with `temperature_sensor_select: external`). Its own `local_temperature` sensor is unreliable and unused. If a device silently falls back to `local_temperature`, `is_heating` (below) — and every downstream decision — is wrong with no error surfacing.

## Two-time-scale control

```
app (slow, ~30s…10min)                     TRV (dumb binary relay)
───────────────────────────                ─────────────────────────
SetPoint range ─► occupied_heating_setpoint ┐
                 + temperature_accuracy   ──┼─► too cold → OPEN
                                              │   too warm → CLOSE
TargetHeatingDemand ─► valve_opening_degree ─┘   (open  = opening_degree,
                                                close = 100−closing_degree)
room temp ─► Z2mSensorSyncRunner ─► external_temperature_input
```

1. **Inner loop (TRV hysteresis)** — the crude on/off regulator that actually stops heating ("heat until too warm"). The app only *observes* it via `guess_is_heating_from_hyserisis` (a reimplementation of the relay, not a guess about a PID).
2. **Outer loop (app)** — occasionally adjusts the flip point (setpoint band) and the on-amplitude (`TargetHeatingDemand`, stepped ±3–7% per re-evaluation).

The **duty cycle is not controlled**: it emerges from room thermal mass, flow temperature, and the ±0.2…1.0 °C band. The **amplitude is the only lever the app pulls**, coarsely and slowly. The surface-temp sensors exist to *observe* what a given amplitude actually does, because "20% open" delivers different heat on different days.

## Signal chain (file map)

`device_state` (raw) → `home_state` (derived) → `automation` (1 rule/radiator) → `command` → Z2M → TRV → readback → `device_state`.

| Layer | File | Output | Semantics |
|---|---|---|---|
| `TargetHeatingMode` | `home_state/items/heating/target_heating_mode.rs` | `HeatingMode` | the *why* (occupancy, window, presence, time) |
| `SetPoint` | `home_state/items/heating/set_point.rs` | `Range<DegreeCelsius>` | the flip thresholds (band) |
| `TargetHeatingAdjustment` | `home_state/items/heating/target_heating_adjustment.rs` | `AdjustmentDirection` | signed ladder −3…+2, from two strategies |
| `TargetHeatingDemand` | `home_state/items/heating/target_heating_demand.rs` | `Percent` | the on-amplitude (step controller) |
| `HeatingDemand` | `home_state/items/heating/heating_demand.rs` | `Percent` | "is it heating" (model) + `BarelyWarmSurface` estimator |
| `HeatingDemandLimit` | `home_state/items/heating/heating_demand_limit.rs` | `Range<Percent>` | valve on/off positions, Current = readback, Target = 0..demand |
| rule | `automation/domain/action/heating/follow_target_heating_demand.rs` | `Command::SetHeating` | bundles `SetPoint::Target` + `HeatingDemandLimit::Target` |
| executor | `command/adapter/z2m/mod.rs::set_sonoff_heating` | MQTT | maps to setpoint + accuracy + opening/closing degree |
| sync | `command/adapter/z2m/sync.rs::Z2mSensorSyncRunner` | MQTT | pushes room temp into TRV as external input |

Shared helpers live in `home_state/items/heating/mod.rs`: `HeatingAdjustmentStrategy`, `ControlLimits`, `radiator_strategy`, `setpoint_strategy`, `setpoint_for_mode`.

## Decision logic

### Heating mode (priority, first match wins)

1. **Away** — nobody home and no newer override.
2. **Ventilation** — window open in that zone.
3. **Manual** — active user override (HomeKit); expires after **1 h**.
4. **Sleep** — morning `5:20–12:30` before first ventilation of the day; or bedtime windows (Bedroom `21:00–5:30`, LivingRoom `22:00–5:30`, RoR `20:00–5:30`).
5. **Comfort** — occupancy ≥ 0.7, or ≥ 0.5 with recent high-occupancy (hysteresis).
6. **EnergySaving** — fallback.

Kitchen/Bathroom inherit **Sleep** when all three other zones are sleeping. Manual control is folded into this mode (there is **no** `UserTriggerAction` for heating in `resource_plan.rs` — unlike every other device).

### Setpoint (`setpoint_for_mode`)

Hardcoded `(radiator, mode)` lookup → `Range::new(t, t − offset)` = `[t − offset, t]`, e.g. LivingRoom Comfort `[19.1, 19.5]`. Offset: `Comfort`/`Manual` 0.4, `Bedroom Sleep` 0.6, everything else 1.0. Exact values in `setpoint_for_mode`; the structure is the lookup + one seasonal override:

- **`is_warmish_outside`**: month ∈ 3–9 and 7:00–21:00 → `t = 15.0` for every non-Manual/non-Ventilation mode. This is the "block heating in spring" hack (commit `c1e484f`). In summer daytime the room is >> 15 °C, so `setpoint_strategy` emits `MustOff` and heating stays off regardless of mode.

### Adjustment strategies (`target_heating_adjustment.rs`)

Ladder: `MustOff(−3) < MustDecrease(−2) < ShouldDecrease(−1) < Hold(0) < ShouldIncrease(1) < MustIncrease(2)`.

- **`radiator_strategy`** (surface temp, **safety only**): `max = room + offset` (Comfort +11, Manual +14, EnergySaving/Sleep +8, Away +6, Ventilation +3), band = max/3, overshoot 10 °C. Only ever Hold/ShouldDecrease/MustDecrease/MustOff — never increases. Its `min`/`min_heatup` fields are dead.
- **`setpoint_strategy`** (room temp vs setpoint): band = offset/3, overshoot 1 °C, `min_heatup` 0.75 °/h (Comfort/Manual), 0.5 (EnergySaving/Sleep), 0.2 (Ventilation/Away). Can emit `MustIncrease`, but only when `is_heating` **and** room-RoC < min_heatup.

`adjustment_direction` rules (both strategies share them):
```
min_heatup set && roc < min_heatup && current < max && is_heating  → MustIncrease
current in [max−band, max] && is_heating                           → ShouldDecrease
current in (max, max+overshoot]                                    → MustDecrease
current > max+overshoot                                            → MustOff
else                                                               → Hold
```
The lower band is **intentionally dead** — the "increase because below setpoint" rule is commented out. The controller only ever backs off near the top; it never pushes heat from the bottom.

**Merge** (`TargetHeatingAdjustment::HeatingDemand`): `(_, MustIncrease) → MustIncrease`; either `MustOff` → `MustOff`; `(MustDecrease, _) → MustDecrease`; else merge with setpoint winning conflicts. Net effect: radiator can veto *up* (MustDecrease/MustOff), setpoint can force *up* (MustIncrease).

### Target demand (step controller, `target_heating_demand.rs`)

Steps from the **current on-position readback** (`HeatingDemandLimit::Current.to()`):
```
adjustment == MustOff                                  → 0
!is_heating && adjustment ≤ Hold                        → barely_warm   (arm on-position)
ventilation finished recently && !is_heating
  && no heat request since then                          → 0
!adjustment_needed(...)                                  → reference (no change)
MustDecrease/ShouldDecrease                              → −step
MustIncrease (cold: barely_warm + 2·step)                → +step
ShouldIncrease (cold: barely_warm + 1·step)              → +step
Hold                                                    → unchanged
clamp [min_output, max_output]
```

`ControlLimits` (per radiator / mode):

| Radiator | min | step | barely-warm fallback |
|---|---|---|---|
| LR big/small, RoR | 12% | 5% | 20% |
| Bedroom, Kitchen | 6% | 3% | 8% |
| Bathroom | 10% | 7% | 20% |

`max_output`: Comfort 50%, Manual 60%, EnergySaving/Sleep 40%, Away 30%, Ventilation 0% (each `.max(min_output)`).

**`barely_warm` is the armed on-position, not a constant simmer.** `!is_heating && ≤ Hold → barely_warm` means: valve physically closed, on-position re-armed to barely_warm so the *next* flip open starts at the minimum useful amplitude.

### Barely-warm estimator (`estimate_barely_warm_surface`)

Scans the last 3 h of (surface − room) temperature vs demand; finds the smallest valve opening that produced a real surface rise (RoC > 2 °C/10 min, or hot-and-holding > 8 °C above room). Falls back to 20% / 8% / 20% when there's no recent evidence (summer, or radiator never ran). Two TODOs in the source: previous-day search, outside-temperature dependence.

### is_heating (feedback)

`HeatingDemand::Radiator` = `guess_is_heating_from_hyserisis(setpoint, room_temp_history)` — a faithful reimplementation of the relay: below lower setpoint → heating, above upper → not, in-between → hysteresis on last boundary crossing. Returns the valve limit readback (`to()` if heating, `from()` otherwise). Consumed as `is_heating_now = value > 0`.

**`device_state::HeatingDemand` is dead** — defined, read back from DB, plotted in Grafana, but never written by any adapter. `trust_device_reading` (the pre-2026-02-08 path) always returned `None`.

## Runtime cadence & throttling

- Home state recalculates on `DeviceStateEvent::Changed` (50 ms debounce) and a 30 s timer; planning runs on every `SnapshotUpdated` and a 30 s timer over the last snapshot.
- The real throttle is **state reflection, not time**: `should_execute` skips if the same command ran < 30 s ago, < 2 min (`SetHeating`), or if `is_reflected_in_state` (the TRV already reports the target setpoint + demand limit). A *different* command (changed demand value) bypasses the 30 s/2 min cooldowns.
- `adjustment_needed` gates the step controller: no change within 30 s of the valve's last physical move; force re-eval after 10 min or on mode change.

## Control approaches tried (chronological)

1. **Direct valve-position control** (early): `SetThermostatValveOpeningPosition { value }` — the app drove the valve position directly with fake setpoint values; the device fought it.
2. **PID on room temperature** (Dec 20 2025 – Jan 8 2026, `90ca081` → `712dcd9`): classic PID (P + I integrated over 60 min + D), room-temperature error → valve opening %. Multiple stabilization attempts (`f2acded`, `1875762`, `8d319ca`). **Abandoned — "unreliable as hell."** Root cause is the plant mismatch (see "Environment & plant constraints"): a PID assumes a roughly linear, time-invariant, low-latency plant; this plant is a binary relay with ~10 min dead time, non-stationary gain (outside temp, neighbors below), and coarse slow actuation.
3. **Direction-based control** (Jan 8 2026, `712dcd9`): precursor to the current approach; PID remnants cleaned up in `adcb187`.
4. **Setpoint + hysteresis** (Feb 9 2026, `ed56d2c`, current): app sets setpoint + valve limits and lets the TRV relay cycle.

## Design exploration & dead ends (don't repeat)

Record of every approach considered and why it died — some implemented then reverted, some explored only in the Aug 2026 design discussion. The reusable knowledge is the *why*, so future work doesn't re-run these.

| Approach | Status | Why it failed / was rejected |
|---|---|---|
| Direct valve-position control | reverted (early) | device fought the fake setpoint values |
| PID on room temperature | implemented, reverted (Dec 2025–Jan 2026) | plant is binary + ~10 min dead time + unidentifiable gain |
| Surface temp as indicator (`estimate_barely_warm_surface`) | still in code as barely-warm floor, not primary | confounded — "hot" ≠ "winning" |
| Room heatup gradient as `MustIncrease` | current | slow (hours) + confounded by sun/neighbors |
| `T_eq`/τ demand model + heating curve | discussion only | solar + wall memory + neighbors make `T_eq` unidentifiable |
| Feedforward from estimated gain | discussion only | gain is exogenous (central hydraulic + manual neighbors) |
| Learned predictor (context → gain) | investigated, dead end | no observable correlate; neighbors drive it |
| Slow probing (±3–7 % steps) | current, painful | freezing for an hour when cold |
| High initial opening (over-shoot first burst) | discussion only | cost ∝ (radiator − ambient); hot radiator is expensive |

What remains viable (as of Aug 2026):

1. **Cleanup** — make the existing control legible, predictable, and trustable.
2. **Two unambiguous guards** — the only conditions that don't need observability:
   - *cost-cut*: radiator much hotter than any plausible need → close down, and don't let "room needs heat" override it (today the merge lets setpoint `MustIncrease` win over a hot radiator).
   - *cheap-escalate*: radiator ≈ room temp **and** room below target **and** falling → open wider (cheap: a cold radiator costs ~nothing to open wider).
3. **Learned prior** — the mathematical form of "what worked last time": a per-radiator, slowly-updated amplitude prior keyed on coarse observable context (outside temp, time of day, and how warm the room runs free today as a sun+neighbor proxy). A *prior*, not a real-time controller.

## Future direction & current status

**Faster feedback (unproven):** replace the battery surface sensor with a **USB-powered ESP with 3 temperature sensors** — on the radiator (next to the usage meter), on the **inlet pipe**, and on the **outlet pipe** — reporting at high frequency. Rationale: the inlet pipe heats up quickly and gives a much faster "hot water is actually flowing" signal than the radiator body (thermal delay) or the battery sensor (reporting lag); inlet/outlet delta directly reflects realized gain, which the surface sensor can only approximate. Blocker: getting USB power to every radiator (unsolved).

**Until then:** cleanup only — no new control law. The goal is a maintainable, understandable implementation of the current approach.

## Calendar-gated feature flags

Runtime branches on `DateTime::from_static_iso(...).is_passed()`: `heating_demand.rs` `2026-02-08` (hysteresis `is_heating`), `set_point.rs` `2025-11-22` (RoR target setpoint) and `2025-12-21` (all others). All passed as of Aug 2026 → hysteresis + `setpoint_for_mode` are active everywhere; the old fallbacks are unreachable.

## Dead code & rough edges

- **Cold-start factors are unreachable**: `MustIncrease`/`ShouldIncrease` both require `is_heating == true`, but cold start is by definition `is_heating == false`. The real cold start is the `barely_warm` arm; `cold_start_should/must_factor` is dead.
- **Lower-band increase rule** commented out (see above).
- **`device_state::HeatingDemand`** never written (see above).
- **Name collisions** between `device_state` and `home_state` layers: `HeatingDemand`, `HeatingDemandLimit`, `SetPoint`, `Temperature` all exist in both with different semantics. Two distinct structs are both named `HeatingDemandStateProvider` (`heating_demand.rs` and `target_heating_demand.rs`).
- **`HeatingAdjustmentStrategy`** has `min`/`max`/`min_heatup` marked `#[allow(dead_code)]` ("offloaded for now to the device").
- **`Z2mSensorSyncRunner` lives in `command`** though it is not a command — it's a sensor sync (architecture-review P9).

## Environment & plant constraints (not in code)

Established through observation (winter 2025/26) and design discussion (Aug 2026). Not derivable from the code.

- **Central hydraulic loop.** Top-floor apartment in a building with central heating. Water temperature and pressure are set centrally (outside-temperature probe + an unknown schedule). Neighbors heat *manually* (presence, feeling, bedtime) — audible but unobservable. The heat delivered for a given valve opening therefore depends on building-wide state that is invisible and changes faster than the apartment's sensors.
- **No predictor exists.** A winter spent looking for any relation between outside temp / time / neighbors and the realized heating came up empty; the dominant variable is neighbors' manual behavior, which cannot be learned.
- **~10 min dead time.** A valve-opening change takes ~10 min to become large enough for the on-radiator surface sensor to detect; the battery-powered sensor adds its own reporting lag on top.
- **Cost ∝ (radiator − ambient).** Metered by the temperature difference between radiator and ambient. A hotter radiator costs more per minute, so "open fully as a default" is expensive and must be avoided.
- **Solar gain.** South-facing rooms with large windows get significant, fast, hard-to-model free heat; sun vs. clouds matters a lot in winter.
- **Wall thermal memory.** The longer it has been cold, the cooler the walls. Heat loss depends on the multi-day history of outside temperature, not just today's value.
- **Slow probing is not viable.** Set opening → wait ~10 min → adjust → repeat = freezing for an hour. The current step controller (±3–7 % per up-to-10-min re-eval) *is* this slow probing.
- **Only usable prior is memory.** "What opening worked last time on this radiator" — per-radiator, hit-and-miss.

### The observability wall

The decisive limit is **not** "which indicator is best" — it is that **no available signal isolates "what my radiator is doing" from "sun + neighbors + walls"**:

- **Surface temp**: fast but confounded — cannot distinguish "winning" from "just warm", and it is also driven by water temp and room temp.
- **Room-temperature gradient**: answers "am I winning?" but on the hours timescale, and confounded by sun/neighbors.
- **`T_eq` / demand model**: contaminated by solar noise, wall cold-soak, and neighbor heating — not a clean parameter.

Every indicator works until a confound moves, then it lies. This rules out both a reliable *feedback* controller and a predictive *feedforward* model — which is why each attempt "worked, then didn't".

## Failure modes (why winter was painful)

- **Binary relay × thermal inertia = long, variable duty cycle.** "20% now = significant heat" is because 20% open means full flow-temp water for the entire on-cycle; "20% in an hour = barely warm" is thermal lag. The same amplitude produces different heat on different days (flow temp, radiator already warm vs cold).
- **Slow ramp**: corrective amplitude steps are 3–7% every up-to-10 min, and `MustIncrease` only fires when already heating. If `barely_warm` is underestimated (or the fallback is wrong for current conditions), the first burst does nothing and recovery is very slow.
- **`is_heating` trust**: the whole model rests on the TRV honoring `external_temperature_input`. A silent fallback to `local_temperature` corrupts every downstream decision without an error.
- **No direct heat control**: the app sets amplitude + thresholds and waits; it can only *observe* the result via surface/room temperature rate of change.

## Gotchas

- `DataFrame` deduplicates consecutive identical values — timestamps mark when a value *became* active (see AGENTS.md "Core Types & Time DSL").
- `SetPoint::Target` is derived from mode; `SetPoint::Current` is the device readback. `is_reflected_in_state` compares Current against the commanded Target — the command is considered "done" when the TRV reports it.
- `FollowTargetHeatingDemand` never returns `Skip` — it either produces a `SetHeating` or errors (missing `SetPoint::Target`/`HeatingDemandLimit::Target`). So the planner's "first non-Skip wins" doesn't apply to heating (each radiator has exactly one rule); throttling happens entirely in `should_execute`/`adjustment_needed`.
