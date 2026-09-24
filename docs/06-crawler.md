# 06 — Crawler design

**Spec refs:** §25, §26, §27, §28, §53, §57.11, §57.13
**Crate:** `crates/crawler` — no Axum, no tenant types, no direct database access.

The crawler's entire input is a `CrawlRequest` and its entire output is a `CrawlResult`. It does not
know what an organization is. That keeps it unit-testable against fixtures and keeps the SSRF guard
in one place that the public tool (§50) shares.

---

## 1. Pipeline

```
CrawlRequest { source_url, target_url, expected_anchor, expected_rel, check_target }
    │
    ├─ 1. normalize + validate source_url        ──► Invalid            ─┐
    ├─ 2. resolve DNS to IP set                                          │
    ├─ 3. SSRF guard on every resolved IP        ──► Blocked            ─┤
    ├─ 4. GET source (own redirect loop)                                 │
    │      └─ re-run 1–3 on every hop            ──► Blocked            ─┤
    ├─ 5. record status, final URL, hop chain                            │
    ├─ 6. stream body with a hard byte cap       ──► Oversized          ─┤
    ├─ 7. parse HTML                                                     │
    ├─ 8. extract <base href>, canonical, robots meta                    │
    ├─ 9. extract every <a> → (href, anchor text, rel)                   │
    ├─ 10. normalize each href against base                              │
    ├─ 11. match against target_url                                      │
    ├─ 12. optional HEAD/GET on target (steps 1–5 again)                 │
    └─ 13. CrawlResult ◄──────────────────────────────────────────────── ┘
```

Steps 1–10 happen **once per source URL**. Step 11 onward runs **per expected backlink** (§27).

---

## 2. SSRF protection

**Spec §26, §57.11. This is the highest-severity control in the product** — users supply arbitrary
URLs and we fetch them from our infrastructure.

### 2.1 Scheme and shape

Allow `http` and `https` only. Reject everything else explicitly: `file:`, `gopher:`, `ftp:`,
`data:`, `javascript:`, `blob:`, and any scheme not on the allowlist. Reject URLs carrying
credentials (`http://user:pass@host`). Reject non-standard ports — allowlist `80`, `443`, and
whatever the scheme default is; nothing else.

### 2.2 Resolve, then validate **every** address

```rust
// hickory-resolver 0.26 — we resolve explicitly rather than letting reqwest do it,
// because we must inspect the addresses BEFORE connecting.
let addrs: Vec<IpAddr> = resolver.lookup_ip(host).await?.iter().collect();
if addrs.is_empty() { return Err(Blocked::Unresolvable); }
for ip in &addrs {
    guard::check_ip(*ip)?;   // ALL of them must pass, not any
}
```

Blocked ranges (`ipnet` 2.12), checked for IPv4 **and** IPv6:

| Category | Ranges |
|---|---|
| Loopback | `127.0.0.0/8`, `::1/128` |
| Private v4 | `10/8`, `172.16/12`, `192.168/16` |
| Link-local | `169.254/16`, `fe80::/10` |
| **Cloud metadata** | `169.254.169.254`, `fd00:ec2::254`, `100.100.100.200` |
| CGNAT | `100.64/10` |
| Unspecified / broadcast | `0.0.0.0/8`, `255.255.255.255`, `::/128` |
| Multicast | `224/4`, `ff00::/8` |
| Reserved | `192.0.0/24`, `192.0.2/24`, `198.18/15`, `198.51.100/24`, `203.0.113/24`, `240/4` |
| IPv6 traps | ULA `fc00::/7`, v4-mapped `::ffff:0:0/96`, 6to4 `2002::/16`, Teredo `2001::/32` |

v4-mapped and 6to4 addresses are **unwrapped to their embedded IPv4 and re-checked**. `::ffff:127.0.0.1`
is a loopback address wearing a hat, and a naive `is_loopback()` on the v6 form returns false.

Hostname denylist on top of the IP checks: `localhost`, any `*.localhost`, `*.internal`, `*.local`,
`metadata.google.internal`, and bare single-label hostnames (no dot) — an internal hostname that
resolves publicly should still not be fetchable.

### 2.3 DNS rebinding

The classic bypass: pass the IP check, then have DNS return a private address for the actual
connection. We defeat it by **pinning the address we validated**:

```rust
// Connect to the exact IP we checked; carry the hostname for TLS SNI and Host header.
let client = reqwest::Client::builder()
    .resolve(host, SocketAddr::new(validated_ip, port))
    .redirect(reqwest::redirect::Policy::none())   // we drive redirects ourselves
    .timeout(cfg.request_timeout)
    .build()?;
```

`Policy::none()` is not optional. reqwest's built-in redirect following would resolve and connect to
the next hop **without our guard running**, which reintroduces the whole vulnerability at hop 2.

### 2.4 Redirects

