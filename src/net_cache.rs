//! Shared HTTP disk cache for the two places this crate makes real
//! top-level page requests via a bare `reqwest::Client`: `Navigator` (the
//! `/app` web-shell path, `src/browser/navigator.rs`) and the native
//! desktop shell's top-level document fetch (`fetch_document_with_session`
//! in `src/bin/desktop.rs`). Neither had any caching before this — every
//! navigation hit the network even for a page whose response headers say
//! it's safe to reuse.
//!
//! Mirrors the equivalent, already-working setup in
//! `vendor/blitz/packages/blitz-net/src/lib.rs::Provider::new` (a
//! separate, vendored Cargo workspace, so that code can't just be called
//! directly from here) — same crates, same private-cache rationale: the
//! default `HttpCacheOptions` (`shared: true`) evaluates cache policy as a
//! shared/proxy cache, which treats any response carrying `Set-Cookie`
//! without an explicit `Cache-Control: public`/`immutable` as immediately
//! stale. Both of *this* crate's callers attach per-tab session cookies to
//! every request, so nearly everything would look like a `Set-Cookie`
//! response to a shared-cache policy — `shared: false` (a real single-user
//! browser cache, matching what an actual browser does) avoids defeating
//! caching for exactly the requests this crate makes.
//!
//! Uses Himalayas' own cache directory (`directories::ProjectDirs`, same
//! crate `src/bin/desktop.rs`'s pinned-tab session file already uses for
//! its config directory) rather than blitz-net's `com.DioxusLabs.Blitz`
//! identity — a deliberately separate on-disk cache from blitz-net's own
//! (subresource fetches — images/CSS/fonts — still go through
//! `blitz_net::Provider`, unaffected by this), since document responses
//! and subresource responses have different reuse patterns and there's no
//! benefit to forcing them to share one cache bucket.

use http_cache_reqwest::{CACacheManager, Cache, CacheMode, CacheOptions, HttpCache, HttpCacheOptions};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};

fn http_cache_dir() -> std::path::PathBuf {
    // Test builds get a fresh, unique-per-call directory instead of the
    // real shared one — a real regression this fixed, not defensive
    // paranoia: tests spin up many rapid, short-lived local mock HTTP
    // servers (`mockito`) on OS-assigned ephemeral ports, which *can* get
    // reused across otherwise-unrelated test server instances within the
    // real cache's heuristic-freshness window for a response with no
    // explicit `Cache-Control` header. Sharing the real cache directory
    // meant one test's response could get served back for a *different*
    // test's identical `http://127.0.0.1:<reused-port>/` a moment later —
    // `server::tests::test_agent_endpoint_navigate_query_get_text_full_flow`
    // failed exactly this way (asserted title "Test Page", got the
    // previous test's "Untitled" instead) once enough tests ran
    // concurrently to make the port-reuse window realistic. Each call
    // still returns the *same* directory for the lifetime of the
    // `ClientWithMiddleware` it's baked into (`cached_client` calls this
    // once per `Navigator`), so within-test caching behavior (e.g.
    // `test_navigate_does_not_refetch_a_cacheable_url`) is unaffected —
    // only *cross*-test sharing is what this removes.
    #[cfg(test)]
    {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!("himalayas-http-cache-test-{}-{n}", std::process::id()))
    }
    #[cfg(not(test))]
    {
        directories::ProjectDirs::from("com", "Himalayas", "Himalayas")
            .map(|dirs| dirs.cache_dir().join("http"))
            .unwrap_or_else(|| std::env::temp_dir().join("himalayas-http-cache"))
    }
}

/// Wrap an already-configured base client (timeouts, user-agent, redirect
/// policy, etc. — all set up by the caller first, unaffected by this) with
/// the disk-cache middleware. The returned `ClientWithMiddleware` exposes
/// the same `.get()`/`.post()`/`.header()`/`.send()` shape `reqwest::Client`
/// does, so callers don't need any other changes beyond the field type.
///
/// Takes `reqwest_middleware::reqwest::Client` specifically (that crate's
/// own re-export of the exact reqwest version it was built against, 0.13.x
/// as of this writing) rather than this crate's own direct `reqwest`
/// dependency (pinned to 0.12 elsewhere in `Cargo.toml`) — `reqwest` isn't
/// semver-compatible across that boundary (`Client`/`RequestBuilder` are
/// different, incompatible types between the two), so callers need to
/// build their base client via `reqwest_middleware::reqwest::Client::builder()`,
/// not the bare `reqwest::Client::builder()`, or this won't type-check.
pub fn cached_client(base: reqwest_middleware::reqwest::Client) -> ClientWithMiddleware {
    let cache_manager = CACacheManager::new(http_cache_dir(), true);
    spawn_http_cache_eviction();
    ClientBuilder::new(base)
        .with(Cache(HttpCache {
            mode: CacheMode::Default,
            manager: cache_manager,
            options: HttpCacheOptions {
                cache_options: Some(CacheOptions { shared: false, ..Default::default() }),
                ..Default::default()
            },
        }))
        .build()
}

