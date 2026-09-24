# 08 — Backlink state machine

**Spec refs:** §23, §24, §29, §30, §57.8, §57.9, §57.10
**Crate:** `crates/core::backlink` — pure functions, zero I/O, exhaustively unit-tested.

The single rule this document exists to enforce (§24, §57.10):

> **A failed request does not mean a lost backlink.**
> "The link is not on the page" and "we could not look at the page" are different facts and must
> never collapse into the same state.

---

## 1. Observation — what one check actually tells us

A `CrawlResult` is classified into exactly one `Observation` before the state machine sees it. This
is where the critical distinction is made, and it is made by the type system rather than by an
`if` chain that someone can later get wrong.

```rust
pub enum Observation {
    /// Page fetched, parsed, target link found.
    Present(LinkMatch),
    /// Page fetched and parsed successfully, target link genuinely NOT in the document.
    /// This is the ONLY observation that can lead to `Lost`.
    Absent,
    /// Source returned 404/410 — page is gone.
    SourceGone { status: u16 },
    /// Source returned another 4xx/5xx.
    SourceHttpError { status: u16 },
    /// Timeout, DNS failure, connection refused, TLS failure. We learned nothing.
    SourceUnreachable { kind: ConnErrorKind },
    /// robots.txt disallow, SSRF guard, oversized body, non-HTML. We chose not to / could not look.
    SourceBlocked { reason: BlockReason },
    /// Source resolved to a materially different URL.
    SourceRedirected { to: Url, permanent: bool },
}
```

`Absent` is deliberately narrow: it is only produced when we got a `200`, the content type was HTML,
the body parsed, and no `<a>` matched. Every other path produces something else.

---

## 2. States