We follow at most `max_redirects` (default **5**) hops, and each hop re-enters the full pipeline:
normalize → resolve → guard every IP → pin → fetch. A hop that fails the guard aborts the crawl with
`Blocked`, and the partial hop chain is recorded so the user can see *where* it went wrong. Redirect
loops are detected by tracking visited normalized URLs.

### 2.5 Response limits

- Byte cap: stream the body, abort past **2 MiB** (`max_response_bytes`). A publisher page that
  large is not a page we need to parse, and an unbounded read is a memory DoS.
- `Content-Type` must be HTML-ish (`text/html`, `application/xhtml+xml`); anything else short-circuits
  to `CrawlResult` with the status recorded and no parse attempted.
- Total wall-clock budget per crawl (**30 s** default) covering all hops, independent of the
  per-request timeout.

### 2.6 One guard, two callers

`crates/crawler::guard` is the single implementation. The public backlink checker (§50) calls the
same function — the spec requires this, and it is also how we avoid the guard drifting between an
authenticated path and an anonymous one. The public tool adds aggressive rate limits on top; it does
**not** get its own relaxed guard.

---

## 3. Link matching

The subtle part is deciding whether a page "contains" the target URL.

### 3.1 Normalization

Both the target and each discovered `href` go through the same normalizer:

1. Resolve relative hrefs against `<base href>` if present, else the **final** source URL after
   redirects (not the original — this is a common bug).
2. Lowercase scheme and host; strip the default port.
3. IDN → punycode (`idna` 1.1).
4. Strip the fragment.
5. Strip known tracking params (`utm_*`, `gclid`, `fbclid`, `mc_eid`, `ref`), then sort the
   remaining query params.
6. Normalize a trailing slash on an empty path (`example.com` → `example.com/`).
7. Percent-encoding normalized to uppercase hex.

Matching is on the normalized pair, with **`www` and scheme treated as equivalent by default**
(configurable per project). A link that moved `http` → `https` is not a changed target; treating it
as one would generate a false `TARGET_CHANGED` event for half the web.

### 3.2 Anchor text

`element.text()` joined, whitespace collapsed, trimmed. If the anchor content is an `<img>`, the
anchor is recorded as `[img: {alt}]` so an image link is not reported as an empty anchor. Comparison
against `expected_anchor_text` is case-insensitive and whitespace-normalized.

### 3.3 rel attributes

Parse `rel` into a set: `nofollow`, `sponsored`, `ugc`, `noopener`, `noreferrer`, `me`, …
**Dofollow is the absence of `nofollow`/`sponsored`/`ugc`**, not the presence of a `dofollow` token
(there is no such token). We store the full detected set in `detected_rel_attributes` so a change
from `nofollow` to `sponsored` is visible, not flattened to "still nofollow."

### 3.4 Multiple matches

A source page may link to the target more than once. We record all matches and select the
**"best"** one for the backlink's current state, preferring: exact anchor match → dofollow →
first in document order. All matches go into the check payload so the history is not lossy.

### 3.5 Signals that change interpretation

- `<meta name="robots" content="noindex|nofollow">` and `X-Robots-Tag` header → `NOINDEX_DETECTED`.
  A page-level `nofollow` makes every link on it nofollow regardless of per-link `rel`.
- `<link rel="canonical">` pointing elsewhere → `CANONICAL_CHANGED`. The link may still exist but
  its value is affected; we surface it rather than judging it.

---

## 4. Deduplication — fetch each source URL once per cycle

**Spec §27.** This is the difference between a viable and a non-viable unit economic model. If 20
customers monitor `example.com/article`, we fetch it once.

### 4.1 Mechanism

Scheduling groups by **normalized source URL**, not by backlink:

```sql
SELECT source_url_normalized, array_agg(id) AS backlink_ids
FROM backlinks
WHERE status <> 'archived' AND next_check_at <= now()
GROUP BY source_url_normalized
LIMIT $1;
```

One `CheckSourcePage` job per group, carrying the backlink id list. The worker fetches and parses
once, then evaluates every expected link against the single parsed document and writes one
`backlink_checks` row per backlink.

A `source_url_normalized` generated column plus its index makes this grouping cheap; see
[03](03-database-schema.md).

### 4.2 The tenant question

The fetched bytes are shared across organizations. That is safe because:

- The content is **public web content** that each tenant independently supplied a URL for. We are
  not reading anything a tenant could not read themselves.
- Each tenant only ever sees results about **their own** expected backlinks. The shared artifact is
  the parsed document, which is never persisted or exposed.
- The parsed document is held **in memory for the duration of one job** and dropped. There is no
  cross-request cache to leak, and no timing oracle: results reach users through async jobs, so
  "this completed suspiciously fast" is not observable.

The one place we deliberately **do not** dedupe is the public tool (§50). Sharing a cache with the
authenticated path would turn the anonymous endpoint into an oracle for "is anyone monitoring this
URL." The public tool always fetches fresh, under its own rate limit.

