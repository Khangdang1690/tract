# tract — Design Document

A first-principles design for the open-source primitive that gives agents
human-equivalent web access through a single function call.

> `tract` — give your agent the page, the way a human would read it.


## 1. The problem, in one sentence

Agents read the web like humans, but they can't tell when the web is lying to
them — and the modern web actively fights anything that looks automated.

Every agent that touches the web today depends on the same broken stack: a
script tries to fetch a URL, fails 30-60% of the time, and when it does
succeed, it might get hallucinated honeypot data, ad-laden HTML, or content
that's nothing like what a human would see. There is no clean
"URL → reliable, clean, agent-ready content" primitive.


## 2. The finished product

A developer installs one package. They write one line. They get back content.

```python
from tract import fetch
content = fetch(url)
```

For this to amaze, every one of these must be true:

- Works on any URL a human browser would have worked on.
- Returns within ~1 second common case, ~3 seconds hard case.
- Returns what a human would have read — not nav, not ads, not banners.
- Output is markdown, structured, ready for an LLM.
- Content is real, not a poisoned honeypot or stale cache.
- Tells you what kind of source it is and the confidence in the result.
- No configuration, API keys, proxies, or operator decisions required.
- Doesn't break next month when bot-detection shifts.

If any one is false, `tract` is just another scraping library.


## 3. The four irreducible operations

Stripped to fundamentals, `tract` does exactly four things:

1. Send bytes to a remote server and receive bytes back (network I/O).
2. Optionally execute JavaScript on those bytes (JS runtime computation).
3. Parse the result and extract meaningful content (text processing).
4. Decide which combination of the above to use for this URL (a decision).

Everything else — caching, retries, proxies, fingerprinting, stealth — is
implementation detail in service of one of these four.


## 4. Binding constraints

- **Latency floor:** TCP+TLS handshake ~100-300ms, JS execution ~200ms-2s.
  Anything `tract` adds over ~50ms of overhead is a bug.
- **JS requires a real engine:** large fraction of sites need JavaScript
  execution. Either embed V8/Chromium/WebKit or delegate to a process that does.
- **Detection is fingerprint-based:** TLS (JA3/JA4), HTTP/2, browser
  (canvas/WebGL/fonts/timing), behavioral patterns. Must match a real
  browser exactly at every layer. Requires low-level network control.
- **Browser cost:** headless Chromium ~150-300MB RAM, 1-2s cold start.
  Cannot spawn per-request. Requires pooling, requires long-running process.

These constraints force most architectural decisions.


## 5. The twelve components

Each is forced by a requirement. Remove one and a requirement fails.

1. **Strategy pool** — set of fetching implementations per target class
2. **Strategy selector** — picks one strategy per URL, learns from outcomes
3. **HTTP engine** — fast-path: precise TLS/HTTP fingerprints, no JS
4. **Browser engine** — slow-path: real browsers with stealth, JS-capable
5. **Resource pool** — reuses TLS sessions, browsers, DNS, cookies
6. **Cache** — content-addressed storage with freshness rules
7. **Extractor** — HTML to structured markdown
8. **Verifier** — statistical and contextual poisoning checks
9. **Classifier** — labels source type (primary, blog, AI-generated)
10. **Learning loop** — turns outcomes into selector updates
11. **Profile channel** — pushes fingerprint/stealth/extractor updates
12. **Daemon + client SDKs** — long-lived process + thin per-language wrappers


## 6. Language choices (derived, not preferred)

- **Daemon (components 1-11):** Rust — only language giving TLS-level control
  + async + memory safety + no GC pauses + memory efficiency for long uptime.
- **Persistence:** SQLite — boring correct answer.
- **Extractor:** Rust (porting readability) — latency budget too tight for
  IPC to a Python/Node sidecar.
- **Client SDKs:** Python, TypeScript, Go — thin wrappers (~200 lines each).

**Topology:** one Rust daemon (`tractd`), all logic inside, accessed through
SDKs over a Unix domain socket (or localhost TCP on Windows). Single binary,
no external dependencies, SQLite for state.


## 7. End-to-end flow of a single fetch

1. SDK receives URL, sends over local socket to `tractd` (<1ms).
2. `tractd` checks cache. If fresh + verified → return. Done.
3. Selector examines URL: domain, path, history, cluster, time signals.
   Outputs strategy choice (<1ms).