/// Default on-disk size cap `spawn_http_cache_eviction` enforces — a
/// "don't let a long-running headless/desktop session grow this forever"
/// bound, not derived from any specific device-tier memory budget (this is
/// disk, not RAM).
pub const DEFAULT_HTTP_CACHE_LIMIT_BYTES: u64 = 250 * 1024 * 1024;

/// Evict the least-recently-written entries (both the `cacache` index entry
/// and its content blob, via `RemoveOpts::remove_fully(true)` — the same
/// `true` already passed to `CACacheManager::new` above, so this doesn't
/// leave orphaned content the manager itself wouldn't have left either)
/// until the cache's total tracked size is at or under `max_bytes`.
///
/// `cacache` has no automatic size-based eviction at all (see
/// `clear_http_cache`'s doc comment) — entries persist until manually
/// cleared. This is that missing policy, LRU-ish: `cacache`'s index only
/// tracks a write timestamp, not a last-read one, so "oldest write wins" is
/// the closest approximation available without adding a separate read-time
/// tracking layer of our own.
pub fn evict_http_cache_over(max_bytes: u64) -> std::io::Result<()> {
    evict_http_cache_dir_over(&http_cache_dir(), max_bytes)
}

fn evict_http_cache_dir_over(dir: &std::path::Path, max_bytes: u64) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    // `cacache`'s own per-entry `Metadata.size` is only populated when the
    // writer explicitly calls `WriteOpts::size()` — http-cache's
    // `CACacheManager::put` (the only real writer into this cache; see that
    // crate's `src/managers/cacache.rs`) writes via the plain
    // `cacache::write()` helper, which never sets it. Trusting
    // `meta.size` here would read back as 0 for every real entry this
    // cache ever holds, making every sweep a no-op. Reading each entry's
    // real content length via `read_sync` is the only reliable way to know
    // actual sizes for this cache's entries specifically.
    let mut entries: Vec<(String, u128, u64)> = cacache::list_sync(dir)
        .filter_map(|entry| entry.ok())
        .map(|meta| {
            let size = cacache::read_sync(dir, &meta.key).map(|data| data.len() as u64).unwrap_or(0);
            (meta.key, meta.time, size)
        })
        .collect();

    let total: u64 = entries.iter().map(|(_, _, size)| *size).sum();
    if total <= max_bytes {
        return Ok(());
    }

    entries.sort_by_key(|(_, time, _)| *time);

    let mut remaining = total;
    for (key, _, size) in entries {
        if remaining <= max_bytes {
            break;
        }
        if cacache::RemoveOpts::new().remove_fully(true).remove_sync(dir, &key).is_ok() {
            remaining = remaining.saturating_sub(size);
        }
    }
    Ok(())
}

/// Runs `evict_http_cache_over` on a plain OS thread rather than inline.
/// `cached_client` (above) is on `Navigator::new()`'s path, which in turn is
/// on `himalayas daemon`'s startup path (`src/daemon/mod.rs`) — the same
/// path this project measures and markets as ~30ms to ready (see README's
/// "The Proof"). A synchronous full-cache listing there would add real,
/// user-visible latency to that number on a long-lived cache. `std::thread`
/// (not `tokio::spawn`) specifically so this works identically from the
/// daemon (already inside a tokio runtime) and the native desktop shell
/// (winit event loop, no tokio runtime guaranteed to be running yet at this
/// point) without caring which one called it.
// Skipped entirely under `#[cfg(test)]`: `http_cache_dir()` already hands
// every test its own unique throwaway directory (see that function's doc
// comment for why), so a background sweep here would just be dead work
// racing the test process's own shutdown — not a correctness issue, just
// pointless.
#[cfg(not(test))]
fn spawn_http_cache_eviction() {
    std::thread::spawn(|| {
        let _ = evict_http_cache_over(DEFAULT_HTTP_CACHE_LIMIT_BYTES);
    });
}

#[cfg(test)]
fn spawn_http_cache_eviction() {}