### 4.3 Economics tracked

Per §53, every crawl records `bytes_downloaded` and `duration_ms`, and the worker reports
`requests_per_1000_backlinks`. The dedup ratio (`backlinks_checked / http_requests`) is the headline
metric — if it approaches 1.0, dedup has stopped working and the cost model is broken.

---

## 5. Politeness and rate limiting

**Spec §28, §57 — "never hammer publisher websites."**

Two nested limits, both `tokio::sync::Semaphore`:

```
GLOBAL SEMAPHORE       (default 50 concurrent requests, CRAWLER_MAX_CONCURRENCY)
        │
        ▼
DOMAIN SEMAPHORE       (default 2 per registrable domain, CRAWLER_MAX_PER_DOMAIN)
        │
        ▼
DOMAIN MIN INTERVAL    (default 1000 ms between requests to the same domain)
```

- The domain key is the **registrable domain** via the public suffix list (`publicsuffix` 2.3), so
  `a.blog.example.co.uk` and `b.blog.example.co.uk` share a budget. Using the raw host would let a
  single publisher's subdomains multiply our request rate against one server.
- Domain semaphores live in a `DashMap` with idle eviction, so we do not leak a semaphore per domain
  seen, forever.
- **Retries**: exponential backoff, base 2 s, factor 2, cap 5 min, with full jitter. Applied to
  timeouts, connection errors and `5xx`. **Not** applied to `4xx` — a 404 is an answer, not a
  failure.
- `429` and `503` honour `Retry-After` when present, up to a cap.
- `robots.txt` is fetched per domain and cached for 24 h. We respect `Disallow` for our user agent.
  A blocked source page yields status `blocked`, which is an honest "could not verify," **not**
  `lost`. This distinction is the whole point of §24.
- User agent identifies us and points at a policy page:
  `Backlio/1.0 (+https://backlio.app/bot)`.

---

## 6. `CrawlResult`

```rust
pub struct CrawlResult {
    pub source: FetchOutcome,          // status, final_url, redirect_chain, bytes, duration
    pub source_canonical:  Option<Url>,
    pub source_robots:     RobotsDirectives,   // noindex / nofollow, from meta AND header
    pub matches: Vec<LinkMatch>,       // every <a> matching the target
    pub target: Option<FetchOutcome>,  // when check_target was requested
    pub crawled_at: DateTime<Utc>,
    pub crawl_id: Uuid,                // correlates logs, checks and events
}

pub struct LinkMatch {
    pub href_raw: String,
    pub href_normalized: Url,
    pub anchor_text: String,
    pub rel: RelSet,
    pub is_dofollow: bool,             // derived: no nofollow/sponsored/ugc AND page not nofollow
    pub position: usize,               // document order
}

pub enum FetchOutcome {
    Ok { status: u16, final_url: Url, redirect_chain: Vec<Url>, bytes: usize, duration_ms: u32 },
    HttpError { status: u16, final_url: Url },
    Timeout,
    ConnectionFailed { kind: ConnErrorKind },
    Blocked { reason: BlockReason },   // SSRF guard, robots.txt, oversized, bad content type
    TooManyRedirects { chain: Vec<Url> },
}
```

`FetchOutcome` distinguishes **"the link is not there"** from **"we could not look"** at the type
level. The state machine ([08](08-backlink-state-machine.md)) consumes this distinction; a
`Timeout` can never be collapsed into "lost" by accident, because they are different variants.

---

## 7. Test fixtures

Per §47, `crates/crawler/tests/fixtures/` covers, against a local `wiremock` server:

| | |
|---|---|
| backlink present, dofollow | backlink present, nofollow |
| `rel="sponsored"` | `rel="ugc"` |
| backlink absent | anchor text changed |
| target URL changed | relative href + `<base href>` |
| protocol-relative href (`//host/path`) | IDN / punycode host |
| 301 → 302 → 200 chain | redirect loop |
| redirect to private IP (**must block**) | `>5` redirects |
| 404 source | 500 source |
| timeout | oversized body (>2 MiB, must abort) |
| malformed / unclosed HTML | non-HTML content type |
| `<meta robots noindex>` | `X-Robots-Tag: noindex` header |
| canonical pointing elsewhere | multiple matching links on one page |
| `<a>` wrapping `<img>` | empty anchor |

SSRF cases get their own suite (`tests/ssrf.rs`) asserting `Blocked` for: `localhost`, `127.0.0.1`,
`[::1]`, `::ffff:127.0.0.1`, `169.254.169.254`, `10.0.0.1`, `192.168.1.1`, `172.16.0.1`,
`fd00::1`, `2002:7f00:1::`, `file:///etc/passwd`, `http://user:pass@host/`, `http://host:22/`,
a DNS name resolving to a private address, and a redirect chain whose second hop is private.