4. Selected strategy executes (HTTP engine or browser engine).
5. Verifier scores the raw response. If too low → escalate strategy or
   return clear failure (no silent garbage).
6. Extractor produces clean markdown. Classifier labels source.
7. Cache stores raw HTML with freshness metadata. Learning loop logs
   outcome (async).
8. `tractd` returns content + classification + confidence + trace_id to SDK.
9. SDK returns the content string to the user.


## 8. The strategy selector

**Framing:** contextual bandit problem. Context: URL features + host history.
Arms: ~20-50 distinct strategies (engine × fingerprint × IP × budget).
Reward: success weighted by inverse cost (latency + dollars).
NOT a deep learning problem. NOT RL with discounting.

**Policy representation (hierarchical, 3 layers):**

1. Per-host: empirical success/latency per strategy. Dominates after ~20 samples.
2. Per-cluster: hosts grouped by detected protection vendor / CMS / response patterns.
3. Cold-start prior: URL-only features. Few hundred hand-curated rules initially.

**Storage:** a table, not a model. Per host: counts of (attempts, successes)
and latency samples per strategy. Fits in tens of MB at million-host scale.
Loads instantly. Ships in single SQLite file. Debuggable by reading rows.

**Selection rule:** Thompson sampling.
Per strategy per host: success rate ~ Beta(successes+α, failures+β).
Per strategy per host: latency ~ log-normal from samples.
Pick strategy minimizing `sampled_latency / sampled_success_probability`.
Naturally handles explore/exploit. ~200 lines of Rust.


## 9. The verifier

**Core question:** is this response a genuine representation of what a human
would see at this URL?

**Failure modes to catch:**

1. **Soft blocks** — 200 OK with block/CAPTCHA/challenge page content.
2. **Honeypots** — 200 OK with deliberately fabricated content.
3. **Stub responses** — minimal HTML where JS-rendered content was expected.
4. **Wrong-page** — redirects to error/generic landing as 200 OK.
5. **Truncated** — real content but cut off mid-response.
6. **Stale / wrong-ver** — days-old cache or default version served.

**Combination:** calibrated logistic regression over signals. NOT naive
average. Interpretable, cheap, easy to recalibrate.

**Calibration discipline (critical):** score of 0.7 must mean "70% of
responses scored this way were real." Reliability diagrams in evaluation.
Periodic recalibration.

**Operating point:** biased toward false positives. Better to retry an
actually-real page than to return a fabricated one.


## 10. Evaluation framework

**"Works" means:** for URLs an agent dev cares about, `tract` returns clean
LLM-ready content quickly and reliably.

**Measure three things on three axes:**

- Things: success rate, latency, content quality.
- Axes: web diversity, time evolution, realistic agent workload.

**URL corpus:** distribution matches real agent traces; spread of difficulty
(25% trivial / 50% moderate / 20% hard / 5% adversarial); refreshes
continuously; practical size 2,000-5,000 URLs.

**Ground truth:** high-effort reference fetch using real browser, no
automation flags, manual cookie handling, full JS execution, no time
pressure. Compare extracted markdown to reference.

**Metrics that matter:**

- **Wall-clock time:** p50 < 1s, p90 < 3s, p99 < 8s.
- **Content fidelity rate:** >95% moderate, >80% hard, >40% adversarial.
- **Hard-failure rate:** <5% on moderate; should be >10x more common than silent-wrong.
- **Silent-wrong rate:** <1%. THE WORST FAILURE MODE.
- **Cost per successful fetch.**

**Discipline:** every bug report becomes a test case. The corpus becomes
institutional memory of every problem encountered.


## 11. Where the engineering risk lives

Most of the code is glue. The hard parts:

1. **HTTP engine's fingerprint accuracy** — permanent maintenance. The team
   that owns this owns the moat.
2. **Browser engine's stealth patches** — Chrome leaks "I am automated"
   through dozens of channels.
3. **Strategy selector's accuracy** — easy URLs to browser = slow; hard URLs
   to HTTP = fail.
4. **Extractor's coverage** — bad extraction = LLM sees junk.
5. **Verifier's calibration** — false positives waste fetches; false
   negatives let poisoned content through.

Everything else is solved-problem engineering.


## 12. The two foundations to stress-test

1. **Calibration discipline on the verifier.** Miscalibrated scores poison
   every downstream decision.
2. **Reference content generation for the eval.** If ground truth is wrong,
   every metric is wrong.
