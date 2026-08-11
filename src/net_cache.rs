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
}
