# Probabilistic State Estimation in Smart Home Systems

A conceptual guide to the mathematical tools and their interplay when building continuous, smooth state estimates from noisy, time-series sensor data.

---

## The Problem

Binary sensors give you `true/false` snapshots. But real states — "is someone in the room?", "is it occupied?", "is someone about to leave?" — are **continuous and uncertain**. You want a probability, not a threshold.

The challenge is turning a time series of raw readings into a meaningful, stable probability that captures the *right* temporal patterns.

---

## Core Tools

### 1. The Sigmoid Function

Maps any real number to the interval (0, 1). This is the bridge between "a feature value" and "a probability."

```
sigmoid(x) = 1 / (1 + e^(-x))
```

**Key properties:**
- `sigmoid(0) = 0.5` — the center point is always 50%
- Symmetric around its center
- The steepness controls how sharply it transitions

**Parameterizing the sigmoid:**

You rarely work with raw `sigmoid(x)`. You parameterize it by:

- **Center** — the input value where the probability is 50%
- **Width** — how wide the transition zone is (e.g., "the transition from 10% to 90% happens over X units")

Or by fitting it through **two known example points**: "I want input X₁ to give probability p₁, and input X₂ to give probability p₂."

**Logit** is the inverse of sigmoid — it converts a probability back to an unbounded real number:

```
logit(p) = ln(p / (1 - p))
```

This is useful for specifying desired outputs in a form that can be linearly manipulated.

---

### 2. Exponential Decay and the Tau Parameter

When you want recent data to matter more than old data, you weight each past observation by how old it is:

```
weight(t) = e^(-t / τ)
```

Where `t` is the age of the observation and `τ` (tau) is the **time constant**.

**Half-life relationship:**

```
t½ = τ · ln(2) ≈ τ · 0.693
```

So tau = 30 minutes → half-life ≈ 20.8 minutes. A measurement from 20.8 minutes ago has half the weight of a measurement from right now.

**Weighted aged sum:**

The integral of past values, each weighted by recency:

```
weighted_aged_sum = ∫ value(t) · e^(-t/τ) dt
```

This gives you a single feature number that summarizes recent history, emphasizing the recent past.

**What tau controls:**
- Large tau: slow to change, deep memory, emphasizes sustained patterns
- Small tau: fast to react, short memory, recent moments dominate

---

### 3. Logistic Regression for Probability Estimation

Given one or more features, estimate a probability using:

```
P = sigmoid(prior + w₁·f₁ + w₂·f₂ + ...)
```

Where:
- `prior` — the log-odds baseline when all features are zero (equivalent to the "background" probability)
- `w₁, w₂, ...` — weights that control how strongly each feature pushes the probability up or down
- `f₁, f₂, ...` — engineered features (e.g., `weighted_aged_sum`)

**Training:** You provide labeled examples — "this feature value should produce probability p" — and fit the weights using linear regression on the logit-transformed targets. The logit transform linearizes the sigmoid so ordinary linear regression applies.

---

## The Coupling Problem: tau, prior, and weights

This is a subtle but important point that often causes confusion.

**tau, prior, and the weights are a coupled triple.** You cannot change tau in isolation and expect the model to still behave correctly. Here is why:

1. `weighted_aged_sum` is computed with a specific tau.
2. The resulting **scale** of that feature depends on tau. A smaller tau produces smaller feature values (less history is accumulated); a larger tau produces larger values.
3. The **weight** `w_presence` converts a feature value into a log-odds increment. If the feature scale changes (because tau changed), the same weight produces a completely different probability.

**The rule:** Changing tau changes the feature scale, which requires retraining to find new weights that correctly map the new feature range to the desired probabilities.

**The labels don't change — the behavior intent stays the same.** You still want "someone has been here continuously for an hour" → probability 0.9. What changes is the numeric value of the feature for that scenario, which requires different weights to hit the same target.

---

## Training Labels: Geometric Consistency

When training a single-feature logistic regression model, the feature must be **monotone** with respect to the labels. This is easy to violate.

**Example of the trap:**

Imagine a feature: "how much presence has been detected recently" (weighted aged sum).

You define two scenarios:
- "Just arrived" (absent for an hour, present for 3 minutes) → label: 0.7 (should be high)
- "Just left" (present for an hour, absent for 3 minutes) → label: 0.2 (should be low)

This seems intuitive. But the feature value for "just left" is actually **much higher** than "just arrived" — there's an hour of presence in the history. So you've asked the model to assign 0.7 to a small feature and 0.2 to a large feature. A monotone sigmoid cannot do this.

