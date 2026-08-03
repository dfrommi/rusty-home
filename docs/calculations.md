# Home Calculation Physics

The theory/rationale behind the home-state calculations. Read this before modifying
any calculation item (`RiskOfMould`, `DewPoint`, `AbsoluteHumidity`,
`Temperature::BedroomCorner`) or the `dehumidify` automation rule. It exists so the
math is never silently re-derived or broken; if you change a formula, update this doc
in the same change.

For the mathematical toolbox (sigmoids, weighted aging, state machines) see
`docs/probabilistic-state-estimation.md`.

## 1. Fundamental quantities

Both derive from temperature + relative humidity via the Magnus formula (same
saturation-vapor-pressure core, `a=7.5, b=237.3` above 0 °C; `7.6, 240.7` below).

### Dew point — `app/src/home_state/items/dewpoint.rs`

`DewPoint::Room`, `Outside`, `BathroomShower`, `BathroomDehumidifier` (all in °C).

- Compares **directly to surface temperatures** without exponentials — the right
  quantity for condensation / mould calculations.

### Absolute humidity — `app/src/home_state/items/absolute_humidity.rs`

Same items, in g/m³.

- **Conserved under heating/cooling** — the right quantity for ventilation decisions
  (indoor vs outdoor moisture). Dew point is *not* conserved: heating a room lowers
  its dew point relative to the outdoor one even though the absolute moisture is
  unchanged.

## 2. The mould risk model

### Thermal bridge (f_Rsi) — `Temperature::BedroomCorner` in `temperature.rs`

There is no physical sensor in the mould-prone bedroom corner (one would be
invalidated by the dehumidifier's air drift), so the surface temperature is derived:

```
T_surface = T_outside + f_Rsi · (T_room − T_outside)      (ISO 13788 / DIN 4108-2)
```

- `f_Rsi = 0.55` currently — conservative estimate for an old-building exterior corner.
  New-building code minimum is 0.7; old-building corners are typically 0.55–0.65.
- Calibration TODO: one-off IR thermometer reading on a cold day via
  `f_Rsi = (T_surface − T_outside) / (T_room − T_outside)`.

### 3-Kelvin rule / DIN 4108-2 — `RiskOfMould` in `risk_of_mould.rs`

```
margin = T_surface_corner − DewPoint_room
```

- `margin ≤ 0 K` → condensation on the wall
- `margin ≈ 3 K` → ~80 % surface RH (DIN 4108-2 mould threshold, >12 h/day)
- `margin ≥ 5 K` → conservative safe zone (~70 % surface RH)

Risk is `margin < 3.0 K` on a **weighted-aged mean** (`weighted_aged_mean(tau, ...)`
over a lookback window, `tau` = aging half-life).

### Per-room wiring (as currently implemented)

**Bathroom** — two gates:
1. Bathroom humidity < 70 % → no risk (shower-sensor gate).
2. Instantaneous shower dew point more than **3.0 K** above a reference:
   weighted-aged mean (τ = 2 h, 3 h window) of the living room + room-of-requirements
   dew points. Pattern: *"is this room anomalously moist vs the rest of the house?"*

**Bedroom** — margin model over the last 3 h:
- weighted-aged mean (τ = 1 h) of `Temperature::BedroomCorner` and bedroom dew point,
- risk = `mean_corner − mean_dewpoint < 3.0 K`.

The averaging window has drifted over time (an earlier decision used 6 h for the
bedroom's larger thermal mass); the code now uses τ = 1 h over a 3 h window, with a
TODO to adapt the window seasonally (longer in summer, shorter in winter).

## 3. Known issues / calibration TODOs

- **Fixed dewpoint threshold in the bedroom dehumidify rule is temperature-dependent.**
  `dehumidify.rs` still uses an absolute 10.0/10.5 °C hysteresis, optimized for ~19 °C
  room temperature — it fires constantly in warmer weather where mould risk is low.
  Code TODO: derive the threshold from room temperature. Relative gates were already
  added: the mould-risk gate and a "dew point within 4 K of outside → skip" gate.
- **The indoor/outdoor moisture comparison should use absolute humidity**, not dew
  point diff (see §1 — dew point isn't conserved under heating). The dehumidify rule
  still uses a 4 K dew-point diff; `AbsoluteHumidity` items exist but are not yet used.
- **f_Rsi needs empirical calibration** (§2).
- **Window tuning** — seasonal averaging windows (§2, bedroom).

## 4. Design principle

**Physically grounded over empirical.** Prefer direct physical quantities (dew point,
absolute humidity, surface temperature) over indirect proxies (relative humidity) when
both convey the same information. Relative humidity alone mixes temperature and
moisture, so thresholds on it break when temperatures shift.
