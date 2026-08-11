# Native Rendering Engine — Findings & Follow-Up Plan

**Status**: Phases 1–3 complete and integrated into the repo. `himalayas-desktop` (behind the `js_engine` feature) is a real, working native window with a styled tab strip/address bar, link navigation, keyboard input and form submission inside loaded pages, and a v0 JS/DOM binding surface (`getElementById`, `textContent`, `style`, `classList`, `addEventListener`) — all verified live, including an end-to-end real-world flow on wikipedia.org (type a search term, press Enter, get results).
**Goal this serves**: render real web pages without embedding Chromium/CEF — the thing that would quietly reintroduce the exact binary-size and memory overhead Himalayas is built to avoid.

This document started as a spike investigation into [Blitz](https://github.com/DioxusLabs/blitz) (DioxusLabs' pure-Rust HTML/CSS rendering engine) as Himalayas' native page-rendering engine, and a follow-on investigation into whether JavaScript execution could be added on top of it. Both produced working proofs, which have since been built out into a real integration:

- `vendor/blitz/` — a vendored, trimmed, patched copy of Blitz (commit `990a90bfa1f8dc7034a601922339b027142a3bdc`), not a git fork, so no external push was required.
- `src/bin/desktop.rs` — the `himalayas-desktop` binary: RSX-authored tab strip/address bar shell with a JS-scripted content pane, unifying what were originally two separate, mutually-exclusive spikes.
- Cargo feature `js_engine` (off by default) gates all of it out of the default build; `cargo test`/`cargo build` for the existing `himalayas` binary are unaffected (375 tests still pass).

Run it: `cargo run --bin himalayas-desktop --features js_engine`. Try `himalayas://test` in the address bar for a built-in page that exercises the JS pipeline without depending on any real site's DOM API surface.

## Relationship to existing work

- [UI_UX_VISION.md](./UI_UX_VISION.md) describes the browser shell (tabs, address bar, sidebar) as a **web frontend served by the daemon** (`src/ui/web/`, mounted at `/app`). That shell's content pane is currently a placeholder — "Page rendering isn't wired up yet." This document is about what fills that placeholder.
- `src/browser/mod.rs` and `src/browser/tabs.rs` already implement real, tested tab/session management (`Browser::open_tab`, `IsolationMode::{Isolated, Shared}`) — device-tier-aware, matching the pattern this plan reuses for JS-engine gating. That backend is independent of the rendering-engine question and doesn't need to change for any of the below.
- The `/app` web shell and a native Blitz-based shell are **not the same thing** and can't share UI code — see "The shell question" below.

---

## Executive summary

1. **Blitz genuinely works** as a non-Chromium renderer: real HTTP fetch, real HTML/CSS parsing (via Stylo, Firefox's CSS engine), real GPU paint (via Vello), in a real native window, verified against live sites (youtube.com, google.com, wikipedia.org, fluke.com) and with working multi-tab session handling.
2. **Blitz has no JavaScript engine**, and none is on its near-term roadmap. Confirmed against real sites, not just docs: MSN.com's server-sent HTML has 3 characters of visible text (everything else is JS-injected); buttons on otherwise-working pages don't respond to clicks. This is the real boundary — static/reader content works, the interactive web doesn't, out of the box.
3. **JS execution is addable and is now integrated**, not just spiked: a custom `Document` implementation + [Boa](https://github.com/boa-dev/boa) (pure-Rust JS engine) + a hand-wired native function let a real click run real JS that mutated the DOM and repainted on screen — inside the actual RSX-shell browser window, not just a bare standalone test window.
4. **A second, unplanned gap had to be closed to get there**: Blitz's `<web-view>` sub-document embedding is display-only upstream — it paints embedded pages but never forwards any pointer/click event into them (not even plain link clicks). This wasn't in the original plan; see "Patch 2" below. Fixed with a small, targeted patch to `blitz-dom`'s event driver.
5. **A real v0 DOM binding surface is built and live**, not just the pipe: `document.getElementById`, `textContent`, `style.setProperty`, `classList.{add,remove,toggle,contains}`, and `addEventListener` all work, verified against a real page exercising all of them together. It has real, stated limitations (mirror-based reads, not live document reads — see Phase 3 in the phased plan below), and it's still a small slice of the full DOM/HTML surface real sites use (no `querySelector`, no DOM construction, no `fetch`) — comparable in eventual scope to Servo's `script` crate, historically one of the largest subsystems in any browser engine — but the shape is proven and the remaining work is now "add more bindings," not "figure out if this is possible."
6. **Do not build a custom JS engine.** Boa and QuickJS already exist and are the right build-vs-buy call; the "different devices need different capability" requirement is solved by gating an *existing* engine's inclusion/limits per device tier (the same pattern already used for `desktop_ui` and `IsolationMode` in this repo), not by authoring a new parser/VM/GC.

---

## Architecture: the Blitz stack

```
blitz-html     → parses HTML/XHTML into blitz-dom's tree
blitz-dom      → DOM + style resolution (Stylo) + layout (Taffy) + text (Parley) + event dispatch
blitz-net      → HTTP fetch (reqwest-based)
blitz-paint    → DOM tree → anyrender draw commands
blitz-shell    → window + winit event loop + accessibility + native menus
dioxus-native  → wraps the above so a window's UI can be authored as Dioxus RSX
```

`blitz` (no suffix) is a thin convenience crate re-exporting all of the above with two entry points: `launch_static_html(html)` and `launch_url(url)`.

None of this depends on Chromium, CEF, or any other browser engine. The binary and runtime cost is Rust crates only (Stylo, Taffy, Parley, Vello, wgpu).

---

## Spike 1: native shell with tabs (proven working)

A window with a tab strip, address bar, and a content pane that fetches and renders arbitrary URLs, using `dioxus-native` for the shell and its `web-view` element to embed fetched pages.

**Verified live**: opened youtube.com and google.com; opened two simultaneous tabs (msn.com + fluke.com) and confirmed each kept its own independently-loaded document, switching correctly between them.

### Key gotchas (would cost real time to rediscover)

- **`[patch.crates-io]` is mandatory**, not optional. Blitz's own `Cargo.toml` patches `usvg`, `anyrender`, and `anyrender_svg` to unreleased forked branches (`DioxusLabs/resvg` and `DioxusLabs/anyrender`, both on branches named `devin/...`). Cargo patches don't propagate through a git dependency — any consumer crate must declare the same `[patch.crates-io]` itself, or it fails to compile against upstream `usvg` (missing `intrinsic_dimensions()`, missing `usvg::svgtypes`).
- **`dioxus-native` needs explicit features**: `features = ["prelude", "net", "system-fonts"]`. Without `"prelude"`, `dioxus_native::prelude::*` and the `rsx!` macro don't resolve.
- **The `web-view` element needs explicit sizing.** `web-view { "__webview_document": doc }` compiles and the document loads successfully (confirmed via logs) but paints nothing without `style: "display:block;width:100%;height:100%;"` — it behaves like an `<iframe>` with no intrinsic size.
- **`dioxus-native`'s crates.io version (`0.8.0-alpha.1`) lags the git `main` branch.** Building against a specific commit's API (as documented here) requires a `git` dependency pinned to a `rev`, not the published crate.

### Reference: `Cargo.toml`

```toml
[package]
name = "himalayas-native-shell-spike"
version = "0.1.0"
edition = "2021"

[dependencies]
dioxus-native = { git = "https://github.com/DioxusLabs/blitz", rev = "990a90bfa1f8dc7034a601922339b027142a3bdc", features = ["prelude", "net", "system-fonts"] }
blitz-dom = { git = "https://github.com/DioxusLabs/blitz", rev = "990a90bfa1f8dc7034a601922339b027142a3bdc" }
blitz-html = { git = "https://github.com/DioxusLabs/blitz", rev = "990a90bfa1f8dc7034a601922339b027142a3bdc" }
blitz-net = { git = "https://github.com/DioxusLabs/blitz", rev = "990a90bfa1f8dc7034a601922339b027142a3bdc" }
blitz-traits = { git = "https://github.com/DioxusLabs/blitz", rev = "990a90bfa1f8dc7034a601922339b027142a3bdc" }
linebender_resource_handle = "0.1"
tracing = "0.1"
tracing-subscriber = "0.3"

[patch.crates-io]
usvg = { git = "https://github.com/DioxusLabs/resvg", branch = "devin/1785858271-intrinsic-dimensions" }
anyrender = { git = "https://github.com/DioxusLabs/anyrender", branch = "devin/1785858394-usvg-048" }
anyrender_svg = { git = "https://github.com/DioxusLabs/anyrender", branch = "devin/1785858394-usvg-048" }
```

### Reference: `src/main.rs`

```rust
use blitz_dom::{DocumentConfig, FontContext};
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_net::Provider as NetProvider;
use blitz_traits::net::Request;
use blitz_traits::shell::ShellProvider;
use dioxus_native::SubDocumentAttr;
use dioxus_native::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
struct Tab {
    id: u32,
    url: String,
    title: String,
    document: Option<SubDocumentAttr>,
    status: String,
}

impl Tab {
    fn new(id: u32, url: &str) -> Self {
        Self {
            id,
            url: url.to_string(),
            title: "New Tab".to_string(),
            document: None,
            status: "Enter a URL and press Enter".to_string(),
        }
    }
}

fn app() -> Element {
    let mut tabs = use_signal(|| vec![Tab::new(0, "https://example.com")]);
    let mut active_id = use_signal(|| 0u32);
    let mut next_id = use_signal(|| 1u32);
    let mut address_input = use_signal(|| "https://example.com".to_string());

    let mut navigate = move || {
        let id = active_id();
        let url = address_input.read().clone();
        {
            let mut t = tabs.write();
            if let Some(tab) = t.iter_mut().find(|t| t.id == id) {
                tab.status = format!("Loading {url}...");
                tab.url = url.clone();
            }
        }
        spawn(async move {
            match load_page(&url).await {
                Ok(doc) => {
                    let mut t = tabs.write();
                    if let Some(tab) = t.iter_mut().find(|t| t.id == id) {
                        tab.title = url.clone();
                        tab.document = Some(doc);
                        tab.status = format!("Loaded {url}");
                    }
                }
                Err(e) => {
                    let mut t = tabs.write();
                    if let Some(tab) = t.iter_mut().find(|t| t.id == id) {
                        tab.status = format!("Failed to load {url}: {e}");
                    }
                }
            }
        });
    };

    let mut new_tab = move || {
        let id = next_id();
        next_id.set(id + 1);
        tabs.write().push(Tab::new(id, "https://example.com"));
        active_id.set(id);
        address_input.set("https://example.com".to_string());
    };

    rsx! {
        div {
            style: "display:flex;flex-direction:column;height:100vh;font-family:sans-serif;",

            // Tab strip
            div {
                style: "display:flex;gap:4px;padding:6px 6px 0;background:#e5e5e5;align-items:center;",
                for tab in tabs.read().iter().cloned() {
                    div {
                        key: "{tab.id}",
                        style: if tab.id == active_id() {
                            "padding:6px 10px;background:white;border-radius:6px 6px 0 0;cursor:pointer;font-size:12px;max-width:160px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;"
                        } else {
                            "padding:6px 10px;background:#d5d5d5;border-radius:6px 6px 0 0;cursor:pointer;font-size:12px;max-width:160px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;"
                        },
                        onclick: move |_| {
                            active_id.set(tab.id);
                            address_input.set(tab.url.clone());
                        },
                        "{tab.title} "
                        span {
                            onclick: move |evt| {
                                evt.stop_propagation();
                                tabs.write().retain(|t| t.id != tab.id);
                                if active_id() == tab.id {
                                    if let Some(first) = tabs.read().first() {
                                        active_id.set(first.id);
                                        address_input.set(first.url.clone());
                                    }
                                }
                            },
                            " ✕"
                        }
                    }
                }
                button { onclick: move |_| new_tab(), "+" }
            }

            // Address bar
            div {
                style: "display:flex;gap:8px;padding:8px;border-bottom:1px solid #ccc;background:#f5f5f5;",
                input {
                    style: "flex:1;padding:6px;",
                    value: "{address_input}",
                    oninput: move |evt| address_input.set(evt.value()),
                    onkeydown: move |evt| {
                        if evt.key() == Key::Enter {
                            navigate();
                        }
                    }
                }
                button { onclick: move |_| navigate(), "Go" }
            }

            // Status bar for the active tab
            div {
                style: "padding:4px 8px;font-size:12px;color:#666;",
                {tabs.read().iter().find(|t| t.id == active_id()).map(|t| t.status.clone()).unwrap_or_default()}
            }

            // Content pane: the active tab's document
            div {
                style: "flex:1;overflow:auto;",
                if let Some(doc) = tabs.read().iter().find(|t| t.id == active_id()).and_then(|t| t.document.clone()) {
                    web-view {
                        key: "{active_id()}",
                        style: "display:block;width:100%;height:100%;",
                        "__webview_document": doc,
                    }
                } else {
                    div { style: "padding:20px;color:#888;", "No page loaded in this tab yet" }
                }
            }
        }
    }
}

async fn load_page(raw_url: &str) -> Result<SubDocumentAttr, String> {
    let url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        raw_url.to_string()
    } else {
        format!("https://{raw_url}")
    };

    let net_provider = Arc::new(NetProvider::new(None));
    let request = Request::get(url.parse().map_err(|e| format!("bad url: {e}"))?);
    let (resolved_url, bytes) = net_provider
        .fetch_async(request)
        .await
        .map_err(|e| format!("fetch failed: {e:?}"))?;

    let html = String::from_utf8_lossy(&bytes).to_string();

    let mut font_ctx = FontContext::default();
    font_ctx.collection.register_fonts(
        linebender_resource_handle::Blob::new(Arc::new(blitz_dom::BULLET_FONT) as _),
        None,
    );

    let shell_provider = consume_context::<Arc<dyn ShellProvider>>();

    let config = DocumentConfig {
        base_url: Some(resolved_url),
        net_provider: Some(net_provider as _),
        shell_provider: Some(shell_provider),
        html_parser_provider: Some(Arc::new(HtmlProvider)),
        font_ctx: Some(font_ctx),
        ..Default::default()
    };

    let document = HtmlDocument::from_html(&html, config).into_inner();
    Ok(SubDocumentAttr::new(document))
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    dioxus_native::launch(app);
}
```

---

## Spike 2: JavaScript execution (proven working)

Proves click → JS execution → DOM mutation → repaint, on a real native window, using a hand-built `Document` implementation (not `dioxus-native`/`web-view` — see "The shell question" below for why).

**Verified live**: a rendered button's `onclick="handleClick()"` ran through an embedded Boa context, which called a native Rust function, which queued and applied a DOM text mutation. The button's label visibly changed on screen. Trace confirmed in logs:

```
click on node NodeId(6v1), running JS: handleClick()
JS called set_text("out", "Clicked via JS!")
applied DOM mutation: #out text -> "Clicked via JS!"
```

### The extension point

`blitz_dom::Document` is a trait with an overridable `handle_ui_event`. The default implementation hardcodes `NoopEventHandler` — no scripting. But this is genuinely extensible: `dioxus-native-dom`'s own `DioxusDocument` overrides it to run Dioxus's event system:

```rust
// packages/dioxus-native-dom/src/dioxus_document.rs (Blitz's own source)
fn handle_ui_event(&mut self, event: UiEvent) {
    let handler = DioxusEventHandler { vdom: &mut self.vdom, vdom_state: &mut self.vdom_state };
    let mut driver = EventDriver::new(&mut self.inner, handler);
    driver.handle_ui_event(event);
}
```

This is a real, production-used hook, not a workaround. A custom `Document` + custom `EventHandler` following the same shape is how the spike wires in JS.

### Why not `web-view`

`dioxus-native-dom`'s `SubDocumentAttr::new(doc: BaseDocument)` always wraps its argument in `PlainDocument`, whose `Document` impl never overrides `handle_ui_event` — it's permanently `NoopEventHandler`. There's no public constructor that accepts a custom `Document`. This spike instead bypasses `dioxus-native` for the content pane entirely and builds directly on `blitz-shell`'s lower-level API (`WindowConfig::new(Box::new(doc) as _, renderer)`, the same primitive the `blitz` crate's own `launch_static_html` uses), which accepts any `Document` impl.

### Reference: `Cargo.toml`

```toml
[package]
name = "himalayas-js-spike"
version = "0.1.0"
edition = "2021"

[dependencies]
blitz-dom = { git = "https://github.com/DioxusLabs/blitz", rev = "990a90bfa1f8dc7034a601922339b027142a3bdc" }
blitz-html = { git = "https://github.com/DioxusLabs/blitz", rev = "990a90bfa1f8dc7034a601922339b027142a3bdc" }
blitz-net = { git = "https://github.com/DioxusLabs/blitz", rev = "990a90bfa1f8dc7034a601922339b027142a3bdc" }
blitz-traits = { git = "https://github.com/DioxusLabs/blitz", rev = "990a90bfa1f8dc7034a601922339b027142a3bdc" }
blitz-shell = { git = "https://github.com/DioxusLabs/blitz", rev = "990a90bfa1f8dc7034a601922339b027142a3bdc" }
anyrender_vello = { git = "https://github.com/DioxusLabs/anyrender", branch = "devin/1785858394-usvg-048" }
linebender_resource_handle = "0.1"
boa_engine = "0.20"
tokio = { version = "1", features = ["rt-multi-thread"] }
tracing = "0.1"
tracing-subscriber = "0.3"

[patch.crates-io]
usvg = { git = "https://github.com/DioxusLabs/resvg", branch = "devin/1785858271-intrinsic-dimensions" }
anyrender = { git = "https://github.com/DioxusLabs/anyrender", branch = "devin/1785858394-usvg-048" }
anyrender_svg = { git = "https://github.com/DioxusLabs/anyrender", branch = "devin/1785858394-usvg-048" }
```

Note `anyrender_vello` must come from the same forked repo/branch as the patch, not crates.io — the published `anyrender_vello` (crates.io) depends on unpatched `anyrender`, causing a duplicate-trait-implementation error (`the trait anyrender::WindowRenderer is not implemented` with two versions of `anyrender` visible in the error).

### Reference: `src/main.rs`

```rust
use anyrender_vello::VelloWindowRenderer;
use blitz_dom::{
    BaseDocument, DocGuard, DocGuardMut, Document, DocumentConfig, EventDriver, EventHandler,
    FontContext,
};
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_net::Provider as NetProvider;
use blitz_shell::{BlitzApplication, BlitzShellProxy, WindowConfig, create_default_event_loop};
use blitz_traits::events::{DomEvent, DomEventData, EventState, UiEvent};
use blitz_traits::node_id::NodeId;
use boa_engine::{Context as JsContext, JsValue, NativeFunction, Source, js_string};
use std::cell::RefCell;
use std::sync::Arc;

thread_local! {
    static JS: RefCell<JsContext> = RefCell::new(JsContext::default());
    static MUTATIONS: RefCell<Vec<(String, String)>> = RefCell::new(Vec::new());
}

fn init_js() {
    JS.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        let set_text = NativeFunction::from_copy_closure(|_this, args, _ctx| {
            let id = args
                .first()
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let text = args
                .get(1)
                .and_then(JsValue::as_string)
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            MUTATIONS.with(|m| m.borrow_mut().push((id, text)));
            Ok(JsValue::undefined())
        });
        ctx.register_global_callable(js_string!("set_text"), 2, set_text)
            .unwrap();
    });
}

/// A Document that runs page-authored JS on click, in place of dioxus-native's
/// hardcoded-Noop web-view embedding.
struct ScriptedDocument {
    base: BaseDocument,
}

impl Document for ScriptedDocument {
    fn inner(&self) -> DocGuard<'_> {
        DocGuard::Ref(&self.base)
    }
    fn inner_mut(&mut self) -> DocGuardMut<'_> {
        DocGuardMut::Ref(&mut self.base)
    }

    fn handle_ui_event(&mut self, event: UiEvent) {
        let handler = ScriptEventHandler;
        let mut driver = EventDriver::new(&mut self.base, handler);
        driver.handle_ui_event(event);

        // Apply any DOM mutations the JS queued during event handling.
        let pending: Vec<(String, String)> = MUTATIONS.with(|m| m.borrow_mut().drain(..).collect());
        for (id, text) in pending {
            if let Some(el_id) = self.base.get_element_by_id(&id) {
                if let Some(node) = self.base.get_node(el_id) {
                    if let Some(text_node_id) = node.children.first().copied() {
                        self.base.mutate().set_node_text(text_node_id, &text);
                    }
                }
            }
        }
    }
}

struct ScriptEventHandler;
impl EventHandler for ScriptEventHandler {
    fn handle_event(
        &mut self,
        _chain: &[NodeId],
        event: &mut DomEvent,
        doc: &mut dyn Document,
        _event_state: &mut EventState,
    ) {
        if let DomEventData::Click(_) = &event.data {
            let target = event.target;
            let onclick = doc
                .inner()
                .get_node(target)
                .and_then(|n| n.attr("onclick".into()))
                .map(|s| s.to_string());
            if let Some(code) = onclick {
                JS.with(|ctx| {
                    let _ = ctx.borrow_mut().eval(Source::from_bytes(code.as_bytes()));
                });
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::WARN).init();
    init_js();

    let html = r#"<html><body style="font-family: sans-serif; padding: 40px;">
        <button onclick="handleClick()" style="padding: 10px 20px; font-size: 16px;">Click me</button>
        <p id="out">Not clicked yet</p>
        <script>
            function handleClick() {
                set_text('out', 'Clicked via JS!');
            }
        </script>
    </body></html>"#;

    // Register the page's <script> functions before the window opens.
    // A real implementation must discover <script> tags from the parsed DOM
    // (respecting parse-blocking/defer/async order per the HTML spec) rather
    // than string-searching the source, and must re-run this on navigation.
    if let (Some(start), Some(end)) = (html.find("<script>"), html.find("</script>")) {
        let script_src = &html[start + "<script>".len()..end];
        JS.with(|ctx| {
            let _ = ctx.borrow_mut().eval(Source::from_bytes(script_src.as_bytes()));
        });
    }

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let _guard = rt.enter();

    let event_loop = create_default_event_loop();
    let (proxy, receiver) = BlitzShellProxy::new(event_loop.create_proxy());
    let net_provider = Arc::new(NetProvider::new(Some(Arc::new(proxy.clone()))));

    let mut font_ctx = FontContext::default();
    font_ctx.collection.register_fonts(
        linebender_resource_handle::Blob::new(Arc::new(blitz_dom::BULLET_FONT) as _),
        None,
    );

    let config = DocumentConfig {
        base_url: None,
        net_provider: Some(net_provider),
        html_parser_provider: Some(Arc::new(HtmlProvider)),
        font_ctx: Some(font_ctx),
        ..Default::default()
    };

    let base = HtmlDocument::from_html(html, config).into_inner();
    let doc = ScriptedDocument { base };

    let renderer = VelloWindowRenderer::new();
    let window = WindowConfig::new(Box::new(doc) as _, renderer);

    let mut application = BlitzApplication::new(proxy, receiver);
    application.add_window(window);

    event_loop.run_app(application).unwrap();
}
```

---

## The shell question: resolved (Option A), plus a second patch it exposed

Spike 1 (tabs, RSX shell) and Spike 2 (scripting) were originally mutually exclusive, because `web-view` couldn't carry a custom `Document`. This is now built and working: `vendor/blitz/` carries two patches, and `src/bin/desktop.rs` uses both together.

### Patch 1: `SubDocumentAttr` accepts a custom `Document`

`vendor/blitz/packages/dioxus-native-dom/src/write_once_attr.rs` — `SubDocumentAttr::new(doc: BaseDocument)` still exists unchanged; a new `SubDocumentAttr::from_document<D: Document + 'static>(doc: D)` sits alongside it and skips the hardcoded `PlainDocument` wrap that made scripting unreachable. `set_sub_document` in `blitz-dom` already accepted `Box<dyn Document>` — the concrete-type lock-in was entirely in this one wrapper — so the patch is genuinely small (~15 lines added, one line changed in `mutation_writer.rs`'s downcast target).

### Patch 2 (found during integration, not in the original plan): sub-documents never received events at all

Wiring `ScriptedDocument` through `SubDocumentAttr::from_document` and clicking the test button did nothing — not a scripting bug, but a rendering-vs-interaction gap. Searching the entire vendored codebase for `sub_document_nodes` (the mechanism tracking which nodes host an embedded sub-document) turned up exactly two call sites: layout resolution (`resolve.rs`) and painting (`blitz-paint`). Zero references anywhere in event dispatch — `blitz-dom/src/events/driver.rs`, all of `blitz-shell`, `dioxus-native-dom`'s `DioxusDocument`. Confirmed live: plain `<a href>` link clicks inside a `web-view`-embedded page didn't work either, patched or not.

**In this Blitz commit, `<web-view>` is display-only.** It paints and lays out embedded content correctly but never forwards a pointer event into it, for any purpose.

Fixed with a small patch in `vendor/blitz/packages/blitz-dom/src/events/driver.rs`, inside `EventDriver::handle_ui_event`, right after the outer document resolves its hit-tested/focused `target` node. **Updated since first written** — the initial version only forwarded pointer events (down/up/move/cancel); typing into a form field inside a sub-document silently did nothing, because keyboard events target `focussed_node_id` (not a screen position), and nothing ever told the *outer* document that focus had moved into the sub-document when it was clicked. Current version:

```rust
let has_subdoc = self.doc.inner().get_node(target).is_some_and(|n| n.subdoc().is_some());
if has_subdoc {
    if matches!(event, UiEvent::PointerDown(_) | UiEvent::PointerUp(_) | UiEvent::PointerMove(_) | UiEvent::PointerCancel(_)) {
        let rect = self.doc.inner().get_client_bounding_rect(target);
        if let Some(rect) = rect {
            let forwarded = shift_pointer_event(&event, rect.x as f32, rect.y as f32);
            let mut doc = self.doc.inner_mut();
            // A pointer-down on the sub-document's mount point becomes the
            // *outer* document's notion of "focus" too, so a keyboard event
            // arriving afterwards still resolves to this sub-document.
            if matches!(event, UiEvent::PointerDown(_)) {
                doc.set_focus_to(target);
            }
            if let Some(sub_doc) = doc.get_node_mut(target).and_then(|n| n.subdoc_mut()) {
                sub_doc.handle_ui_event(forwarded);
            }
        }
        return;
    }
    if matches!(event, UiEvent::KeyDown(_) | UiEvent::KeyUp(_) | UiEvent::Ime(_)) {
        // No coordinate translation needed — targeted by focus, not position.
        let mut doc = self.doc.inner_mut();
        if let Some(sub_doc) = doc.get_node_mut(target).and_then(|n| n.subdoc_mut()) {
            sub_doc.handle_ui_event(event);
        }
        return;
    }
}
```

`shift_pointer_event` is a small free function alongside it that clones a `UiEvent`'s pointer variant and shifts `coords.page_x/page_y/client_x/client_y` by the embed's offset. Verified live: mouse hover, click, *and* typing into wikipedia.org's search box all correctly reach the sub-document now.

**Known limitation, intentionally deferred**: hover-chain bookkeeping (`pointerenter`/`pointerleave` on the *outer* document's own ancestor chain) isn't forwarding-aware — cosmetic, doesn't block interaction. `AppleStandardKeybinding` isn't forwarded either.

**Consider upstreaming this patch** — Blitz's maintainers likely want `<web-view>` to support interaction regardless of what Himalayas does with it; this isn't a Himalayas-specific hack.

### Patch 3: form submission needed a `NavigationProvider`, not another event-forwarding fix

Typing into wikipedia.org's search box worked once keyboard forwarding landed, but pressing Enter to submit did nothing. Not a forwarding gap this time — `blitz-dom`'s own default action for *both* link clicks and form submission calls `NavigationProvider::navigate_to(options)` (`blitz_traits::navigation`), and `DocumentConfig.navigation_provider` was never set, so it fell back to `DummyNavigationProvider` — a documented no-op. (Link clicks worked anyway because `find_href`/`PENDING_NAVIGATION` in `ScriptEventHandler` is a separate, earlier-in-the-pipeline mechanism that doesn't touch this at all; forms have no equivalent hand-rolled path.)

Fixed by implementing a one-line `NavigationProvider` in `src/bin/desktop.rs` (`SubDocNavigationProvider`) that pushes `options.url` onto the same `PENDING_NAVIGATION` queue link clicks use, and wiring it into `DocumentConfig.navigation_provider`. GET forms work correctly (the query string is already in `options.url`). **POST forms are not handled** — `navigate_tab`/`load_page` only ever does a GET fetch; extending them to carry method + body is a smaller, separate follow-up whenever a real POST form is hit.

Verified live: typing a search term into wikipedia.org and pressing Enter now navigates to real search results.

### Patch 4: wheel events were never forwarded at all — scroll was hitting the wrong document

Same shape of bug as Patch 2, found later via a live user report ("scroll fluidity is not there"): the Patch 2 forwarding `matches!` block only covered `PointerDown`/`PointerUp`/`PointerMove`/`PointerCancel` and, separately, `KeyDown`/`KeyUp`/`Ime` — `UiEvent::Wheel` matched neither, so it fell through to the *outer* document's own default action, scrolling the shell's content container instead of the loaded page's own scroll offset. The page itself never received a scroll event at all; "not fluid" was an accurate way to describe never actually scrolling the thing you're looking at.

Fixed with a third forwarding branch in `EventDriver::handle_ui_event` (same rect-lookup + coordinate-shift shape as pointer forwarding, no `set_focus_to` since wheel doesn't move focus) and a `shift_wheel` counterpart added to `shift_pointer_event`. `BlitzWheelEvent` has the same `coords: PointerCoords` shape as pointer events, so the shift logic is identical.

### Patch 5: `requestAnimationFrame` — sub-documents had no per-frame hook at all

CSS `@keyframes`/`transition` animations turned out to already be fully wired end-to-end (Stylo's `DocumentAnimationSet` → `BaseDocument::is_animating()` → `subdoc_is_animating` propagation in `resolve.rs` → `blitz-shell`'s window loop already checks `is_animating()` and keeps redrawing) — nothing to fix there. JS-driven animation (`requestAnimationFrame`) was a different story: completely absent, and building it exposed a real gap in the sub-document integration, not just a missing JS binding.

`Document::poll(&mut self, task_context: Option<Context>) -> bool` already exists upstream for exactly this ("poll any pending async operations... flush changes"), but it was only ever called on the *top-level* document (`WindowState::poll` in `blitz-shell/src/window.rs`) — `resolve.rs`'s sub-document loop called `resolve()` and `is_animating()` on each sub-document but never `poll()`, so a `ScriptedDocument` had no per-frame tick at all.

Fixed with a small patch to `resolve.rs`'s sub-document loop: call `sub_doc.poll(None)` on the `&mut dyn Document` trait object *before* converting it to a `DocGuardMut` (the conversion is what the existing code did immediately, losing the trait object), and OR the result into `subdoc_is_animating` alongside the existing `sub_doc.is_animating()` check — same redraw-keeps-going mechanism CSS animations already used, just with a second source feeding it.

On the Himalayas side (`src/bin/desktop.rs`): `requestAnimationFrame`/`cancelAnimationFrame` globals queue/remove `(handle, JsFunction)` pairs in a `RAF_CALLBACKS` thread_local; `ScriptedDocument::poll` drains whatever's queued *at that moment* (not callbacks a callback itself queues while running — matches real "next frame" semantics, so a self-requeuing `function tick() { ...; requestAnimationFrame(tick); }` loop still works, just one frame later each time) and runs them via the same `run_js_with_live_doc` live-document-publishing pattern event handlers already use, returning `true` iff anything ran. Verified with three unit tests (`request_animation_frame_runs_on_poll_and_stops_when_queue_empties`, `cancel_animation_frame_prevents_callback_from_running`, `self_requeuing_animation_frame_keeps_running_across_polls` — the last one specifically exercises the self-requeue pattern real sites use) and a new section on `himalayas://test` with a start/stop-able sliding box.

`setTimeout` was deliberately left as its existing immediate-execution shim rather than rebuilt on this new hook — "run once after a delay" and "run every frame until cancelled" are different enough contracts that conflating them wasn't worth the risk in the same pass. Real delayed timers remain a separate follow-up.

### Patch 6 (not actually a patch): right-click menu — the plumbing already existed, upstream just left it a `// TODO`

A real user report ("right click on the website does not work") turned out to need no new vendor patch at all. `blitz-dom`'s `pointer.rs` already synthesizes `DomEventData::ContextMenu` from a secondary-button pointer event, and — since `PointerDown`/`PointerUp` are already forwarded into sub-documents (Patch 2) — that synthesis already happens correctly inside a loaded page's own event processing, reaching `ScriptEventHandler::handle_event` like any other DOM event. The only reason nothing happened on right-click was that (a) `ScriptEventHandler` only ever checked for `DomEventData::Click`, and (b) blitz-dom's own default action for `ContextMenu` is a literal `// TODO: Open context menu` with an empty body — there's no menu implementation anywhere upstream to fall back to.

Fixed entirely in `src/bin/desktop.rs`: `ScriptEventHandler` now also matches `DomEventData::ContextMenu`, capturing the click's local coordinates and (via the existing `find_href`) the nearest link's resolved `href`, queued through the same `PENDING_*`/polling pattern `PENDING_NAVIGATION` already established. `app()` renders a small floating menu (Back / Reload / Copy link address when the click landed on a link / Close) positioned at the click's coordinates plus a fixed `CONTENT_AREA_TOP` offset approximating the shell's height above the content area — not pixel-exact (would need threading the `<web-view>`'s real bounding rect all the way out), but close enough to land near the cursor. "Copy link address" uses `ShellProvider::set_clipboard_text`, which already existed on the trait and required no new dependency.

### Patch 7: a real crash, not just a blank render — thehindu.com panicked the whole renderer

A live report ("thehindu.com — nothing is showing on the page rendering") turned out to be worse than the apple.com/SVG rendering gaps already logged here: the app wasn't rendering blank, it was **crashing outright** (`thread 'main' panicked... 'style' is not available on this node kind`), taking the whole window down. Reproduced without needing the GUI at all — a `#[tokio::test]` (`repro_thehindu_crash` in `src/bin/desktop.rs`, `#[ignore]`d as network-dependent) fetches the real page and drives it through the same fetch → parse → `resolve()` (layout/style) pipeline the app uses, hitting the identical panic outside any window/Dioxus context.

Root cause, traced via `RUST_BACKTRACE=1`: `blitz-dom`'s `Node` has a family of accessors (`stylo_element_data`, `style`, `cache`, `unrounded_layout`, `final_layout`, `scroll_offset`, `scrollable_overflow`, `transform`, `display_constructed_as`, ...) generated by a `universal_accessors!` macro that **panics** if the node isn't an Element/AnonymousBlock/Document — a "should never happen" assertion. It does happen: taffy's block-layout algorithm (`LayoutBlockContainer::get_block_child_style`, `CacheTree::cache_get`, etc., all in `layout/mod.rs`) calls these on every node in a container's `layout_children`, and on thehindu.com's real markup, something (almost certainly a `Text` node — `layout/construct.rs`'s `block_item_needs_wrap` correctly identifies `NodeKind::Text` as needing anonymous-block wrapping for mixed inline/block containers, but the actual wrapping in `collect_complex_layout_children` has some edge case that doesn't catch this specific structure) ends up in that list without being wrapped first.

**The anonymous-box-generation root cause was not chased further** — `layout/construct.rs`'s inline/block classification and wrapping logic is genuinely intricate CSS box-generation code (the kind of thing that takes mature engines years of accumulated edge-case fixes to get exhaustively right), and precisely isolating the one real-world markup pattern that trips it, in an alpha-stage engine, is a different order of effort than everything else fixed this session. Instead, fixed at the layer that actually matters for the user-facing symptom: **a layout-tree inconsistency should degrade, never crash the whole renderer** — the same principle already applied to apple.com and the SVG NaN-path bug, just enforced systemically here instead of case-by-case.

Concretely: `Node::style()`/`style_mut()` were pulled out of the `universal_accessors!` macro (read path fixed to fall back gracefully; write path left panicking, since it's only ever reached during style resolution over nodes already confirmed to be elements — not from taffy's node-kind-agnostic layout traversal, which is what made the read path unsafe to leave panicking). The other layout-reachable fields (`cache`, `unrounded_layout`, `final_layout`, `scroll_offset`, `scrollable_overflow`, `transform`, `display_constructed_as`) got the same treatment via a new `graceful_layout_accessors!` macro, found iteratively — fix the one currently panicking, rerun `repro_thehindu_crash`, repeat, until the whole `resolve()` pass completes clean. Each falls back to a shared **per-thread** scratch default (`Style::DEFAULT`, `Cache::default()`, `Layout::default()`, etc., leaked once via `thread_local!` rather than a plain `static`, since e.g. `Style<Atom>`'s compact internal representation isn't `Sync`) rather than a plain `static` — sound because a phantom node's scratch data is inherently throwaway (nothing ever depends on it meaning anything), and this pipeline never runs the same document's resolve pass concurrently on two threads at once.

Verified: `repro_thehindu_crash` now passes (full fetch → parse → layout completes without panicking); full Himalayas DOM suite (56 tests, run via `cargo test -p blitz-dom` from `vendor/blitz` — that crate's own tests aren't part of the outer `cargo test` run, being a separate Cargo workspace) and the outer suite (411 tests) both pass; live launch survives. **Not independently confirmed that the page now renders *correctly*** (pixel-level correctness for whatever node got the scratch-default treatment is a separate question from "doesn't crash") — worth a live check.

### Patch 8: lazy-loaded images — real content stuck on placeholder pixels

Follow-up live report after Patch 7 unblocked thehindu.com from crashing: "images... still not rendering." Root cause had nothing to do with the crash fix — it's a real, extremely common web convention our image loading never accounted for. Checked thehindu.com's actual markup directly (`curl` + a quick script counting `<img>` attributes): zero uses of `srcset` on this page (Patch on that front is simply inert here, not broken), but **97 of ~124 `<img>` tags** use the lazy-load pattern popularized by libraries like lazysizes: `src="/theme/images/th-online/1x1_spacer.png"` (a deliberate placeholder) with the real URL sitting in `data-original` (and this site's own `data-src-template` variant) — meant to be swapped in by JS on scroll/visibility via `IntersectionObserver`, which we don't run.

Fixed in `Node::load_image` (`mutator.rs`): checks `data-src`/`data-original` — the two attribute names the most widely-deployed lazy-load libraries actually use — and prefers either over a present-but-placeholder `src`. Also handles the lazy+responsive combo (`data-srcset`, same sizes-aware selection as plain `srcset`, given priority over it since a lazy responsive image is still fundamentally lazy). Deliberately did **not** chase thehindu.com's own site-specific `data-src-template` attribute — recognizing two near-universal de facto conventions is a reasonable, generalizable bet; chasing every site's bespoke variant is not.

One mechanical snag: `local_name!("data-src")` doesn't compile — that macro only accepts attribute names already in `markup5ever`'s precompiled static atom table (standard HTML attributes), not arbitrary strings. Custom/non-standard attribute names need the runtime constructor instead: `LocalName::from("data-src")`.

### Patch 9: `dioxus-native`'s `default-features = false` bit us a third time — clipboard and the overlay scrollbar

Two more live reports landed together: no scrollbar visible, and copy/paste not working. Both traced to the exact same root cause as Patches from earlier in this doc (raster image codecs, then `svg`) — a `default-features = false` on a dependency, set once early on for footprint reasons, silently dropping defaults nobody re-checked. This time it was `dioxus-native` itself: its real default feature list is `["accessibility", "hot-reload", "net", "html", "svg", "system-fonts", "clipboard", "file-dialog", "vello-hybrid", "woff", "apple-font-embolden"]`, and the main `Cargo.toml` had `default-features = false, features = ["prelude", "net", "system-fonts", "vello-hybrid"]` — keeping only 3 of 11 real defaults, silently dropping `clipboard` (which gates `blitz-shell/clipboard`, the real OS clipboard backend `ShellProvider::get_clipboard_text`/`set_clipboard_text` need) along with `accessibility`/`hot-reload`/`html`/`svg`/`file-dialog`/`woff`/`apple-font-embolden`. `scrollbars` wasn't the same kind of miss — it's genuinely not a default even upstream (`scrollbars = ["blitz-paint/scrollbars"]`, opt-in), so it needed explicitly adding either way.

Third time this exact mistake has caused a real, live bug (raster codecs and `svg` earlier, now this) — same fix, same lesson: stop cherry-picking individual features from a dependency whose own defaults are already a reasonable, curated set for a desktop-tier build. Removed `default-features = false` entirely; kept only the two genuinely-additional features beyond dioxus-native's own defaults: `prelude` (Dioxus hooks/signals macros this file uses throughout) and `scrollbars`.

Verified: full outer suite (411 tests) and Himalayas DOM's own suite (56 tests) both pass; live launch survives. Scrollbar confirmed live: it's a Chromium-style overlay scrollbar (invisible at rest, flashes in at full opacity during active scrolling, fades out after a delay — that behavior is in blitz-dom's own doc comments, not a bug), and it now correctly flashes in while scrolling. Clipboard copy/paste (including the `Copy link address` context-menu action from Patch 6, silently a no-op the whole time it existed until this fix) still wants a live confirmation.

**Not yet addressed: video.** Distinct from all of the above — this isn't a disabled feature flag, it's a capability that doesn't exist anywhere in Blitz at all. Confirmed via direct source search during the earlier Media Optimization Engine research: `<video>` appears only as a CSS-selector-matchable tag name (`stylo.rs`), with zero decode/playback machinery anywhere in the vendored tree. There is no flag to flip for this — it would mean building real video decode/playback support from scratch, a substantially larger undertaking than anything fixed in this pass.

### Patch 10: momentum scrolling — wheel/trackpad input had none, only touch-drag did

Live report: "scroll experience is not good, has to be more fluid." Real cause, not a vague feel issue: blitz-dom already has a full momentum/inertial-scroll system (`ScrollAnimationState::Fling`, velocity-tracked over a rolling sample window, decelerating smoothly frame-to-frame) — but upstream only ever triggered it from `handle_pointerup` ending a touch-drag pan. Trackpad/wheel scrolling mapped 1:1 to scroll offset with nothing continuing once the wheel events stopped — no coast, no deceleration, just an abrupt halt the instant input stopped.

Fixed by adding a parallel path: `WheelMomentumState` (`events/pointer.rs`) accumulates a rolling window of recent wheel-delta samples, fed by every `handle_wheel` call; a new `BaseDocument::resolve_wheel_momentum` (called every frame, same as the existing `resolve_scroll_animation`) detects when wheel input has gone quiet for `WHEEL_MOMENTUM_GRACE_MS` (80ms — real trackpads fire every 8-16ms while active, so a gap that size is a deliberate stop, not a dropped frame) and hands the computed velocity off to the *same* `Fling` animation touch-panning already used, rather than building a second deceleration system. Real wheel input arriving cancels any in-flight fling first, so a still-coasting fling and fresh direct input never fight over the same frame. Required widening `FlingState.target` from a bare `NodeId` to `Option<NodeId>`, since wheel scrolling (unlike touch panning, which always has a concrete resolved target) can start from a plain viewport-level scroll with no specific hovered element — `None` now means the same thing there as it already does in `BaseDocument::scroll_by`'s own `anchor_node_id` parameter.

Verified with 3 new unit tests (velocity computation over a sample window, minimum-sample-count guard, per-target sample reset when scrolling a different element mid-session) plus the full existing suite.

### Patch 11: the vertical (and horizontal) scrollbar was never there for ordinary whole-page scrolling — only for a node's own `overflow: auto`

Follow-up live report after Patch 9 restored the `scrollbars` feature: "still not there" on real sites, even though the feature was confirmed compiled in and working (it flashed correctly for a nested `overflow: auto` container). Root cause, found by reading `Node::wants_scrollbar`: it checks a *node's own* computed `overflow-x`/`overflow-y` CSS property — which is `visible` (not `auto`), by far the common case, for ordinary pages that don't wrap their content in an explicit scrollable `<div>`. Real whole-page scrolling goes through a completely separate mechanism (`BaseDocument::scroll_viewport_by`, keyed to a document-level `viewport_scroll` field, not any node's `overflow` style or own `scroll_offset`) that the overlay-scrollbar system had no path to at all — no activity tracking, no geometry, nothing. So the single most common way a real page actually scrolls never got a scrollbar, full stop; only a nested `overflow: auto` container (comparatively rare) did.

Two-part fix:
1. **Activity tracking**: `scroll_viewport_by_has_changed` now calls `show_scrollbars` (keyed to the root element's id, standing in for "the viewport") whenever it actually changes the scroll position — previously it updated `viewport_scroll` and returned whether it changed, with zero interaction with the scrollbar-opacity/fade system.
2. **Geometry and painting**: added `BaseDocument::viewport_wants_scrollbar`/`viewport_scrollbar_thumb` (`document.rs`), computing the same "does content overflow the window, and by how much" question `Node::wants_scrollbar`/`scrollbar_thumb` answer per-node, but from document-level metrics (root element's content size vs. `viewport.window_size`) instead. Rather than duplicate the thumb-sizing math, pulled the actual geometry formula out of what used to be `Node::scrollbar_thumb`'s body into a shared free function (`thumb_rect_for`, `node/scrollbar.rs`) both the per-node and viewport-level callers now share — one copy, not two that could quietly drift apart. On the paint side, a new `BlitzDomPainter::draw_viewport_scrollbars` (`blitz-paint/render.rs`) — a sibling to the existing per-node `ElementCx::draw_scrollbars`, not a modification of it — draws the viewport-level thumb pinned to the window edge (deliberately *outside* the scrolled content transform, so it doesn't move with the page, matching how a real overlay scrollbar behaves), reusing the same opacity-fade/color logic.

Explicitly out of scope for this pass: hover/drag-to-scroll interactivity for the viewport scrollbar thumb (the per-node version has it via `ScrollbarRef`/`hovered_scrollbar`/`scrollbar_drag_target`; the viewport version is paint-only — shows while/just after scrolling, fades out, can't be grabbed and dragged). That needs threading viewport-scrollbar hit-testing through the same machinery, a separate, smaller follow-up if it turns out to matter (most real interaction with this browser is trackpad/wheel-first, where drag-the-thumb isn't the primary way anyone scrolls anyway).

Verified: 5 new unit tests directly on the extracted `thumb_rect_for` geometry (no-thumb-when-content-fits, correct top-of-track position when unscrolled, monotonic thumb movement with scroll progress, minimum-thumb-length floor); full Himalayas DOM suite (64 tests) and outer suite (411 tests) both pass; live launch survives.

**Confirmed live**: the vertical scrollbar appears — but only during/just after scrolling, per the Chromium-overlay design it inherited. Live follow-up feedback landed immediately after: it should stay visible "irrespective of whether you are scrolling or not," and should be thicker and longer. Both are small, deliberate policy changes on top of what Patch 11 built, not new mechanisms:

- `BaseDocument::scrollbar_opacity` (`document.rs`) is now a flat `1.0` — the Chromium-style fade-based version (`crate::node::scrollbar::opacity_at`, driven by `scrollbar_activity` timestamps) is gone (removed, not left as unreachable dead code, once it had zero remaining callers). `scrollbar_activity`/`show_scrollbars`/`scrollbars_animating`/`FADE_DELAY`/`FADE_DURATION` are all still in place and still populated/read — reverting to overlay-on-scroll behavior later is a one-function change, not rebuilding that infrastructure.
- `THUMB_THICKNESS`/`THIN_THUMB_THICKNESS` (`node/scrollbar.rs`) went from Chromium's `10px`/`6px` to `14px`/`9px`; `MIN_THUMB_LENGTH` from `32px` to `48px` — explicitly called out as mattering most on small viewports, where the proportional thumb-sizing formula alone would otherwise shrink the thumb to a barely-visible, hard-to-grab sliver on a short track.

Verified: full Himalayas DOM suite (62 tests — the one fade-timing test that only exercised the now-removed `opacity_at` was deleted along with it, not kept around testing dead code) and outer suite (411 tests) both pass; live launch survives.

### Default zoom raised to 130%

Live request: start already zoomed in, "at least 3 levels higher" than before. Blitz already has a real, working zoom system (`Viewport::zoom`, `Viewport::zoom_by`/`set_zoom`) with a built-in Cmd/Ctrl+=/-/0 keyboard shortcut (`blitz-shell/src/window.rs`, upstream — not something Himalayas added), each `+`/`-` press stepping by `0.1`. "3 levels" maps directly onto that existing increment: `Viewport::default()`'s `zoom` (`blitz-traits/src/shell.rs`) went from `1.0` to `1.3`. Since sub-documents (loaded pages) inherit `zoom` from the outer shell document every frame (`resolve.rs`'s subdoc loop), this one constant raises the starting zoom everywhere without needing to reach into per-tab state.

One real behavior change worth being explicit about: Cmd/Ctrl+0 still resets to `set_zoom(1.0)` — true 100%, not back to this new 130% default. That matches what "reset zoom" means in every mainstream browser (100%, not "wherever you started"), so left as-is rather than changed to match the new default.

Verified: full Himalayas DOM suite (62 tests, no regressions from a `Default` impl change used broadly across the crate) and outer suite (411 tests) both pass; live launch survives.

**Superseded almost immediately**: follow-up clarified the actual ask was page content at real 100% with only the *browser shell* (address bar/tabs) enlarged — an accessibility request (bigger text and click targets), not a zoom preference. `Viewport::zoom` can't express that split (one value, inherited by every loaded page from the outer shell document each frame), so reverted back to `1.0` and moved the sizing entirely into `src/bin/desktop.rs`'s RSX instead: address bar row and tab strip font-sizes/dimensions initially set to ~3x their original values directly (tab text 12.5px→38px, address input 13px→39px, icon buttons 32px→96px square, etc.) — real layout-affecting sizes, not a visual scale trick, so hit-testing/click targets grow to match, which is the actual point of an accessibility-motivated size increase. **Dialed back to ~2x (200%)** per direct "too much, put it at 200%" feedback (tab text 12.5px→25px, address input 13px→26px, icon buttons 32px→64px square, etc. — the current, live values in `src/bin/desktop.rs`). Hardcoded for now, not a toggle; promoting it into the settings panel is the natural next step if not everyone wants the shell permanently this large.

### Scrollbar drag-to-scroll — the interactivity explicitly scoped out of Patch 11, now built

Patch 11 shipped the viewport scrollbar paint-only, deliberately: "no hover/drag-to-scroll interactivity... a separate follow-up if it turns out to matter." It turned out to matter — direct live report that dragging the thumb with the mouse did nothing. Built now, reusing the *existing* per-node scrollbar-drag machinery (`DragMode::ScrollbarDrag`, `ScrollbarRef`, `handle_pointerdown`/`handle_pointermove`'s drag handling in `events/pointer.rs`) rather than a second, parallel interaction system:

- `hit_viewport_scrollbar` (`document.rs`) checks pointer coordinates against the viewport thumb's geometry directly (it isn't a DOM node, so it can't participate in `hit_inner`'s node-tree traversal the way a real `overflow: auto` container's thumb does) and is folded into `hit_with_scrollbar`, checked ahead of the node-based result since the viewport scrollbar paints on top of everything.
- The viewport scrollbar's `ScrollbarRef` reuses the root element's id as a stand-in (same convention Patch 11 already established for `show_scrollbars`/`scrollbar_opacity` keying) — which meant `handle_pointermove`'s existing drag-continuation code (unconditionally treating `node_id` as a real per-node scroll target) would have tried to scroll the root element's own `overflow`, not the viewport, doing effectively nothing. Disambiguated via a new `is_viewport_scrollbar` check (root id + root doesn't actually want its own scrollbar in that axis ⇒ must be the viewport one), branching to a new `viewport_scrollbar_drag_ratio` and `scroll_by(None, ...)` instead of the per-node path when true.

Verified: full Himalayas DOM suite (62 tests) and outer suite (411 tests) both pass; live launch survives. Live drag-to-scroll behavior itself still wants confirmation.

### Scrollbar drag sensitivity, then size/contrast — two more rounds of "bigger" feedback

Drag confirmed working after the above, but felt too slow (native 1:1 proportional dragging) — `viewport_scrollbar_drag_ratio` (`document.rs`) got a `DRAG_SENSITIVITY = 2.0` multiplier on top of the strict proportional ratio.

Separately, direct feedback that the bar itself needed to be "bigger and more prominent": `THUMB_THICKNESS` 14px→20px, `THIN_THUMB_THICKNESS` 9px→13px, `MIN_THUMB_LENGTH` 48px→64px (`node/scrollbar.rs`) — second round on top of Patch 11's original bump from Chromium's 10px/6px/32px defaults. `draw_viewport_scrollbars` (`blitz-paint/src/render.rs`) also gained a full-length track rect painted behind the thumb (previously just the thumb pill floated with nothing behind it) and darker/more opaque colors (thumb alpha 178→230, actual gray deepened) so the bar reads clearly against any page background, not just while actively scrolled.

Verified: `cargo build -p blitz-paint -p blitz-dom -p blitz-traits --features blitz-paint/scrollbars` and the outer `cargo build --bin himalayas-desktop --features js_engine` both succeed; the 4 `node::scrollbar::tests` still pass unchanged (they assert against the constants symbolically, not hardcoded literals).

### Page zoom control in the settings panel

Live request: a real, adjustable page-content zoom in the settings panel, floored at 100% (no zooming out below real size). The existing keyboard-shortcut zoom (`blitz-shell/src/window.rs`'s Cmd/Ctrl+=/-/0) lives entirely inside `WindowState`, which Himalayas' `app()`/RSX code has no access to — only `ShellProvider` (a one-directional, fire-and-forget interface) is reachable from there. `ShellProvider` had no zoom method, and `BlitzShellProvider` (its concrete impl) only wraps `window: Arc<dyn Window>` + `proxy: BlitzShellProxy` — no reference to the document, so it can't call `viewport_mut()` directly either.

Fixed by extending the same async event-proxy pattern `request_window_close`/`BlitzShellEvent::CloseWindow` already use: a new `BlitzShellEvent::SetZoom { window_id, zoom }` (`event.rs`), handled in `BlitzApplication::handle_blitz_shell_event` (`application.rs`) via `View::with_viewport` (an existing method, already used for other viewport mutations — sets the value, then resizes the renderer and requests a redraw). `ShellProvider::set_zoom(&self, zoom: f32)` (default no-op, matching every other optional method on the trait) sends the event; `BlitzShellProvider::set_zoom` is the only real implementation.

`src/bin/desktop.rs`: `page_zoom: Signal<f32>` is the source of truth (no getter exists on `ShellProvider` — it's fire-and-forget by design), starting at `1.0`. Settings panel gained a "Page zoom" row (−/percentage/+ buttons, `page_zoom() <= 1.0` disables the − button) that updates the signal and calls `shell.set_zoom(zoom)` via the same `try_consume_context::<Arc<dyn ShellProvider>>()` pattern the existing "Copy link address" clipboard feature already uses. The 100% floor is enforced in the UI layer (`.max(1.0)` on decrement), not in the engine — `Viewport::set_zoom` itself stays a generic, unconstrained primitive.

Verified: `cargo build -p blitz-shell -p blitz-traits` and the outer desktop build both succeed; full Himalayas DOM suite (62 tests) and the outer bin's suite (8 passed, 3 ignored live-network tests) both pass.

### Address bar: click-to-select-all, matching real browser chrome behavior

Live request: clicking an unfocused address bar should select the whole URL (so typing immediately replaces it); clicking again while it's already focused should just place the caret; Cmd/Ctrl+L should focus *and* select all; Ctrl/Cmd+A should explicitly select all (this last one already worked — existing upstream behavior, `node/text.rs`'s keyboard handler already calls `driver.select_all()` on Cmd/Ctrl+A for any text input).

This is chrome-only browser behavior, not something ordinary web-page `<input>`s should also start doing — so it's opted in per-element via a `data-select-all-on-focus` attribute (checked with the runtime `LocalName::from(...)` constructor, same pattern as the lazy-image attribute allowlist in Patch 8, since it's not a precompiled `local_name!` attribute) rather than being a document-wide default. Only Himalayas' own address-bar `<input>` (`desktop.rs`) sets it.

Implemented in `BaseDocument::set_focus_to` (`document.rs`) — the single choke point every real focus *transition* already passes through (the method early-returns before this point if the target was already focused, which is exactly the "second click, just move the caret" case falling through naturally with no extra code). When focus lands on an element carrying the attribute, it builds a `PlainEditorDriver` (same construction `handle_pointerdown` already uses) and calls `select_all()` — overriding whatever caret position a focusing click's `move_to_point` just set moments earlier in `handle_pointerdown` (harmless, since select-all is the last word). This one hook covers both triggers Himalayas has (a focusing click, and Cmd/Ctrl+L's `MountedData::set_focus(true)` → `doc.set_focus_to`) without duplicating logic between them.

Verified: `cargo build -p blitz-dom` and the outer desktop build both succeed; full Himalayas DOM suite (62 tests) unaffected. Live click/drag-to-select-a-portion behavior (unaffected by this change — it's the pre-existing `handle_pointerdown`/pointermove drag path) still wants visual confirmation.

### Bookmark star — add/edit, no folders/import-export yet

Live request: a star button at the right of the address bar that adds a bookmark for the current page on first click (with a confirmation popover: title, URL, folder, Edit/Done) and opens a full editor (rename, change folder, remove, cancel) on a second click of an *already*-bookmarked page's star — explicitly not a silent toggle-off. Deliberately scoped to just this: no nested folders, drag-and-drop, import/export, or a dedicated Bookmark Manager view — those are real, separately-sized asks, tracked but not started (see "still unsolved" below).

Entirely `src/bin/desktop.rs` app-level state, no vendor changes needed. `Bookmark { title, url, folder }` keyed by `url` (so a page reads as bookmarked from whichever tab has it open, not per-tab). `bookmarks: Signal<Vec<Bookmark>>` is in-memory only — cleared on restart, matching every other piece of session state in this file so far (tabs, isolation mode, theme). Star button sits between the address input and the "⋮" settings button; fills solid (★, accent-colored) when the active tab's URL has a matching bookmark, outline (☆) otherwise. Popover has two views gated by `bookmark_editing`: the just-added confirmation (name/folder read-only, Edit/Done) and the full editor (name input, folder toggle between "Bookmarks"/"Bookmarks Bar", Remove/Cancel/Save) — clicking the star while already bookmarked jumps straight to the editor.

Verified: outer desktop build and its 8-test suite both pass unchanged. Live click-through (add → see popover → Edit → rename → Save → star turns solid → click again → editor opens pre-filled → Remove → star turns outline) still wants visual confirmation.

### Pin a tab

Live request: right-click a tab → Pin Tab, moving it to a compact, favicon-only slot at the front of the strip; pinned tabs skip the one-click close "x" (right-click → Close Tab, or Cmd/Ctrl+W on the active tab, still work — just not an easy accidental miss-click); dragging a normal tab into the pinned area pins it, dragging a pinned tab out unpins it.

`Tab` gained a `pinned: bool` field (`false` by default). Rather than filtering into two separately-rendered groups (which would let a tab's on-screen position drift from its actual position in the underlying `Vec<Tab>`, breaking the existing drag-reorder math), pinned tabs are kept *physically* contiguous at the front of the vec and the strip is still one single render loop over it, branching styling on `tab.pinned`: compact 56×56 favicon-badge square (a letter-badge fallback — there's no real favicon-fetching pipeline yet, tracked separately) with no close button, vs. the existing full tab. Pin/unpin (from a new right-click menu, `TabContextMenuRequest`/`tab_context_menu`, a native `oncontextmenu` RSX handler distinct from the existing page-content `ContextMenuRequest`) moves the tab to `other_pinned_count.min(len)` — the count of pinned tabs *excluding itself* — which lands it at the end of the pinned block when pinning and the start of the unpinned block when unpinning, the same formula for both directions since the tab isn't counted as pinned in either case by the time the count is taken.

The existing mousedown-to-arm/mouseenter-to-swap drag reorder (no real HTML5 drag events fire in this shell, see the comment above `dragging_tab`) got one addition: on swap, the dragged tab adopts the hovered tab's `pinned` flag before being reinserted — which is what makes "drag into/out of the pinned area (un)pins it" fall out of the *same* code path as ordinary reordering, rather than needing a second, separate drag-zone-detection system.

Right-click menu also has Reload, Duplicate Tab (via the same `Tab::new` + push `new_tab()` already uses), and Close Tab — a reasonably scoped set, not the full "Mute/Unmute, Move to New Window" list from the original ask (no audio/media-session or multi-window support exists to hang those off yet).

Verified: outer desktop build and its 8-test suite both pass unchanged. Live confirmation (pin/unpin via menu, drag in/out of the pinned zone, compact rendering, close-button suppression) still wants visual confirmation.

### Bookmark Manager: folders, rename, HTML/JSON import & export

Live request: a real Bookmark Manager beyond the star popover's quick add/edit — folders, rename-without-changing-URL, HTML (Netscape format, for Chrome/Firefox/Edge interop) and JSON (Himalayas-native backup) import/export, with the explicit rules "don't overwrite existing bookmarks," "preserve folders," "handle duplicate URLs intelligently." Deliberately scoped down from the full recommended checklist in the original ask: flat (single-level) folders, no drag-and-drop reordering/move-between-folders, no multi-select bulk actions — real, separately-sized asks, not started.

`Bookmark` gained `added_at: i64` (unix seconds, `#[serde(default)]` so older/hand-written JSON without it still parses — just sorts as epoch) and `Serialize`/`Deserialize`, since the JSON wire format (`BookmarkExportFile { version: 1, bookmarks: Vec<Bookmark> }`) *is* the struct plus a version tag, no separate DTO needed. A new `folders: Signal<Vec<String>>` (default `["Bookmarks Bar", "Other Bookmarks"]`) replaces the star popover's old hardcoded two-button folder toggle with a loop over the live list.

The manager itself is a `position:fixed` modal (a full overlay, unlike every other popover in this file so far, which are all `position:absolute` anchored to a chrome button) with search (title/URL substring, case-insensitive), a Name/Date-added sort toggle, per-folder sections (click a folder name to rename it inline — Enter saves, Escape cancels; renaming re-labels every bookmark in it; "Delete folder" reassigns its contents to another existing folder rather than deleting bookmarks, and is hidden when only one folder remains so there's always somewhere to reassign to), and per-bookmark rows (click to open in the current tab, ↗ opens in a new tab via the same `Tab::new`/`new_tab()` pattern, ✎ renames inline, × deletes).

Import/export uses `rfd::FileDialog` directly from `desktop.rs` — added as a plain outer `Cargo.toml` dependency (same 0.17 version blitz-shell's own, separate workspace already pins) rather than a new `ShellProvider` method like the zoom feature needed. The difference: zoom has to reach `Viewport`, state that only exists inside blitz-shell's `WindowState`, so it needed a proxied event; a file dialog is a self-contained OS call with no engine state to reach, so `desktop.rs` (a native, unsandboxed binary) can just call it. Format is picked by file extension on import (`.json` vs `.html`/`.htm`) and by which export button was clicked. `export_bookmarks_html` writes one flat `<DL>` per folder in the real Netscape Bookmark File Format (no nested sub-folders, matching `Bookmark::folder` being a flat label); `parse_bookmarks_html` is a small regex-based scanner (`<H3>`/`<A HREF>`, sorted by source position so each link picks up whichever folder header most recently preceded it) rather than a full HTML parser — proportionate for a format that's realistically just that one repeating shape in every real browser's export. `merge_imported_bookmarks` is the shared duplicate/new-folder logic behind the import button's completion summary ("N bookmarks imported, M folders imported, K duplicates skipped") — same-URL entries are treated as duplicates and skipped regardless of folder, existing bookmarks are never overwritten.

Caught by the new tests, not by inspection: the first HTML-import implementation stripped `<tag>` markup via regex but never unescaped HTML entities, so `Docs &amp; Guides` round-tripped as the literal string `"Docs &amp; Guides"` instead of `"Docs & Guides"` — a real browser's actual export escapes `&` in both titles and `href` query strings. Fixed with a small `html_unescape` (mirroring the existing `html_escape` used on export), ordered to unescape `&amp;` *last* so an already-escaped `&amp;lt;` in the source doesn't get double-unescaped into `<`.

Verified: outer desktop build succeeds; 13 tests pass (5 new — HTML export→import round-trip, a real-shaped browser-export import including the entity-unescaping case, JSON export→import round-trip, JSON import of the documented bare `{"version":1,...}` shape, and duplicate/new-folder merge behavior — plus the 8 pre-existing). Live click-through (open manager → search/sort → add/rename/delete folder → rename/delete a bookmark → import a real exported HTML file → export HTML and JSON and confirm they reopen correctly) still wants visual confirmation.

### HTTP cache: enabling a real cache that already existed, dormant

First slice of the much larger caching-subsystem vision (memory/disk/service-worker cache, partitioning, eviction policy, an observability inspector, etc. — all separately-sized, not started). Investigated where Himalayas actually fetches things first rather than guessing: three independent fetch paths exist (native desktop top-level document via a bare `reqwest::Client` in `desktop.rs`'s `http_client()`; native desktop subresources — CSS/images/fonts — via a fresh `blitz_net::Provider` per navigation; the `/app` web-shell path via `Navigator`'s own separate `reqwest::Client`, `src/browser/navigator.rs`), none of which shared any caching.

The subresource path turned out to already have a *complete, correct* disk HTTP cache implementation sitting in `vendor/blitz/packages/blitz-net/src/lib.rs` — `Provider::new` wraps its `reqwest::Client` in `reqwest-middleware`'s `Cache(HttpCache{...})` (via `http-cache-reqwest`, disk-backed by `cacache`/`CACacheManager`), correctly honoring Cache-Control/ETag/Last-Modified/Expires, and deliberately configured `shared: false` (private-cache semantics — the existing code comment there explains this was already tuned once: the default `shared: true` treats any `Set-Cookie` response without explicit `public`/`immutable` as instantly stale, which was defeating caching and getting real image CDNs to rate-limit). All of this was gated behind a `cache` Cargo feature that was simply **never turned on** — the outer `Cargo.toml`'s `js_engine` feature list enabled `dep:blitz-net` but no features on it at all, so every subresource fetch hit the network every time regardless of cache headers. This is a fourth occurrence of the same class of bug that's bitten this session three times already (raster image codecs, `svg`, then `dioxus-native`'s clipboard/scrollbars) — a real, already-correct feature quietly not doing anything because nothing opted into it. Fixed with one line: `"blitz-net/cache"` added to `js_engine` in the outer `Cargo.toml`.

Verified with an actual cache-hit test, not just a compile check (`vendor/blitz/packages/blitz-net/src/lib.rs`'s new `cache_tests` module, `cargo test -p blitz-net --features cache`): a minimal hand-rolled local TCP server (no new prod dependency — just `tokio`'s `net`/`io-util`, added as a `[dev-dependencies]`-only feature addition) answers with `Cache-Control: max-age=3600`; two `Provider::fetch_async` calls for the same URL leave the server's connection-count at 1, proving the second was actually served from the on-disk cache rather than hitting the network again. `provider.clear_cache().await` runs before and after the test, since the cache lives in a fixed, real OS cache directory (`get_cache_path`, not test-scoped or configurable) shared across every `Provider` instance and every test run — without clearing it first, a stale entry from a previous run could make the test pass for the wrong reason.

Deliberately not touched in this pass: the top-level document fetch (`desktop.rs`'s `http_client()`) and the `/app` `Navigator` path are still uncached — both use a bare `reqwest::Client` directly rather than `blitz_net::Provider` (the top-level path needs response headers, which `Provider::fetch_async` doesn't expose, by design — see that code's own comment), so enabling this feature doesn't reach either of them. Caching HTML documents is also a genuinely different, more nuanced problem than static subresources (`Vary`, more frequent `no-store`), not just "flip the same switch again." Also not touched: `cookies` (blitz-net's own feature, separate from `cache`) — left off since Himalayas already has its own hand-rolled per-`Session`/`IsolationMode` cookie jar, and turning on blitz-net's built-in cookie store too risked two independent cookie stores fighting each other, not evaluated in this pass.

Verified: `cargo test -p blitz-net --features cache` (1 new test, passing) and the outer desktop build/13-test suite, plus the non-`js_engine` `himalayas` binary build (`cargo build --features full`), all pass unchanged. Live confirmation (a real repeat-navigation actually skipping a network round-trip for a cacheable image/CSS/font) still wants a live before/after check — the unit test proves the mechanism works, not that it fires correctly against real-world response headers from real sites.

### Real `<img loading="lazy">` support

First slice of "Image Compatibility"'s Lazy Loading section, chosen after auditing what actually exists today: the standard `loading="lazy"` attribute did nothing at all — `DocumentMutator::load_image` (`mutator.rs`) fetched every `<img>` unconditionally the instant `src` was set, real attribute or not, regardless of position. (Same audit found three other real gaps not picked up this pass — animated images, `<picture>`/`<source>`, and `<video>`/`<audio>` — noted below since `<video>`/`<audio>` in particular would be new subsystems from zero, not extensions of something existing, unlike this one.)

`load_image` now checks the `loading` attribute first: `loading="lazy"` adds the node to a new `BaseDocument.lazy_images: HashSet<NodeId>` and returns *without* fetching (the existing fetch body was renamed to `load_image_now`, called both by the non-lazy path and by the trigger below). A new `check_lazy_images` (`document.rs`), called from `resolve()` right after transforms are resolved (so layout positions are current), walks `lazy_images` each pass — a no-op the instant the set is empty, the common case — and for each node compares `absolute_position(0.0, 0.0).y` (document-relative, not scroll-adjusted — see below) against a load boundary of `viewport_scroll.y + viewport_height + 1.5×viewport_height`, matching real browsers' "start loading somewhat before it's actually visible" behavior rather than an exact-intersection trigger. Anything past that boundary triggers a real `load_image_now` and drops out of the pending set.

Two real bugs the test below caught, not inspection:
1. **A deadlock.** The first version skipped any node whose `final_layout().size` was `(0, 0)`, reasoning that meant "hasn't been laid out yet" (e.g. a `display:none` ancestor). But an unloaded `<img>` with no explicit `width`/`height` *legitimately* lays out at 0×0 — there's nothing to size it by until it loads. That guard meant a lazy image could never become eligible in the first place, since becoming eligible is exactly what would give it a real size. Removed the guard entirely; a node's document position is meaningful even at zero size.
2. **An inverted sign convention** in the test itself (not production code): `scroll_viewport_by(x, y)` computes `viewport_scroll - (x, y)`, so scrolling *down* (increasing `viewport_scroll.y`) needs a *negative* `y` argument — the opposite of what "scroll by y" reads as. Worth knowing for the next person calling this method, since nothing about the name suggests the inversion.

Verified with an actual fetch-observing test, not just a compile check: `desktop.rs`'s new `loading_lazy_defers_offscreen_images_until_scrolled_near` builds a real document (two images — one immediately visible, one `loading="lazy"` ~4000px below a spacer div) against a `RecordingNetProvider` (a small test-only `NetProvider` impl that just records which URLs were asked for, since blitz-dom's `lazy_images` field is private to that crate — this is a black-box, fetch-observed check from the outer crate, not a white-box one). First `resolve()`: the visible image fetched, the lazy one didn't. Scroll near it, `resolve()` again: it fetches. `cargo test -p blitz-dom` (62 tests) and the outer desktop suite (14 tests, 1 new) both pass; full desktop build succeeds.

Not built this pass (from the same audit): animated GIF/WebP/APNG (every image decodes and renders as a single static frame — `ImageHandler::parse` calls `.decode()`, never `AnimationDecoder`/`into_frames`; would need a real new subsystem — a per-image frame list plus an animation clock/repaint scheduler, not a small extension); `<picture>`/`<source>` (unhandled entirely — no `local_name!("picture")`/`"source"` matching anywhere in blitz-html/blitz-dom, so a `<picture>`'s children are just inert and the engine falls through to whatever plain `<img>` is nested inside, ignoring `<source>` media/type candidates — a real, scoped follow-up that could reuse the existing `select_srcset_candidate`/`eval_sizes` from the srcset/sizes work); `<video>`/`<audio>` (see the caching section's sibling investigation — `<video>` gets CSS box-model treatment only, `<audio>` isn't referenced anywhere, no `poster` support, no media `SpecialElementData` variant at all — genuinely build-from-zero, not something with an existing half to extend).

### Pin tab persistence (Phase 1.2)

First real session persistence anywhere in `desktop.rs` — every other piece of state (tabs, bookmarks, folders, theme) is still in-memory only, cleared on restart. `SessionState { restore_pinned_tabs, pinned_tabs: Vec<PinnedTabRecord> }` round-trips through a JSON file at `directories::ProjectDirs::from("com", "Himalayas", "Himalayas").config_dir()/session.json` (a new `directories` dependency, same crate/major version blitz-net already pins in its own workspace for the HTTP cache directory).

Persisted on: pin/unpin (the tab context-menu action), drag-end if the drag crossed the pinned/unpinned boundary (written once at `onmouseup`, not on every `onmouseenter` during the drag — cheap enough either way, but no reason to write repeatedly mid-drag), closing a pinned tab, and toggling the new "Restore pinned tabs" setting itself (a toggle switch in the settings panel's new "Startup" section — the setting is persisted independently of the tab list, so turning it off is remembered too, not just acted on once). Loaded on startup: each of `tabs`/`active_id`/`next_id`'s `use_signal` initializers independently calls `load_session_state()` (a cheap file read) rather than sharing one precomputed value — `Tab::new` has a real side effect (`browser().open_tab(...)`, a real backend session), so the values it produces can only safely be computed inside a `use_signal` closure, which Dioxus guarantees runs exactly once; sharing a value *across* multiple `use_signal`s would need a different hook shape than this codebase already established elsewhere.

The disk I/O itself (`load_session_state_from`/`save_session_state_to`) is split from the real-config-dir-path wrappers (`load_session_state`/`save_session_state`) specifically so tests can exercise actual read/write/default-on-missing-or-corrupt behavior against a throwaway path instead of touching the user's real config directory — and the tab-list-to-`SessionState` conversion (`pinned_tabs_session_state`) is split out as a pure function for the same reason, no I/O needed to test the filtering logic at all.

Verified: 4 new tests (pinned-only filtering, round-trip through a real temp-dir file, default-on-missing-file, default-on-corrupt-file-instead-of-panicking) plus the existing 14 all pass (18 total); full desktop build succeeds. Live confirmation (pin a tab, quit, relaunch, see it restored; toggle the setting off, relaunch, confirm it's not) still wants a human pass — this environment has no way to actually quit-and-relaunch the GUI itself.

### `<picture>`/`<source>` (Phase 1.3)

Confirmed unhandled entirely before this (per the earlier audit): no `local_name!("picture")`/`"source"` matching anywhere in blitz-html/blitz-dom. `DocumentMutator::picture_source_for` (`mutator.rs`) is checked first in `load_image_now`, before the existing `<img src/srcset>` resolution: if the `<img>`'s parent is a `<picture>`, it walks the parent's children up to (not including) the `<img>` itself, and for each `<source>` checks `type` (if present, must be one of the codecs actually compiled into the `image` crate — `png`/`jpeg`/`gif`/`webp`/`bmp`/`avif`/`ico`, a new `is_supported_image_mime_type`, `responsive_images.rs`) and `media` (if present, evaluated via the *existing* `eval_media_condition` — now `pub(crate)` instead of private — the same `min-width`/`max-width`-only subset already used for `sizes` conditions, not a full CSS conditions parser). The first `<source>` that passes both checks has its own `srcset`/`sizes` resolved via the same `select_srcset_candidate`/`eval_sizes` the plain-`<img>` path already uses, and that candidate wins outright — per spec, a matched source replaces the `<img>`'s own `src`/`srcset` entirely rather than just adding one more fallback option. No `<picture>` parent, or a `<picture>` where nothing matched, falls through to the exact existing `<img>`-only logic unchanged.

Caught by the new integration tests, not by inspection: the first test run failed because it set the viewport *after* calling `HtmlDocument::from_html` (matching the existing lazy-load test's pattern) — but unlike lazy-loading (resolved later, during `resolve()`), `<picture>`/`srcset` selection happens at *parse* time, the instant `load_image` fires while the tree is being built. By the time the test patched in a real viewport afterward, the image had already loaded against the default `(0, 0)` viewport, which both the picture-source and plain-srcset paths already correctly treat as "unknown, don't select — fall back to the plain src" (a pre-existing, deliberate behavior, not a bug). Fixed by setting `DocumentConfig.viewport` directly instead of patching it in after construction — a real timing constraint worth knowing for anyone else writing a srcset/picture test against a document built without a real window.

Verified: 2 new integration tests (`picture_source_wins_over_type_and_media_mismatches_and_the_fallback_img`, `picture_falls_back_to_img_when_no_source_matches`) against a `RecordingNetProvider`, exercising a real `<picture>` with a type-mismatched source, a media-mismatched source, a matching source, and a fallback `<img>` together — plus 2 new pure unit tests in `responsive_images.rs` (`recognizes_the_codecs_actually_compiled_in`, `media_condition_shares_min_max_width_logic_with_sizes`). `cargo test -p blitz-dom` (64, up from 62) and the outer desktop suite (20, up from 18) both pass; full desktop build succeeds.

### Bookmark Manager: drag-to-move and multi-select (Phase 1.4)

The two pieces explicitly scoped out of the first Bookmark Manager pass. Both reuse patterns this file already established rather than introducing new mechanisms:

- **Drag-to-move**: a bookmark row's `onmousedown` arms `dragging_bookmark: Signal<Option<String>>` (the URL), a folder header's `onmouseup` reassigns that bookmark's `folder` and clears the signal — the same mousedown-arm/mouseup-resolve shape the tab strip's drag reorder already uses (no real HTML5 drag events fire in this shell, per that code's own comment). Deliberately *not* full manual reordering within a folder: the visible list order comes from `items.sort_by(...)` (name or date), which a drag-to-reorder would either have to fight or replace with a new "custom" sort mode plus a persisted per-bookmark order field — a real, separate follow-up, not bundled in here. The folder header highlights (background tint) while a drag is in progress and targeting a *different* folder than the dragged bookmark's current one, so hovering a bookmark's own folder doesn't visually flicker.
- **Multi-select**: `selected_bookmarks: Signal<HashSet<String>>` (by URL, consistent with every other bookmark signal in this file), a checkbox per row, and a bulk-action toolbar that appears above the list only while the selection is non-empty (move to any folder via a button row, delete, clear). The "move to" control is a row of folder buttons, not a native `<select>` — `<select>`/`<option>` support in this engine wasn't confirmed working when the star popover's folder picker was built earlier (that decision is documented in that section), so this reuses the same already-proven button-list pattern rather than gambling on `<select>` for the first time here. (`<input type="checkbox">`, by contrast, *is* confirmed real/working in this engine — `blitz-paint/src/render/form_controls.rs` and `blitz-dom/src/form.rs`/`events/pointer.rs` all handle it — so the checkboxes themselves aren't a similar risk.)

Not covered by automated tests this round: both features are pure RSX-closure UI logic (drag state, folder reassignment, bulk mutation) with no pure/extractable function to unit test in isolation — the same depth of coverage the existing folder rename/delete and pin/unpin handlers already have (verified by build + manual reasoning, not a dedicated test), not a regression in rigor for this specific addition.

Verified: outer desktop build succeeds; existing 20-test suite unaffected (no behavior in it touches this code path). Live confirmation (drag a bookmark onto a different folder header and see it move, multi-select several bookmarks and bulk-move/delete them) still wants a human pass.

### Caching, the rest of it: top-level document + `/app` Navigator (Phase 2.6)

Picks up exactly where the earlier HTTP-cache section left off: "the top-level document fetch and the `/app` `Navigator` path are still uncached — both use a bare `reqwest::Client` directly rather than `blitz_net::Provider`... so enabling this feature doesn't reach either of them." Both now get the same real Cache-Control/ETag/Last-Modified-aware disk cache blitz-net's subresource path already has, via a new shared `src/net_cache.rs` (`pub mod` in both `lib.rs` and `main.rs` — this codebase's existing pattern of a second, narrower module tree for the `himalayas` binary, mirrored here like everywhere else) rather than two independent implementations: `net_cache::cached_client(base)` wraps an already-configured `reqwest::Client` in `reqwest-middleware`'s `Cache(HttpCache{...})`, same crates/private-cache configuration as `blitz-net`'s own (`vendor/blitz/packages/blitz-net/src/lib.rs::Provider::new`, a separate vendored workspace, so that code couldn't just be called directly) — `shared: false`, for the same reason documented there: a shared-cache policy treats any `Set-Cookie` response as instantly stale, and both of this crate's callers attach per-tab session cookies to nearly every request. Uses its own cache directory under Himalayas' own `directories::ProjectDirs` identity (`com.Himalayas.Himalayas`, the same one the pinned-tabs session file uses for its config dir, just `.cache_dir()` instead) — deliberately separate from blitz-net's own `com.DioxusLabs.Blitz` cache, since document and subresource responses have different reuse patterns.

**A real, non-obvious blocker, not a small detail**: `reqwest-middleware` requires its own re-exported `reqwest` version (0.13.x) — this crate's own direct `reqwest` dependency is pinned to 0.12 elsewhere in `Cargo.toml`, and the two are *not* interchangeable types (`reqwest::Client` built from 0.12 doesn't satisfy a function expecting 0.13's `Client`, despite being "the same crate"). Both `Navigator::new()` (`src/browser/navigator.rs`) and `desktop.rs`'s `http_client()` had to switch from `reqwest::Client::builder()` to `reqwest_middleware::reqwest::Client::builder()` specifically to build their base client — a one-line-looking change that's actually load-bearing. This isn't a new problem introduced here: `Cargo.lock` already carried both reqwest versions once `blitz-net/cache` was enabled (the earlier HTTP-cache phase), so this reuses an already-resolved dependency rather than introducing a second duplicate tree.

`Navigator`'s `client` field type changed from `reqwest::Client` to `reqwest_middleware::ClientWithMiddleware` — a small, mostly mechanical change since only two call sites use it (`fetch_page`'s `.get()`, `submit_form`'s `.post()`), both only chaining `.header()`/`.form()`/`.send()`, all of which `reqwest-middleware`'s `RequestBuilder` forwards (needed enabling its `form` feature explicitly, matching blitz-net's own feature list for the same crate).

Also added: a real "Clear cache" button (Settings panel, new "Privacy & Storage" section) — `net_cache::clear_http_cache()` wipes the on-disk store. This exists because there's **no automatic eviction policy yet** — `cacache` (the storage backend both this and blitz-net's cache use) doesn't cap size or evict on its own, so without a manual clear the cache directory only grows. A real size-bounded/LRU eviction policy, a bounded in-memory tier in front of the disk cache (real browsers check memory before disk — though blitz-net's separate `image_cache` already covers the single highest-value case, decoded images, which is the expensive-to-redo work an in-memory tier would mostly be protecting), and per-origin cache partitioning (matching the `IsolationMode`-per-`Session` precedent already established for cookies) are all real, explicitly **not built this pass** — each individually sized enough to be its own follow-up rather than something to bolt on alongside everything else here.

Verified: a real cache-hit test for `Navigator` (`test_navigate_does_not_refetch_a_cacheable_url`, using the existing `mockito`-based test pattern already in `navigator.rs` — sets `Cache-Control: max-age=3600` and asserts the mock's `.expect(1)` holds after two `navigate()` calls to the same URL) plus 2 new tests for `clear_http_cache`. Full outer workspace suite: 409 (up from 403 pre-Phase-2, +6 new) across the `himalayas` lib/bin targets; desktop suite unaffected (20); `cargo build --features full` and `cargo build --bin himalayas-desktop --features js_engine` both succeed. Live confirmation (a real repeat navigation actually skipping the network for a cacheable page, and the Clear Cache button actually freeing disk space) still wants a live check.

### MCP server: `himalayas mcp` (Phase 2.5)

The earliest deferral in this whole session ("Add a MCP to this browser first so that Claude can check directly") — picked up once the underlying capability it needed (`AgentContext`, `src/api/mod.rs`) was confirmed already real and complete: navigate/query/click/input/get_text/submit_form/go_back/go_forward/get_current_url/get_history, the same set the `/agent` HTTP endpoint already exposes (`src/server.rs::dispatch_agent_action`). A Model Context Protocol server is genuinely just a second, MCP-native transport over that same capability, not new browser capability — confirmed correct rather than assumed: protocol framing and every message shape below were checked against the live spec (`modelcontextprotocol.io/specification/2025-06-18`) via `WebFetch`, not guessed from general knowledge, since a subtly-wrong framing detail would produce something that *looks* done but fails to actually handshake with a real client.

New `src/mcp.rs` (`pub mod` in both `lib.rs` and `main.rs`, this codebase's established pattern), reachable via a new `himalayas mcp` CLI subcommand (`main.rs`'s existing `daemon`/`benchmark` `Subcommand` enum, same shape). `run_stdio_server` builds one `Browser` + one `AgentContext` for the process's whole lifetime (stdio MCP is inherently single-client-per-subprocess — the client launches the server, unlike the HTTP endpoint's per-`session_id` map, so there's no equivalent multi-tenancy concern), then loops reading newline-delimited JSON-RPC messages from stdin and writing responses to stdout — MCP's specified stdio framing (confirmed via the spec fetch): one JSON-RPC message per line, no embedded newlines, and **`stdout` must carry only valid MCP messages** (this already held without any change needed: `init_logging`, `main.rs`, already sends every `tracing` log line to `stderr`, and `mcp.rs` never uses `println!`).

Handles `initialize` (responds with protocol version `2025-06-18`, a `tools` capability, and `serverInfo`), the `notifications/initialized` notification (no response — notifications never get one, matching JSON-RPC 2.0), `ping`, `tools/list` (all 10 `AgentContext` methods, each with a real JSON Schema `inputSchema`), and `tools/call` (dispatches by name, wraps the result as `{content: [{type: "text", text: ...}], isError: false}` — or `isError: true` with the error message on a tool-*execution* failure, e.g. a bad selector or unreachable URL; a missing/unknown tool name or missing required argument is a JSON-RPC *protocol* error instead, per the spec's own distinction between the two error classes). Unknown request methods get a real `-32601 method not found`; unknown *notifications* are silently ignored (forward-compatibility convention) rather than erroring.

Verified two ways: 7 new unit tests (`src/mcp.rs`'s own `mod tests`) covering the message-handling logic directly, and a real subprocess smoke test — `himalayas mcp` launched as an actual child process, fed real `initialize`/`notifications/initialized`/`tools/list`/`tools/call` messages over its real stdin, output read back from its real stdout. Confirmed: `initialize` returns the right protocol version and `serverInfo`; the notification produces zero output lines (not a blank line, no output *at all*); `tools/list` returns all 10 tools with real schemas; a `tools/call` for `navigate` against `https://example.com` correctly propagated a real network error as `isError: true` with a readable message (this sandboxed shell environment can't reach arbitrary external hosts — a real environment limitation, not a code defect, same category as this repo's other `#[ignore]`d live-network tests) rather than panicking or hanging; stderr contained only log lines, stdout contained only valid JSON-RPC. Full outer workspace suite: 423 (up from 409, +14 — 7 real, the rest from this repo's existing dual lib/bin module-tree compilation pattern already noted elsewhere in this doc, which runs shared test files twice).

**A real, separate bug this surfaced while re-running the full suite, not related to MCP itself**: `net_cache::tests::clear_http_cache_removes_the_real_cache_directory` (added in the caching phase just above) intermittently failed with `DirectoryNotEmpty` — Rust runs `#[test]`s in parallel by default, and that test's `remove_dir_all` on the real, shared `http_cache_dir()` was racing with `Navigator`'s own cache test (`test_navigate_does_not_refetch_a_cacheable_url`) actively writing cache entries into that *same* real directory concurrently — a genuine TOCTOU race from testing a destructive operation against shared global state, not a flaky test to just retry. Fixed with the same split already used for session persistence: `clear_http_cache_at(path)` (the real, testable logic) separated from `clear_http_cache()` (the real, path-hardcoded wrapper), with both remaining tests now targeting a throwaway temp path instead of the real one. Confirmed fixed by running the full suite 5 times in a row with zero failures (previously intermittent, not deterministic, so a single clean run wouldn't have been enough to trust).

### Animated images: real GIF playback (Phase 3.8)

First of the three Phase 3 items, chosen deliberately as the tractable one: confirmed by direct audit that `ImageHandler::parse` (`blitz-dom/src/net.rs`) only ever called `.decode()` (a single `DynamicImage`), never the `image` crate's own `AnimationDecoder` — every animated GIF/WebP rendered as its first frame, forever. Scoped to GIF only for this pass (the most common real-world animated format, and the case this was actually verified against) — animated WebP (`WebPDecoder` also implements `AnimationDecoder`) and APNG (`PngDecoder::apng()`) are real, scoped widenings for later, same incremental-growth pattern already used for the lazy-load data-attribute allowlist.

**Decode**: `ImageHandler::decode_animated_gif` (new) is tried first, gated behind a cheap `image::guess_format` sniff so only actual GIFs pay for it; `None` (not `Err`) on decode failure or a "GIF" with only one frame falls straight through to the unchanged static-image path, so this can only make GIF handling *more* capable, never worse. A new `Resource::AnimatedImage` variant (parallel to `Resource::Image`, not folded into it, so no existing `Resource::Image` call site had to change) carries the decoded frames through to `apply_loaded_image` (`document.rs`), which now recognizes `ImageData::Animated` and starts that node's playback clock.

**Data model**: `ImageData` gained an `Animated(AnimatedImageData)` variant — `{ width, height, frames: Arc<Vec<AnimatedFrame>>, current_frame: Arc<AtomicUsize> }`. `current_frame` is an `AtomicUsize` (not a `Cell`) specifically to keep the type `Sync` — nodes need to stay shareable across threads for the `parallel-construct` rayon traversal even though nothing mutates this concurrently in practice; `Cell` would have silently broken that. It's also wrapped in an `Arc` (not bare) so `AnimatedImageData::clone()` — which happens whenever the same cached animated image gets applied to more than one waiting node — shares one counter across all of them, matching real browsers: multiple `<img>` tags pointing at the identical animated GIF stay frame-synchronized rather than each running an independent, gradually-diverging clock.

**Playback clock**: a new `BaseDocument.animated_images: HashMap<NodeId, Instant>` (when the current frame started showing) and `advance_animated_images()` (called every `resolve()`, same shape as `check_lazy_images` — a no-op the instant the map is empty), which compares elapsed time against the current frame's delay and, once it's passed, advances `current_frame` (wrapping back to 0 after the last frame) and marks the node dirty for repaint. `is_animating()` now also considers `!self.animated_images.is_empty()`, which is what actually keeps the shell scheduling another `resolve()` call while anything is playing (matching the exact pattern `wheel_momentum`/`scrollbars_animating` already established) — `advance_animated_images` itself only decides whether a frame boundary was crossed *within* that already-running loop, not whether to keep the loop running at all.

**Paint**: `ElementData::raster_image_data()` (blitz-paint's actual read path, `draw_image` in `render.rs`) changed its return type from `Option<&RasterImageData>` to `Option<Cow<RasterImageData>>` — a plain `Raster` image is still borrowed directly at zero cost, but an animated image's current frame has to be assembled on the fly (there's no standing `RasterImageData` for it to borrow), and `Blob::clone()` is an `Arc` bump, not a pixel copy, so the owned case stays cheap per paint call. CSS `background-image`/`mask-image` animation is explicitly out of scope for this pass — `background.rs`'s `let ImageData::Raster(image_data) = &bg_image.image else { return; }` simply doesn't match `Animated` and no-ops, same as it already did for SVG or anything else non-Raster; a real, separate follow-up if a page's background actually needs it.

A real "GIF encoder" test approach, not fixtures or guesswork: `net.rs`'s new tests build actual multi-frame GIFs in memory via the `image` crate's own `GifEncoder` (3 solid-color 2×2 frames at distinct delays), then decode them back and check exact frame count, delays, and pixel colors — including a real bug this line of testing would have caught if it existed: the "floor a near-zero delay to 100ms" behavior (many real encoders write `0` expecting exactly this convention) is asserted directly, not just implied by the code. Separately, `document.rs`'s new `animated_image_tests` module drives the *playback clock* end-to-end without needing blitz-html/blitz-net at all — same `document.load_resource(fabricated_response)` pattern the existing `font_face_override_tests` module already established for driving the resource-application pipeline directly in a unit test. Confirms: loading an animated image registers it and makes `is_animating()` true; a frame does *not* advance before its delay elapses; frames advance in order and loop back to 0 after the last; a stale entry (node no longer resolvable) gets cleaned up rather than left stuck in the map forever.

Verified: `cargo test -p blitz-dom` — 72 passing, up from 64 (+8: 4 GIF-decode unit tests in `net.rs`, 4 playback-clock tests in `document.rs`). Full outer desktop build and its 20-test suite both pass unchanged (nothing in that suite touches image decoding directly). Live confirmation (a real animated GIF on a real page actually cycling frames on screen at the right rate) still wants a human pass — the tests prove the decode and clock logic is correct, not that it looks right rendered.

### Real `<audio>` decode + playback (Phase 3.9)

The second of the three Phase 3 items — and, unlike animated images, a genuinely new dependency tree (real OS audio device access), not an extension of something already pulled in. Confirmed by the earlier audit: `<audio>` wasn't referenced anywhere in blitz-dom/blitz-paint at all before this.

**Scope, deliberately bounded**: real decode and real playback, triggered by the `autoplay` attribute — the concrete, verifiable "does audio actually come out of the speakers" behavior. Explicitly *not* built this pass: `currentTime`/seeking, `volume`/`muted` element properties, a visual `controls` widget (a real UI subsystem of its own, same category as the existing `custom-widget` form controls), and JS-facing `HTMLMediaElement` bindings (`.play()`/`.pause()` from script) — this engine's JS DOM binding surface has no media element methods at all yet; that's a separate, real addition on top of what's here, not included by having the native playback exist. Opus isn't decoded (not in symphonia's built-in codec set) — MP3/AAC/WAV/Vorbis/FLAC cover the rest of the "core priority" set from the original ask.

**New dependency, feature-gated off by default**: `rodio` 0.20 (symphonia-backed decoders), added as blitz-dom's new `audio` feature (`dep:rodio`) — matching how `blitz-net`'s `cache` feature is opt-in rather than folded into `default`, since this is real added binary weight, not a footprint-neutral extension. The outer `Cargo.toml`'s `js_engine` feature list now also flips on `blitz-dom/audio`, the same pattern already established for `blitz-net/cache`.

**A real architectural constraint, resolved by checking the actual API docs rather than assuming**: `rodio::OutputStream` (the live OS audio device handle) is deliberately `!Send`/`!Sync` — confirmed via `docs.rs` before writing any code, not discovered by a compile error — because many platform audio APIs require the stream to stay on the thread that opened it. That rules out storing it anywhere in the DOM tree, which needs to stay `Send + Sync` for `parallel-construct`'s rayon traversal (the exact same constraint that shaped `AnimatedImageData::current_frame` using `AtomicUsize` instead of `Cell` in the animated-image work above — recurring theme, not a coincidence). The fix: a `thread_local!` holds the one `OutputStream` for whichever thread first plays something, and only ever hands out clones of `OutputStreamHandle` (confirmed `Send + Sync + Clone`) to create a `Sink` per `<audio>` element — `Sink` itself (confirmed `Send + Sync`) *can* live directly on the node (`AudioElementData.player: Arc<AudioPlayer>`, in the new `crate::audio` module), no thread-local indirection needed there.

**Wiring**, following the exact pattern `<img>` already established: `mutator.rs`'s `process_added_subtree` tag-match gained an `"audio"` arm (queuing a new `SpecialOp::LoadAudio`, gated `#[cfg(feature = "audio")]`); `load_audio` reads `src`/`autoplay`, sets up `SpecialElementData::Audio`, and fetches the raw bytes via a new `AudioBytesHandler`/`Resource::Audio` (a plain pass-through, unlike `ImageHandler` — no decode happens at fetch time; `rodio::Decoder` decodes lazily when `AudioPlayer::play` actually starts, since decode is comparatively cheap and there's no equivalent motivation to decode-then-cache the way image pixel data has). `document.rs`'s resource-application match gained the `Resource::Audio` arm: records the resolved `src`, and calls `player.play(bytes)` if `autoplay` was set. Removing an `<audio>` node from the tree now pauses its player (`process_removed_subtree`'s special-data cleanup match) — playback stops when the element is removed, not left running detached in the background.

Every fallible step degrades silently rather than erring or panicking — no audio output device available, or bytes that don't decode as a supported format — matching how a real `<audio>` element degrades (an `error` event a page *might* listen for, not a crash); every one of `AudioPlayer`'s methods is deliberately infallible (`()`, not `Result`) for exactly this reason.

Verified two ways, both against *real* audio, not mocks: `audio.rs`'s own tests build a real, valid minimal WAV file by hand (a 44-byte RIFF/WAVE header + real PCM16 samples — no encoder dependency needed for something this simple) and confirm `AudioPlayer::play` actually reaches `is_playing() == true`, `pause()`/`resume()` actually toggle it, and undecodable bytes correctly no-op rather than panicking. `document.rs`'s new `audio_resource_tests` module drives the *resource-application* half end-to-end the same `document.load_resource(fabricated_response)` way `animated_image_tests`/`font_face_override_tests` already do — confirming `autoplay` does trigger real playback and its absence doesn't. Every device-dependent assertion is skipped gracefully (not failed) when no real output device is available, the same pattern the existing font-metrics tests already use for "no usable system font" — in this environment specifically, a real device *was* available, so every one of those assertions ran for real, not just the skip path. Not independently re-verified end-to-end through actual HTML parsing (`<audio autoplay src>` → html5ever → `mutator.rs`'s tag-detection → fetch → decode → play, as one continuous path) — the two halves (tag-detection-and-fetch-dispatch, and resource-application-and-playback) are each thoroughly tested separately, and the tag-detection wiring itself is the same one-line-match-arm pattern `<img>`'s already-proven code follows, but a real full-page live confirmation is still worth doing.

Verified: `cargo test -p blitz-dom --features audio` — 78 passing, up from 72 (+6: 4 in `audio.rs`, 2 in `document.rs`'s `audio_resource_tests`); `cargo test -p blitz-dom` (the `audio` feature off, the default) still 72, confirming the feature gate actually keeps this code out of non-audio builds rather than just hiding it behind a flag that doesn't do anything. Full outer desktop build (`js_engine`, now also flipping on `blitz-dom/audio`) and its 20-test suite both pass unchanged.

### A real cache-poisoning bug, found re-running the full suite for the Phase 3 wrap-up

Not related to audio or animated images — surfaced by re-running the *entire* outer workspace suite (`cargo test --features full`) repeatedly while closing out the phase, the same discipline that already caught the `net_cache` race during Phase 2. `server::tests::test_agent_endpoint_navigate_query_get_text_full_flow` started failing intermittently — asserting a page title of `"Test Page"` and getting `"Untitled"` instead — but only at specific parallel test-thread counts (deterministic 6/6 at `--test-threads=8`, never at the default thread count or `--test-threads=16`, which made it easy to dismiss as unrelated flakiness at first rather than actually chase down).

Root cause, once reproduced deterministically: `Navigator`'s new HTTP cache (added earlier in Phase 2, for the `/app` web-shell path) uses one real, shared, on-disk cache directory for its whole process lifetime. Tests spin up many rapid, short-lived local `mockito` HTTP servers on OS-assigned ephemeral ports — and under enough parallelism, the OS reuses a recently-freed port for a *new*, completely unrelated mock server before the cache's heuristic-freshness window (the fallback behavior `http-cache-reqwest`/`http-cache` use for a response with no explicit `Cache-Control` header, which none of these test mocks set) had expired. The cache correctly saw "same URL, still fresh" and served back an earlier, unrelated test's cached response — the earlier test's mock never set a `<title>`, hence `"Untitled"`. This is a real characteristic of HTTP caching in general (a real browser would do the same thing revisiting a router login page at a reused local IP within a cache's freshness window), not a logic bug in the caching code itself — but it made the *test suite* unsafe to share one persistent cache directory across independently-constructed `Navigator`s.

Fixed at the source: `net_cache::http_cache_dir()` now returns a fresh, unique directory per call when compiled for tests (`#[cfg(test)]` — real, non-test builds are unaffected, still the one real persistent path), via a process-local atomic counter. Each `Navigator`/cached client still only calls this once at construction time, so within a single test's own multiple requests through the same client (e.g. `test_navigate_does_not_refetch_a_cacheable_url`'s whole point), caching still works exactly as before — only *cross*-test sharing is removed, which is what was actually unsafe.

Verified: the specific failing test passed 8/8 consecutive runs at the exact `--test-threads=8` setting that reproduced it deterministically before the fix (previously 6/6 failures), plus 3 more clean runs each at `--test-threads=16` and the default thread count. Full outer suite (`cargo test --features full`) run 6 additional times after the fix with zero failures at any thread count tried.

---

## What's still unsolved (the real remaining scope)

The v0 DOM binding surface (Phase 3, done) covers `document.getElementById`, `element.textContent` (get/set), `element.style.setProperty`, `element.classList.{add,remove,toggle,contains}`, and `element.addEventListener` — verified live against a real page exercising all of them, plus real-world confirmation that link clicks, typing, and form submission (GET) all work together on wikipedia.org. What's still missing, in likely order of what a real site hits first:

- `document.querySelector`/`querySelectorAll` — most real sites use these far more than `getElementById`
- `element.innerHTML` (read/write) — `textContent` alone can't build markup
- `document.createElement` / `appendChild` / `removeChild` — needed for any page that builds UI dynamically (most real ones do)
- Basic `window` globals: `setTimeout`, `console.log`
- Live document reads instead of the `*_MIRROR` thread_locals (see Phase 3's design-constraint note above) — needed once a real site's first `classList`/`style` touch demonstrably clobbers pre-existing HTML-authored state

Each of these is individually small; the discipline is in keeping the surface minimal and adding to it only when a real site's failure demonstrably needs it (confirmed live already: wikipedia.org needs more before it's fully usable; google.com's search explicitly detects the gap and shows "Turn on JavaScript to search" — accurate, since real DOM construction and likely `fetch`/XHR are well beyond this surface) rather than trying to spec-complete against DOM/HTML standards upfront.

**Explicitly out of scope, don't attempt**: `fetch`/`XMLHttpRequest` wiring, Web Components, Shadow DOM, Service Workers, WebAssembly interop, `<canvas>` 2D/WebGL contexts. These are each their own large projects; a minimal reader/interactive-forms browser doesn't need them.

## Shell visual design

The RSX shell in `src/bin/desktop.rs` now uses the same design language as `src/ui/web/style.css` (glacier/slate/alpine palette, 10px tab radius, pill-shaped address bar) — duplicated as literal hex values in inline `style` strings rather than shared, since dioxus-native renders inline styles per-element with no linked stylesheet mechanism connecting it to the web shell's CSS. If the two ever need to stay in sync deliberately (not just coincidentally matching), that duplication is worth revisiting.

---

## Security note

None of this spike sandboxes JS execution. A `ScriptedDocument` running arbitrary page JS with direct native access to `BaseDocument::mutate()` (and, once built, DOM APIs) is running untrusted code with host privileges. Before this touches any page a real user navigates to (as opposed to a controlled test page), it needs at minimum: a memory/CPU budget per Boa `Context` (Boa supports execution limits), no filesystem/process access reachable from registered native functions, and a clear boundary around what native functions are ever exposed to script. This matters more here than in a typical embedding use case, given Himalayas' existing security-first positioning (17 policies, agent sandboxing, permission expiry) — a JS engine that can reach outside its sandbox would undercut that story specifically.

---

## Device-tier gating

Matches the pattern already in this repo (`desktop_ui` Cargo feature; `Daemon::ui_enabled` from `DeviceTier`; `IsolationMode` default from `DeviceTier` in `src/browser/mod.rs`):

- New Cargo feature, e.g. `js_engine` (off by default, only meaningful when `desktop_ui` is also on) — excludes Boa and the DOM binding layer from the binary entirely on builds that don't want them.
- Runtime default mirrors `tier_supports_desktop_features()` in `src/daemon/mod.rs`: JS on by default at Standard tier and above, off by default below, always overridable via an explicit flag (same `--ui`/`--no-ui` shape).
- On mid-tier devices where the engine is present but resources are tight, cap Boa's execution budget (time/memory) rather than disabling outright — Boa has hooks for this.

---

## Phased plan

1. ✅ **Decide the shell question** — Option A, done. `vendor/blitz/` carries both patches described above.
2. ✅ **Vendor + patch, integrated and verified live**: `vendor/blitz/` (trimmed Blitz workspace) + `src/bin/desktop.rs` (`himalayas-desktop`, feature `js_engine`). Tabs, address bar, `web-view` scripted content, and the event-forwarding patch all confirmed working together against a real click, not just in isolation. `cargo test` (375 tests) unaffected — feature is fully opt-in.
3. ✅ **v0 DOM binding surface, built and verified live**: `document.getElementById(id)` returns a real JS object with `textContent` (get/set accessor), `style.setProperty(name, value)`, `classList.{add,remove,toggle,contains}`, and `addEventListener('click', fn)` — a real per-`(id, type)` listener table alongside the original `onclick`-attribute path, not a replacement for it. Confirmed on `himalayas://test` (updated to exercise the new surface): both an `onclick`-attribute button and an `addEventListener` button correctly mutate text, inline style, and classes.

   **Design constraint worth knowing for anyone extending this**: Boa native closures can't safely hold a live `&BaseDocument` reference (it doesn't outlive the closure, and there's no sound way to smuggle a raw pointer across the JS/Rust boundary here). So every binding is one of two shapes — *writes* queue a `DomOp` (`SetText`/`SetAttribute`), applied by `ScriptedDocument::handle_ui_event` after `EventDriver` finishes with a live `&mut BaseDocument` in hand; *reads* (`textContent` getter, `classList.contains`) answer from a `thread_local` mirror (`TEXT_MIRROR`/`CLASS_MIRROR`/`STYLE_MIRROR`) tracking only what our *own* JS has written, not the document's original server-rendered state. That means `el.textContent` returns `""` until something writes it first, and `classList.add('x')` on an element that already had `class="y"` in the HTML silently drops `y` on first touch (the mirror doesn't know about it yet) — real, stated v0 limitations, not silent bugs. Fixing them means seeding each mirror from the live document the first time an id is looked up, which is exactly the kind of thing to add once a real site's breakage demonstrates it's needed (see item 8).
4. ✅ **Script discovery from the parsed DOM**, not string search: `extract_inline_scripts` in `src/bin/desktop.rs` now walks the real tree via `Node::children` (root-first stack traversal), collecting `<script>` elements without a `src` attribute in document order and reading their content via `node.text_content()` — the same text-flattening helper `find_title_node` already used. Fixes the v0 string-search approach's real failure mode: a literal `</script>` inside a JS string or comment used to truncate the script early; the new version can't hit that at all, since it's walking parsed elements, not searching raw text. `TreeTraverser` (blitz-dom's own pre-order iterator) turned out to be private to that crate, so this is a small hand-rolled equivalent in `desktop.rs` rather than a further vendor patch — external `<script src=...>` is still explicitly unhandled, same as before.
5. ✅ **Security pass**: `new_js_context` now sets a bounded `RuntimeLimits::loop_iteration_limit` (10,000,000) on every fresh Boa `Context` — previously unbounded (`u64::MAX`, Boa's own default), meaning a page with an unintentional or hostile `while(true){}` would hang the whole native window forever, since script runs synchronously inside `EventDriver::handle_ui_event` with nothing else able to run until `eval()` returns. Now it fails with a catchable JS error instead. The "explicit allowlist of native functions" and "no ambient filesystem/network access" parts of this item were already true by construction (`new_js_context` registers exactly three globals — `document`, `console`, `setTimeout` — nothing resembling `fetch`/`require`/file I/O), just not previously called out as a deliberate security property; now documented as one directly on `new_js_context`.
6. ✅ **Device-tier gating**: `himalayas-desktop`'s `main()` now calls `DeviceCapabilities::detect()` (cached via `detected_device_tier()`) and refuses to launch below Standard tier — same threshold as `tier_supports_desktop_features` in `src/daemon/mod.rs` — printing an explanation and exiting, unless `--force` is passed. Matches the existing `--ui`/`--no-ui` precedent's shape (explicit override always available) rather than a silent slow launch on constrained hardware.
7. ✅ **Wired to the real backend**: `src/bin/desktop.rs` now owns a real `Browser` + `TabManager` (`browser()`/`tab_manager()`, lazily-initialized statics), and `Tab::new` calls `Browser::open_tab` with an `IsolationMode` chosen by `default_isolation_mode()` (`Isolated` at Standard tier and above — the same threshold as item 6 — `Shared` otherwise). Closing a tab calls `Browser::close_tab`, which cleans up an isolated tab's session if nothing else references it (exactly the behavior already covered by `src/browser/mod.rs`'s own test suite). This is more than bookkeeping: the top-level document fetch was switched from `blitz_net::Provider` (which only returns `(resolved_url, bytes)`, no headers) to `reqwest` directly, attaching the tab's `Session` cookies as a `Cookie` request header and writing any `Set-Cookie` response headers back into that `Session` — so `IsolationMode::Isolated` vs. `Shared` is now an observable difference in the native shell (one tab's cookies are or aren't visible to another), not just internal state nothing reads. `blitz_net::Provider` is still used for sub-resource fetches (images/CSS/fonts) inside `DocumentConfig`, which don't need cookie handling. Covered by new unit tests in `src/bin/desktop.rs` (`new_tabs_get_distinct_isolated_sessions_by_default`, cookie-header parsing).
8. ✅ **Expanded the binding surface**, driven by the same "real site would hit this" reasoning as before, plus one bug the mirror design was quietly carrying:
   - **Fixed a real data-loss bug in `classList`/`style`**: `CLASS_MIRROR`/`STYLE_MIRROR` started empty per element and were serialized wholesale back onto the `class`/`style` attribute on every write — so `el.classList.add('open')` on an element with `class="existing"` in the HTML would silently overwrite it to `class="open"` on the very first touch, discarding `existing`; same failure for `style.setProperty` against an inline `style="..."` attribute. Fixed by seeding both mirrors from the element's live attribute (`class=` split on whitespace; `style=` parsed via a small `parse_style_attr` — `;`-then-`:` splitting, not a full CSS parser, which is adequate for inline declarations) the first time either is touched, not just on read.
   - **`textContent` and `classList.contains` now fall back to a live document read** when nothing's been JS-written yet, instead of always returning empty/false — a page's own server-rendered text or HTML-authored classes now read back as themselves the first time a script inspects them, not as if the element were blank.
   - **Added `document.createElement(tag)` / `document.createTextNode(text)`**, backed by `DocumentMutator::create_element`/`create_text_node`. Unlike every other JS-facing write in this file, this can't be queued onto the deferred `MUTATIONS` list — the calling script needs a real, usable `NodeId` back in the same tick (`var el = document.createElement(...)` immediately followed by using `el`) — so it goes through a new `with_live_doc_mut` (a mutable-borrow sibling of the existing `with_live_doc`, same soundness argument: exclusive access for the duration of one synchronous call).
   - **Added `element.appendChild(child)` / `removeChild(child)`** (queued `DomOp::AppendChild`/`RemoveChild`, applied via `DocumentMutator::append_children`/`remove_node`) and **`element.innerHTML`** (setter queues `DomOp::SetInnerHtml`, applied via `DocumentMutator::set_inner_html` — which, conveniently, already existed and reuses the same `HtmlParserProvider` the page itself was parsed with; getter is mirror-only, same limitation as the other properties, since there's no live HTML serializer to read a subtree back out).
   - **Fixed `SetText`'s apply logic to match real `textContent =` semantics**: the old version only patched an existing first-child text node in place, silently no-opping on anything else (a freshly `createElement`-d node with zero children, or a node whose first child is an element, not text) — exactly the case `createElement` + `appendChild` + `textContent` immediately exposed. Now replaces *all* of the target's children with one fresh text node, matching what a real DOM does.
   - Verified: `himalayas://test` extended with sections exercising all of the above (a `classList`/`style`-seeding check against an element with pre-existing HTML class/style, and a `createElement`/`appendChild`/`removeChild`/`innerHTML` list-building section); full `cargo test --features js_engine` suite (227 lib + 144 `himalayas` bin + 5 `himalayas-desktop` bin + 4 integration = 380, up from 375 — the 5 new tests are in `src/bin/desktop.rs`, covering `Set-Cookie` parsing, the device-tier gate, and end-to-end isolated-session cookie separation) passes.
   - Still out of scope, same reasoning as before: `document.querySelector`/`querySelectorAll` were actually already done in Phase 3 (this list in the plan was stale); real remaining gaps are `fetch`/`XMLHttpRequest`, dynamic `<script src>` loading, and anything needing a live HTML serializer (a real `innerHTML` getter, `outerHTML`).

### Known separate gaps

- **Raster image decoding: fixed and confirmed.** Root cause: `vendor/blitz/Cargo.toml`'s workspace-level `image` dependency had `default-features = false` with no format features re-added anywhere in the dependency graph, so `image::ImageReader::decode()` (called from `blitz-dom/src/net.rs`) had zero codecs compiled in — every raster image on every page failed to decode, universally. Fixed: `features = ["png", "jpeg", "gif", "webp", "bmp", "avif", "ico"]` on that one line. Confirmed with instrumentation: zero decode failures logged across two different real sites (wikipedia.org, yahoo.com) after the fix, versus consistent `decode FAILED: The image format could not be determined` before it.

- **SVG support: enabled, but exposed a separate, deeper upstream bug — not fixed.** `blitz-dom`'s own default features (`svg`, `woff`, `accessibility`, `system-fonts`, `file-input`, `custom-widget`) were fully disabled by an overly-aggressive `default-features = false` on the `blitz-dom` line in the main `Cargo.toml` (a footprint-minimization choice made without checking what it actually cut) — no SVGs rendered at all, which on real sites (logos, icons) looks like "most images missing" even with the raster fix in place. Restored to blitz-dom's own defaults (removed the override). Doing so surfaced a new problem: real SVGs frequently fail to *paint* correctly, even though the raster fix above is unrelated and unaffected. Diagnosed, not fixed:
  - Confirmed via instrumentation across two different real sites (wikipedia.org, yahoo.com): **zero image decode failures**, but 1000+ occurrences of `vello_common::flatten: A path contains NaN, ignoring it` per page, growing consistently as more pages are visited — a paint-time bug, not a decode/feature-flag gap.
  - One correlated warning points at a likely cause: `usvg::parser::svgtree: Failed to parse clip-path value: 'url(#ArrowUp-outline-16_svg__a)'`. The `svg` feature's dependency chain requires the same unreleased `usvg` fork (`DioxusLabs/resvg`, branch `devin/1785858271-intrinsic-dimensions`) that's already flagged as a fragility risk elsewhere in this document — its `intrinsic_dimensions()` addition (used specifically to size SVGs during layout) is the prime suspect for producing invalid/NaN geometry on real-world SVGs it wasn't fully tested against.
  - **Also required a second copy of the `[patch.crates-io]` section**, this time in the *outer* `Cargo.toml` (not just `vendor/blitz/Cargo.toml`): Cargo's `[patch]` only applies to the workspace whose manifest declares it, and the outer Himalayas package (which pulls `vendor/blitz/packages/blitz-dom` in as a path dependency) is a separate workspace root from `vendor/blitz` itself. `custom-widget` alone never needed this (doesn't touch `usvg`); `svg` does. Anything else in `vendor/blitz` that starts pulling `usvg`/`anyrender_svg` transitively for the first time will hit the same thing — the fix is copying the same three `[patch.crates-io]` lines to wherever the new build root is.
  - Same category as the apple.com blank-render bug: a genuine defect in an unreleased third-party fork, not introduced by this integration, and not something to chase further without directly debugging that fork's `intrinsic_dimensions()` implementation — disproportionate scope for this pass. Left `svg` enabled (net improvement — raster images plus *some* working SVG geometry beats no SVG at all) rather than reverting.

- **apple.com: diagnosed, not fixed — pre-existing upstream Blitz bug, not something this integration introduced.** The page fetches fully (verified via a standalone `blitz-net` test: 254KB, correct redirect to `www.apple.com`) and `load_page` completes successfully end-to-end (confirmed via logging: fetch → parse → `HtmlDocument` construction all return `Ok`). The server-rendered HTML has substantial real text content (~6.6KB of visible text, not a JS-shell page like msn.com). Yet nothing paints — a blank white screen. **Isolated the cause to Blitz itself, not Himalayas' patches**: a minimal, completely unmodified `blitz::launch_url("https://www.apple.com")` test — zero custom code, no `ScriptedDocument`, no event-forwarding patch, not even a shell window — reproduces the identical blank screen. Root-causing further would mean debugging Stylo's layout engine or the Vello paint pipeline directly against apple.com's specific CSS, which is a much larger, separate investigation from anything in this repo. Logged as a known limitation of this Blitz commit; not blocking anything else in this plan.
- **Link navigation: fixed.** `ScriptEventHandler::handle_event` now walks the click's ancestor `chain` (`find_href`) for the nearest `href` (handles clicking on inline content inside an `<a>`, not just the tag itself), skips placeholder hrefs (`""`, `"#"`, `javascript:`), resolves the target against the document's own `url()`, and queues it. Verified live on wikipedia.org: clicking "English" loads English Wikipedia in the same tab.

  **This surfaced a genuine Dioxus 0.7 API gap, not a Himalayas bug**, worth documenting precisely since it'll recur for any future work that needs to react to sub-document events: `ScriptEventHandler` runs from deep inside the sub-document's own event dispatch (via the event-forwarding patch), entirely outside RSX/the component tree. A first attempt called `Signal::set`/`spawn` directly from there and panicked — `"Must be called from inside a Dioxus runtime."` Capturing `dioxus_core::Runtime::current()` and reinstalling it via `RuntimeGuard` fixed *that* panic, but exposed a second one one layer down: `Runtime::current_scope_id()` unwraps an empty scope stack. Dioxus 0.7 tracks two separate thread-locals — which `Runtime` is active (public: `Runtime::current`/`RuntimeGuard`) and which *scope* is active within it (`push_scope`/`pop_scope`/`with_current_scope`, all `pub(crate)`, not reachable from outside `dioxus-core`). There's no public way to spawn a task or write a signal from a bare native callback.

  **Fix**: don't call into Dioxus from the callback at all. `ScriptEventHandler` only ever touches a plain `thread_local RefCell<Option<String>>` (`PENDING_NAVIGATION`) — no Dioxus API surface, so no scope requirement. `app()` drains it via a `use_future` polling loop (50ms interval), which *is* a legitimately Dioxus-spawned task with a valid scope, so the `Signal::set`/`spawn` calls inside `navigate_tab` work correctly there. Same pattern to reach for any future need to act on a sub-document event from `app()`'s side (e.g. updating tab title/favicon from in-page JS) — a plain thread_local queue drained by a poll loop, not a direct callback into Dioxus internals.

## Next phases (current queue, rebuilt)

The "Phased plan" above is the original spike-to-integration plan and is fully complete (all 8 steps checked off) — kept as history, not touched further. Everything below is what's actually left outstanding across this whole session's work: shipped-but-extendable features, explicitly-deferred larger asks, and vision docs that were reacted to but never scoped into work. Ordered so each phase either finishes something already load-bearing or lays groundwork the next phase needs — not by how the requests originally arrived.

### Phase 1 — Finish what's already half-built

Small, well-understood extensions of systems that already exist and work; no new subsystem, no new dependency risk.

1. **Live visual QA pass on everything verified only by unit test/build so far**: viewport scrollbar drag feel, the 2x chrome sizing, bookmark star/manager click-through (add → edit → import/export a real file), pin/unpin + drag into/out of the pinned zone, `loading="lazy"` against a real long page. All of these have passing automated tests but no human-eyes confirmation in the running app.
2. **Pin tab persistence**: `Settings → Startup → Restore pinned tabs`, the one piece of the original pinned-tab ask explicitly deferred (`docs/NATIVE_RENDERING_PLAN.md`'s "Pin a tab" section). Needs a real on-disk session file (nothing in `desktop.rs` persists anything today — tabs/bookmarks/folders are all in-memory, cleared on restart) — the first feature to actually need that, so scope it as "add minimal session persistence," not just "remember pinned tabs."
3. **`<picture>`/`<source>` responsive images**: confirmed unhandled entirely (see the "Real `<img loading="lazy">` support" section's audit). Reuses the existing `select_srcset_candidate`/`eval_sizes` from the srcset/sizes work — pick a `<source>` by `media`/`type` before falling back to the inner `<img>`.
4. **Bookmark Manager: drag-and-drop + multi-select**, the two pieces explicitly scoped out of the first pass. Drag-and-drop reordering/move-between-folders can likely reuse the tab strip's mousedown-arm/mouseenter-swap pattern rather than needing real HTML5 drag events (which don't fire in this shell). Multi-select needs a `selected: HashSet<String>` (keyed by URL) plus bulk move/delete actions in the manager toolbar.

### Phase 2 — Wire existing capability outward, round out caching

Nothing here is a new rendering subsystem — it's exposing/completing things that already exist underneath.

5. **MCP server** (Model Context Protocol), the earliest deferral in this session. Most of the real capability already exists — `AgentContext`'s navigate/query/click/input/get_text/submit_form plus the `/agent` HTTP endpoint (`src/api/mod.rs`) — an MCP server is mostly a protocol adapter over that, not new browser capability. Scope: stdio or HTTP MCP transport, tool definitions mapping 1:1 to the existing `AgentContext` methods.
6. **Caching, the rest of it**: top-level document fetch (`desktop.rs`'s `http_client()`) and the `/app` `Navigator` path are still fully uncached (see the HTTP-cache section's "deliberately not touched" note) — need a real cache-header-aware path now that `Provider::fetch_async`'s header-blindness is the actual blocker, not a missing feature flag. Beyond that: a bounded in-memory cache tier in front of the disk cache (real browsers check memory before disk), a real eviction policy instead of relying entirely on `cacache`'s own defaults, and per-origin cache partitioning (matches the isolation-by-`Session` precedent `IsolationMode` already established for cookies).
7. **UI automation for Claude** (`osascript`/System Events), blocked and unresolved — my last working theory (unconfirmed) was that `osascript`/`/usr/bin/osascript` needs its own separate Accessibility entry distinct from Terminal's. This one needs the user's own machine access to retest, not something to make further progress on unattended.

### Phase 3 — Real media, starting with the simpler of the two subsystems

Both "Video Compatibility" and "Audio Compatibility" are genuinely build-from-zero (confirmed: `<video>` gets CSS box-model treatment only, `<audio>` isn't referenced anywhere in blitz-dom/blitz-paint at all, no `SpecialElementData::Video`/`Audio` variant exists). Audio first — smaller surface (no frame decode/paint pipeline, "just" a decode-and-output-samples problem), and animated images share real groundwork with it (both need a first "this element has a decode timeline, not a single static frame" primitive that doesn't exist in the engine yet).

8. ✅ **Animated images** (GIF today; WebP/APNG a real, scoped widening for later): done — see the "Animated images: real GIF playback" section above.
9. ✅ **`<audio>` decode + playback**: done — see the "Real `<audio>` decode + playback" section above. Built: real decode/playback triggered by `autoplay`. Deliberately not built: seek/`currentTime`, `volume`/`muted`, a visual `controls` widget, JS `.play()`/`.pause()` bindings, Web Audio API, WebRTC audio, Media Session API — each a separate, real follow-up.
10. **`<video>` + basic HTML5 playback** — not started, and deliberately not picked up automatically the way audio was. Real video decode (H.264 for broadest coverage) has no comparable pure/mostly-pure-Rust option the way audio did (`rodio`/`symphonia`): the realistic choices are FFI bindings to a real system codec library (e.g. `ffmpeg-next`, which needs `libav`/FFmpeg present on the machine) or an incomplete/experimental pure-Rust decoder. Either one is a real, consequential architecture decision — FFI bindings materially change this project's "lean binary, minimal dependencies" positioning (the same positioning that made `rodio` a comfortable, in-scope choice for audio) — worth the user's explicit input rather than a unilateral pick. `poster` attribute support (a static preview image, no decode pipeline needed — reuses the existing image-decode path directly) is real, low-risk, and buildable independently of that decision whenever it's worth doing.

### Phase 4 — Speculative / revisit later

Vision docs that are either genuinely dependent on Phase 3 landing first, or broad enough that they need their own scoping pass once there's more to adapt.

11. **Adaptive Media Optimization Engine** (device/network/battery-aware quality selection for video/images, beyond the srcset/sizes work already done) — the video/audio half of this doesn't have anything to adapt yet without Phase 3.
12. **Display-configuration modes** ("Normal Laptop Mode" / "External Monitor Mode") — never scoped past the original vision-doc reaction.
13. **HDR/color management, Canvas/WebGL/WebGPU, AI/computer-vision image pipeline integration, image security hardening** (decompression bombs, decoder sandboxing) — each real, each individually large; revisit once Phase 3's media pipeline exists, since several of these (WebGPU texture upload, AI/CV pipeline efficiency) are specifically about *not* copying decoded video/image frames unnecessarily, which only means something once there's a frame pipeline to avoid copying in.
14. **Context menu expansion** — "right-click is working with limited options" was noted once, live, with no specific follow-up ask since. Revisit if it comes up again with something concrete.

---

## Risks & open questions

- **Vendored, not forked**: `vendor/blitz/` is a trimmed copy of Blitz's source committed directly into this repo, not a git fork with a remote — avoids needing to push to/maintain an external GitHub fork, but means upstream improvements have to be manually re-applied by re-vendoring rather than `git pull`. Given the two patches are small and localized (one file each), re-vendoring against a newer Blitz commit and reapplying them by hand is expected to be low-effort when needed.
- **Fork fragility**: the `usvg`/`anyrender` patches (referenced from `vendor/blitz/Cargo.toml`, not vendored themselves) point at commits (pinned by SHA, not branch, as of this writing) on named branches (`devin/...`) in forks of external maintainers' repos, not tagged releases. Those branches could be rebased or deleted without notice — the commit pin protects against that, but if the fork's remote disappears entirely, those two dependencies would need re-hosting or re-vendoring too.
- **Boa is self-described as experimental**, ~90% ECMAScript spec coverage, generally slower than QuickJS/V8. Real-world compatibility against the kind of sites Himalayas users actually visit is unknown at scale — the v0 binding surface phase above will surface this quickly.
- **Blitz itself is pre-1.0** (`0.3.0-beta.1`) with an actively moving API — the exact method signatures documented here (confirmed working against commit `990a90bfa1f8dc7034a601922339b027142a3bdc`) will drift.
- **Effort**: even the v0 binding surface (step 3 above) is a multi-week effort for one engineer; a genuinely useful subset of the interactive web is a multi-month initiative. Treat this plan as sequenceable, not a one-sprint task.