**The constraint:** With a single monotone feature, labels must be strictly consistent with the feature ordering. If scenario A has a higher feature value than scenario B, then `label(A) > label(B)`.

---

## When Logistic Regression Is Not the Right Tool

Logistic regression (with a `weighted_aged_sum` feature) can answer: "How much presence has accumulated recently?"

It **cannot** answer: "Is the presence *continuous* or *intermittent*?"

Both of these produce similar accumulated sums:
- Solid presence for 5 minutes
- On/off alternating every 30 seconds for 10 minutes

The integral is purely additive — the pattern of arrivals and departures is invisible to it.

**When you need pattern sensitivity, use a different feature:**

Instead of "how much presence" ask "how long has presence been *unbroken*?" This directly captures continuous vs. flickering behavior.

---

## Using Small Tau as a Recent-Density Estimator

When the question is not "is it *continuous*?" but "how *dense* has it been recently?", `weighted_aged_sum` with a **small tau** works well and is naturally noise-tolerant.

A single short-lived false reading barely dents the integral. A brief true spike barely inflates it. The feature only moves meaningfully if the signal is sustained.

**The key insight:** with a small tau, `weighted_aged_sum` becomes a sliding-window density estimator. Only the recent window matters — older history decays to near-zero automatically. "Longer presence doesn't move the needle much" is a free property of small tau.

**Choosing tau:** if you care about presence over a window of N minutes, use tau ≈ N/2 to N minutes. Data older than ~3×tau contributes less than 5% of its original weight.

**Deriving consistent training labels:** the two user-specified probability anchors (e.g., "20% density → 0.2, 90% density → 0.8") directly determine the slope of the logistic model. Use those two anchor feature values to compute `w = (logit(p₂) − logit(p₁)) / (f₂ − f₁)` and then back-compute what probabilities all other training scenarios should receive. This ensures the regression fits perfectly at the anchors.

**Saturation is natural and expected.** With small tau, "present for 3 minutes" and "present for an hour" produce very similar feature values. The model saturates — both give high probability. This is the intended behavior when you only care about recency.

---

## Fulfilled Since: A Feature for Continuity

A complementary approach: find the timestamp at which the current *unbroken* true-streak began.

- If the value is currently `false`, return nothing (streak = 0 seconds)
- If the value is currently `true`, walk backwards and find when it last turned true without interruption

This gives you a **duration of continuous presence**, which you can then pass through a sigmoid:

```
P = sigmoid_fitted(duration_of_continuous_presence)
```

Parameterize the sigmoid by specifying two behavioral anchors:
- "3 minutes continuous → probability 0.1" (just barely arrived)
- "2.5 minutes → 0.9" (committed presence)

This model is simple, interpretable, and directly expresses the intended behavior without needing training data.

---

## Choosing the Right Model

| If you want to detect... | Use |
|---|---|
| Recent presence density (noise-tolerant, forgets quickly) | `weighted_aged_sum` with small tau + logistic regression |
| Accumulated/sustained presence over longer time | `weighted_aged_sum` with large tau + logistic regression |
| Continuous, unbroken presence (vs. flickering) | `fulfilled_since` + fitted sigmoid |
| A smooth threshold on a continuous measurement | Direct sigmoid with `around(center, width)` |
| A trend or rate of change | Rate-of-change + sigmoid |

The general pattern: **engineer a feature that directly captures what you care about**, then map it to a probability with a sigmoid. The simpler and more direct the feature, the easier the model is to reason about and tune.

---

## Key Conceptual Summary

| Concept | Key insight |
|---|---|
| **Sigmoid** | Converts an unbounded score into a probability; parameterize via center+width or two example points |
| **Logit** | Inverse of sigmoid; linearizes probability targets so linear regression applies |
| **Tau** | Controls the temporal horizon of exponential decay; tau = half-life / 0.693 |
| **Prior** | Log-odds baseline; sets the "background" probability when all features are zero |
| **Weight** | How strongly a feature changes the log-odds; must be retrained when feature scale changes |
| **Tau coupling** | Changing tau changes feature scale → always retrain weights after changing tau |
| **Monotone constraint** | Single-feature logistic regression requires labels to be consistent with feature ordering |
| **Integration blindness** | Accumulated-sum features cannot distinguish continuous from intermittent patterns |
| **Small tau = density estimator** | With small tau, weighted_aged_sum measures recent density; saturation is natural and expected |
| **Consistent labels** | Derive labels from two anchor points via the implied slope, not from intuition alone |
| **Fulfilled since** | Alternative feature that captures duration of unbroken streak; enables continuity detection |