/// Wipe this cache's entire on-disk store — no automatic size-bounded
/// eviction exists yet (`cacache`, the storage backend, doesn't evict on
/// its own; a real LRU/size-cap policy is a separate, real follow-up, not
/// built this pass), so this manual clear is the only lever a user has
/// today if the cache directory grows larger than they want. Exposed via
/// the desktop shell's settings panel ("Clear cache").
pub fn clear_http_cache() -> std::io::Result<()> {
    clear_http_cache_at(&http_cache_dir())
}

/// Split from `clear_http_cache` so tests can target a throwaway
/// directory instead of the real, shared, production cache path — the
/// first version of this test raced with `test_navigate_does_not_refetch_a_cacheable_url`
/// (`navigator.rs`) and the other test below, both of which read/write
/// the *same* real `http_cache_dir()` concurrently (Rust runs `#[test]`s
/// in parallel by default): `remove_dir_all` walking the tree while
/// another thread was actively creating new cache entries inside it via
/// `cacache` intermittently failed with `DirectoryNotEmpty` — a genuine
/// TOCTOU race from testing a destructive operation against shared global
/// state, not a bug in `clear_http_cache` itself. Same fix already
/// applied to session persistence (`load_session_state_from`/
/// `save_session_state_to` in `src/bin/desktop.rs`): split the path out
/// so tests never touch the real path at all.
fn clear_http_cache_at(dir: &std::path::Path) -> std::io::Result<()> {
    if dir.exists() { std::fs::remove_dir_all(dir) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_http_cache_removes_the_directory() {
        let dir = std::env::temp_dir().join(format!("himalayas-net-cache-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("marker"), b"x").unwrap();
        assert!(dir.exists());

        clear_http_cache_at(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn clear_http_cache_is_a_no_op_when_nothing_exists_yet() {
        let dir = std::env::temp_dir().join(format!("himalayas-net-cache-test-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(clear_http_cache_at(&dir).is_ok());
    }

    // `clear_http_cache()` itself (the real, path-hardcoded public
    // function real callers use) is intentionally *not* exercised against
    // the real `http_cache_dir()` here — that's exactly the shared,
    // concurrently-written path that caused the original race (see this
    // module's other doc comment). `clear_http_cache_at`'s two tests above
    // cover its actual logic; `clear_http_cache` is a one-line delegation
    // to it plus `http_cache_dir()`, not worth a second, race-prone test.

    // Each test below gets its own throwaway directory (never
    // `http_cache_dir()`) for the same reason `clear_http_cache_at`'s tests
    // do: destructive/mutating operations against a directory other tests
    // might concurrently touch is a real, previously-hit race, not
    // hypothetical caution.
    fn evict_test_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("himalayas-net-cache-evict-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn evict_http_cache_dir_over_is_a_no_op_when_the_directory_does_not_exist() {
        let dir = evict_test_dir("missing");
        assert!(evict_http_cache_dir_over(&dir, 100).is_ok());
    }

    #[test]
    fn evict_http_cache_dir_over_is_a_no_op_when_already_under_the_cap() {
        let dir = evict_test_dir("under-cap");
        cacache::write_sync(&dir, "a", vec![0u8; 10]).unwrap();

        evict_http_cache_dir_over(&dir, 1_000_000).unwrap();

        assert!(cacache::read_sync(&dir, "a").is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn evict_http_cache_dir_over_removes_the_oldest_entries_until_under_the_cap() {
        // Distinct content per key (not just distinct keys) — `cacache` is
        // content-addressed, so identical bytes under different keys share
        // one physical content blob, and removing one key's "fully" would
        // silently delete the content the other keys still point to.
        let dir = evict_test_dir("over-cap");
        cacache::write_sync(&dir, "a", vec![1u8; 100]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        cacache::write_sync(&dir, "b", vec![2u8; 100]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        cacache::write_sync(&dir, "c", vec![3u8; 100]).unwrap();

        // Total is 300 bytes; capping at 150 should evict the two oldest
        // ("a" then "b") and leave the newest ("c").
        evict_http_cache_dir_over(&dir, 150).unwrap();

        assert!(cacache::read_sync(&dir, "a").is_err());
        assert!(cacache::read_sync(&dir, "b").is_err());
        assert!(cacache::read_sync(&dir, "c").is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn evict_http_cache_dir_over_removes_both_the_index_entry_and_its_content() {
        let dir = evict_test_dir("remove-fully");
        cacache::write_sync(&dir, "a", vec![0u8; 100]).unwrap();
        let sri = cacache::metadata_sync(&dir, "a").unwrap().unwrap().integrity;

        evict_http_cache_dir_over(&dir, 0).unwrap();

        assert!(cacache::metadata_sync(&dir, "a").unwrap().is_none());
        assert!(cacache::read_hash_sync(&dir, &sri).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