| State | Meaning | Actionable? |
|---|---|---|
| `pending` | Created, never successfully checked | no |
| `live` | Present and matching expectations | no |
| `changed` | Present, but anchor and/or target URL differs from expected | **yes** |
| `nofollow` | Present, but not dofollow while `expected_rel` was dofollow | **yes** |
| `redirected` | Source page redirects; link state evaluated at destination | **yes** |
| `lost` | **Confirmed** absent from a page we successfully fetched and parsed | **yes** |
| `source_error` | Source page persistently 4xx/5xx | **yes** |
| `target_error` | Target page persistently 4xx/5xx (the link works; the destination doesn't) | **yes** |
| `blocked` | We are persistently unable to verify (robots, WAF, SSRF guard) | **yes, softly** |
| `unknown` | Persistently unreachable — network-level, cause undetermined | **yes, softly** |

`blocked` and `unknown` are the honest states. They say *"we cannot tell you,"* which is a different
customer message from *"your link is gone,"* and they are counted separately everywhere — dashboard,
Value at Risk, alerts.

### 2.1 Precedence when several conditions hold at once

A link can be simultaneously present, nofollow, and pointing at a changed target. `status` holds one
value, so precedence is fixed and documented rather than incidental:

```
redirected  >  lost  >  source_error  >  target_error
            >  changed  >  nofollow  >  blocked  >  unknown  >  live
```

`changed` outranks `nofollow` because a wrong target URL is a harder failure than a lost follow
signal. The full detail is never lost: `detected_anchor_text`, `detected_target_url` and
`detected_rel_attributes` always carry the complete picture, and **events are emitted per condition**
regardless of which one won the `status` field. A link that goes nofollow *and* changes anchor emits
both `DOFOLLOW_TO_NOFOLLOW` and `ANCHOR_CHANGED`.

---

## 3. Confirmation — the anti-flapping rule

No negative state is entered on a single observation. Each carries a required number of
**consecutive** confirming observations:

| Observation | Confirmations to transition | Target state |
|---|---|---|
| `Absent` | **2** | `lost` |
| `SourceGone` (404/410) | **2** | `source_error` |
| `SourceHttpError` (5xx) | **3** | `source_error` |
| `SourceUnreachable` | **3** | `unknown` |
| `SourceBlocked` | **2** | `blocked` |
| `Present` (any form) | **1** | immediate |

Positive news is applied immediately; negative news must be confirmed. That asymmetry is
intentional — a false `live` corrects itself on the next cycle, while a false `lost` sends an alert
that costs the customer a phone call to a publisher.

### 3.1 The intermediate state is not a state

Between "first failure" and "confirmed," the backlink **keeps its previous public status** and
carries `consecutive_failure_count > 0`. The spec's `SUSPECTED FAILURE` is modelled as this counter
rather than as a visible status, so the customer's dashboard does not flicker and no alert fires on
a transient blip.

While `consecutive_failure_count > 0`, the backlink is rescheduled **aggressively** — the retry runs
in minutes, not at the normal cadence — so confirmation is fast:

```
next_check_at = now() + min(base_retry * 2^(n-1), 6h)    # n = consecutive_failure_count
```

### 3.2 Recovery resets

Any `Present` observation sets `consecutive_failure_count = 0`, updates `last_seen_at`, and
restores the normal cadence. `failure_count` (lifetime) is **not** reset — it feeds the publisher
reliability metrics in §31.

---

## 4. Transition diagram

```
                            ┌──────────┐
                            │ pending  │
                            └────┬─────┘
                                 │ first successful check
             ┌───────────────────┼───────────────────┐
             ▼                   ▼                   ▼
        Present(ok)        Present(differs)        Absent ×2
             │                   │                   │
             ▼                   ▼                   ▼
        ┌────────┐      ┌─────────────────┐    ┌──────────┐
   ┌───►│  live  │◄────►│ changed/nofollow │◄──►│   lost   │
   │    └───┬────┘      └────────┬─────────┘    └────┬─────┘
   │        │                    │                   │
   │        │  errors ×N         │                   │ Present
   │        ▼                    │                   │
   │  ┌──────────────┐           │                   │
   │  │ source_error │           │                   │
   │  │ target_error │           │                   │
   │  │   blocked    │           │                   │
   │  │   unknown    │           │                   │
   │  └──────┬───────┘           │                   │
   │         │ Present           │                   │
   └─────────┴───────────────────┴───────────────────┘
                    (any Present observation, 1 check, immediate)
```

Every state is recoverable. There is no terminal state except `archived`, which is a user action and
lives in `archived_at`, not in `status`.

---

## 5. The function

```rust
pub fn transition(
    current: &BacklinkState,     // status, expected_*, consecutive_failure_count, detected_*
    obs: &Observation,
    now: DateTime<Utc>,
    policy: &TransitionPolicy,   // confirmation thresholds, retry curve — config, not constants
) -> Transition;

pub struct Transition {
    pub status: BacklinkStatus,
    pub previous_status: BacklinkStatus,
    pub consecutive_failure_count: i32,
    pub next_check_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub detected: DetectedFields,
    pub events: Vec<BacklinkEvent>,   // may be empty, may be several
}
```

Properties the unit tests assert:

1. **Pure.** No clock, no database, no network — `now` and `policy` are arguments. Same inputs, same
   output, every time.
2. **`Absent` is the only path to `lost`.** A property test generates every non-`Absent`
   observation, repeated arbitrarily many times, and asserts `status != lost`. This is the §24
   guarantee, mechanically enforced.
3. **One check never skips confirmation.** From `live`, a single failing observation of any kind
   leaves `status == live`.
4. **Events match reality.** A transition emits `BACKLINK_LOST` if and only if status became `lost`.
   No transition emits an event describing a change that did not occur.
5. **`previous_status` is always the status before this transition**, enabling the history timeline
   in §30 without a separate query.

---

## 6. Events (§29)

Events are **immutable, append-only** rows in `backlink_events`. Nothing updates or deletes them;
the current state in `backlinks` is a cache of the latest truth, while the events are the record.
This is what makes §30's timeline possible and §57.8's "every backlink has history" true.

| Event | Emitted when |
|---|---|
| `BACKLINK_FOUND` | `pending` → `live` (first ever detection) |
| `BACKLINK_LOST` | any → `lost` |
| `BACKLINK_RESTORED` | `lost`/`source_error`/`unknown`/`blocked` → present again |
| `ANCHOR_CHANGED` | `detected_anchor_text` differs from previous detected value |
| `TARGET_CHANGED` | `detected_target_url` differs from previous detected value |
| `DOFOLLOW_TO_NOFOLLOW` | was dofollow, now is not |
| `NOFOLLOW_TO_DOFOLLOW` | was not dofollow, now is |
| `REL_CHANGED` | rel set changed without crossing the follow boundary (e.g. `ugc` → `sponsored`) |
| `SOURCE_REDIRECTED` | source final URL materially differs from `source_url` |
| `SOURCE_404` | source returned 404/410, confirmed |
| `SOURCE_ERROR` | source returned another error, confirmed |
| `TARGET_REDIRECTED` / `TARGET_404` / `TARGET_ERROR` | equivalents for the target |
| `NOINDEX_DETECTED` | `<meta robots noindex>` or `X-Robots-Tag` appeared |
| `CANONICAL_CHANGED` | source canonical changed |

Each row stores `previous_data` and `current_data` as `JSONB`, plus `detected_at`, `backlink_id`,
`organization_id` and the `crawl_id` that produced it. The event payload is a **snapshot, not a
reference** — it stays meaningful even if the backlink is later edited, which is the §30 requirement
that history is never destroyed.

### 6.1 "Changed" is measured against last *detected*, not against *expected*

`ANCHOR_CHANGED` compares the new detection to the **previous detection**, not to
`expected_anchor_text`. Otherwise a backlink that was imported with a wrong expectation would emit a
change event on every single check, forever. The `expected_*` fields drive the `changed` *status*;
the `detected_*` fields drive the *events*. These are different questions:

- *"Is this what I paid for?"* → status, compares detected vs expected.
- *"Did something happen?"* → event, compares detected vs previously detected.

---

## 7. Scheduling

`next_check_at` is set by every transition, never by a cron sweep that guesses.

| Situation | Next check |
|---|---|
| `live`, stable | project `default_check_frequency` (default 7 d) |
| `live`, changed within 30 d | frequency ÷ 2 (recently volatile links get watched harder) |
| `consecutive_failure_count = n > 0` | `min(base_retry × 2^(n-1), 6h)` |
| `lost` (confirmed) | 3 d — we keep watching, because links come back (§30) |
| `blocked` by robots.txt | 30 d — the answer will not change soon |
| `archived` | `NULL` — excluded from the scheduler's partial index entirely |

Jitter of ±10% is applied to every computed `next_check_at` so that a bulk CSV import does not
create a thundering herd that re-fires in lockstep forever.
