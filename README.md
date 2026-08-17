# Himalayas Browser

**The browser that makes mainstream browsers look slow. Built for agents, loved by speed fanatics.**

[![GitHub Stars](https://img.shields.io/github/stars/Mullassery/Himalayas-Browser?style=social)](https://github.com/Mullassery/Himalayas-Browser) [![CI](https://github.com/Mullassery/Himalayas-Browser/actions/workflows/ci.yml/badge.svg)](https://github.com/Mullassery/Himalayas-Browser/actions/workflows/ci.yml) [![License](https://img.shields.io/badge/license-Proprietary-blue?style=flat-square)](LICENSE) [![Version](https://img.shields.io/badge/version-0.5.2-blue?style=flat-square)](.)

---

## The Proof

Two different things, kept separate rather than blended into one table: the **headless daemon** (`himalayas daemon` — no window, no rendering, the thing automation/agents actually talk to) and the **native GUI shell** (`himalayas-desktop` — real tabs, real page rendering). Mixing their numbers together would be misleading, since they're not doing the same work.

### Headless daemon (measured)

```
                      Startup         Memory (Idle)
                      -------         -----
Himalayas:            ~30ms*          ~8MB*
Mainstream:            2.1s           120MB
Firefox:               2.4s            80MB
Safari:                1.2s           200MB

ADVANTAGE:            ~70x faster*    ~15x lighter*
```
\* Time to first `/health` 200 response and idle RSS, measured directly against a local release build (`cargo build --release`, `strip = true`) on Apple Silicon — see PERFORMANCE.md for methodology. Mainstream/Firefox/Safari figures are their own full application startup (window + rendering), not a directly equivalent comparison — included for orientation, not an apples-to-apples claim.

**Real-world impact:**
- Daemon ready in ~30ms (headless automation immediately)
- ~8MB idle (run 50+ concurrent agents on 1GB RAM)
- Runs on 256MB IoT devices (only browser that does)

### Native GUI shell (`himalayas-desktop`, not yet independently re-measured)

```
                      5 Tabs         Battery (4h Idle)
                      ------         -----------------
Himalayas:            95MB           94%
Mainstream:           600MB          52%
Firefox:              450MB          78%
Safari:               500MB          87%

ADVANTAGE:            6x leaner      8x longer
```
From the original benchmark pass (see PERFORMANCE.md) — not re-measured against the current native shell build the way the headless daemon numbers above were. Treat as the earlier estimate it is until re-verified.

- 94% battery after 4 hours (one charge = 3 days of browsing) — original benchmark, not re-verified

---

## Note: Initial Release (Local Testing)

> **This initial release is built for local testing (running on `localhost`). DNS and CDN support will be added in upcoming updates, and the full suite of other features is rolling out soon!**

---

## Why Himalayas?

### 1. Actually Private

```
Your data:            Stays local, zero tracking, no cloud sync
Mainstream browsers:  Vendor gets everything (by default)
Firefox's data:       Mozilla gets telemetry (by default)
Safari's data:        Apple iCloud sync (by default)
```

- Every session private by default (no configuration needed)
- No cloud sync (memory-first architecture)
- Works offline (no internet required)
- `block_trackers` config option (on by default) — not yet wired to enforcement logic in the current source, see Known Issues

**Mainstream browsers:** Public by default, vendor tracks clicks, surveillance capitalism model  
**Himalayas:** Private by default, zero tracking, user-centric design

---

### 2. Genuinely Fast

```
Time to Ready               Memory per Tab
---                         ---
Mainstream 2.1 seconds      Mainstream 120MB
Firefox    2.4 seconds      Firefox    80MB
Safari     1.2 seconds      Safari     200MB
Himalayas  ~30 milliseconds Himalayas  ~8MB (idle daemon)
```

- Daemon ready in ~30ms (measured; see PERFORMANCE.md)
- GUI optional (lazy loaded on demand)
- Headless automation first (not an afterthought)
- Scales to 50+ concurrent agents on 1GB RAM
- 60fps scrolling on all screen sizes (360px to 4K)

---

### 3. Works Everywhere

```
Device Type              Himalayas   Mainstream      Firefox         Safari
-----------              ---------   ------          -------         ------
Desktop (Windows/Mac)    Yes         Yes             Yes             macOS only
Mobile (iOS/Android)     Yes         Yes             Yes             iOS only
Raspberry Pi             Yes         No (too large)  No (too large)  No
256MB IoT Devices        Yes         No              No              No
```
Binary-size/RAM figures from the original benchmark pass (see PERFORMANCE.md), not independently re-measured this round.

- ~6MB binary (vs 150MB+ competitors)
- Runs on 256MB devices (only browser viable here, per the original benchmark pass)
- Fleet management (coordinate 100+ devices) — planned, see Phase 7 in Status below, not built yet

---

## Native Browser Shell (`himalayas-desktop`)

A real, GPU-rendered browser window — not just the headless daemon — built on a vendored, patched fork of [Blitz](https://github.com/DioxusLabs/blitz) (pure-Rust HTML/CSS/JS rendering, no Chromium/CEF embedded). Opt-in today via the `js_engine` Cargo feature (`cargo run --bin himalayas-desktop --features js_engine`) — off by default while it matures, not yet the primary build.

What's real and working, not aspirational:
- Tabs (drag to reorder, pin/unpin with compact favicon-style pinned tabs that survive a restart), address bar with real navigation/history, keyboard shortcuts
- Bookmarks: a star button plus a full Bookmark Manager (folders, search, sort, drag-to-move, multi-select, HTML/JSON import & export)
- A real disk HTTP cache (Cache-Control/ETag/Last-Modified-aware) covering both page navigation and subresources, automatically size-capped (250MB default, evicting oldest entries first, in the background so it never adds to daemon startup time) rather than growing unbounded
- Real image support: JPEG/PNG/WebP/AVIF/GIF/BMP, `srcset`/`sizes` responsive images, `<picture>`/`<source>` art direction, `loading="lazy"`, animated GIF playback — with real decoder resource limits against oversized/malicious images
- Real `<audio>` playback (MP3/AAC/WAV/Vorbis/FLAC, via `rodio`/`symphonia`) — `<video>` isn't built yet, a deliberate call pending a real decode-architecture decision (see `docs/NATIVE_RENDERING_PLAN.md`)
- Accessibility-motivated shell scaling (bigger address bar/tabs, independent of page-content zoom) and a page-zoom setting
- `himalayas mcp`: a real [Model Context Protocol](https://modelcontextprotocol.io) server over stdio, so Claude Desktop, Claude Code, and other MCP clients can drive Himalayas directly — navigate/query/click/input/get_text/submit_form and more, as real MCP tools

Full patch-by-patch engineering log (what's built, what's deliberately deferred, and why): `docs/NATIVE_RENDERING_PLAN.md`.

---

## Install

No packaged installer (.dmg/.deb/.exe) is published yet — this initial release is build-from-source/cargo-only (matches the local-testing note above). Two real, working ways to get it running today:

### Homebrew (macOS/Linux)
```bash
brew tap mullassery/himalayas-browser https://github.com/Mullassery/Himalayas-Browser
brew install --HEAD himalayas
```

### From source (any platform with Rust 1.75+)
```bash
git clone https://github.com/Mullassery/Himalayas-Browser.git
cd Himalayas-Browser
cargo install --path . --locked
```

Verify installation:
```bash
himalayas daemon &
curl http://127.0.0.1:8080/health
# {"status":"healthy","uptime_seconds":0}
```

---

## Full Comparison

### Security & Privacy

| Feature | Himalayas | Mainstream | Firefox | Safari |
|---------|-----------|--------|---------|--------|
| **Default Private Session** | Every session | Public | Public | Public |
| **Permission Expiry** | Auto (time-bound) | Never | Never | Never |
| **Cloud Sync Required** | No | Vendor account (default) | Mozilla (default) | iCloud (default) |

### Performance

Headless daemon (measured, see PERFORMANCE.md for methodology) and native GUI shell (original benchmark pass, not re-measured) are kept in separate tables — they're not doing the same work, so a single blended table would overstate how much of this is freshly verified.

**Headless daemon (`himalayas daemon`) — measured**

| Metric | Himalayas | Mainstream | Firefox | Safari |
|--------|-----------|--------|---------|--------|
| **Startup** | ~30ms (measured) | 2.1s | 2.4s | 1.2s |
| **Memory (idle)** | ~8MB (measured) | 120MB | 80MB | 200MB |

**Native GUI shell (`himalayas-desktop`) — original benchmark pass, not re-verified**

| Metric | Himalayas | Mainstream | Firefox | Safari |
|--------|-----------|--------|---------|--------|
| **5 Concurrent Tabs** | 95MB | 600MB | 450MB | 500MB |
| **50 Concurrent Agents** | 580MB | 6GB | Not supported | Not supported |
| **Battery (4h idle)** | 94% remaining | 52% remaining | 78% remaining | 87% remaining |
| **Paint Time (TTI)** | 12-18ms | 24-35ms | 18-26ms | 11-17ms |

### Features

| Feature | Himalayas | Mainstream | Firefox | Safari |
|---------|-----------|--------|---------|--------|
| **Daemon Architecture** | Native | Workaround (headless) | Workaround (headless) | No |
| **Multi-Agent Isolation** | Per-tab/per-session cookies & storage (`IsolationMode`) | Shared resources | Shared resources | Shared resources |
| **IoT Support** | ~6MB binary | 350MB (not viable) | 220MB (not viable) | No |
| **Fleet Orchestration** | Planned (Phase 7) | No | No | No |

---

## Real-World Use Cases

No language-specific SDK (Python or otherwise) ships today — the real interfaces are the `POST /agent` HTTP endpoint and the `himalayas mcp` stdio MCP server. The examples below use the HTTP endpoint directly via `curl`; see `src/server.rs` and `src/mcp.rs` for the full request/response shapes.

### Autonomous Agents
```bash
himalayas daemon &

curl -X POST http://127.0.0.1:8080/agent \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"scraper-bot","session_id":"s1","action":"navigate","parameters":{"url":"https://example.com"}}'

curl -X POST http://127.0.0.1:8080/agent \
  -H 'Content-Type: application/json' \
  -d '{"agent_id":"scraper-bot","session_id":"s1","action":"query","parameters":{"selector":"a"}}'
```
Supported `action` values today: `navigate`, `query`, `click`, `get_text`, `submit_form`.

### Multi-Agent Orchestration
```bash
# Each distinct session_id gets its own isolated cookies/storage (IsolationMode)
for i in $(seq 1 10); do
  curl -X POST http://127.0.0.1:8080/agent \
    -H 'Content-Type: application/json' \
    -d "{\"agent_id\":\"bot-$i\",\"session_id\":\"session-$i\",\"action\":\"navigate\",\"parameters\":{\"url\":\"https://example.com\"}}" &
done
wait
```
The 50-concurrent-agents/580MB figure elsewhere in this document is from the original benchmark pass (see PERFORMANCE.md), not re-verified this round.

### Privacy-First Browsing
```bash
# Headless (no window):
himalayas daemon
# Native GUI shell (real tabs/rendering, opt-in, see "Native Browser Shell" above):
cargo run --bin himalayas-desktop --features js_engine
```
Every session is private by default, no configuration required — see PERFORMANCE.md and the tables above for what's measured vs. original-pass estimates.

---

## Hardware Tier Support

From the original benchmark pass (see PERFORMANCE.md) — not independently re-measured this session; treat as an estimate pending re-verification, same as the native GUI shell numbers in "The Proof" above.

### Constrained Devices (256-512 MB RAM)
- Binary: 8-12 MB (smallest browser available)
- Runtime: 18-22 MB (viable)
- Status: ONLY viable browser for IoT automation
- Competitors: All unable to run

### Standard Tier (2-4 GB RAM)
- Binary: 25-35 MB
- Runtime: 65-80 MB (80% less than mainstream browsers)
- 5 Tabs: 95 MB (6x more efficient)
- Status: Optimal for laptops and desktops

### High-Performance (8+ GB RAM)
- Binary: 50-65 MB
- Runtime: 120-180 MB (aggressive caching enabled)
- Multi-agent: 50+ concurrent agents supported
- Status: Workstation and server orchestration ready

---

## Screen Size Performance

From the original benchmark pass (see PERFORMANCE.md) — not independently re-measured this session. Paint time consistency across all devices:

| Screen Size | Device Type | Himalayas | Mainstream | Firefox | Safari |
|-------------|-------------|-----------|--------|---------|--------|
| 360x640 | Mobile | 12-18ms | 24-35ms | 20-28ms | 10-16ms |
| 768x1024 | Tablet | 16-22ms | 28-40ms | 22-30ms | 14-20ms |
| 1366x768 | Desktop | 14-18ms | 24-32ms | 20-26ms | 12-18ms |
| 1920x1080 | Full HD | 12-18ms | 24-35ms | 18-26ms | 11-17ms |
| 2560x1440 | QHD | 15-20ms | 28-38ms | 22-30ms | 15-21ms |
| 3840x2160 | 4K | 18-26ms | 35-50ms | 28-40ms | 18-28ms |

Himalayas maintains sub-26ms paint time across all screen sizes. Competitors vary 10-50ms with degradation at higher resolutions.

---

## Battery Efficiency

From the original benchmark pass (see PERFORMANCE.md) — not independently re-measured this session. Mobile device battery test (4 hours idle, screen off):

| Browser | Battery After 4h | Drain Rate | Estimated Daily |
|---------|------------------|-----------|-----------------|
| Himalayas | 94% | 1.5%/hour | 62+ hours (2.5 days) |
| Safari | 87% | 3.25%/hour | 31 hours (1.3 days) |
| Firefox | 78% | 5.5%/hour | 18 hours (<1 day) |
| Mainstream | 52% | 12%/hour | 8 hours (<1 day) |

One phone charge with Himalayas lasts 8x longer than mainstream browsers in idle mode. For active users, the advantage is 3-4x longer battery life.

---

## Architecture Advantages

Daemon-First Design:
- Headless operation by default (GUI optional, lazy-loaded)
- Ready for automation immediately
- Scales to multiple concurrent agents
- Resource efficient (no GUI overhead)

Hardware-Adaptive Loading:
- Different binaries for each hardware tier
- Constrained (8MB) to High-Perf (65MB)
- Automatic tier detection on install

Runtime-First Model:
- Browser runtime is infrastructure
- GUI is optional client application
- Can run headless forever
- Multi-client support possible

---

## Security by Default

What's actually implemented and verifiable in source today (see `src/permission/engine.rs`, `src/browser/tabs.rs`, `src/browser/mod.rs`):
- Per-tab/per-session isolation: each session gets its own cookies and storage (`IsolationMode::Isolated` vs. `Shared`), so concurrent agents/tabs don't see each other's data by default
- Explicit, resource+action-scoped permission grants with automatic time-bound expiry (`PermissionEngine`/`PermissionGrant::is_expired`) rather than standing, indefinite permissions

An earlier draft of this README listed 17 named security policies (prompt injection detection, a secret vault, malware/download scanning, audit logging, network policy enforcement, and others). Searching the current source turned up no implementation for most of those, so the list has been cut down to only what's verifiable above; see Known Issues below.

---

## Performance Benchmarks

Full detailed benchmarks available in PERFORMANCE.md (630 lines) — note that document predates the headless-daemon measurements in "The Proof" above and hasn't been fully reconciled with them yet; where the two disagree, treat the numbers in "The Proof" as current.

Key findings (see "The Proof" above for what's independently measured this round vs. original-benchmark-pass figures):
- Startup: headless daemon ~30ms measured; GUI shell startup not yet re-measured
- Memory: headless daemon ~8MB idle measured; GUI shell memory not yet re-measured
- Scaling: 6.7x better scaling per concurrent tab (original benchmark pass)
- Battery: 8x better battery efficiency, mobile (original benchmark pass)
- Paint time: competitive with Safari, faster than mainstream browsers (original benchmark pass)
- Multi-agent: only browser supporting true bot isolation and scaling

---

## Documentation

- **PERFORMANCE.md** - Detailed benchmarks (630 lines)
- **docs/GETTING_STARTED.md** - Step-by-step setup per platform
- **docs/USAGE.md** - Usage guide (daily usage, AI features, document management, privacy, workspaces)
- **docs/UI_UX_VISION.md** - UI/UX design vision (adaptive interface, AI workspace, workspaces, design language)
- **docs/ROADMAP.md** - Keyboard & trackpad support specification (planned, Phase 6)
- **docs/NATIVE_RENDERING_PLAN.md** - Native rendering engine spike findings and follow-up plan
- **MCP Server** - `himalayas mcp` runs a real Model Context Protocol server over stdio, exposing navigate/query/click/input/get_text/submit_form/go_back/go_forward/get_current_url/get_history as MCP tools — for Claude Desktop, Claude Code, and other MCP clients to drive Himalayas directly
- **docs/browser-comparison-interactive.html** - Browser feature matrix (HTML)

There is currently no `SECURITY.md` in this repo despite an earlier README draft referencing one — see Known Issues.

---

## Status

| Phase | Status | Features |
|-------|--------|----------|
| Phase 0 | Complete | Health server, daemon foundation (20 tests) |
| Phase 1-4 | Complete | Core platform, clipboard, tabs, config (102 tests) |
| Phase 2.5 | Complete | Adaptive intelligence engine (143 tests) |
| Phase 3+ | In Progress | Multi-form factors, fleet management |

Latest Version: 0.5.2 (matches `Cargo.toml`)  
Tests Passing: 524 across the full workspace (`cargo test --features full`, the native-shell binary's own suite, and the vendored rendering engine's — 2 additional tests are `#[ignore]`d live-network checks, not counted here)  
Code: Rust. No Python (or other language) SDK ships today — automation goes through the HTTP `/agent` endpoint or the `himalayas mcp` MCP server (see Real-World Use Cases above).

---

## Known Issues

This is an experimental, pre-1.0 project. Fixed in this documentation pass (source-verified, not guessed):
- Removed "Native sensor support (RGB, Thermal, LiDAR, IMU)" and "ROS 2 integration" claims — no ROS 2 dependency and no camera/thermal/LiDAR code exist anywhere in `src/`; only IMU exists, and only as one input to on-device location fusion, not general robotics sensor support
- Cut the "17 security policies implemented" list down to the two that are actually in source (per-session cookie/storage isolation, time-bound permission grants) — the other ~15 named policies (prompt injection detection, a secret vault, malware/download scanning, audit logging, network policy enforcement, localhost protection, extension capability control, data classification, risk-adaptive policy, re-auth binding, age-based profiles) returned no matches anywhere in `src/`
- Removed "100% ads blocked" / "100% tracking pixels blocked" claims — `block_trackers` in `src/config.rs` is a config field that defaults to `true` but is never read anywhere else in the codebase; there's no enforcement logic behind it yet
- Replaced the "Real-World Use Cases" examples: the old ones called `himalayas tab create ...`, `himalayas daemon --sensors rgb,thermal,lidar --ros2`, a bare `himalayas <url>`, and a Python `from himalayas import Agent` SDK — none of which exist. The real CLI has exactly three subcommands (`daemon`, `benchmark`, `mcp`); automation goes through the `POST /agent` HTTP endpoint or the `himalayas mcp` stdio server. Examples now use real `curl` calls against `/agent`
- Fixed the License section — the repo has no `LICENSE` file despite the badge and old text pointing to one
- Removed the `SECURITY.md` documentation entry — that file doesn't exist in the repo
- Genericized one "Google" vendor mention in a comparison table

Known gaps not fixed here (flagging rather than fabricating a fix):
- No `LICENSE` file is committed — add one before treating the "Proprietary" claim as legally meaningful
- The India Stack module (`src/india_stack/`, referenced from `docs/GETTING_STARTED.md`'s "Government workflows" bullet) is stubbed: its own source comments read "TODO: Implement actual DigiLocker OAuth2 flow / API call", "TODO: Implement actual eSign flow", "TODO: Implement signature verification", "TODO: Implement actual tesseract integration" — not functional yet
- `himalayas-desktop` (native GUI shell) performance numbers throughout this README are from an earlier benchmark pass and have not been re-measured against the current build, as already noted inline above
- `<video>` isn't implemented in the native shell yet (see "Native Browser Shell" above and `docs/NATIVE_RENDERING_PLAN.md`)
- As of this pass: 0 open GitHub issues on this repo; roughly a dozen `TODO`/`FIXME` comments in `src/`, mostly concentrated in the India Stack module above

---

## Contributing

Report bugs or suggest features:
- [GitHub Issues](https://github.com/Mullassery/Himalayas-Browser/issues)
- [GitHub Discussions](https://github.com/Mullassery/Himalayas-Browser/discussions)

---

## License

Proprietary (matches `license = "Proprietary"` in `Cargo.toml` and the Homebrew formula).

No `LICENSE` file is currently committed to this repository, so the specific terms are not yet written down anywhere authoritative — the badge above and any link to `LICENSE` will 404 until one is added. Treat this as all-rights-reserved until a `LICENSE` file is committed. See Known Issues.

---

## Why This Exists

Himalayas is not a mainstream-browser replacement. It's a different category entirely.

Mainstream browsers optimize for web compatibility and surveillance.  
Himalayas optimizes for autonomous agents, privacy, and efficiency.

We solve different problems:
- Speed where it matters (startup, responsiveness, battery)
- Privacy by default (not by configuration)
- Efficiency at scale (50+ agents on 1GB RAM)
- Agent automation (first-class design, not afterthought)
- Universal deployment (desktop, mobile, IoT)

---

**Himalayas Browser: Reaching the peak of autonomous computing.**

[Star on GitHub](https://github.com/Mullassery/Himalayas-Browser) | [Quick Start](#install) | [Full Benchmarks](./PERFORMANCE.md) | [Report Issue](https://github.com/Mullassery/Himalayas-Browser/issues)

Made by [Mullassery](https://github.com/Mullassery)
