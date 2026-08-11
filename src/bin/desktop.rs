//! Native rendering spike (himalayas-desktop): a real GPU-rendered window,
//! not the /app web shell. Combines the tab-strip shell (dioxus-native RSX)
//! with a scripted content pane (Boa JS engine over blitz-dom) via the
//! `SubDocumentAttr::from_document` patch in vendor/blitz — see
//! docs/NATIVE_RENDERING_PLAN.md for the full writeup.
//!
//! Known v0 limitation (tracked as a Phase 3/4 follow-up, not a bug to chase
//! now): all open tabs currently share one global JS scope, so two tabs
//! defining a same-named function collide. Real isolation needs a JS context
//! (or Realm) per tab, keyed by tab id, once the DOM binding surface exists.

use blitz_dom::{
    BaseDocument, DocGuard, DocGuardMut, Document, DocumentConfig, EventDriver, EventHandler,
    FontContext, LocalName, Namespace, QualName,
};
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_net::Provider as NetProvider;
use blitz_traits::events::{DomEvent, DomEventData, EventState, UiEvent};
use blitz_traits::navigation::{NavigationOptions, NavigationProvider};
use blitz_traits::node_id::NodeId;
use blitz_traits::shell::ShellProvider;
use boa_engine::object::ObjectInitializer;
use boa_engine::object::builtins::{JsArray, JsFunction};
use boa_engine::property::Attribute as JsPropAttribute;
use boa_engine::{Context as JsContext, JsValue, NativeFunction, Source, js_string};
use dioxus_native::SubDocumentAttr;
use dioxus_native::prelude::*;
use himalayas::browser::tabs::{IsolationMode, TabManager};
use himalayas::browser::{Browser, Session};
use himalayas::intelligence::device_detection::{DeviceCapabilities, DeviceTier};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

/// A pending write to the live DOM, queued by a JS-side DOM binding call
/// (`element.textContent = ...`, `classList.add(...)`, etc.) and applied by
/// `ScriptedDocument::handle_ui_event` once the triggering event has finished
/// running through `EventDriver` — see that impl for why these can't be
/// applied immediately (no live `&mut BaseDocument` reachable from inside a
/// Boa native closure).
enum DomOp {
    SetText(NodeId, String),
    SetAttribute(NodeId, QualName, String),
    /// `element.innerHTML = html` — reparses `html` via the same
    /// `HtmlParserProvider` the page itself was parsed with (already wired
    /// into `DocumentConfig` in `build_scripted_document`), replacing all of
    /// the target's existing children.
    SetInnerHtml(NodeId, String),
    /// `parent.appendChild(child)` — `child` must already exist (created via
    /// `document.createElement`/`createTextNode`, or moved from elsewhere in
    /// the tree); this only reparents it.
    AppendChild(NodeId, NodeId),
    /// `parent.removeChild(child)` — detaches `child` without dropping it
    /// (matches real DOM `removeChild` semantics: the node still exists and
    /// could be re-appended), via `DocumentMutator::remove_node`.
    RemoveChild(NodeId),
}

thread_local! {
    static JS: RefCell<JsContext> = RefCell::new(new_js_context());
    static MUTATIONS: RefCell<Vec<DomOp>> = RefCell::new(Vec::new());

    /// Last-known `textContent` per element, as set by our own JS calls. v0
    /// limitation: doesn't reflect the page's original server-rendered text
    /// until JS writes it at least once — see `with_live_doc`'s doc comment
    /// for why reads generally have to be mirror-only. Reset per navigation.
    static TEXT_MIRROR: RefCell<HashMap<NodeId, String>> = RefCell::new(HashMap::new());
    /// Same limitation as `TEXT_MIRROR`, for `classList`: only reflects
    /// classes added/removed via JS, not the element's original `class=`
    /// attribute. Reset per navigation.
    static CLASS_MIRROR: RefCell<HashMap<NodeId, HashSet<String>>> = RefCell::new(HashMap::new());
    /// Same limitation, for `style.setProperty`: only reflects properties set
    /// via JS, not the element's original `style=` attribute. Order-preserving
    /// (`Vec`, not a map) so repeated serialization is stable. Reset per
    /// navigation.
    static STYLE_MIRROR: RefCell<HashMap<NodeId, Vec<(String, String)>>> = RefCell::new(HashMap::new());
    /// Last-known `innerHTML` per element, as set by our own JS calls — same
    /// write-tracking-only limitation as the other mirrors (no live
    /// serializer to turn the current DOM subtree back into an HTML string
    /// for the getter). Reset per navigation.
    static INNER_HTML_MIRROR: RefCell<HashMap<NodeId, String>> = RefCell::new(HashMap::new());
    /// `addEventListener` registrations: (element, event type) -> callbacks,
    /// checked by `ScriptEventHandler` alongside the legacy `onclick`
    /// attribute path. Keyed by `NodeId` directly (not an `id` attribute),
    /// so this works on elements found via `querySelector` that have no
    /// `id=` at all. Reset per navigation (the `JsFunction`s belong to
    /// whichever `JsContext` was current when registered).
    static LISTENERS: RefCell<HashMap<(NodeId, String), Vec<JsFunction>>> = RefCell::new(HashMap::new());

    /// `requestAnimationFrame` callbacks queued for the *next* frame, keyed
    /// by the handle `cancelAnimationFrame` removes them by. Drained once
    /// per frame by `ScriptedDocument::poll` — see that impl for how this
    /// connects to the actual redraw loop (via the vendor/blitz patch in
    /// `resolve.rs`'s sub-document loop). Reset per navigation.
    static RAF_CALLBACKS: RefCell<Vec<(u64, JsFunction)>> = RefCell::new(Vec::new());
    /// Monotonically increasing `requestAnimationFrame` handle counter.
    /// Reset per navigation (handles from a previous page are meaningless
    /// after `RAF_CALLBACKS` itself has been cleared).
    static RAF_NEXT_ID: Cell<u64> = Cell::new(1);
    /// Set once per navigation (`reset_js_bindings`), read by
    /// `ScriptedDocument::poll` to compute each callback's
    /// `DOMHighResTimeStamp` argument — milliseconds since this page started
    /// loading, matching the real API's "time origin" semantics closely
    /// enough for v0 (not adjusted for navigation-timing subtleties like
    /// redirects).
    static PAGE_TIME_ORIGIN: Cell<Option<std::time::Instant>> = Cell::new(None);

    /// Raw pointer to the live `BaseDocument`, valid only for the duration of
    /// a single synchronous `eval()`/callback invocation inside
    /// `ScriptEventHandler::handle_event` — see that impl for the safety
    /// argument. Lets `document.getElementById`/`querySelector` resolve
    /// elements for real instead of just wrapping a caller-supplied id
    /// string, which is what makes `querySelector` results usable at all
    /// (most matched elements have no `id=` attribute to defer resolution
    /// on, unlike the original `getElementById`-only design).
    static CURRENT_DOC: Cell<Option<*mut BaseDocument>> = Cell::new(None);

    /// Set by `ScriptEventHandler` when a link click is detected (see
    /// `find_href`), drained by a polling loop in `app()`.
    ///
    /// `ScriptEventHandler` is invoked from deep inside the sub-document's
    /// own event dispatch (via the event-forwarding patch in vendor/blitz),
    /// not from RSX — there's no Dioxus runtime *scope* on that call stack
    /// (only `dioxus_core::Runtime::current()` succeeds there; the
    /// scope-stack push/pop that `Signal::set`/`spawn` also need is a
    /// `pub(crate)` API in dioxus-core 0.7, not reachable from outside the
    /// crate). A plain thread_local + polling from a real `use_future` (which
    /// *does* run with a valid scope) sidesteps that entirely. Only the
    /// active tab's sub-document ever receives forwarded events, so there's
    /// no ambiguity about which tab a pending navigation belongs to.
    static PENDING_NAVIGATION: RefCell<Option<String>> = RefCell::new(None);

    /// Set by `ScriptEventHandler` on a right-click (`DomEventData::ContextMenu`
    /// — upstream blitz-dom already synthesizes this from a secondary-button
    /// pointer event, see `pointer.rs`; it just has `// TODO: Open context
    /// menu` for a default action, i.e. nothing happens with it upstream).
    /// Drained by the same polling loop as `PENDING_NAVIGATION`, for the same
    /// reason (no Dioxus runtime scope this deep in sub-document dispatch).
    static PENDING_CONTEXT_MENU: RefCell<Option<ContextMenuRequest>> = RefCell::new(None);
}

/// A right-click captured inside a sub-document (loaded page), queued for
/// `app()` to render a menu for. `x`/`y` are the sub-document-*local*
/// coordinates the event carried (post `shift_pointer_event` translation) —
/// `app()` adds a fixed shell-height offset to approximate outer-window
/// position rather than threading the exact `<web-view>` bounding rect all
/// the way out here, since the shell's own layout is already a small,
/// known set of fixed-height rows (tab strip/address bar/status line) — see
/// where this is consumed in `app()`.
#[derive(Clone)]
struct ContextMenuRequest {
    x: f32,
    y: f32,
    href: Option<String>,
}

/// Right-click menu anchored to a specific tab in the strip — see
/// `tab_context_menu` in `app()`.
#[derive(Clone, Copy, PartialEq)]
struct TabContextMenuRequest {
    tab_id: u32,
    x: f64,
    y: f64,
}

/// Read from the live document, if one is currently available (see
/// `CURRENT_DOC`'s doc comment for exactly when that is and why it's sound).
/// `None` outside of JS execution triggered from `ScriptEventHandler` — there
/// is currently no other place script runs from.
fn with_live_doc<R>(f: impl FnOnce(&BaseDocument) -> R) -> Option<R> {
    CURRENT_DOC.with(|c| {
        c.get().map(|ptr| {
            // SAFETY: only ever set by `ScriptEventHandler::handle_event` for
            // the duration of one synchronous eval()/callback call, which
            // holds exclusive access to this same `BaseDocument` for that
            // entire call (via `doc: &mut dyn Document`) with nothing else
            // touching it concurrently — see that function.
            let doc: &BaseDocument = unsafe { &*ptr };
            f(doc)
        })
    })
}

/// Mutable counterpart to `with_live_doc`, for JS operations that need to
/// mutate the document *synchronously* and hand something back the same
/// tick — `document.createElement`, specifically, which must return a usable
/// `NodeId` to the calling script immediately, unlike every other mutation
/// in this file (`textContent =`, `classList.add`, etc.), which can be
/// queued onto `MUTATIONS` and applied once the triggering event finishes.
/// Same soundness argument as `with_live_doc`: exclusive access for the
/// duration of one synchronous call, nothing else touching `CURRENT_DOC`'s
/// target concurrently.
fn with_live_doc_mut<R>(f: impl FnOnce(&mut BaseDocument) -> R) -> Option<R> {
    CURRENT_DOC.with(|c| {
        c.get().map(|ptr| {
            let doc: &mut BaseDocument = unsafe { &mut *ptr };
            f(doc)
        })
    })
}

/// Reset all per-document JS binding state. Must run on every navigation
/// (fresh page = fresh DOM = stale node ids/mirrors/listeners from the
/// previous page would silently misfire otherwise — `NodeId`s aren't stable
/// across documents).
fn reset_js_bindings() {
    JS.with(|ctx| *ctx.borrow_mut() = new_js_context());
    TEXT_MIRROR.with(|m| m.borrow_mut().clear());
    CLASS_MIRROR.with(|m| m.borrow_mut().clear());
    STYLE_MIRROR.with(|m| m.borrow_mut().clear());
    INNER_HTML_MIRROR.with(|m| m.borrow_mut().clear());
    LISTENERS.with(|m| m.borrow_mut().clear());
    RAF_CALLBACKS.with(|m| m.borrow_mut().clear());
    RAF_NEXT_ID.with(|c| c.set(1));
    PAGE_TIME_ORIGIN.with(|c| c.set(Some(std::time::Instant::now())));
}

/// Split an inline `style="..."` attribute value into `(property, value)`
/// pairs — just `;`-then-`:` splitting, not a real CSS value parser (adequate
/// for inline declarations, which have no nesting/selectors to worry about).
/// Used to seed `STYLE_MIRROR` from an element's original attribute before
/// the first JS `style.setProperty` call — see that function for why seeding
/// matters (without it, the first JS-set property would serialize *only*
/// itself back onto `style=`, silently discarding every property the page's
/// own HTML/CSS had set).
fn parse_style_attr(style_attr: &str) -> Vec<(String, String)> {
    style_attr
        .split(';')
        .filter_map(|decl| {
            let (prop, value) = decl.split_once(':')?;
            let prop = prop.trim();
            if prop.is_empty() {
                return None;
            }
            Some((prop.to_string(), value.trim().to_string()))
        })
        .collect()
}

fn attr_name(local: &str) -> QualName {
    QualName::new(None, Namespace::from(""), LocalName::from(local))
}

fn js_string_arg(args: &[JsValue], i: usize) -> String {
    args.get(i)
        .and_then(JsValue::as_string)
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default()
}

fn element_node_id_of(this: &JsValue, ctx: &mut JsContext) -> NodeId {
    this.as_object()
        .and_then(|o| o.get(js_string!("__nodeId"), ctx).ok())
        .and_then(|v| v.as_number())
        .map(|n| NodeId::from_u64(n as u64))
        .unwrap_or_default() // the null id — never resolves, mutations on it silently no-op
}

fn serialize_style(props: &[(String, String)]) -> String {
    props
        .iter()
        .map(|(k, v)| format!("{k}: {v};"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A `document.getElementById`/`querySelector` return value: a JS object
/// carrying the element's `NodeId` plus `textContent` (accessor),
/// `style.setProperty`, `classList.{add,remove,toggle,contains}`, and
/// `addEventListener`. See the `*_MIRROR` thread_locals above for what "read"
/// means here — mirror-only for these four, not a live document read (unlike
/// element *lookup*, which is live — see `with_live_doc`).
fn make_element(ctx: &mut JsContext, node_id: NodeId) -> JsValue {
    let realm = ctx.realm().clone();

    // style.setProperty(name, value) — built before `obj` since both would
    // otherwise need to borrow `ctx` mutably at the same time.
    let set_property = NativeFunction::from_copy_closure(|this, args, ctx| {
        let id = element_node_id_of(this, ctx);
        let name = js_string_arg(args, 0);
        let value = js_string_arg(args, 1);
        let style_str = STYLE_MIRROR.with(|m| {
            let mut m = m.borrow_mut();
            // Seed from the element's original `style=` attribute the first
            // time this element's style is touched — otherwise the
            // serialization below would emit *only* JS-set properties,
            // silently wiping out every property the page's own markup set
            // (e.g. `style="display:flex"` followed by a single JS
            // `setProperty('color', 'red')` would overwrite `style=` with
            // just `color: red;`, dropping `display:flex` and breaking
            // layout).
            if !m.contains_key(&id) {
                let live = with_live_doc(|doc| {
                    doc.get_node(id)
                        .and_then(|n| n.attr(LocalName::from("style")))
                        .map(parse_style_attr)
                })
                .flatten()
                .unwrap_or_default();
                m.insert(id, live);
            }
            let props = m.get_mut(&id).unwrap();
            if let Some(existing) = props.iter_mut().find(|(k, _)| *k == name) {
                existing.1 = value;
            } else {
                props.push((name, value));
            }
            serialize_style(props)
        });
        MUTATIONS.with(|m| m.borrow_mut().push(DomOp::SetAttribute(id, attr_name("style"), style_str)));
        Ok(JsValue::undefined())
    });
    let mut style_obj = ObjectInitializer::new(ctx);
    style_obj.function(set_property, js_string!("setProperty"), 2);
    let style_obj = style_obj.build();

    // classList.{add, remove, toggle, contains}
    fn apply_class_op(id: NodeId, class: &str, op: fn(&mut HashSet<String>, &str) -> bool) -> (bool, String) {
        CLASS_MIRROR.with(|m| {
            let mut m = m.borrow_mut();
            // Seed from the element's original `class=` attribute the first
            // time this element's classes are touched — same reasoning as
            // `set_property`'s seeding above: without it, `classList.add`
            // would serialize *only* JS-added classes back onto `class=`,
            // discarding whatever classes the page's own markup already had.
            if !m.contains_key(&id) {
                let live: HashSet<String> = with_live_doc(|doc| {
                    doc.get_node(id)
                        .and_then(|n| n.attr(LocalName::from("class")))
                        .map(|c| c.split_whitespace().map(str::to_string).collect())
                })
                .flatten()
                .unwrap_or_default();
                m.insert(id, live);
            }
            let set = m.get_mut(&id).unwrap();
            let result = op(set, class);
            let mut classes: Vec<&str> = set.iter().map(String::as_str).collect();
            classes.sort_unstable();
            (result, classes.join(" "))
        })
    }
    let class_add = NativeFunction::from_copy_closure(|this, args, ctx| {
        let id = element_node_id_of(this, ctx);
        let class = js_string_arg(args, 0);
        let (_, class_str) = apply_class_op(id, &class, |set, c| set.insert(c.to_string()));
        MUTATIONS.with(|m| m.borrow_mut().push(DomOp::SetAttribute(id, attr_name("class"), class_str)));
        Ok(JsValue::undefined())
    });
    let class_remove = NativeFunction::from_copy_closure(|this, args, ctx| {
        let id = element_node_id_of(this, ctx);
        let class = js_string_arg(args, 0);
        let (_, class_str) = apply_class_op(id, &class, |set, c| set.remove(c));
        MUTATIONS.with(|m| m.borrow_mut().push(DomOp::SetAttribute(id, attr_name("class"), class_str)));
        Ok(JsValue::undefined())
    });
    let class_toggle = NativeFunction::from_copy_closure(|this, args, ctx| {
        let id = element_node_id_of(this, ctx);
        let class = js_string_arg(args, 0);
        let (now_present, class_str) = apply_class_op(id, &class, |set, c| {
            if set.remove(c) {
                false
            } else {
                set.insert(c.to_string());
                true
            }
        });
        MUTATIONS.with(|m| m.borrow_mut().push(DomOp::SetAttribute(id, attr_name("class"), class_str)));
        Ok(JsValue::from(now_present))
    });
    let class_contains = NativeFunction::from_copy_closure(|this, args, ctx| {
        let id = element_node_id_of(this, ctx);
        let class = js_string_arg(args, 0);
        // Read-only, so unlike `apply_class_op` this doesn't need to seed
        // `CLASS_MIRROR` — just fall back to a live attribute read when
        // nothing's been mirrored yet (e.g. `classList.contains(...)` called
        // before any `add`/`remove`/`toggle` on this element).
        let present = CLASS_MIRROR.with(|m| m.borrow().get(&id).map(|set| set.contains(&class))).unwrap_or_else(|| {
            with_live_doc(|doc| doc.get_node(id).and_then(|n| n.attr(LocalName::from("class"))).map(str::to_string))
                .flatten()
                .is_some_and(|c| c.split_whitespace().any(|token| token == class))
        });
        Ok(JsValue::from(present))
    });
    let mut class_list_obj = ObjectInitializer::new(ctx);
    class_list_obj.function(class_add, js_string!("add"), 1);
    class_list_obj.function(class_remove, js_string!("remove"), 1);
    class_list_obj.function(class_toggle, js_string!("toggle"), 1);
    class_list_obj.function(class_contains, js_string!("contains"), 1);
    let class_list_obj = class_list_obj.build();

    let get_text = NativeFunction::from_copy_closure(|this, _args, ctx| {
        let id = element_node_id_of(this, ctx);
        // Fall back to a live read when nothing's been JS-written yet — a
        // page's *original* server-rendered text should read back as
        // itself, not empty-string, the first time a script touches it.
        let text = TEXT_MIRROR
            .with(|m| m.borrow().get(&id).cloned())
            .or_else(|| with_live_doc(|doc| doc.get_node(id).map(|n| n.text_content())).flatten())
            .unwrap_or_default();
        Ok(JsValue::from(js_string!(text)))
    });
    let set_text = NativeFunction::from_copy_closure(|this, args, ctx| {
        let id = element_node_id_of(this, ctx);
        let text = js_string_arg(args, 0);
        TEXT_MIRROR.with(|m| m.borrow_mut().insert(id, text.clone()));
        MUTATIONS.with(|m| m.borrow_mut().push(DomOp::SetText(id, text)));
        Ok(JsValue::undefined())
    });
    let get_text = get_text.to_js_function(&realm);
    let set_text = set_text.to_js_function(&realm);

    let get_inner_html = NativeFunction::from_copy_closure(|this, _args, ctx| {
        let id = element_node_id_of(this, ctx);
        let html = INNER_HTML_MIRROR.with(|m| m.borrow().get(&id).cloned()).unwrap_or_default();
        Ok(JsValue::from(js_string!(html)))
    });
    let set_inner_html = NativeFunction::from_copy_closure(|this, args, ctx| {
        let id = element_node_id_of(this, ctx);
        let html = js_string_arg(args, 0);
        INNER_HTML_MIRROR.with(|m| m.borrow_mut().insert(id, html.clone()));
        MUTATIONS.with(|m| m.borrow_mut().push(DomOp::SetInnerHtml(id, html)));
        Ok(JsValue::undefined())
    });
    let get_inner_html = get_inner_html.to_js_function(&realm);
    let set_inner_html = set_inner_html.to_js_function(&realm);

    let add_event_listener = NativeFunction::from_copy_closure(|this, args, ctx| {
        let id = element_node_id_of(this, ctx);
        let event_type = js_string_arg(args, 0);
        if let Some(callback) = args.get(1).and_then(JsValue::as_object) {
            if let Some(f) = JsFunction::from_object(callback.clone()) {
                LISTENERS.with(|m| m.borrow_mut().entry((id, event_type)).or_default().push(f));
            }
        }
        Ok(JsValue::undefined())
    });

    let append_child = NativeFunction::from_copy_closure(|this, args, ctx| {
        let parent_id = element_node_id_of(this, ctx);
        let child_id = args.first().map(|v| element_node_id_of(v, ctx)).unwrap_or_default();
        MUTATIONS.with(|m| m.borrow_mut().push(DomOp::AppendChild(parent_id, child_id)));
        Ok(JsValue::undefined())
    });
    let remove_child = NativeFunction::from_copy_closure(|this, args, ctx| {
        let _parent_id = element_node_id_of(this, ctx);
        let child_id = args.first().map(|v| element_node_id_of(v, ctx)).unwrap_or_default();
        MUTATIONS.with(|m| m.borrow_mut().push(DomOp::RemoveChild(child_id)));
        Ok(JsValue::undefined())
    });

    let mut obj = ObjectInitializer::new(ctx);
    obj.property(js_string!("__nodeId"), node_id.as_u64() as f64, JsPropAttribute::all());
    obj.accessor(js_string!("textContent"), Some(get_text), Some(set_text), JsPropAttribute::all());
    obj.accessor(js_string!("innerHTML"), Some(get_inner_html), Some(set_inner_html), JsPropAttribute::all());
    obj.property(js_string!("style"), style_obj, JsPropAttribute::all());
    obj.property(js_string!("classList"), class_list_obj, JsPropAttribute::all());
    obj.function(add_event_listener, js_string!("addEventListener"), 2);
    obj.function(append_child, js_string!("appendChild"), 1);
    obj.function(remove_child, js_string!("removeChild"), 1);
    obj.build().into()
}

/// Phase 5 security hardening (docs/NATIVE_RENDERING_PLAN.md): page-authored
/// JS runs synchronously on the UI thread inside `EventDriver::handle_ui_event`
/// (see `ScriptedDocument::handle_ui_event`), so an unbounded `while(true){}`
/// in any loaded page — malicious or just buggy — would hang the entire
/// window with no way to recover, since nothing else runs until `eval()`
/// returns. Boa's own default is *no* loop-iteration cap (`u64::MAX`); this
/// bounds it to something a real page will never hit but a runaway script
/// will, so it fails with a catchable JS error instead of freezing the app.
/// Recursion/stack limits are left at Boa's own (already-bounded) defaults.
///
/// The rest of the sandboxing is structural rather than a limit to configure:
/// this context is never given `fetch`, `XMLHttpRequest`, `require`, or any
/// filesystem binding — `new_js_context` below registers exactly three
/// globals (`document`, `console`, `setTimeout`), a fixed allowlist, not an
/// escape hatch a page script could widen. Boa's own built-ins (`Object`,
/// `Array`, `Math`, `JSON`, etc.) are pure-computation with no ambient I/O.
const JS_LOOP_ITERATION_LIMIT: u64 = 10_000_000;

fn new_js_context() -> JsContext {
    let mut ctx = JsContext::default();
    ctx.runtime_limits_mut().set_loop_iteration_limit(JS_LOOP_ITERATION_LIMIT);

    let get_element_by_id = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let id = js_string_arg(args, 0);
        let node_id = with_live_doc(|doc| doc.get_element_by_id(&id)).flatten();
        Ok(node_id.map(|n| make_element(ctx, n)).unwrap_or(JsValue::null()))
    });

    let query_selector = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let selector = js_string_arg(args, 0);
        let node_id = with_live_doc(|doc| doc.query_selector(&selector).ok().flatten()).flatten();
        Ok(node_id.map(|n| make_element(ctx, n)).unwrap_or(JsValue::null()))
    });

    let query_selector_all = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let selector = js_string_arg(args, 0);
        let node_ids = with_live_doc(|doc| doc.query_selector_all(&selector).ok()).flatten();
        let elements: Vec<JsValue> = node_ids
            .unwrap_or_default()
            .into_iter()
            .map(|n| make_element(ctx, n))
            .collect();
        Ok(JsArray::from_iter(elements, ctx).into())
    });

    let create_element = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let tag = js_string_arg(args, 0);
        // Unlike every other DOM write in this file, this can't be queued
        // onto `MUTATIONS` — the calling script needs a real, usable
        // `NodeId` back *immediately* (`var el = document.createElement(...)`
        // followed by `el.textContent = ...` in the same script), so it goes
        // through `with_live_doc_mut` instead. The new node isn't attached to
        // the tree yet — nothing renders until `appendChild` (or similar)
        // places it, same as the real DOM.
        let node_id = with_live_doc_mut(|base| {
            let name = QualName::new(None, Namespace::from("http://www.w3.org/1999/xhtml"), LocalName::from(tag.as_str()));
            base.mutate().create_element(name, Vec::new())
        });
        Ok(node_id.map(|n| make_element(ctx, n)).unwrap_or(JsValue::null()))
    });
    let create_text_node = NativeFunction::from_copy_closure(|_this, args, ctx| {
        let text = js_string_arg(args, 0);
        let node_id = with_live_doc_mut(|base| base.mutate().create_text_node(&text));
        Ok(node_id.map(|n| make_element(ctx, n)).unwrap_or(JsValue::null()))
    });

    let mut document_obj = ObjectInitializer::new(&mut ctx);
    document_obj.function(get_element_by_id, js_string!("getElementById"), 1);
    document_obj.function(query_selector, js_string!("querySelector"), 1);
    document_obj.function(query_selector_all, js_string!("querySelectorAll"), 1);
    document_obj.function(create_element, js_string!("createElement"), 1);
    document_obj.function(create_text_node, js_string!("createTextNode"), 1);
    let document_obj = document_obj.build();
    ctx.register_global_property(js_string!("document"), document_obj, JsPropAttribute::all())
        .expect("registering document");

    let console_log = NativeFunction::from_copy_closure(|_this, args, _ctx| {
        let parts: Vec<String> = args
            .iter()
            .map(|v| v.as_string().map(|s| s.to_std_string_escaped()).unwrap_or_else(|| format!("{v:?}")))
            .collect();
        eprintln!("[console.log] {}", parts.join(" "));
        Ok(JsValue::undefined())
    });
    let mut console_obj = ObjectInitializer::new(&mut ctx);
    console_obj.function(console_log, js_string!("log"), 0);
    let console_obj = console_obj.build();
    ctx.register_global_property(js_string!("console"), console_obj, JsPropAttribute::all())
        .expect("registering console");

    // Minimal shim, not a real timer: runs the callback immediately rather
    // than after `delay`. `requestAnimationFrame` below now has a real
    // per-frame hook (`ScriptedDocument::poll`) this could in principle be
    // rebuilt on to get genuine delayed timing — not done here, since
    // `setTimeout`'s existing "run once, immediately" behavior is a
    // different contract than "run every frame until cancelled" and
    // deserves its own pass rather than piggybacking on rAF's queue.
    // Genuinely delayed timers, and `clearTimeout`, remain unsupported.
    let set_timeout = NativeFunction::from_copy_closure(|_this, args, ctx| {
        if let Some(callback) = args.first().and_then(JsValue::as_object) {
            if let Some(f) = JsFunction::from_object(callback.clone()) {
                let _ = f.call(&JsValue::undefined(), &[], ctx);
            }
        }
        Ok(JsValue::undefined())
    });
    ctx.register_global_callable(js_string!("setTimeout"), 2, set_timeout)
        .expect("registering setTimeout");

    // Real per-frame scheduling — see `ScriptedDocument::poll` for the other
    // half (draining this queue) and the vendor/blitz patch in
    // `resolve.rs`'s sub-document loop for how a sub-document's `poll` gets
    // called every frame at all (upstream only ever polled the top-level
    // document). Handles are just an incrementing counter, not spec-exact,
    // but round-trip correctly through `cancelAnimationFrame`.
    let request_animation_frame = NativeFunction::from_copy_closure(|_this, args, _ctx| {
        if let Some(callback) = args.first().and_then(JsValue::as_object) {
            if let Some(f) = JsFunction::from_object(callback.clone()) {
                let id = RAF_NEXT_ID.with(|c| {
                    let id = c.get();
                    c.set(id + 1);
                    id
                });
                RAF_CALLBACKS.with(|m| m.borrow_mut().push((id, f)));
                return Ok(JsValue::from(id as f64));
            }
        }
        Ok(JsValue::from(0.0))
    });
    ctx.register_global_callable(js_string!("requestAnimationFrame"), 1, request_animation_frame)
        .expect("registering requestAnimationFrame");

    let cancel_animation_frame = NativeFunction::from_copy_closure(|_this, args, _ctx| {
        if let Some(id) = args.first().and_then(JsValue::as_number) {
            let id = id as u64;
            RAF_CALLBACKS.with(|m| m.borrow_mut().retain(|(cb_id, _)| *cb_id != id));
        }
        Ok(JsValue::undefined())
    });
    ctx.register_global_callable(js_string!("cancelAnimationFrame"), 1, cancel_animation_frame)
        .expect("registering cancelAnimationFrame");

    ctx
}

/// Extract inline `<script>` contents from the already-parsed DOM, in
/// document (pre-)order, skipping any `<script src=...>` (external scripts
/// aren't fetched — same limitation as before, just no longer conflated with
/// the extraction mechanism itself).
///
/// Phase 4: walks the real tree via `Node::children` instead of the v0
/// approach of regex-free string search over raw HTML source, which broke on
/// nested quotes/comments containing a literal `</script>` and had no notion
/// of "element" at all (a `</script>` inside a JS string or comment would
/// truncate the script early). Reads `node.text_content()` — the same
/// text-flattening helper already used for `<title>` — so entity decoding
/// and nested-text-node concatenation are handled by the parser, not
/// hand-rolled here.
fn extract_inline_scripts(base: &BaseDocument) -> Vec<String> {
    let script_tag = LocalName::from("script");
    let src_attr = LocalName::from("src");
    let mut scripts = Vec::new();
    let mut stack = vec![base.root_node().id];
    while let Some(node_id) = stack.pop() {
        let Some(node) = base.get_node(node_id) else { continue };
        stack.extend(node.children.iter().rev());
        if node.data.is_element_with_tag_name(&script_tag) && node.attr(src_attr.clone()).is_none() {
            scripts.push(node.text_content());
        }
    }
    scripts
}

/// A Document that runs page-authored JS on click, in place of
/// dioxus-native's `web-view`-default `PlainDocument` (which is hardcoded to
/// `NoopEventHandler` and can never run scripts). See
/// docs/NATIVE_RENDERING_PLAN.md, "Spike 2: JavaScript execution".
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
        apply_pending_dom_ops(&mut self.base);
    }

    /// The real per-frame tick `requestAnimationFrame` runs on — see the
    /// vendor/blitz patch in `resolve.rs`'s sub-document loop, which now
    /// calls this every frame (upstream only ever called `poll` on the
    /// top-level document). Drains whatever callbacks are queued *right
    /// now* — not ones a callback itself queues via a fresh
    /// `requestAnimationFrame` call while running, matching the real spec's
    /// "next frame" semantics (self-re-queuing `function tick() { ...;
    /// requestAnimationFrame(tick); }` loops still work, just one frame
    /// later each time, same as a real browser). Returning `true` when
    /// anything ran is what keeps the redraw loop alive frame-to-frame — see
    /// the `subdoc_wants_more_frames` comment in that patch.
    fn poll(&mut self, _task_context: Option<std::task::Context>) -> bool {
        let callbacks: Vec<(u64, JsFunction)> = RAF_CALLBACKS.with(|m| m.borrow_mut().drain(..).collect());
        if callbacks.is_empty() {
            return false;
        }

        let timestamp = PAGE_TIME_ORIGIN
            .with(|c| c.get())
            .map(|origin| origin.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        run_js_with_live_doc(&mut self.base, || {
            for (_, callback) in callbacks {
                JS.with(|ctx| {
                    let _ = callback.call(&JsValue::undefined(), &[JsValue::from(timestamp)], &mut ctx.borrow_mut());
                });
            }
        });

        true
    }
}

/// Apply and drain whatever `DomOp`s JS queued during the script that just
/// ran (a click/event handler, or the page's initial inline `<script>`
/// evaluation in `build_scripted_document` — both go through this).
fn apply_pending_dom_ops(base: &mut BaseDocument) {
    let pending: Vec<DomOp> = MUTATIONS.with(|m| m.borrow_mut().drain(..).collect());
    for op in pending {
        match op {
            DomOp::SetText(node_id, text) => {
                if base.get_node(node_id).is_some() {
                    // Real `textContent =` semantics: replace *all* existing
                    // children with a single text node, not just patch an
                    // existing one in place — the old version only handled
                    // the case where the first child already happened to be
                    // a text node, silently no-opping otherwise (e.g. on a
                    // freshly `createElement`-d node, which starts with zero
                    // children, or one whose first child is itself an
                    // element).
                    let mut mutator = base.mutate();
                    mutator.remove_and_drop_all_children(node_id);
                    let text_node_id = mutator.create_text_node(&text);
                    mutator.append_children(node_id, &[text_node_id]);
                }
            }
            DomOp::SetAttribute(node_id, name, value) => {
                base.mutate().set_attribute(node_id, name, &value);
            }
            DomOp::SetInnerHtml(node_id, html) => {
                if base.get_node(node_id).is_some() {
                    base.mutate().set_inner_html(node_id, &html);
                }
            }
            DomOp::AppendChild(parent_id, child_id) => {
                if base.get_node(parent_id).is_some() && base.get_node(child_id).is_some() {
                    base.mutate().append_children(parent_id, &[child_id]);
                }
            }
            DomOp::RemoveChild(child_id) => {
                if base.get_node(child_id).is_some() {
                    base.mutate().remove_node(child_id);
                }
            }
        }
    }
}

/// Run `f` with `base` published as the live document for
/// `document.getElementById`/`querySelector` (see `CURRENT_DOC`'s doc
/// comment), then apply whatever DOM writes it queued. Used both for the
/// page's initial inline-script evaluation (`build_scripted_document`) and,
/// indirectly, for event-triggered script (`ScriptEventHandler`, which
/// manages the same thread_local itself since it only has `&mut dyn Document`,
/// not an owned `BaseDocument`, to hand off).
fn run_js_with_live_doc(base: &mut BaseDocument, f: impl FnOnce()) {
    CURRENT_DOC.with(|c| c.set(Some(base as *mut BaseDocument)));
    f();
    CURRENT_DOC.with(|c| c.set(None));
    apply_pending_dom_ops(base);
}

/// blitz-dom's own default action for both link clicks *and* form submission
/// (e.g. pressing Enter in a search box) calls `NavigationProvider::navigate_to`
/// — unset, it's a no-op (`DummyNavigationProvider`), which is why Enter-to-submit
/// silently did nothing until this was wired in. Queues onto the same
/// `PENDING_NAVIGATION` channel `find_href` uses for link clicks (the two paths
/// overlap for plain `<a href>` clicks — both fire, harmlessly setting the same
/// value — but only this one covers forms). GET forms navigate correctly since
/// the query string is already baked into `options.url`; POST forms are not
/// handled yet (`navigate_tab`/`load_page` only ever does a GET fetch) — a
/// smaller follow-up once needed.
struct SubDocNavigationProvider;
impl NavigationProvider for SubDocNavigationProvider {
    fn navigate_to(&self, options: NavigationOptions) {
        PENDING_NAVIGATION.with(|p| *p.borrow_mut() = Some(options.url.to_string()));
    }
}

/// Find the nearest `href` in `chain` (target-to-root order — see
/// `BaseDocument::node_chain`), i.e. the target element itself or the
/// nearest ancestor that carries one. Handles clicking on inline content
/// (a `<span>`/text run) inside an `<a>`, not just the anchor tag directly.
fn find_href(doc: &dyn Document, chain: &[NodeId]) -> Option<String> {
    let inner = doc.inner();
    chain
        .iter()
        .find_map(|&id| inner.get_node(id).and_then(|n| n.attr("href".into())))
        .map(|s| s.to_string())
}

struct ScriptEventHandler;
impl EventHandler for ScriptEventHandler {
    fn handle_event(
        &mut self,
        chain: &[NodeId],
        event: &mut DomEvent,
        doc: &mut dyn Document,
        _event_state: &mut EventState,
    ) {
        if let DomEventData::Click(_) = &event.data {
            let onclick = doc
                .inner()
                .get_node(event.target)
                .and_then(|n| n.attr("onclick".into()))
                .map(|s| s.to_string());

            // Live document reads (getElementById/querySelector) only work
            // for JS run inside this window — see `CURRENT_DOC`'s doc comment.
            // `ScriptedDocument::inner_mut()` always returns `Ref`, never the
            // other `DocGuardMut` variants, so this is exhaustive in practice.
            if let DocGuardMut::Ref(base) = doc.inner_mut() {
                CURRENT_DOC.with(|c| c.set(Some(base as *mut BaseDocument)));

                if let Some(code) = onclick {
                    JS.with(|ctx| {
                        let _ = ctx.borrow_mut().eval(Source::from_bytes(code.as_bytes()));
                    });
                }

                // addEventListener('click', ...) registrations for this element.
                let callbacks = LISTENERS.with(|m| {
                    m.borrow().get(&(event.target, "click".to_string())).cloned().unwrap_or_default()
                });
                for callback in callbacks {
                    JS.with(|ctx| {
                        let _ = callback.call(&JsValue::undefined(), &[], &mut ctx.borrow_mut());
                    });
                }

                CURRENT_DOC.with(|c| c.set(None));
            }

            if let Some(href) = find_href(doc, chain) {
                // "#"/empty hrefs are the standard idiom for a JS-only click
                // target (already handled above via onclick, if any) — not a
                // real navigation.
                let is_placeholder = href.is_empty() || href == "#" || href.starts_with("javascript:");
                if !is_placeholder {
                    let resolved = doc.inner().url().join(&href).ok().map(|u| u.to_string());
                    if let Some(url) = resolved {
                        PENDING_NAVIGATION.with(|p| *p.borrow_mut() = Some(url));
                    }
                }
            }
        }

        if let DomEventData::ContextMenu(data) = &event.data {
            let href = find_href(doc, chain)
                .filter(|h| !h.is_empty() && h != "#" && !h.starts_with("javascript:"))
                .and_then(|h| doc.inner().url().join(&h).ok())
                .map(|u| u.to_string());

            PENDING_CONTEXT_MENU.with(|p| {
                *p.borrow_mut() = Some(ContextMenuRequest { x: data.coords.client_x, y: data.coords.client_y, href })
            });
        }
    }
}

/// Phase 6/7 shared tier detection: computed once, used both by `main`'s
/// launch gate and by `default_isolation_mode` below (which needs the same
/// classification to decide per-tab session isolation) rather than each
/// calling `DeviceCapabilities::detect()` — a real syscall-driven probe —
/// independently.
fn detected_device_tier() -> DeviceTier {
    static TIER: OnceLock<DeviceTier> = OnceLock::new();
    *TIER.get_or_init(|| DeviceCapabilities::detect().map(|c| c.device_tier()).unwrap_or(DeviceTier::Standard))
}

/// Phase 7 (docs/NATIVE_RENDERING_PLAN.md): the single `Browser` +
/// `TabManager` backing every UI tab's session lifecycle — same types
/// `src/browser/mod.rs` is tested against (isolated vs. shared cookies,
/// session cleanup on tab close), not a native-shell-only reimplementation.
fn browser() -> &'static Browser {
    static BROWSER: OnceLock<Browser> = OnceLock::new();
    BROWSER.get_or_init(|| Browser::new().expect("Browser::new is infallible in practice — see Browser::default's identical .expect()"))
}

fn tab_manager() -> &'static TabManager {
    static TAB_MANAGER: OnceLock<TabManager> = OnceLock::new();
    TAB_MANAGER.get_or_init(|| TabManager::new("himalayas-desktop-default".to_string()))
}

/// Per-tab session isolation cost (a distinct cookie jar/storage map per tab)
/// is only worth paying on hardware that can afford it — see the `DeviceTier`
/// doc comment on `IsolationMode` in `src/browser/tabs.rs`. Reuses the same
/// Standard-and-above threshold `tier_supports_native_shell` already gates
/// launching this binary on: in practice this device is always at least
/// Standard tier by the time a tab is created (unless launched with
/// `--force` on below-tier hardware), in which case falling back to one
/// shared session across tabs is the right degraded behavior anyway.
fn default_isolation_mode() -> IsolationMode {
    if tier_supports_native_shell(detected_device_tier()) {
        IsolationMode::Isolated
    } else {
        IsolationMode::Shared
    }
}

/// Appearance setting exposed through the address bar's settings panel (see
/// `app()`'s `⋮` button) — a real, working toggle, not a placeholder: every
/// color token in the shell is derived from `Theme::palette` at render
/// time rather than fixed `const`s, so switching themes actually recolors
/// the window. Scoped to just this (the one setting from the full
/// "Appearance" section that's tractable without a shared stylesheet
/// mechanism — see the palette doc comment below for why tokens are
/// duplicated per-theme rather than computed).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Theme {
    Dark,
    Light,
}

struct Palette {
    bg: &'static str,
    surface: &'static str,
    border: &'static str,
    text: &'static str,
    text_muted: &'static str,
    accent: &'static str,
}

impl Theme {
    /// Design language from docs/UI_UX_VISION.md #22. Dark matches
    /// `src/ui/web/style.css`'s `@media (prefers-color-scheme: dark)` token
    /// values (this shell has no shared stylesheet with the web shell —
    /// dioxus-native renders inline styles, not a linked CSS file — so the
    /// tokens are duplicated here as literal values rather than referenced).
    /// Light is this browser's own palette, not a mirror of any existing
    /// design doc.
    fn palette(self) -> Palette {
        match self {
            Theme::Dark => Palette {
                bg: "#0b1220",
                surface: "#121a2b",
                border: "rgba(255,255,255,0.08)",
                text: "#eef3f7",
                text_muted: "#93a2b8",
                accent: "#5b8def",
            },
            Theme::Light => Palette {
                bg: "#f7f8fa",
                surface: "#ffffff",
                border: "rgba(0,0,0,0.1)",
                text: "#1a1f27",
                text_muted: "#5c6470",
                accent: "#3b6fd4",
            },
        }
    }
}

#[derive(Clone)]
struct Tab {
    id: u32,
    /// `browser::tabs::Tab::id` — the backend's own identity for this tab,
    /// used to look up/close its `Session` via `browser()`/`tab_manager()`.
    /// Distinct from `id` (a small `u32` used only as a local Dioxus signal
    /// key/reorder handle in `app()`).
    backend_id: String,
    /// The `Session` this tab's requests carry cookies for — either unique to
    /// this tab (`IsolationMode::Isolated`) or shared with every other tab
    /// (`IsolationMode::Shared`), decided once at tab-creation time by
    /// `default_isolation_mode`.
    session_id: String,
    url: String,
    title: String,
    document: Option<SubDocumentAttr>,
    status: String,
    /// Past URLs for this tab, oldest first, *not* including the current
    /// `url`. Popped by the back button; not touched by forward/refresh.
    history: Vec<String>,
    /// See the tab-strip's right-click menu in `app()`. Pinned tabs render
    /// compact (favicon-badge only, no close "x") and are kept contiguous
    /// at the front of `tabs` — both the pin/unpin actions and the
    /// existing drag-reorder code maintain that invariant, so rendering
    /// can just walk `tabs` in order rather than filtering into two
    /// separate groups.
    pinned: bool,
}

/// A pinned tab, as persisted to disk — see `SessionState`. Just enough to
/// recreate the tab (`Tab::new` needs a URL) and show something sensible
/// before it finishes reloading (the title from last time, until the page
/// re-fetches and overwrites it) — not a snapshot of scroll position,
/// history, or anything else about the tab's prior state.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PinnedTabRecord {
    url: String,
    title: String,
}

/// Session persistence — currently just pinned tabs (see the "Pin a tab"
/// section of docs/NATIVE_RENDERING_PLAN.md's "Next phases"; this is the
/// first thing in `desktop.rs` that persists *anything* across a restart,
/// everything else — tabs, bookmarks, folders, theme — is still in-memory
/// only). `restore_pinned_tabs` is itself persisted (not just the tab
/// list) so turning the setting off is remembered too, not just acted on
/// once.
#[derive(Serialize, Deserialize)]
struct SessionState {
    restore_pinned_tabs: bool,
    pinned_tabs: Vec<PinnedTabRecord>,
}

impl Default for SessionState {
    fn default() -> Self {
        // Matches how real browsers default this: once you've pinned
        // something, it survives a restart unless you turn that off
        // explicitly — not an opt-in most users would ever find.
        Self { restore_pinned_tabs: true, pinned_tabs: Vec::new() }
    }
}

fn session_state_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "Himalayas", "Himalayas")
        .map(|dirs| dirs.config_dir().join("session.json"))
}

/// Split from `load_session_state`/`save_session_state` (which hardcode the
/// real OS config dir via `session_state_path`) so tests can exercise the
/// actual read/write/default-on-missing-or-corrupt logic against a
/// throwaway path instead of the user's real config directory.
fn load_session_state_from(path: &std::path::Path) -> SessionState {
    let Ok(content) = std::fs::read_to_string(path) else { return SessionState::default() };
    serde_json::from_str(&content).unwrap_or_default()
}

fn save_session_state_to(path: &std::path::Path, state: &SessionState) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, json);
    }
}

fn load_session_state() -> SessionState {
    let Some(path) = session_state_path() else { return SessionState::default() };
    load_session_state_from(&path)
}

fn save_session_state(state: &SessionState) {
    let Some(path) = session_state_path() else { return };
    save_session_state_to(&path, state);
}

/// Rebuild a `SessionState` from the live `tabs` list — the part of
/// `persist_pinned_tabs` that's pure/worth testing directly without disk
/// I/O. Doesn't track every field of `Tab`, only what `PinnedTabRecord`
/// needs.
fn pinned_tabs_session_state(tabs: &[Tab], restore_pinned_tabs: bool) -> SessionState {
    let pinned_tabs = tabs
        .iter()
        .filter(|t| t.pinned)
        .map(|t| PinnedTabRecord { url: t.url.clone(), title: t.title.clone() })
        .collect();
    SessionState { restore_pinned_tabs, pinned_tabs }
}

/// Called after every pin/unpin, pinned-tab close, or drag that crosses the
/// pinned/unpinned boundary (see the tab strip in `app()`), and whenever
/// the "restore pinned tabs" setting itself changes.
fn persist_pinned_tabs(tabs: &[Tab], restore_pinned_tabs: bool) {
    save_session_state(&pinned_tabs_session_state(tabs, restore_pinned_tabs));
}

/// A saved bookmark — see the star button/popover and the Bookmark Manager
/// overlay in `app()`. In-memory only for now (cleared on restart, like
/// every other piece of session state in this file); `folder` is a flat
/// label (matched against `app()`'s `folders` list), not a nested tree —
/// real browsers' nested-folder trees are a further-out scope than what
/// was actually asked for here (rename/import/export/search/sort).
/// `Serialize`/`Deserialize` back the JSON export/import format directly
/// (see `BookmarkExportFile`) — its "version": 1 shape *is* `Vec<Bookmark>`
/// plus a version tag, so no separate wire type is needed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Bookmark {
    title: String,
    url: String,
    folder: String,
    /// Unix seconds. `#[serde(default)]` so JSON imported from an older
    /// export (or a hand-written one) that omits it doesn't fail to parse
    /// — it just sorts as if added at the epoch, which only affects
    /// "sort by date added" in the manager, not correctness.
    #[serde(default)]
    added_at: i64,
}

/// The JSON export/import wire format — see the "Bookmark Manager" section
/// of `docs/NATIVE_RENDERING_PLAN.md` for the worked example this mirrors
/// exactly (`{"version": 1, "bookmarks": [...]}`).
#[derive(Serialize, Deserialize)]
struct BookmarkExportFile {
    version: u32,
    bookmarks: Vec<Bookmark>,
}

/// Netscape Bookmark File Format export (the format Chrome/Firefox/Edge/etc.
/// all read and write, hence "universal compatibility layer" — see the doc).
/// Deliberately minimal markup: one flat `<DL>` per folder in bookmark
/// order, no nested sub-folders (matches `Bookmark::folder` being a flat
/// label, not a tree) and no extra `<DT>`/timestamp attributes beyond what
/// the format needs to round-trip through a real browser's importer.
fn export_bookmarks_html(bookmarks: &[Bookmark], folders: &[String]) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE NETSCAPE-Bookmark-file-1>\n");
    out.push_str("<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n");
    out.push_str("<TITLE>Bookmarks</TITLE>\n");
    out.push_str("<H1>Bookmarks</H1>\n");
    out.push_str("<DL><p>\n");
    for folder in folders {
        out.push_str(&format!("    <DT><H3>{}</H3>\n", html_escape(folder)));
        out.push_str("    <DL><p>\n");
        for b in bookmarks.iter().filter(|b| &b.folder == folder) {
            out.push_str(&format!(
                "        <DT><A HREF=\"{}\" ADD_DATE=\"{}\">{}</A>\n",
                html_escape(&b.url),
                b.added_at,
                html_escape(&b.title)
            ));
        }
        out.push_str("    </DL><p>\n");
    }
    out.push_str("</DL><p>\n");
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn html_unescape(s: &str) -> String {
    // `&amp;` last — unescaping it first would corrupt a source string like
    // `&amp;lt;` (a literal, already-escaped "&lt;") by letting the `&lt;`
    // it produces get caught by a later `&lt;` replacement too.
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn export_bookmarks_json(bookmarks: &[Bookmark]) -> String {
    let file = BookmarkExportFile { version: 1, bookmarks: bookmarks.to_vec() };
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

/// Result of an import: what actually got merged in, for the completion
/// popover ("N bookmarks imported, M folders imported, K duplicates
/// skipped") — see the Bookmark Manager's Import flow in `app()`.
struct BookmarkImportResult {
    bookmarks: Vec<Bookmark>,
    new_folders: Vec<String>,
    duplicates_skipped: usize,
}

/// Merge `imported` into the existing `(bookmarks, folders)`, matching the
/// explicit asks: don't overwrite existing bookmarks, preserve folders,
/// treat same-URL entries as duplicates (regardless of folder) and skip
/// them rather than creating a second copy.
fn merge_imported_bookmarks(
    imported: Vec<Bookmark>,
    existing: &[Bookmark],
    existing_folders: &[String],
) -> BookmarkImportResult {
    let mut new_folders = Vec::new();
    let mut bookmarks = Vec::new();
    let mut duplicates_skipped = 0;
    for b in imported {
        if existing.iter().any(|e| e.url == b.url) {
            duplicates_skipped += 1;
            continue;
        }
        if !existing_folders.contains(&b.folder) && !new_folders.contains(&b.folder) {
            new_folders.push(b.folder.clone());
        }
        bookmarks.push(b);
    }
    BookmarkImportResult { bookmarks, new_folders, duplicates_skipped }
}

/// Parses `{"version": 1, "bookmarks": [...]}` (see `BookmarkExportFile`).
/// Falls back to a bare `Vec<Bookmark>` (no envelope) so a hand-edited or
/// script-generated file without the version wrapper still imports.
fn parse_bookmarks_json(content: &str) -> Vec<Bookmark> {
    if let Ok(file) = serde_json::from_str::<BookmarkExportFile>(content) {
        return file.bookmarks;
    }
    serde_json::from_str::<Vec<Bookmark>>(content).unwrap_or_default()
}

/// Minimal Netscape Bookmark File Format parser — a real HTML parser is
/// overkill for a format that's really just a flat, line-oriented sequence
/// of `<H3>folder</H3>` and `<A HREF="...">title</A>` tags (every major
/// browser's export looks like this; see `export_bookmarks_html`'s doc
/// comment). Tracks "current folder" as whatever `<H3>` was most recently
/// seen — correct for the flat, one-level-deep folders this browser
/// supports; a real nested `<DL>` tree collapses to its leaf folder names,
/// which is a reasonable, explicit trade-off given `Bookmark::folder` has
/// no tree structure to import into.
fn parse_bookmarks_html(content: &str) -> Vec<Bookmark> {
    let folder_re = Regex::new(r"(?is)<H3[^>]*>(.*?)</H3>").unwrap();
    let link_re = Regex::new(r#"(?is)<A\s+HREF="([^"]*)"[^>]*>(.*?)</A>"#).unwrap();
    let tag_re = Regex::new(r"(?is)<[^>]+>").unwrap();

    // Merge folder/link matches into one stream, ordered by where they
    // appear in the source, so each link picks up whichever folder header
    // most recently preceded it.
    #[derive(Debug)]
    enum Token {
        Folder(usize, String),
        Link(usize, String, String),
    }
    let mut tokens: Vec<Token> = Vec::new();
    for cap in folder_re.captures_iter(content) {
        let m = cap.get(0).unwrap();
        let name = html_unescape(tag_re.replace_all(&cap[1], "").trim());
        tokens.push(Token::Folder(m.start(), name));
    }
    for cap in link_re.captures_iter(content) {
        let m = cap.get(0).unwrap();
        let url = html_unescape(cap[1].trim());
        let title = html_unescape(tag_re.replace_all(&cap[2], "").trim());
        tokens.push(Token::Link(m.start(), url, title));
    }
    tokens.sort_by_key(|t| match t {
        Token::Folder(pos, _) => *pos,
        Token::Link(pos, _, _) => *pos,
    });

    let mut current_folder = "Other Bookmarks".to_string();
    let mut out = Vec::new();
    for token in tokens {
        match token {
            Token::Folder(_, name) if !name.is_empty() => current_folder = name,
            Token::Folder(_, _) => {}
            Token::Link(_, url, title) => {
                if url.is_empty() {
                    continue;
                }
                let title = if title.is_empty() { url.clone() } else { title };
                out.push(Bookmark { title, url, folder: current_folder.clone(), added_at: 0 });
            }
        }
    }
    out
}

impl Tab {
    /// `mode` is passed in explicitly (rather than always calling
    /// `default_isolation_mode()` here) so `app()`'s settings panel can
    /// override it live — see `isolation_mode` signal in `app()`.
    fn new(id: u32, url: &str, mode: IsolationMode) -> Self {
        let backend_id = browser()
            .open_tab(tab_manager(), url.to_string(), mode)
            .expect("open_tab only fails if Session::new does, which is infallible in practice");
        let session_id = tab_manager()
            .get_tab(&backend_id)
            .map(|t| t.session_id)
            .unwrap_or_default();
        Self {
            id,
            backend_id,
            session_id,
            url: url.to_string(),
            title: "New Tab".to_string(),
            document: None,
            status: "Enter a URL and press Enter".to_string(),
            history: Vec::new(),
            pinned: false,
        }
    }
}

/// Load `url` into tab `id`: set "Loading…" status immediately, then swap in
/// the fetched document (or an error status) once `load_page` resolves.
/// Shared by the address bar's Enter/Go, in-page link clicks (via
/// `ON_NAVIGATE`), and the back/refresh buttons, so all four behave
/// identically. `push_history` is false for back (don't re-add the page we're
/// leaving *from* back-navigation) and refresh (reloading isn't a new visit).
fn navigate_tab(mut tabs: Signal<Vec<Tab>>, id: u32, url: String, push_history: bool) {
    let session_id = {
        let mut t = tabs.write();
        if let Some(tab) = t.iter_mut().find(|t| t.id == id) {
            // Only push a real, already-loaded page — not a freshly created
            // tab's placeholder URL that was never actually visited.
            if push_history && tab.document.is_some() && tab.url != url {
                tab.history.push(tab.url.clone());
            }
            tab.status = format!("Loading {url}...");
            tab.url = url.clone();
            tab.session_id.clone()
        } else {
            String::new()
        }
    };
    spawn(async move {
        match load_page(&url, &session_id).await {
            Ok((doc, title)) => {
                let mut t = tabs.write();
                if let Some(tab) = t.iter_mut().find(|t| t.id == id) {
                    tab.title = title;
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
}

fn app() -> Element {
    let mut theme = use_signal(|| Theme::Dark);
    let mut settings_open = use_signal(|| false);
    // Live override for new tabs — see `Tab::new`'s `mode` param. Starts at
    // the same device-tier-based default `default_isolation_mode()` always
    // computed; changing it in the settings panel only affects tabs opened
    // *after* the change, not tabs already open (matches how a real
    // browser's session-isolation setting works — it's not retroactive).
    let mut isolation_mode = use_signal(default_isolation_mode);
    // Page-content zoom (distinct from the shell/chrome accessibility
    // sizing above — this is real `Viewport::zoom`, applied to loaded page
    // content, not the address bar/tab strip). Floored at `1.0`/100% per
    // direct request ("the minimum should be 100%") — no zoom-out below
    // real size. Source of truth lives here, not in the engine, since
    // `ShellProvider` is fire-and-forget (no zoom getter) — see
    // `ShellProvider::set_zoom` in blitz-traits/src/shell.rs.
    let mut page_zoom = use_signal(|| 1.0f32);

    // Bookmarks (star button in the address bar). `bookmarks` is keyed by
    // URL, not tab id, so a bookmarked page reads as bookmarked from
    // whichever tab has it open. `bookmark_popover_open` gates showing the
    // popover at all; `bookmark_editing` switches it between the
    // just-added confirmation view (name/folder read-only, "Edit"/"Done")
    // and the full editor (name/folder editable, "Remove"/"Save"/"Cancel")
    // — clicking an already-bookmarked page's star jumps straight to the
    // editor rather than silently un-bookmarking, per the explicit ask.
    let mut bookmarks = use_signal(Vec::<Bookmark>::new);
    let mut bookmark_popover_open = use_signal(|| false);
    let mut bookmark_editing = use_signal(|| false);
    let mut bookmark_name_draft = use_signal(String::new);
    let mut bookmark_folder_draft = use_signal(|| "Bookmarks Bar".to_string());
    // Flat folder list (no nesting) — "Bookmarks Bar"/"Other Bookmarks"
    // starting set matches the two-folder default most browsers ship with.
    // New folders are added from the Bookmark Manager or picked up
    // automatically on import (`merge_imported_bookmarks`).
    let mut folders = use_signal(|| vec!["Bookmarks Bar".to_string(), "Other Bookmarks".to_string()]);

    // Bookmark Manager overlay (search/sort/rename/delete/folders/
    // import/export) — a separate, bigger view from the star's
    // add/quick-edit popover above. `bookmark_manager_editing_url` is
    // `Some(url)` while that one bookmark's title is inline-editable;
    // `renaming_folder` is `Some(old_name)` while that folder's name is
    // inline-editable. `import_export_status` holds the last
    // import/export result message shown in a small confirmation banner.
    let mut bookmark_manager_open = use_signal(|| false);
    let mut bookmark_manager_search = use_signal(String::new);
    let mut bookmark_manager_sort_by_date = use_signal(|| false);
    let mut new_folder_draft = use_signal(String::new);
    let mut renaming_folder = use_signal(|| None::<String>);
    let mut rename_folder_draft = use_signal(String::new);
    let mut bookmark_manager_editing_url = use_signal(|| None::<String>);
    let mut bookmark_manager_editing_draft = use_signal(String::new);
    let mut import_export_status = use_signal(|| None::<String>);
    // Drag-a-bookmark-onto-a-folder-header to move it (reuses the same
    // mousedown-arm/mouseenter-swap shape the tab strip's drag reorder
    // already established — no real HTML5 drag events fire in this shell).
    // Not full manual reordering: dropping just changes `folder`, it
    // doesn't reposition the bookmark within a folder's sorted list — see
    // the doc comment above the Bookmark Manager's `if` block.
    let mut dragging_bookmark = use_signal(|| None::<String>);
    // Multi-select (checkbox per row, bulk move/delete in the toolbar),
    // keyed by URL like everything else bookmark-related in this file.
    let mut selected_bookmarks = use_signal(HashSet::<String>::new);
    let mut cache_clear_status = use_signal(|| None::<String>);

    // Pinned-tab restore: each `use_signal` initializer is guaranteed to
    // run exactly once (on mount), same guarantee the original single-tab
    // initializer already relied on for its own `Tab::new` side effect
    // (`browser().open_tab(...)`) — so it's safe for each of these three to
    // independently call `load_session_state()` (a cheap file read) rather
    // than sharing one precomputed value across signals, which would need
    // a different hook shape to keep that same one-time-only guarantee.
    let mut restore_pinned_tabs_setting = use_signal(|| load_session_state().restore_pinned_tabs);
    let mut tabs = use_signal(|| {
        let session = load_session_state();
        let mut list = Vec::new();
        let mut next = 0u32;
        if session.restore_pinned_tabs {
            for record in &session.pinned_tabs {
                let mut tab = Tab::new(next, &record.url, default_isolation_mode());
                tab.pinned = true;
                tab.title = record.title.clone();
                list.push(tab);
                next += 1;
            }
        }
        list.push(Tab::new(next, "https://example.com", default_isolation_mode()));
        list
    });
    let mut active_id = use_signal(|| {
        let session = load_session_state();
        if session.restore_pinned_tabs { session.pinned_tabs.len() as u32 } else { 0 }
    });
    let mut next_id = use_signal(|| {
        let session = load_session_state();
        (if session.restore_pinned_tabs { session.pinned_tabs.len() as u32 } else { 0 }) + 1
    });
    let mut address_input = use_signal(|| "https://example.com".to_string());

    // Tab reordering: real HTML5 drag events (ondragstart/ondragover/ondrop)
    // don't fire in this shell — blitz-dom's DomEventData has no Drag*
    // variant, so dioxus-html's drag event types exist but nothing ever
    // dispatches them. Implemented instead with mousedown-to-arm +
    // mouseenter-to-swap, using event types already confirmed working
    // elsewhere in this file (MouseDown/MouseEnter). `dragging_tab` is `Some`
    // for the whole strip while a tab is "picked up"; entering a different
    // tab immediately swaps it into that slot (live-reorder, not a drop-only
    // reorder), and any mouseup over the strip ends the drag.
    let mut dragging_tab = use_signal(|| None::<u32>);

    // Right-click menu on a tab itself (Pin/Unpin, Duplicate, Reload,
    // Close) — a native `oncontextmenu` RSX handler on the tab strip's own
    // chrome, unlike `context_menu` below (queued by `ScriptEventHandler`
    // for right-clicks on *page content*, a completely different capture
    // path since page content isn't Himalayas' own RSX).
    let mut tab_context_menu = use_signal(|| None::<TabContextMenuRequest>);

    // Right-click menu request queued by `ScriptEventHandler` — see
    // `ContextMenuRequest`/`PENDING_CONTEXT_MENU`.
    let mut context_menu = use_signal(|| None::<ContextMenuRequest>);

    // Poll for link clicks and right-clicks queued by `ScriptEventHandler`
    // (see `PENDING_NAVIGATION`'s doc comment for why this can't be a direct
    // callback). Runs as a real Dioxus-spawned task via `use_future`, so the
    // `Signal::set`/`spawn` calls inside `navigate_tab` execute with a valid
    // scope — unlike a raw closure invoked from outside the component tree.
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let pending = PENDING_NAVIGATION.with(|p| p.borrow_mut().take());
            if let Some(url) = pending {
                let id = active_id();
                address_input.set(url.clone());
                navigate_tab(tabs, id, url, true);
            }
            let pending_menu = PENDING_CONTEXT_MENU.with(|p| p.borrow_mut().take());
            if let Some(request) = pending_menu {
                context_menu.set(Some(request));
            }
        }
    });

    let navigate = move || {
        let id = active_id();
        let url = address_input.read().clone();
        navigate_tab(tabs, id, url, true);
    };

    let mut go_back = move || {
        let id = active_id();
        let popped = {
            let mut t = tabs.write();
            t.iter_mut().find(|t| t.id == id).and_then(|tab| tab.history.pop())
        };
        if let Some(url) = popped {
            address_input.set(url.clone());
            navigate_tab(tabs, id, url, false);
        }
    };

    let refresh = move || {
        let id = active_id();
        let url = tabs.read().iter().find(|t| t.id == id).map(|t| t.url.clone());
        if let Some(url) = url {
            navigate_tab(tabs, id, url, false);
        }
    };

    let mut new_tab = move || {
        let id = next_id();
        next_id.set(id + 1);
        tabs.write().push(Tab::new(id, "https://example.com", isolation_mode()));
        active_id.set(id);
        address_input.set("https://example.com".to_string());
    };

    // Shared by the tab strip's own "x" button and the Cmd/Ctrl+W shortcut.
    let mut close_tab_by_id = move |id: u32| {
        let backend_id = tabs.read().iter().find(|t| t.id == id).map(|t| t.backend_id.clone());
        if let Some(backend_id) = backend_id {
            let _ = browser().close_tab(tab_manager(), &backend_id);
        }
        let was_pinned = tabs.read().iter().find(|t| t.id == id).is_some_and(|t| t.pinned);
        tabs.write().retain(|t| t.id != id);
        if was_pinned {
            persist_pinned_tabs(&tabs.read(), restore_pinned_tabs_setting());
        }
        if active_id() == id {
            if let Some(first) = tabs.read().first() {
                active_id.set(first.id);
                address_input.set(first.url.clone());
            }
        }
    };

    // Handle set by the address bar's `onmounted` below, so Cmd/Ctrl+L can
    // actually move OS-level focus there — matches the real address-bar
    // shortcut every mainstream browser has.
    let mut address_input_element = use_signal(|| None::<std::rc::Rc<MountedData>>);

    // Global browser-shell shortcuts. Reserved keys are diverted away from
    // the loaded page *before* this even runs — see `is_reserved_browser_shortcut`
    // in the vendor/blitz event-forwarding patch (`events/driver.rs`); this
    // handler is what a Dioxus `onkeydown` on the outer shell div actually
    // receives once that diversion happens, since ordinary keyboard dispatch
    // (not our forwarding patch) is what reaches RSX event handlers at all.
    let mut handle_keydown = move |evt: KeyboardEvent| {
        let mods = evt.modifiers();
        if mods.meta() || mods.ctrl() {
            let Key::Character(c) = evt.key() else { return };
            match c.as_str() {
                "t" | "T" => {
                    new_tab();
                    evt.prevent_default();
                }
                "w" | "W" => {
                    close_tab_by_id(active_id());
                    evt.prevent_default();
                }
                "r" | "R" => {
                    refresh();
                    evt.prevent_default();
                }
                "l" | "L" => {
                    if let Some(el) = address_input_element() {
                        spawn(async move {
                            let _ = el.set_focus(true).await;
                        });
                    }
                    evt.prevent_default();
                }
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                    if let Some(n) = c.chars().next().and_then(|ch| ch.to_digit(10)) {
                        if let Some(tab) = tabs.read().get(n as usize - 1) {
                            active_id.set(tab.id);
                            address_input.set(tab.url.clone());
                        }
                    }
                    evt.prevent_default();
                }
                _ => {}
            }
        } else if evt.key() == Key::Escape {
            if context_menu().is_some() {
                context_menu.set(None);
            } else if tab_context_menu().is_some() {
                tab_context_menu.set(None);
            } else if bookmark_popover_open() {
                bookmark_popover_open.set(false);
                bookmark_editing.set(false);
            } else if bookmark_manager_open() {
                bookmark_manager_open.set(false);
            } else if settings_open() {
                settings_open.set(false);
            }
        }
    };

    // Dark-by-default (not OS-preference-aware — dark is the standing
    // default, not a detected one), but a real, live-switchable setting now
    // — see `Theme`/`Theme::palette`.
    let palette = theme().palette();
    let bg = palette.bg;
    let surface = palette.surface;
    let border = palette.border;
    let text_color = palette.text;
    let text_muted = palette.text_muted;
    let accent = palette.accent;
    const FONT_STACK: &str = "-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif";

    // Address bar row + tab strip are sized ~2x their original values (first
    // tried 3x/300%, dialed back to 2x/200% per direct "too much" feedback) —
    // an accessibility request (larger text and click targets for users
    // who need them), not a cosmetic choice, and deliberately *not* the
    // same lever as page-content zoom (`Viewport::zoom` in
    // blitz-traits/src/shell.rs, left at real 100% — see that file):
    // zoom is one value inherited by every loaded page from this outer
    // document, so it can't express "shell bigger, page content
    // unaffected." This is hardcoded for every user right now, not a
    // toggle — promoting it into the settings panel (`Theme`-style, a
    // real switchable setting) is the natural next step if it turns out
    // not everyone wants the shell permanently this large.

    rsx! {
        div {
            style: "position:relative;display:flex;flex-direction:column;height:100vh;background:{bg};font-family:{FONT_STACK};color:{text_color};",
            onkeydown: move |evt| handle_keydown(evt),

            // Tab strip
            div {
                style: "display:flex;gap:6px;padding:8px 8px 0;background:{bg};align-items:center;border-bottom:1px solid {border};",
                onmouseup: move |_| {
                    // A drag may have crossed the pinned/unpinned boundary
                    // (see `onmouseenter` below) — persist once here, at
                    // drag-end, rather than on every hover during the drag.
                    if dragging_tab().is_some() {
                        persist_pinned_tabs(&tabs.read(), restore_pinned_tabs_setting());
                    }
                    dragging_tab.set(None);
                },
                for tab in tabs.read().iter().cloned() {
                    div {
                        key: "{tab.id}",
                        style: {
                            let being_dragged = dragging_tab() == Some(tab.id);
                            let opacity = if being_dragged { "opacity:0.5;" } else { "" };
                            if tab.pinned {
                                // Compact, favicon-badge-only — no real
                                // favicon fetching pipeline exists yet, so
                                // this falls back to a letter badge (the
                                // same fallback most browsers use before an
                                // icon loads).
                                let tab_bg = if tab.id == active_id() { surface } else { "transparent" };
                                format!("display:flex;align-items:center;justify-content:center;width:56px;height:56px;flex-shrink:0;background:{tab_bg};border:1px solid {border};border-bottom:none;border-radius:10px 10px 0 0;cursor:grab;font-size:22px;font-weight:600;color:{text_color};{opacity}")
                            } else if tab.id == active_id() {
                                format!("display:flex;align-items:center;gap:6px;padding:16px 24px;background:{surface};border:1px solid {border};border-bottom:none;border-radius:10px 10px 0 0;cursor:grab;font-size:25px;color:{text_color};max-width:360px;box-shadow:0 -1px 3px rgba(0,0,0,0.3);{opacity}")
                            } else {
                                format!("display:flex;align-items:center;gap:6px;padding:16px 24px;background:transparent;border-radius:10px 10px 0 0;cursor:grab;font-size:25px;color:{text_muted};max-width:360px;{opacity}")
                            }
                        },
                        onclick: move |_| {
                            active_id.set(tab.id);
                            address_input.set(tab.url.clone());
                        },
                        onmousedown: move |_| dragging_tab.set(Some(tab.id)),
                        onmouseenter: move |_| {
                            if let Some(dragged_id) = dragging_tab() {
                                if dragged_id != tab.id {
                                    let mut t = tabs.write();
                                    let from = t.iter().position(|x| x.id == dragged_id);
                                    let to = t.iter().position(|x| x.id == tab.id);
                                    if let (Some(from), Some(to)) = (from, to) {
                                        // Dragging into/out of the pinned
                                        // group pins/unpins — adopt the
                                        // hovered tab's pinned state so
                                        // pinned tabs stay contiguous at the
                                        // front even after a cross-group
                                        // drag, not just same-group reorders.
                                        let target_pinned = t[to].pinned;
                                        let mut moved = t.remove(from);
                                        moved.pinned = target_pinned;
                                        t.insert(to, moved);
                                    }
                                }
                            }
                        },
                        oncontextmenu: move |evt| {
                            evt.prevent_default();
                            let pos = evt.client_coordinates();
                            tab_context_menu.set(Some(TabContextMenuRequest { tab_id: tab.id, x: pos.x, y: pos.y }));
                        },
                        if tab.pinned {
                            span {
                                "{tab.title.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()}"
                            }
                        } else {
                            span {
                                style: "overflow:hidden;text-overflow:ellipsis;white-space:nowrap;",
                                "{tab.title}"
                            }
                            button {
                                style: "flex-shrink:0;display:flex;align-items:center;justify-content:center;width:36px;height:36px;border:none;background:{border};color:{text_color};border-radius:50%;font-size:22px;line-height:1;cursor:pointer;padding:0;",
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    close_tab_by_id(tab.id);
                                },
                                "x"
                            }
                        }
                    }
                }
                button {
                    style: "border:none;background:transparent;color:{text_muted};font-size:32px;width:56px;height:56px;border-radius:8px;cursor:pointer;margin-bottom:4px;",
                    onclick: move |_| new_tab(),
                    "+"
                }
            }
            if let Some(menu) = tab_context_menu() {
                div {
                    style: format!("position:absolute;top:{}px;left:{}px;width:190px;background:{surface};border:1px solid {border};border-radius:8px;box-shadow:0 8px 24px rgba(0,0,0,0.35);padding:6px;z-index:30;font-size:14px;color:{text_color};", menu.y, menu.x),
                    {
                        let pinned = tabs.read().iter().find(|t| t.id == menu.tab_id).is_some_and(|t| t.pinned);
                        let item_style = "display:block;width:100%;text-align:left;padding:9px 12px;border:none;background:transparent;color:{text_color};font-size:14px;border-radius:6px;cursor:pointer;";
                        rsx! {
                            button {
                                style: "{item_style}",
                                onclick: move |_| {
                                    let mut t = tabs.write();
                                    if let Some(pos) = t.iter().position(|x| x.id == menu.tab_id) {
                                        // Same formula moves a tab to either
                                        // edge of its new group: the count of
                                        // *other* pinned tabs is both "end of
                                        // the pinned block" (pinning) and
                                        // "start of the unpinned block"
                                        // (unpinning), since this tab isn't
                                        // counted as pinned in either case.
                                        let other_pinned_count = t.iter().filter(|x| x.id != menu.tab_id && x.pinned).count();
                                        let mut tab = t.remove(pos);
                                        tab.pinned = !tab.pinned;
                                        let target_index = other_pinned_count.min(t.len());
                                        t.insert(target_index, tab);
                                    }
                                    persist_pinned_tabs(&t, restore_pinned_tabs_setting());
                                    drop(t);
                                    tab_context_menu.set(None);
                                },
                                if pinned { "Unpin Tab" } else { "Pin Tab" }
                            }
                            button {
                                style: "{item_style}",
                                onclick: move |_| {
                                    let url = tabs.read().iter().find(|t| t.id == menu.tab_id).map(|t| t.url.clone());
                                    if let Some(url) = url {
                                        navigate_tab(tabs, menu.tab_id, url, false);
                                    }
                                    tab_context_menu.set(None);
                                },
                                "Reload"
                            }
                            button {
                                style: "{item_style}",
                                onclick: move |_| {
                                    let url = tabs.read().iter().find(|t| t.id == menu.tab_id).map(|t| t.url.clone());
                                    if let Some(url) = url {
                                        let id = next_id();
                                        next_id.set(id + 1);
                                        tabs.write().push(Tab::new(id, &url, isolation_mode()));
                                        active_id.set(id);
                                        address_input.set(url);
                                    }
                                    tab_context_menu.set(None);
                                },
                                "Duplicate Tab"
                            }
                            div { style: "height:1px;background:{border};margin:4px 2px;" }
                            button {
                                style: "{item_style}",
                                onclick: move |_| {
                                    close_tab_by_id(menu.tab_id);
                                    tab_context_menu.set(None);
                                },
                                "Close Tab"
                            }
                        }
                    }
                }
            }

            // Address bar
            div {
                style: "position:relative;display:flex;gap:8px;padding:10px;background:{surface};border-bottom:1px solid {border};",
                {
                    let has_history = tabs.read().iter().find(|t| t.id == active_id()).is_some_and(|t| !t.history.is_empty());
                    let back_style = if has_history {
                        format!("border:none;background:transparent;color:{text_color};font-size:30px;width:64px;height:64px;border-radius:8px;cursor:pointer;")
                    } else {
                        format!("border:none;background:transparent;color:{text_muted};font-size:30px;width:64px;height:64px;border-radius:8px;cursor:default;opacity:0.4;")
                    };
                    rsx! {
                        button {
                            style: "{back_style}",
                            disabled: !has_history,
                            onclick: move |_| go_back(),
                            "←"
                        }
                    }
                }
                button {
                    style: "border:none;background:transparent;color:{text_color};font-size:28px;width:64px;height:64px;border-radius:8px;cursor:pointer;",
                    onclick: move |_| refresh(),
                    "↻"
                }
                input {
                    // Enter submits — no separate "Go" button; a browser
                    // address bar doesn't need one alongside it.
                    style: "flex:1;padding:16px 28px;border:1px solid {border};border-radius:999px;background:{bg};font-size:26px;color:{text_color};outline:none;",
                    value: "{address_input}",
                    // Real address-bar behavior: first click (or Cmd/Ctrl+L)
                    // selects the whole URL; a click while already focused
                    // just moves the caret. See the `data-select-all-on-focus`
                    // handling in `set_focus_to`, blitz-dom/src/document.rs —
                    // gated by this attribute so it doesn't affect ordinary
                    // page `<input>`s.
                    "data-select-all-on-focus": "true",
                    oninput: move |evt| address_input.set(evt.value()),
                    onmounted: move |evt| address_input_element.set(Some(evt.data())),
                    onkeydown: move |evt| {
                        if evt.key() == Key::Enter {
                            navigate();
                        }
                    }
                }
                {
                    let active_url = tabs.read().iter().find(|t| t.id == active_id()).map(|t| t.url.clone());
                    let is_bookmarked = active_url.as_ref().is_some_and(|url| bookmarks.read().iter().any(|b| &b.url == url));
                    rsx! {
                        button {
                            style: format!("border:none;background:transparent;color:{};font-size:30px;width:64px;height:64px;border-radius:8px;cursor:pointer;", if is_bookmarked { accent } else { text_color }),
                            onclick: move |_| {
                                let Some(tab) = tabs.read().iter().find(|t| t.id == active_id()).cloned() else { return };
                                let already = bookmarks.read().iter().find(|b| b.url == tab.url).cloned();
                                if let Some(existing) = already {
                                    // Star clicked while already bookmarked: open the
                                    // editor directly, not a silent un-bookmark.
                                    bookmark_name_draft.set(existing.title);
                                    bookmark_folder_draft.set(existing.folder);
                                    bookmark_editing.set(true);
                                } else {
                                    let title = if tab.title.is_empty() { tab.url.clone() } else { tab.title.clone() };
                                    let folder = folders().first().cloned().unwrap_or_else(|| "Bookmarks Bar".to_string());
                                    bookmarks.write().push(Bookmark { title: title.clone(), url: tab.url.clone(), folder: folder.clone(), added_at: chrono::Utc::now().timestamp() });
                                    bookmark_name_draft.set(title);
                                    bookmark_folder_draft.set(folder);
                                    bookmark_editing.set(false);
                                }
                                bookmark_popover_open.set(true);
                            },
                            if is_bookmarked { "★" } else { "☆" }
                        }
                    }
                }
                button {
                    style: "border:none;background:transparent;color:{text_color};font-size:26px;width:64px;height:64px;border-radius:8px;cursor:pointer;",
                    onclick: move |_| { bookmark_manager_open.set(true); bookmark_popover_open.set(false); },
                    title: "Bookmark Manager",
                    "📑"
                }
                button {
                    style: "border:none;background:transparent;color:{text_color};font-size:32px;width:64px;height:64px;border-radius:8px;cursor:pointer;letter-spacing:1px;",
                    onclick: move |_| settings_open.set(!settings_open()),
                    "⋮"
                }
                if bookmark_popover_open() {
                    div {
                        style: "position:absolute;top:60px;right:80px;width:280px;background:{surface};border:1px solid {border};border-radius:10px;box-shadow:0 8px 24px rgba(0,0,0,0.35);padding:14px;z-index:20;font-size:14px;color:{text_color};",
                        if bookmark_editing() {
                            div { style: "font-size:14px;color:{text_muted};text-transform:uppercase;letter-spacing:0.5px;margin-bottom:8px;", "Edit bookmark" }
                            input {
                                style: "width:100%;box-sizing:border-box;padding:8px 10px;border:1px solid {border};border-radius:6px;background:{bg};font-size:14px;color:{text_color};margin-bottom:10px;",
                                value: "{bookmark_name_draft}",
                                oninput: move |evt| bookmark_name_draft.set(evt.value()),
                            }
                            div { style: "font-size:11px;color:{text_muted};margin-bottom:6px;", "Folder" }
                            div {
                                style: "display:flex;gap:6px;margin-bottom:12px;flex-wrap:wrap;",
                                for folder in folders() {
                                    {
                                        let active = bookmark_folder_draft() == folder;
                                        let folder_click = folder.clone();
                                        rsx! {
                                            button {
                                                style: format!("padding:6px 10px;border-radius:6px;border:1px solid {border};background:{};color:{text_color};cursor:pointer;font-size:13px;", if active { accent } else { "transparent" }),
                                                onclick: move |_| bookmark_folder_draft.set(folder_click.clone()),
                                                "{folder}"
                                            }
                                        }
                                    }
                                }
                            }
                            div {
                                style: "display:flex;justify-content:space-between;align-items:center;",
                                button {
                                    style: "border:none;background:transparent;color:#e05555;font-size:13px;cursor:pointer;padding:6px 0;",
                                    onclick: move |_| {
                                        if let Some(tab) = tabs.read().iter().find(|t| t.id == active_id()).cloned() {
                                            bookmarks.write().retain(|b| b.url != tab.url);
                                        }
                                        bookmark_popover_open.set(false);
                                        bookmark_editing.set(false);
                                    },
                                    "Remove"
                                }
                                div {
                                    style: "display:flex;gap:10px;",
                                    button {
                                        style: "border:none;background:transparent;color:{text_muted};font-size:13px;cursor:pointer;padding:6px 10px;",
                                        onclick: move |_| { bookmark_popover_open.set(false); bookmark_editing.set(false); },
                                        "Cancel"
                                    }
                                    button {
                                        style: format!("border:none;background:{accent};color:{text_color};font-size:13px;cursor:pointer;padding:6px 14px;border-radius:6px;"),
                                        onclick: move |_| {
                                            let Some(tab) = tabs.read().iter().find(|t| t.id == active_id()).cloned() else { return };
                                            let name = bookmark_name_draft();
                                            let folder = bookmark_folder_draft();
                                            let mut list = bookmarks.write();
                                            if let Some(b) = list.iter_mut().find(|b| b.url == tab.url) {
                                                b.title = if name.trim().is_empty() { tab.title.clone() } else { name };
                                                b.folder = folder;
                                            }
                                            drop(list);
                                            bookmark_popover_open.set(false);
                                            bookmark_editing.set(false);
                                        },
                                        "Save"
                                    }
                                }
                            }
                        } else {
                            div { style: "font-size:14px;margin-bottom:4px;", "Bookmark added" }
                            div { style: "font-size:13px;color:{text_muted};margin-bottom:2px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;", "{bookmark_name_draft}" }
                            div { style: "font-size:11px;color:{text_muted};margin-bottom:10px;", "Folder: {bookmark_folder_draft}" }
                            div {
                                style: "display:flex;justify-content:flex-end;gap:14px;",
                                button {
                                    style: "border:none;background:transparent;color:{text_color};font-size:13px;cursor:pointer;padding:6px 0;",
                                    onclick: move |_| bookmark_editing.set(true),
                                    "Edit"
                                }
                                button {
                                    style: format!("border:none;background:{accent};color:{text_color};font-size:13px;cursor:pointer;padding:6px 14px;border-radius:6px;"),
                                    onclick: move |_| bookmark_popover_open.set(false),
                                    "Done"
                                }
                            }
                        }
                    }
                }
                if settings_open() {
                    div {
                        style: "position:absolute;top:60px;right:10px;width:300px;background:{surface};border:1px solid {border};border-radius:10px;box-shadow:0 8px 24px rgba(0,0,0,0.35);padding:12px;z-index:10;font-size:14px;color:{text_color};",

                        div { style: "font-size:14px;color:{text_muted};text-transform:uppercase;letter-spacing:0.5px;margin-bottom:6px;", "Appearance" }
                        div {
                            style: "display:flex;gap:6px;margin-bottom:14px;",
                            {
                                let dark_active = theme() == Theme::Dark;
                                let light_active = theme() == Theme::Light;
                                rsx! {
                                    button {
                                        style: format!("flex:1;padding:6px 0;border-radius:6px;border:1px solid {border};background:{};color:{text_color};cursor:pointer;font-size:14px;", if dark_active { accent } else { "transparent" }),
                                        onclick: move |_| theme.set(Theme::Dark),
                                        "Dark"
                                    }
                                    button {
                                        style: format!("flex:1;padding:6px 0;border-radius:6px;border:1px solid {border};background:{};color:{text_color};cursor:pointer;font-size:14px;", if light_active { accent } else { "transparent" }),
                                        onclick: move |_| theme.set(Theme::Light),
                                        "Light"
                                    }
                                }
                            }
                        }

                        div { style: "font-size:14px;color:{text_muted};text-transform:uppercase;letter-spacing:0.5px;margin-bottom:6px;", "Privacy & Security" }
                        div {
                            style: "display:flex;gap:6px;margin-bottom:6px;",
                            {
                                let isolated_active = isolation_mode() == IsolationMode::Isolated;
                                let shared_active = isolation_mode() == IsolationMode::Shared;
                                rsx! {
                                    button {
                                        style: format!("flex:1;padding:6px 0;border-radius:6px;border:1px solid {border};background:{};color:{text_color};cursor:pointer;font-size:14px;", if isolated_active { accent } else { "transparent" }),
                                        onclick: move |_| isolation_mode.set(IsolationMode::Isolated),
                                        "Isolated"
                                    }
                                    button {
                                        style: format!("flex:1;padding:6px 0;border-radius:6px;border:1px solid {border};background:{};color:{text_color};cursor:pointer;font-size:14px;", if shared_active { accent } else { "transparent" }),
                                        onclick: move |_| isolation_mode.set(IsolationMode::Shared),
                                        "Shared"
                                    }
                                }
                            }
                        }
                        div {
                            style: "font-size:10.5px;color:{text_muted};margin-bottom:14px;",
                            "New-tab session isolation. Applies to tabs opened from now on, not tabs already open."
                        }

                        div { style: "font-size:14px;color:{text_muted};text-transform:uppercase;letter-spacing:0.5px;margin-bottom:6px;", "Startup" }
                        div {
                            style: "display:flex;align-items:center;gap:8px;margin-bottom:14px;",
                            {
                                let on = restore_pinned_tabs_setting();
                                rsx! {
                                    button {
                                        style: format!("width:36px;height:20px;border-radius:10px;border:1px solid {border};background:{};cursor:pointer;position:relative;padding:0;", if on { accent } else { "transparent" }),
                                        onclick: move |_| {
                                            let new_value = !restore_pinned_tabs_setting();
                                            restore_pinned_tabs_setting.set(new_value);
                                            persist_pinned_tabs(&tabs.read(), new_value);
                                        },
                                        div {
                                            style: format!("width:14px;height:14px;border-radius:50%;background:{text_color};position:absolute;top:2px;left:{};transition:left 0.1s;", if on { "18px" } else { "2px" }),
                                        }
                                    }
                                    span { style: "font-size:13px;color:{text_color};", "Restore pinned tabs" }
                                }
                            }
                        }
                        div {
                            style: "font-size:10.5px;color:{text_muted};margin-bottom:14px;",
                            "Pinned tabs are remembered across restarts when this is on."
                        }

                        div { style: "font-size:14px;color:{text_muted};text-transform:uppercase;letter-spacing:0.5px;margin-bottom:6px;", "Page zoom" }
                        div {
                            style: "display:flex;align-items:center;gap:6px;margin-bottom:6px;",
                            {
                                let at_min = page_zoom() <= 1.0;
                                rsx! {
                                    button {
                                        style: format!("width:32px;height:32px;border-radius:6px;border:1px solid {border};background:transparent;color:{text_color};font-size:16px;cursor:{};opacity:{};", if at_min { "default" } else { "pointer" }, if at_min { "0.4" } else { "1.0" }),
                                        disabled: at_min,
                                        onclick: move |_| {
                                            let zoom = (page_zoom() - 0.1).max(1.0);
                                            page_zoom.set(zoom);
                                            let shell: Option<Arc<dyn ShellProvider>> = try_consume_context();
                                            if let Some(shell) = shell {
                                                shell.set_zoom(zoom);
                                            }
                                        },
                                        "−"
                                    }
                                    div {
                                        style: "flex:1;text-align:center;font-size:14px;color:{text_color};",
                                        {format!("{}%", (page_zoom() * 100.0).round() as i32)}
                                    }
                                    button {
                                        style: format!("width:32px;height:32px;border-radius:6px;border:1px solid {border};background:transparent;color:{text_color};font-size:16px;cursor:pointer;"),
                                        onclick: move |_| {
                                            let zoom = page_zoom() + 0.1;
                                            page_zoom.set(zoom);
                                            let shell: Option<Arc<dyn ShellProvider>> = try_consume_context();
                                            if let Some(shell) = shell {
                                                shell.set_zoom(zoom);
                                            }
                                        },
                                        "+"
                                    }
                                }
                            }
                        }
                        div {
                            style: "font-size:10.5px;color:{text_muted};margin-bottom:14px;",
                            "Zooms loaded page content only — the address bar and tabs are unaffected. 100% is as small as it goes."
                        }

                        div { style: "font-size:14px;color:{text_muted};text-transform:uppercase;letter-spacing:0.5px;margin-bottom:6px;", "Privacy & Storage" }
                        div {
                            style: "display:flex;align-items:center;gap:8px;margin-bottom:6px;",
                            button {
                                style: "padding:6px 12px;border-radius:6px;border:1px solid {border};background:transparent;color:{text_color};cursor:pointer;font-size:13px;",
                                onclick: move |_| {
                                    let result = himalayas::net_cache::clear_http_cache();
                                    cache_clear_status.set(Some(if result.is_ok() { "Cache cleared".to_string() } else { "Failed to clear cache".to_string() }));
                                },
                                "Clear cache"
                            }
                            if let Some(status) = cache_clear_status() {
                                span { style: "font-size:12px;color:{text_muted};", "{status}" }
                            }
                        }
                        div {
                            style: "font-size:10.5px;color:{text_muted};margin-bottom:14px;",
                            "Clears the on-disk HTTP cache for page navigations (not subresources like images/CSS/fonts, and not bookmarks/pinned tabs)."
                        }

                        div { style: "font-size:14px;color:{text_muted};text-transform:uppercase;letter-spacing:0.5px;margin-bottom:6px;", "About" }
                        div {
                            style: "color:{text_muted};",
                            {format!("Device tier: {:?}", detected_device_tier())}
                        }
                    }
                }
            }

            // Bookmark Manager — a bigger, separate view from the star
            // button's quick add/edit popover above. Flat (single-level)
            // folders with search/sort/rename/delete, HTML (Netscape
            // format, universal browser compatibility) and JSON
            // (Himalayas-native backup) import/export, drag-a-bookmark-onto-
            // a-folder-header to move it, and checkbox multi-select with
            // bulk move/delete. Not built: true manual same-folder
            // reordering (would need a "custom" sort mode plus a persisted
            // per-bookmark order field, conflicting with the existing
            // sort-by-name/date modes — a separate, real follow-up) and
            // nested folder trees.
            if bookmark_manager_open() {
                div {
                    style: "position:fixed;inset:0;background:rgba(0,0,0,0.5);z-index:50;display:flex;align-items:center;justify-content:center;",
                    onclick: move |_| bookmark_manager_open.set(false),
                    div {
                        style: "width:720px;max-width:92vw;height:640px;max-height:88vh;background:{surface};border:1px solid {border};border-radius:12px;box-shadow:0 16px 48px rgba(0,0,0,0.5);display:flex;flex-direction:column;overflow:hidden;",
                        onclick: move |evt| evt.stop_propagation(),

                        div {
                            style: "display:flex;align-items:center;justify-content:space-between;padding:14px 18px;border-bottom:1px solid {border};",
                            div { style: "font-size:16px;font-weight:600;color:{text_color};", "Bookmark Manager" }
                            button {
                                style: "border:none;background:transparent;color:{text_muted};font-size:20px;cursor:pointer;",
                                onclick: move |_| bookmark_manager_open.set(false),
                                "×"
                            }
                        }

                        div {
                            style: "display:flex;gap:8px;align-items:center;padding:10px 18px;border-bottom:1px solid {border};flex-wrap:wrap;",
                            input {
                                style: "flex:1;min-width:160px;padding:7px 10px;border:1px solid {border};border-radius:6px;background:{bg};color:{text_color};font-size:13px;",
                                placeholder: "Search bookmarks",
                                value: "{bookmark_manager_search}",
                                oninput: move |evt| bookmark_manager_search.set(evt.value()),
                            }
                            button {
                                style: "padding:7px 10px;border-radius:6px;border:1px solid {border};background:transparent;color:{text_color};cursor:pointer;font-size:13px;",
                                onclick: move |_| bookmark_manager_sort_by_date.set(!bookmark_manager_sort_by_date()),
                                {if bookmark_manager_sort_by_date() { "Sort: Date added" } else { "Sort: Name" }}
                            }
                            input {
                                style: "width:130px;padding:7px 10px;border:1px solid {border};border-radius:6px;background:{bg};color:{text_color};font-size:13px;",
                                placeholder: "New folder",
                                value: "{new_folder_draft}",
                                oninput: move |evt| new_folder_draft.set(evt.value()),
                                onkeydown: move |evt| {
                                    if evt.key() == Key::Enter {
                                        let name = new_folder_draft().trim().to_string();
                                        if !name.is_empty() && !folders().contains(&name) {
                                            folders.write().push(name);
                                        }
                                        new_folder_draft.set(String::new());
                                    }
                                }
                            }
                            button {
                                style: format!("padding:7px 12px;border-radius:6px;border:none;background:{accent};color:{text_color};cursor:pointer;font-size:13px;"),
                                onclick: move |_| {
                                    let name = new_folder_draft().trim().to_string();
                                    if !name.is_empty() && !folders().contains(&name) {
                                        folders.write().push(name);
                                    }
                                    new_folder_draft.set(String::new());
                                },
                                "+ Folder"
                            }
                        }

                        div {
                            style: "display:flex;gap:8px;padding:10px 18px;border-bottom:1px solid {border};",
                            button {
                                style: "flex:1;padding:8px 0;border-radius:6px;border:1px solid {border};background:transparent;color:{text_color};cursor:pointer;font-size:13px;",
                                onclick: move |_| {
                                    if let Some(path) = rfd::FileDialog::new().add_filter("Bookmarks", &["html", "htm", "json"]).pick_file() {
                                        match std::fs::read_to_string(&path) {
                                            Ok(content) => {
                                                let is_json = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json"));
                                                let imported = if is_json { parse_bookmarks_json(&content) } else { parse_bookmarks_html(&content) };
                                                let existing = bookmarks.read().clone();
                                                let existing_folders = folders.read().clone();
                                                let result = merge_imported_bookmarks(imported, &existing, &existing_folders);
                                                let imported_count = result.bookmarks.len();
                                                let new_folder_count = result.new_folders.len();
                                                let skipped = result.duplicates_skipped;
                                                folders.write().extend(result.new_folders);
                                                bookmarks.write().extend(result.bookmarks);
                                                import_export_status.set(Some(format!("{imported_count} bookmarks imported, {new_folder_count} folders imported, {skipped} duplicates skipped")));
                                            }
                                            Err(e) => import_export_status.set(Some(format!("Import failed: {e}"))),
                                        }
                                    }
                                },
                                "Import…"
                            }
                            button {
                                style: "flex:1;padding:8px 0;border-radius:6px;border:1px solid {border};background:transparent;color:{text_color};cursor:pointer;font-size:13px;",
                                onclick: move |_| {
                                    let list = bookmarks.read().clone();
                                    let folder_list = folders.read().clone();
                                    if let Some(path) = rfd::FileDialog::new().set_file_name("Himalayas Bookmarks.html").add_filter("HTML", &["html"]).save_file() {
                                        let content = export_bookmarks_html(&list, &folder_list);
                                        match std::fs::write(&path, content) {
                                            Ok(_) => import_export_status.set(Some(format!("Exported {} bookmarks to {}", list.len(), path.display()))),
                                            Err(e) => import_export_status.set(Some(format!("Export failed: {e}"))),
                                        }
                                    }
                                },
                                "Export HTML…"
                            }
                            button {
                                style: "flex:1;padding:8px 0;border-radius:6px;border:1px solid {border};background:transparent;color:{text_color};cursor:pointer;font-size:13px;",
                                onclick: move |_| {
                                    let list = bookmarks.read().clone();
                                    if let Some(path) = rfd::FileDialog::new().set_file_name("Himalayas Bookmarks.json").add_filter("JSON", &["json"]).save_file() {
                                        let content = export_bookmarks_json(&list);
                                        match std::fs::write(&path, content) {
                                            Ok(_) => import_export_status.set(Some(format!("Exported {} bookmarks to {}", list.len(), path.display()))),
                                            Err(e) => import_export_status.set(Some(format!("Export failed: {e}"))),
                                        }
                                    }
                                },
                                "Export JSON…"
                            }
                        }

                        if let Some(status) = import_export_status() {
                            div {
                                style: "padding:8px 18px;font-size:12px;color:{text_muted};background:{bg};display:flex;justify-content:space-between;align-items:center;",
                                span { "{status}" }
                                button {
                                    style: "border:none;background:transparent;color:{text_muted};cursor:pointer;font-size:12px;",
                                    onclick: move |_| import_export_status.set(None),
                                    "Dismiss"
                                }
                            }
                        }

                        if !selected_bookmarks().is_empty() {
                            div {
                                style: "display:flex;align-items:center;gap:8px;padding:8px 18px;border-bottom:1px solid {border};background:{bg};",
                                span { style: "font-size:12px;color:{text_muted};", {format!("{} selected", selected_bookmarks().len())} }
                                span { style: "font-size:11px;color:{text_muted};", "Move to:" }
                                div {
                                    style: "display:flex;gap:4px;flex:1;flex-wrap:wrap;",
                                    // Folder buttons directly (not a `<select>`) — matches the
                                    // already-proven folder-picker pattern from the star
                                    // popover's editor, rather than a native `<select>` this
                                    // codebase hasn't otherwise exercised.
                                    for folder in folders() {
                                        {
                                            let folder_click = folder.clone();
                                            rsx! {
                                                button {
                                                    style: "padding:4px 8px;border-radius:6px;border:1px solid {border};background:transparent;color:{text_color};cursor:pointer;font-size:11px;",
                                                    onclick: move |_| {
                                                        let target = folder_click.clone();
                                                        let selected = selected_bookmarks();
                                                        for b in bookmarks.write().iter_mut() {
                                                            if selected.contains(&b.url) { b.folder = target.clone(); }
                                                        }
                                                        selected_bookmarks.write().clear();
                                                    },
                                                    "{folder}"
                                                }
                                            }
                                        }
                                    }
                                }
                                button {
                                    style: "padding:5px 10px;border-radius:6px;border:1px solid {border};background:transparent;color:#e05555;cursor:pointer;font-size:12px;",
                                    onclick: move |_| {
                                        let selected = selected_bookmarks();
                                        bookmarks.write().retain(|b| !selected.contains(&b.url));
                                        selected_bookmarks.write().clear();
                                    },
                                    "Delete"
                                }
                                button {
                                    style: "padding:5px 10px;border-radius:6px;border:none;background:transparent;color:{text_muted};cursor:pointer;font-size:12px;",
                                    onclick: move |_| selected_bookmarks.write().clear(),
                                    "Clear"
                                }
                            }
                        }

                        div {
                            style: "flex:1;overflow:auto;padding:8px 18px 18px;",
                            onmouseup: move |_| dragging_bookmark.set(None),
                            {
                                let search = bookmark_manager_search().to_lowercase();
                                let sort_by_date = bookmark_manager_sort_by_date();
                                let all = bookmarks.read().clone();
                                let folder_list = folders.read().clone();
                                rsx! {
                                    for folder in folder_list.clone() {
                                        {
                                            let mut items: Vec<Bookmark> = all.iter()
                                                .filter(|b| b.folder == folder)
                                                .filter(|b| search.is_empty() || b.title.to_lowercase().contains(&search) || b.url.to_lowercase().contains(&search))
                                                .cloned()
                                                .collect();
                                            if sort_by_date {
                                                items.sort_by_key(|b| std::cmp::Reverse(b.added_at));
                                            } else {
                                                items.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
                                            }
                                            let folder_for_rename = folder.clone();
                                            let folder_for_rename_start = folder.clone();
                                            let folder_for_delete = folder.clone();
                                            let folder_for_drop = folder.clone();
                                            let is_renaming = renaming_folder() == Some(folder.clone());
                                            let can_delete = folder_list.len() > 1;
                                            let item_count = items.len();
                                            // Drop target: dragging a bookmark row and releasing over
                                            // this folder's header moves it here — see `dragging_bookmark`
                                            // and each row's `onmousedown` below. Highlighted only while
                                            // a drag targeting a *different* folder is in progress, so it
                                            // doesn't visually flicker when hovering the bookmark's own folder.
                                            let drop_highlight = dragging_bookmark().is_some_and(|url| {
                                                all.iter().find(|b| b.url == url).is_some_and(|b| b.folder != folder)
                                            });
                                            rsx! {
                                                div {
                                                    key: "{folder}",
                                                    style: "margin-bottom:16px;",
                                                    div {
                                                        style: format!("display:flex;align-items:center;gap:8px;margin-bottom:6px;padding:2px 4px;border-radius:6px;{}", if drop_highlight { format!("background:{accent};") } else { String::new() }),
                                                        onmouseup: move |evt| {
                                                            evt.stop_propagation();
                                                            if let Some(url) = dragging_bookmark() {
                                                                let target = folder_for_drop.clone();
                                                                if let Some(b) = bookmarks.write().iter_mut().find(|b| b.url == url) {
                                                                    b.folder = target;
                                                                }
                                                            }
                                                            dragging_bookmark.set(None);
                                                        },
                                                        if is_renaming {
                                                            input {
                                                                style: "flex:1;padding:5px 8px;border:1px solid {border};border-radius:6px;background:{bg};color:{text_color};font-size:13px;",
                                                                value: "{rename_folder_draft}",
                                                                oninput: move |evt| rename_folder_draft.set(evt.value()),
                                                                onkeydown: move |evt| {
                                                                    if evt.key() == Key::Enter {
                                                                        let new_name = rename_folder_draft().trim().to_string();
                                                                        if !new_name.is_empty() {
                                                                            let old = folder_for_rename.clone();
                                                                            if let Some(f) = folders.write().iter_mut().find(|f| **f == old) {
                                                                                *f = new_name.clone();
                                                                            }
                                                                            for b in bookmarks.write().iter_mut() {
                                                                                if b.folder == old { b.folder = new_name.clone(); }
                                                                            }
                                                                        }
                                                                        renaming_folder.set(None);
                                                                    } else if evt.key() == Key::Escape {
                                                                        renaming_folder.set(None);
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            div {
                                                                style: "flex:1;font-size:13px;font-weight:600;color:{text_muted};text-transform:uppercase;letter-spacing:0.4px;cursor:pointer;",
                                                                onclick: move |_| {
                                                                    rename_folder_draft.set(folder_for_rename_start.clone());
                                                                    renaming_folder.set(Some(folder_for_rename_start.clone()));
                                                                },
                                                                "{folder} ({item_count})"
                                                            }
                                                        }
                                                        if can_delete {
                                                            button {
                                                                style: "border:none;background:transparent;color:{text_muted};font-size:12px;cursor:pointer;",
                                                                onclick: move |_| {
                                                                    let old = folder_for_delete.clone();
                                                                    let fallback = folders.read().iter().find(|f| **f != old).cloned().unwrap_or_else(|| "Other Bookmarks".to_string());
                                                                    folders.write().retain(|f| *f != old);
                                                                    for b in bookmarks.write().iter_mut() {
                                                                        if b.folder == old { b.folder = fallback.clone(); }
                                                                    }
                                                                },
                                                                "Delete folder"
                                                            }
                                                        }
                                                    }
                                                    for b in items {
                                                        {
                                                            let url_for_row = b.url.clone();
                                                            let url_for_new_tab = b.url.clone();
                                                            let url_for_edit = b.url.clone();
                                                            let url_for_save = b.url.clone();
                                                            let url_for_delete = b.url.clone();
                                                            let title_for_edit = b.title.clone();
                                                            let is_editing = bookmark_manager_editing_url() == Some(b.url.clone());
                                                            let url_for_select = b.url.clone();
                                                            let url_for_drag = b.url.clone();
                                                            let is_selected = selected_bookmarks().contains(&b.url);
                                                            let being_dragged = dragging_bookmark() == Some(b.url.clone());
                                                            rsx! {
                                                                div {
                                                                    key: "{b.url}",
                                                                    style: format!("display:flex;align-items:center;gap:8px;padding:6px 4px;border-radius:6px;cursor:grab;{}", if being_dragged { "opacity:0.5;" } else { "" }),
                                                                    onmousedown: move |_| dragging_bookmark.set(Some(url_for_drag.clone())),
                                                                    input {
                                                                        r#type: "checkbox",
                                                                        checked: is_selected,
                                                                        onclick: move |evt| {
                                                                            evt.stop_propagation();
                                                                            let mut sel = selected_bookmarks.write();
                                                                            if sel.contains(&url_for_select) {
                                                                                sel.remove(&url_for_select);
                                                                            } else {
                                                                                sel.insert(url_for_select.clone());
                                                                            }
                                                                        },
                                                                    }
                                                                    if is_editing {
                                                                        input {
                                                                            style: "flex:1;padding:5px 8px;border:1px solid {border};border-radius:6px;background:{bg};color:{text_color};font-size:13px;",
                                                                            value: "{bookmark_manager_editing_draft}",
                                                                            oninput: move |evt| bookmark_manager_editing_draft.set(evt.value()),
                                                                            onkeydown: move |evt| {
                                                                                if evt.key() == Key::Enter {
                                                                                    let new_name = bookmark_manager_editing_draft().trim().to_string();
                                                                                    let mut list = bookmarks.write();
                                                                                    if let Some(bm) = list.iter_mut().find(|x| x.url == url_for_save) {
                                                                                        if !new_name.is_empty() { bm.title = new_name; }
                                                                                    }
                                                                                    drop(list);
                                                                                    bookmark_manager_editing_url.set(None);
                                                                                } else if evt.key() == Key::Escape {
                                                                                    bookmark_manager_editing_url.set(None);
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        div {
                                                                            style: "flex:1;overflow:hidden;cursor:pointer;",
                                                                            onclick: move |_| {
                                                                                navigate_tab(tabs, active_id(), url_for_row.clone(), true);
                                                                                address_input.set(url_for_row.clone());
                                                                                bookmark_manager_open.set(false);
                                                                            },
                                                                            div { style: "font-size:13px;color:{text_color};overflow:hidden;text-overflow:ellipsis;white-space:nowrap;", "{b.title}" }
                                                                            div { style: "font-size:11px;color:{text_muted};overflow:hidden;text-overflow:ellipsis;white-space:nowrap;", "{b.url}" }
                                                                        }
                                                                        button {
                                                                            style: "border:none;background:transparent;color:{text_muted};font-size:14px;cursor:pointer;padding:2px 4px;",
                                                                            title: "Open in new tab",
                                                                            onclick: move |_| {
                                                                                let id = next_id();
                                                                                next_id.set(id + 1);
                                                                                tabs.write().push(Tab::new(id, &url_for_new_tab, isolation_mode()));
                                                                                active_id.set(id);
                                                                                address_input.set(url_for_new_tab.clone());
                                                                                bookmark_manager_open.set(false);
                                                                            },
                                                                            "↗"
                                                                        }
                                                                        button {
                                                                            style: "border:none;background:transparent;color:{text_muted};font-size:14px;cursor:pointer;padding:2px 4px;",
                                                                            title: "Rename",
                                                                            onclick: move |_| {
                                                                                bookmark_manager_editing_draft.set(title_for_edit.clone());
                                                                                bookmark_manager_editing_url.set(Some(url_for_edit.clone()));
                                                                            },
                                                                            "✎"
                                                                        }
                                                                        button {
                                                                            style: "border:none;background:transparent;color:#e05555;font-size:14px;cursor:pointer;padding:2px 4px;",
                                                                            title: "Delete",
                                                                            onclick: move |_| {
                                                                                bookmarks.write().retain(|x| x.url != url_for_delete);
                                                                            },
                                                                            "×"
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Status line
            div {
                style: "padding:8px 14px;font-size:13px;color:{text_muted};background:{surface};border-bottom:1px solid {border};",
                {tabs.read().iter().find(|t| t.id == active_id()).map(|t| t.status.clone()).unwrap_or_default()}
            }

            // Content
            div {
                style: "flex:1;overflow:auto;background:{surface};",
                if let Some(doc) = tabs.read().iter().find(|t| t.id == active_id()).and_then(|t| t.document.clone()) {
                    web-view {
                        key: "{active_id()}",
                        style: "display:block;width:100%;height:100%;",
                        "__webview_document": doc,
                    }
                } else {
                    div {
                        style: "display:flex;align-items:center;justify-content:center;height:100%;color:{text_muted};font-size:13px;",
                        "No page loaded in this tab yet"
                    }
                }
            }

            // Right-click menu (see `ContextMenuRequest`/`ScriptEventHandler`).
            // `request.x/y` are the click's coordinates *within the loaded
            // page*, not the outer window — `CONTENT_AREA_TOP` approximates
            // where the content area starts below the fixed-height shell
            // rows above it (tab strip + address bar + status line) rather
            // than threading the `<web-view>`'s exact bounding rect all the
            // way out here. Good enough to land near the cursor, not
            // pixel-exact.
            if let Some(request) = context_menu() {
                {
                    const CONTENT_AREA_TOP: f32 = 122.0;
                    let has_history = tabs.read().iter().find(|t| t.id == active_id()).is_some_and(|t| !t.history.is_empty());
                    let href = request.href.clone();
                    let item_style = format!("display:block;width:100%;text-align:left;padding:9px 12px;border:none;background:transparent;color:{text_color};font-size:14px;border-radius:6px;cursor:pointer;");
                    let disabled_item_style = format!("display:block;width:100%;text-align:left;padding:9px 12px;border:none;background:transparent;color:{text_muted};font-size:14px;border-radius:6px;cursor:default;opacity:0.5;");
                    rsx! {
                        div {
                            style: "position:absolute;top:{request.y + CONTENT_AREA_TOP}px;left:{request.x}px;width:220px;background:{surface};border:1px solid {border};border-radius:8px;box-shadow:0 8px 24px rgba(0,0,0,0.35);padding:6px;z-index:30;",
                            button {
                                style: if has_history { "{item_style}" } else { "{disabled_item_style}" },
                                disabled: !has_history,
                                onclick: move |_| { go_back(); context_menu.set(None); },
                                "Back"
                            }
                            button {
                                style: "{item_style}",
                                onclick: move |_| { refresh(); context_menu.set(None); },
                                "Reload"
                            }
                            if let Some(href) = href {
                                button {
                                    style: "{item_style}",
                                    onclick: move |_| {
                                        let shell: Option<Arc<dyn ShellProvider>> = try_consume_context();
                                        if let Some(shell) = shell {
                                            let _ = shell.set_clipboard_text(href.clone());
                                        }
                                        context_menu.set(None);
                                    },
                                    "Copy link address"
                                }
                            }
                            div { style: "height:1px;background:{border};margin:4px 2px;" }
                            button {
                                style: "{item_style}",
                                onclick: move |_| context_menu.set(None),
                                "Close"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Built-in test page exercising the real v0 DOM binding surface —
/// `getElementById`, `querySelector`/`querySelectorAll`, `textContent`,
/// `innerHTML`, `style.setProperty`, `classList`, `createElement`/
/// `appendChild`/`removeChild`, `addEventListener`, `console.log`,
/// `setTimeout` — see docs/NATIVE_RENDERING_PLAN.md, Phase 3/8.
const JS_TEST_PAGE: &str = r##"<html><body style="font-family: sans-serif; padding: 40px;">
    <h2>himalayas://test</h2>
    <p>Exercises the real DOM binding surface.</p>

    <button id="onclick-btn" onclick="handleOnclick()" style="padding: 8px 16px;">onclick attribute</button>
    <button id="listener-btn" style="padding: 8px 16px; margin-left: 8px;">addEventListener</button>
    <p id="out">Not clicked yet</p>
    <p id="counter" class="badge">Count: 0</p>

    <p>querySelector/querySelectorAll (no id= attributes, top-level script — proves live reads work outside event handlers too):</p>
    <button class="qs-target" style="padding: 8px 16px;">first .qs-target</button>
    <button class="qs-target" style="padding: 8px 16px; margin-left: 8px;">second .qs-target</button>
    <p id="qs-out">querySelectorAll not run yet</p>

    <p>classList/style seeding (this element already has "existing" class and a red background from HTML, not JS — clicking must NOT lose either):</p>
    <button id="seed-btn" class="existing" style="padding: 8px 16px; background: #fdd;">toggle "added" class + blue text</button>
    <p id="seed-out">Not clicked yet. class=existing, style has background set from HTML.</p>

    <p>createElement/appendChild/removeChild/innerHTML:</p>
    <button id="build-btn" style="padding: 8px 16px;">build a list item</button>
    <button id="clear-btn" style="padding: 8px 16px; margin-left: 8px;">clear via innerHTML</button>
    <ul id="list"></ul>

    <p>requestAnimationFrame (the box should slide smoothly while running, not jump):</p>
    <button id="raf-start-btn" style="padding: 8px 16px;">Start animation</button>
    <button id="raf-stop-btn" style="padding: 8px 16px; margin-left: 8px;">Stop animation</button>
    <p id="raf-out">Not started. Frames: 0</p>
    <div id="raf-box" style="width: 24px; height: 24px; background: #5b8def; margin-left: 0px;"></div>

    <script>
        console.log('himalayas://test: initial script running');

        var count = 0;

        function handleOnclick() {
            var out = document.getElementById('out');
            out.textContent = 'onclick handler ran, prior text was: "' + out.textContent + '"';
            out.style.setProperty('color', 'green');
            out.classList.add('done');
        }

        var listenerBtn = document.getElementById('listener-btn');
        listenerBtn.addEventListener('click', function() {
            count = count + 1;
            var counter = document.getElementById('counter');
            counter.textContent = 'Count: ' + count;
            if (counter.classList.toggle('badge')) {
                counter.style.setProperty('color', 'blue');
            } else {
                counter.style.setProperty('color', 'red');
            }
        });

        // Top-level querySelector/querySelectorAll — not inside any event
        // handler, proves live document reads work during initial script
        // evaluation, not just later during a click.
        var targets = document.querySelectorAll('.qs-target');
        document.getElementById('qs-out').textContent = 'querySelectorAll found ' + targets.length + ' elements';
        for (var i = 0; i < targets.length; i++) {
            targets[i].addEventListener('click', function() {
                document.getElementById('qs-out').textContent = 'a .qs-target button was clicked (via querySelectorAll + addEventListener)';
            });
        }

        // classList.add/style.setProperty must not discard the "existing"
        // class or the HTML-set background — proves the mirror-seeding fix
        // (Phase 8) rather than the old wipe-on-first-write bug.
        var seedBtn = document.getElementById('seed-btn');
        seedBtn.addEventListener('click', function() {
            seedBtn.classList.toggle('added');
            seedBtn.style.setProperty('color', 'blue');
            document.getElementById('seed-out').textContent =
                'has "existing" class: ' + seedBtn.classList.contains('existing') +
                ', has "added" class: ' + seedBtn.classList.contains('added');
        });

        var listCounter = 0;
        document.getElementById('build-btn').addEventListener('click', function() {
            listCounter = listCounter + 1;
            var item = document.createElement('li');
            item.textContent = 'item ' + listCounter + ' (via createElement)';
            document.getElementById('list').appendChild(item);
        });
        document.getElementById('clear-btn').addEventListener('click', function() {
            document.getElementById('list').innerHTML = '<li>cleared via innerHTML =</li>';
        });

        setTimeout(function() {
            console.log('himalayas://test: setTimeout callback ran');
        }, 100);

        // requestAnimationFrame: a self-requeuing tick loop, same pattern
        // real sites use for smooth JS-driven animation. Verifies both the
        // Phase-8-adjacent binding itself and the vendor/blitz per-frame
        // sub-document hook it depends on.
        var rafId = null;
        var frame = 0;
        function rafTick() {
            frame = frame + 1;
            var pos = frame % 200;
            document.getElementById('raf-box').style.setProperty('margin-left', pos + 'px');
            document.getElementById('raf-out').textContent = 'Running. Frames: ' + frame;
            rafId = requestAnimationFrame(rafTick);
        }
        document.getElementById('raf-start-btn').addEventListener('click', function() {
            if (rafId === null) {
                rafId = requestAnimationFrame(rafTick);
            }
        });
        document.getElementById('raf-stop-btn').addEventListener('click', function() {
            if (rafId !== null) {
                cancelAnimationFrame(rafId);
                rafId = null;
                document.getElementById('raf-out').textContent = 'Stopped. Frames: ' + frame;
            }
        });
    </script>
</body></html>"##;

/// Real Cache-Control/ETag/Last-Modified-aware disk caching for the
/// top-level document fetch (see `fetch_document_with_session`'s own doc
/// comment for why it can't just use `blitz_net::Provider`) — previously
/// none: every navigation hit the network regardless of cache headers.
/// Built via `reqwest_middleware::reqwest` specifically, not this crate's
/// own `reqwest` dependency — see `himalayas::net_cache::cached_client`'s
/// doc comment for why the two aren't interchangeable.
fn http_client() -> &'static reqwest_middleware::ClientWithMiddleware {
    static CLIENT: OnceLock<reqwest_middleware::ClientWithMiddleware> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let base = reqwest_middleware::reqwest::Client::builder().build().expect("building reqwest client");
        himalayas::net_cache::cached_client(base)
    })
}

/// `name=value` from a single raw `Set-Cookie` header value (the part before
/// the first `;` — attributes like `Path`/`HttpOnly`/`Expires` are ignored,
/// same v0 scope as `Session`'s cookie jar itself, which is a flat
/// name→value map with no attribute/expiry tracking).
fn parse_set_cookie_pair(raw: &str) -> Option<(String, String)> {
    let (name, value) = raw.split(';').next()?.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), value.trim().to_string()))
}

/// Fetch the top-level document with `session`'s cookies attached, and store
/// any `Set-Cookie` response headers back into it — the piece of Phase 7
/// (docs/NATIVE_RENDERING_PLAN.md) that makes `IsolationMode::Isolated` vs.
/// `Shared` an observable difference in the native shell (e.g. logging in on
/// one tab and checking whether an isolated sibling tab sees it) rather than
/// just backend bookkeeping nothing reads.
///
/// Uses `reqwest` directly rather than the `blitz_net::Provider` used
/// elsewhere in this file (still passed to `DocumentConfig` for sub-resource
/// fetches — images/CSS/fonts, which don't need cookie handling here):
/// `Provider::fetch_async` only returns `(resolved_url, bytes)`, with no way
/// to read response headers to extract `Set-Cookie`.
async fn fetch_document_with_session(url: &str, session: &Session) -> Result<(String, String), String> {
    let mut req = http_client().get(url);
    let cookies = session.get_cookies();
    if !cookies.is_empty() {
        let header = cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");
        req = req.header(reqwest::header::COOKIE, header);
    }

    let resp = req.send().await.map_err(|e| format!("fetch failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let resolved_url = resp.url().to_string();
    for raw in resp.headers().get_all(reqwest::header::SET_COOKIE) {
        if let Some((name, value)) = raw.to_str().ok().and_then(parse_set_cookie_pair) {
            session.set_cookie(name, value);
        }
    }

    let bytes = resp.bytes().await.map_err(|e| format!("reading body failed: {e}"))?;
    Ok((resolved_url, String::from_utf8_lossy(&bytes).to_string()))
}

/// Page title for a tab: `Ok` carries the loaded document plus a display
/// title (the page's `<title>`, falling back to the URL's host, falling back
/// to the raw URL — never the full URL when a better name is available, so
/// the tab strip doesn't just repeat what's already in the address bar).
async fn load_page(raw_url: &str, session_id: &str) -> Result<(SubDocumentAttr, String), String> {
    if raw_url == "himalayas://test" {
        return build_scripted_document(JS_TEST_PAGE, None, "himalayas://test").await;
    }

    let url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        raw_url.to_string()
    } else {
        format!("https://{raw_url}")
    };

    let session = browser().get_session(session_id).ok_or_else(|| "tab session missing".to_string())?;
    let (resolved_url, html) = fetch_document_with_session(&url, &session).await?;
    session.set_current_url(resolved_url.clone());

    build_scripted_document(&html, Some(resolved_url.clone()), &resolved_url).await
}

/// Fallback display name when the page has no `<title>`: the URL's host
/// (e.g. "example.com"), or the raw string if it doesn't parse as a URL.
fn fallback_title(raw_url: &str) -> String {
    blitz_traits::net::Url::parse(raw_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| raw_url.to_string())
}

async fn build_scripted_document(
    html: &str,
    base_url: Option<String>,
    fallback_url: &str,
) -> Result<(SubDocumentAttr, String), String> {
    let net_provider = Arc::new(NetProvider::new(None));

    // Fresh JS scope + DOM binding state per navigation (see module-level
    // "known limitation" note re: per-tab isolation still being unbuilt).
    reset_js_bindings();

    let mut font_ctx = FontContext::default();
    font_ctx.collection.register_fonts(
        linebender_resource_handle::Blob::new(Arc::new(blitz_dom::BULLET_FONT) as _),
        None,
    );

    let shell_provider = consume_context::<Arc<dyn ShellProvider>>();

    let config = DocumentConfig {
        base_url,
        net_provider: Some(net_provider as _),
        shell_provider: Some(shell_provider),
        html_parser_provider: Some(Arc::new(HtmlProvider)),
        navigation_provider: Some(Arc::new(SubDocNavigationProvider)),
        font_ctx: Some(font_ctx),
        ..Default::default()
    };

    let mut base = HtmlDocument::from_html(html, config).into_inner();

    // Run initial inline <script>s with the document already built, so
    // top-level `document.getElementById(...)` (an extremely common pattern
    // — e.g. `var btn = document.getElementById('x'); btn.addEventListener(...)`
    // executed directly in a <script> body, not inside a later event handler)
    // resolves for real instead of always returning null.
    let scripts = extract_inline_scripts(&base);
    run_js_with_live_doc(&mut base, || {
        for script in scripts {
            JS.with(|ctx| {
                let _ = ctx.borrow_mut().eval(Source::from_bytes(script.as_bytes()));
            });
        }
    });

    let title = base
        .find_title_node()
        .map(|n| n.text_content())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| fallback_title(fallback_url));

    Ok((SubDocumentAttr::from_document(ScriptedDocument { base }), title))
}

/// Phase 6 device-tier gating (docs/NATIVE_RENDERING_PLAN.md): this binary's
/// dependency tree — winit, wgpu, vello, stylo, boa — is meaningfully heavier
/// than the `/app` web shell it sits alongside, which already gates itself on
/// device tier (see `daemon::tier_supports_desktop_features` in
/// `src/daemon/mod.rs`; same Standard-and-above threshold reused here so the
/// two UI tracks agree on what counts as "capable enough"). Unlike the daemon,
/// this binary is launched directly by a human running it, not auto-started —
/// so a below-tier device gets a clear refusal-with-explanation and an escape
/// hatch (`--force`), not a silent slow launch.
fn tier_supports_native_shell(tier: DeviceTier) -> bool {
    matches!(tier, DeviceTier::Standard | DeviceTier::HighCapability | DeviceTier::UltraCapability)
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let force = std::env::args().skip(1).any(|a| a == "--force");
    let tier = detected_device_tier();

    if !force && !tier_supports_native_shell(tier) {
        eprintln!(
            "himalayas-desktop: this device's detected tier ({tier:?}) is below what the \
             native GPU-rendered shell (winit/wgpu/vello/stylo/boa) needs to run well. Use the \
             lighter web shell instead (`himalayas --ui`), or pass --force to launch anyway."
        );
        std::process::exit(1);
    }

    // Plain `dioxus_native::launch(app)` defaults the window title to
    // "Dioxus App" (its own hardcoded fallback when no Dioxus.toml
    // `app_title` is configured — see `dioxus-native/src/config.rs`) — a
    // real user-visible identity bug, not cosmetic. `launch_cfg` is the
    // same entry point with a config slot for exactly this.
    let window_attributes = dioxus_native::WindowAttributes::default().with_title("Himalayas Browser");
    dioxus_native::launch_cfg(app, vec![], vec![Box::new(window_attributes)]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_set_cookie() {
        assert_eq!(
            parse_set_cookie_pair("session_id=abc123; Path=/; HttpOnly"),
            Some(("session_id".to_string(), "abc123".to_string()))
        );
    }

    #[test]
    fn parses_set_cookie_with_no_attributes() {
        assert_eq!(parse_set_cookie_pair("id=42"), Some(("id".to_string(), "42".to_string())));
    }

    #[test]
    fn rejects_set_cookie_with_no_name() {
        assert_eq!(parse_set_cookie_pair("=novalue"), None);
        assert_eq!(parse_set_cookie_pair("not-a-cookie"), None);
    }

    #[test]
    fn tier_gating_matches_standard_and_above() {
        assert!(tier_supports_native_shell(DeviceTier::Standard));
        assert!(tier_supports_native_shell(DeviceTier::HighCapability));
        assert!(tier_supports_native_shell(DeviceTier::UltraCapability));
        assert!(!tier_supports_native_shell(DeviceTier::LowMemory));
        assert!(!tier_supports_native_shell(DeviceTier::Constrained));
    }

    fn sample_bookmark(title: &str, url: &str, folder: &str) -> Bookmark {
        Bookmark { title: title.to_string(), url: url.to_string(), folder: folder.to_string(), added_at: 1_700_000_000 }
    }

    #[test]
    fn html_export_round_trips_through_html_import() {
        let bookmarks = vec![
            sample_bookmark("GitHub", "https://github.com", "Work"),
            sample_bookmark("Shopping", "https://shop.example.com", "Personal"),
        ];
        let folders = vec!["Work".to_string(), "Personal".to_string()];
        let html = export_bookmarks_html(&bookmarks, &folders);
        let imported = parse_bookmarks_html(&html);
        assert_eq!(imported.len(), 2);
        assert!(imported.iter().any(|b| b.url == "https://github.com" && b.title == "GitHub" && b.folder == "Work"));
        assert!(imported.iter().any(|b| b.url == "https://shop.example.com" && b.title == "Shopping" && b.folder == "Personal"));
    }

    #[test]
    fn html_import_handles_a_real_browser_export_shape() {
        // Trimmed but structurally real Netscape Bookmark File Format, the
        // shape Chrome/Firefox/Edge all actually emit.
        let html = r#"<!DOCTYPE NETSCAPE-Bookmark-file-1>
<META HTTP-EQUIV="Content-Type" CONTENT="text/html; charset=UTF-8">
<TITLE>Bookmarks</TITLE>
<H1>Bookmarks</H1>
<DL><p>
    <DT><H3 ADD_DATE="1600000000">Work</H3>
    <DL><p>
        <DT><A HREF="https://example.com/docs" ADD_DATE="1600000001">Docs &amp; Guides</A>
    </DL><p>
</DL><p>
"#;
        let imported = parse_bookmarks_html(html);
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].url, "https://example.com/docs");
        assert_eq!(imported[0].title, "Docs & Guides");
        assert_eq!(imported[0].folder, "Work");
    }

    #[test]
    fn json_export_round_trips_through_json_import() {
        let bookmarks = vec![sample_bookmark("Example", "https://example.com", "Work")];
        let json = export_bookmarks_json(&bookmarks);
        assert!(json.contains("\"version\": 1"));
        let imported = parse_bookmarks_json(&json);
        assert_eq!(imported, bookmarks);
    }

    #[test]
    fn json_import_accepts_the_documented_bare_shape_without_a_version_wrapper() {
        let json = r#"{"version": 1, "bookmarks": [{"title": "Example", "url": "https://example.com", "folder": "Work"}]}"#;
        let imported = parse_bookmarks_json(json);
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].title, "Example");
        assert_eq!(imported[0].added_at, 0);
    }

    #[test]
    fn merge_skips_duplicate_urls_and_reports_only_new_folders() {
        let existing = vec![sample_bookmark("Existing", "https://existing.com", "Work")];
        let existing_folders = vec!["Work".to_string()];
        let imported = vec![
            sample_bookmark("Existing (dup)", "https://existing.com", "Personal"),
            sample_bookmark("New", "https://new.com", "Travel"),
        ];
        let result = merge_imported_bookmarks(imported, &existing, &existing_folders);
        assert_eq!(result.duplicates_skipped, 1);
        assert_eq!(result.bookmarks.len(), 1);
        assert_eq!(result.bookmarks[0].url, "https://new.com");
        assert_eq!(result.new_folders, vec!["Travel".to_string()]);
    }

    #[test]
    fn pinned_tabs_session_state_includes_only_pinned_tabs() {
        let mut pinned = Tab::new(200, "https://pinned.example", default_isolation_mode());
        pinned.pinned = true;
        pinned.title = "Pinned Site".to_string();
        let unpinned = Tab::new(201, "https://unpinned.example", default_isolation_mode());

        let state = pinned_tabs_session_state(&[pinned, unpinned], true);
        assert!(state.restore_pinned_tabs);
        assert_eq!(state.pinned_tabs.len(), 1);
        assert_eq!(state.pinned_tabs[0].url, "https://pinned.example");
        assert_eq!(state.pinned_tabs[0].title, "Pinned Site");
    }

    #[test]
    fn session_state_round_trips_through_disk() {
        let path = std::env::temp_dir().join(format!("himalayas-test-session-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let state = SessionState {
            restore_pinned_tabs: true,
            pinned_tabs: vec![PinnedTabRecord { url: "https://example.com".to_string(), title: "Example".to_string() }],
        };
        save_session_state_to(&path, &state);
        let loaded = load_session_state_from(&path);
        assert!(loaded.restore_pinned_tabs);
        assert_eq!(loaded.pinned_tabs.len(), 1);
        assert_eq!(loaded.pinned_tabs[0].url, "https://example.com");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn session_state_defaults_to_restore_on_when_no_file_exists() {
        let path = std::env::temp_dir().join(format!("himalayas-test-session-missing-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let loaded = load_session_state_from(&path);
        assert!(loaded.restore_pinned_tabs);
        assert!(loaded.pinned_tabs.is_empty());
    }

    #[test]
    fn session_state_defaults_on_corrupt_file_instead_of_panicking() {
        let path = std::env::temp_dir().join(format!("himalayas-test-session-corrupt-{}.json", std::process::id()));
        std::fs::write(&path, "not valid json").unwrap();

        let loaded = load_session_state_from(&path);
        assert!(loaded.restore_pinned_tabs);
        assert!(loaded.pinned_tabs.is_empty());

        let _ = std::fs::remove_file(&path);
    }

    /// Builds a `ScriptedDocument` directly rather than going through
    /// `build_scripted_document` — that function calls `consume_context`,
    /// which panics outside a live Dioxus scope (there isn't one in a plain
    /// `#[test]`). Same `DocumentConfig` shape, just with `shell_provider:
    /// None` — fine for this test, which never touches a real window.
    fn scripted_document_for_test(html: &str) -> ScriptedDocument {
        reset_js_bindings();
        let mut font_ctx = FontContext::default();
        font_ctx
            .collection
            .register_fonts(linebender_resource_handle::Blob::new(Arc::new(blitz_dom::BULLET_FONT) as _), None);
        let config = DocumentConfig {
            html_parser_provider: Some(Arc::new(HtmlProvider)),
            font_ctx: Some(font_ctx),
            ..Default::default()
        };
        let mut base = HtmlDocument::from_html(html, config).into_inner();
        let scripts = extract_inline_scripts(&base);
        run_js_with_live_doc(&mut base, || {
            for script in scripts {
                JS.with(|ctx| {
                    let _ = ctx.borrow_mut().eval(Source::from_bytes(script.as_bytes()));
                });
            }
        });
        ScriptedDocument { base }
    }

    /// Regression test for a live crash report ("thehindu.com renders
    /// nothing" — actually crashed the whole app, not just a blank render).
    /// Fetches the real page and drives it through the exact same fetch ->
    /// parse -> resolve(layout/style) pipeline `build_scripted_document`/the
    /// render loop use. Root cause: some node reachable from a block
    /// container's `layout_children` on this page isn't an Element/
    /// AnonymousBlock/Document (likely a Text node that should have been
    /// wrapped in an anonymous block by `layout/construct.rs`'s box
    /// generation but wasn't, for a case not chased further — see
    /// docs/NATIVE_RENDERING_PLAN.md) — every layout-reachable
    /// `universal_accessors!` field panicked on it one at a time as each got
    /// patched (`style`, then `cache`, ...). Fixed at the crash-safety
    /// layer instead of the anonymous-box-generation root cause: those
    /// fields now degrade to a shared scratch default instead of panicking
    /// (`graceful_layout_accessors!`), so a layout-tree inconsistency
    /// degrades the affected node rather than crashing the whole renderer —
    /// not `#[ignore]`d for being flaky, but because it depends on live
    /// network access to a real, complex, real-world page (the whole point
    /// of this test), which the rest of the suite deliberately avoids.
    /// Run explicitly: `cargo test --bin himalayas-desktop --features
    /// js_engine -- --ignored repro_thehindu_crash`.
    #[tokio::test]
    #[ignore]
    async fn repro_thehindu_crash() {
        fetch_and_resolve("https://www.thehindu.com").await;
    }

    /// Shared by all `repro_*`/`diagnose_*` live-site tests: fetch -> parse
    /// -> resolve(layout/style), the exact pipeline `build_scripted_document`/
    /// the render loop use, outside any window/Dioxus context. A panic here
    /// has the same root cause as a live crash; printed diagnostics (node
    /// count, root element size, title) are the closest thing to "did this
    /// render something" available without a real GPU surface — not proof
    /// of pixel-correct rendering, just a sanity signal.
    async fn fetch_and_resolve(url: &str) -> BaseDocument {
        let net_provider = Arc::new(NetProvider::new(None));
        let request = blitz_traits::net::Request::get(url.parse().unwrap());
        let (resolved_url, bytes) = net_provider.fetch_async(request).await.unwrap();
        let html = String::from_utf8_lossy(&bytes).to_string();
        println!("[{url}] fetched {} bytes, resolved to {resolved_url}", bytes.len());

        reset_js_bindings();
        let mut font_ctx = FontContext::default();
        font_ctx
            .collection
            .register_fonts(linebender_resource_handle::Blob::new(Arc::new(blitz_dom::BULLET_FONT) as _), None);
        let config = DocumentConfig {
            base_url: Some(resolved_url),
            net_provider: Some(Arc::new(NetProvider::new(None)) as _),
            html_parser_provider: Some(Arc::new(HtmlProvider)),
            font_ctx: Some(font_ctx),
            ..Default::default()
        };
        let mut base = HtmlDocument::from_html(&html, config).into_inner();
        {
            let mut viewport = base.viewport_mut();
            viewport.window_size = (1280, 800);
            viewport.hidpi_scale = 1.0;
            viewport.zoom = 1.0;
        }

        base.resolve(0.0);

        let root = base.root_element();
        let layout = root.final_layout();
        println!(
            "[{url}] resolve() completed. root layout size: {}x{}, title: {:?}",
            layout.size.width,
            layout.size.height,
            base.find_title_node().map(|n| n.text_content()),
        );

        base
    }

    #[tokio::test]
    #[ignore]
    async fn diagnose_apple_render() {
        fetch_and_resolve("https://www.apple.com").await;
    }

    #[tokio::test]
    #[ignore]
    async fn diagnose_yahoo_load() {
        fetch_and_resolve("https://www.yahoo.com").await;
    }

    /// Records every URL `fetch` is asked for and never resolves any of
    /// them — for `loading_lazy_defers_offscreen_images_until_scrolled_near`,
    /// which only needs to observe *whether* a fetch was started, not the
    /// (fake, non-existent) response. blitz-dom has no test-only mock
    /// `NetProvider` of its own to reuse; this is the outer crate's own
    /// black-box substitute — it can't reach `BaseDocument::lazy_images`
    /// (private to blitz-dom) directly, so recording what actually got
    /// fetched is the only way to observe the deferral from here.
    struct RecordingNetProvider {
        fetched: std::sync::Mutex<Vec<String>>,
    }
    impl blitz_traits::net::NetProvider for RecordingNetProvider {
        fn fetch(&self, _doc_id: usize, request: blitz_traits::net::Request, _handler: Box<dyn blitz_traits::net::NetHandler>) {
            self.fetched.lock().unwrap().push(request.url.to_string());
        }
    }

    #[test]
    fn loading_lazy_defers_offscreen_images_until_scrolled_near() {
        // A tall spacer pushes the second image ~4000px below the fold —
        // comfortably past the 1.5x-viewport-height "near" margin
        // `check_lazy_images` (document.rs) uses at the 600px-tall viewport
        // this test sets below.
        let html = r#"<html><body>
            <img src="https://example.com/eager.png">
            <div style="height: 4000px;"></div>
            <img src="https://example.com/lazy.png" loading="lazy">
        </body></html>"#;

        reset_js_bindings();
        let mut font_ctx = FontContext::default();
        font_ctx
            .collection
            .register_fonts(linebender_resource_handle::Blob::new(Arc::new(blitz_dom::BULLET_FONT) as _), None);
        let net_provider = Arc::new(RecordingNetProvider { fetched: std::sync::Mutex::new(Vec::new()) });
        let config = DocumentConfig {
            base_url: Some("https://example.com/".parse().unwrap()),
            net_provider: Some(net_provider.clone() as _),
            html_parser_provider: Some(Arc::new(HtmlProvider)),
            font_ctx: Some(font_ctx),
            ..Default::default()
        };
        let mut base = HtmlDocument::from_html(html, config).into_inner();
        {
            let mut viewport = base.viewport_mut();
            viewport.window_size = (800, 600);
            viewport.hidpi_scale = 1.0;
            viewport.zoom = 1.0;
        }

        base.resolve(0.0);
        {
            let fetched = net_provider.fetched.lock().unwrap();
            assert!(fetched.iter().any(|u| u.ends_with("eager.png")), "the in-view image should fetch immediately: {fetched:?}");
            assert!(!fetched.iter().any(|u| u.ends_with("lazy.png")), "the far-offscreen lazy image should NOT fetch yet: {fetched:?}");
        }

        // Scroll to just above the lazy image — well within the margin now.
        // `scroll_viewport_by`'s delta convention is inverted from the
        // intuitive "positive == scroll down" — it computes
        // `viewport_scroll - delta`, so revealing content further down
        // (increasing `viewport_scroll.y`) needs a *negative* y.
        base.scroll_viewport_by(0.0, -3900.0);
        base.resolve(0.0);
        {
            let fetched = net_provider.fetched.lock().unwrap();
            assert!(fetched.iter().any(|u| u.ends_with("lazy.png")), "the lazy image should fetch once scrolled near it: {fetched:?}");
        }
    }

    /// Builds a document with a real `<picture>` — a `<source>` whose
    /// `type` isn't a compiled-in codec (should be skipped), one whose
    /// `media` doesn't match the test's 800px-wide viewport (should be
    /// skipped), one that matches on both `type` and `media` (should win),
    /// and a trailing fallback `<img>` (should be ignored, since a
    /// `<source>` matched first) — checked against a `RecordingNetProvider`
    /// so this observes the actual fetched URL, not `picture_source_for`'s
    /// return value directly (which is private to blitz-dom).
    fn picture_source_html(source_extra_attrs: &str) -> String {
        format!(
            r#"<html><body>
                <picture>
                    <source type="image/heic" srcset="unsupported-codec.heic">
                    <source media="(min-width: 2000px)" srcset="too-wide.jpg">
                    <source {source_extra_attrs} srcset="winner.jpg">
                    <img src="fallback.jpg">
                </picture>
            </body></html>"#
        )
    }

    #[test]
    fn picture_source_wins_over_type_and_media_mismatches_and_the_fallback_img() {
        reset_js_bindings();
        let mut font_ctx = FontContext::default();
        font_ctx
            .collection
            .register_fonts(linebender_resource_handle::Blob::new(Arc::new(blitz_dom::BULLET_FONT) as _), None);
        let net_provider = Arc::new(RecordingNetProvider { fetched: std::sync::Mutex::new(Vec::new()) });
        // Unlike `loading_lazy_defers_offscreen_images_until_scrolled_near`
        // (which only cares about position, resolved later during
        // `resolve()`), `<picture>`/`srcset` selection happens at *parse*
        // time (`load_image`, triggered as soon as the `src` attribute is
        // set while building the tree) — so the viewport has to be real
        // *before* `HtmlDocument::from_html` runs, via `DocumentConfig`,
        // not patched in afterward with `viewport_mut()`. Setting it only
        // afterward is too late: by then the image has already loaded
        // against the default (0,0) viewport, which both the picture-source
        // and plain-srcset paths treat as "unknown, don't select — fall
        // back to the plain src" (see `load_image`'s own doc comment).
        let config = DocumentConfig {
            base_url: Some("https://example.com/".parse().unwrap()),
            net_provider: Some(net_provider.clone() as _),
            html_parser_provider: Some(Arc::new(HtmlProvider)),
            font_ctx: Some(font_ctx),
            viewport: Some(blitz_traits::shell::Viewport::new(800, 600, 1.0, blitz_traits::shell::ColorScheme::Light)),
            ..Default::default()
        };
        let html = picture_source_html(r#"type="image/webp" media="(max-width: 1000px)""#);
        let mut base = HtmlDocument::from_html(&html, config).into_inner();

        base.resolve(0.0);

        let fetched = net_provider.fetched.lock().unwrap();
        assert!(fetched.iter().any(|u| u.ends_with("winner.jpg")), "the matching source should be fetched: {fetched:?}");
        assert!(!fetched.iter().any(|u| u.ends_with("too-wide.jpg")), "the media-mismatched source should be skipped: {fetched:?}");
        assert!(!fetched.iter().any(|u| u.ends_with("unsupported-codec.heic")), "the unsupported-type source should be skipped: {fetched:?}");
        assert!(!fetched.iter().any(|u| u.ends_with("fallback.jpg")), "the <img> fallback should be ignored once a <source> matched: {fetched:?}");
    }

    #[test]
    fn picture_falls_back_to_img_when_no_source_matches() {
        reset_js_bindings();
        let mut font_ctx = FontContext::default();
        font_ctx
            .collection
            .register_fonts(linebender_resource_handle::Blob::new(Arc::new(blitz_dom::BULLET_FONT) as _), None);
        let net_provider = Arc::new(RecordingNetProvider { fetched: std::sync::Mutex::new(Vec::new()) });
        let config = DocumentConfig {
            base_url: Some("https://example.com/".parse().unwrap()),
            net_provider: Some(net_provider.clone() as _),
            html_parser_provider: Some(Arc::new(HtmlProvider)),
            font_ctx: Some(font_ctx),
            viewport: Some(blitz_traits::shell::Viewport::new(800, 600, 1.0, blitz_traits::shell::ColorScheme::Light)),
            ..Default::default()
        };
        // Same as above but the third source *also* mismatches on media —
        // nothing in the <picture> should match, so the trailing <img>'s
        // own src is the real fallback per spec.
        let html = picture_source_html(r#"type="image/webp" media="(min-width: 2000px)""#);
        let mut base = HtmlDocument::from_html(&html, config).into_inner();

        base.resolve(0.0);

        let fetched = net_provider.fetched.lock().unwrap();
        assert!(fetched.iter().any(|u| u.ends_with("fallback.jpg")), "should fall back to the <img>'s own src: {fetched:?}");
        assert!(!fetched.iter().any(|u| u.ends_with("winner.jpg")), "the media-mismatched source should be skipped: {fetched:?}");
    }

    #[test]
    fn request_animation_frame_runs_on_poll_and_stops_when_queue_empties() {
        let html = r#"<html><body><p id="out">not run</p>
            <script>
                requestAnimationFrame(function(ts) {
                    document.getElementById('out').textContent = 'ran at ' + ts;
                });
            </script>
        </body></html>"#;

        let mut doc = scripted_document_for_test(html);

        let ran = doc.poll(None);
        assert!(ran, "poll() should return true when a callback was queued and ran");

        let node_id = doc.base.get_element_by_id("out").unwrap();
        let text = doc.base.get_node(node_id).unwrap().text_content();
        assert!(text.starts_with("ran at "), "callback should have run and mutated the DOM, got: {text:?}");

        let ran_again = doc.poll(None);
        assert!(!ran_again, "poll() should return false once the queue is empty (no self-requeue)");
    }

    #[test]
    fn cancel_animation_frame_prevents_callback_from_running() {
        let html = r#"<html><body><p id="out">not run</p>
            <script>
                var id = requestAnimationFrame(function() {
                    document.getElementById('out').textContent = 'should not run';
                });
                cancelAnimationFrame(id);
            </script>
        </body></html>"#;

        let mut doc = scripted_document_for_test(html);

        let ran = doc.poll(None);
        assert!(!ran, "poll() should return false — the only queued callback was cancelled");

        let node_id = doc.base.get_element_by_id("out").unwrap();
        assert_eq!(doc.base.get_node(node_id).unwrap().text_content(), "not run");
    }

    #[test]
    fn self_requeuing_animation_frame_keeps_running_across_polls() {
        let html = r#"<html><body><p id="count">0</p>
            <script>
                var n = 0;
                function tick() {
                    n = n + 1;
                    document.getElementById('count').textContent = String(n);
                    if (n < 3) {
                        requestAnimationFrame(tick);
                    }
                }
                requestAnimationFrame(tick);
            </script>
        </body></html>"#;

        let mut doc = scripted_document_for_test(html);

        assert!(doc.poll(None));
        assert!(doc.poll(None));
        assert!(doc.poll(None));
        // The third tick's callback stops re-queuing (n == 3), so a fourth
        // poll has nothing left to run.
        assert!(!doc.poll(None));

        let node_id = doc.base.get_element_by_id("count").unwrap();
        assert_eq!(doc.base.get_node(node_id).unwrap().text_content(), "3");
    }

    #[test]
    fn new_tabs_get_distinct_isolated_sessions_by_default() {
        // Exercises the real Phase 7 wiring end-to-end: Tab::new opens a
        // backend tab via `browser()`/`tab_manager()`, which on this test
        // machine's tier defaults to `IsolationMode::Isolated` — two tabs
        // should therefore never land on the same session, and each
        // session's cookies should stay private to its own tab.
        let tab_a = Tab::new(100, "https://a.example", default_isolation_mode());
        let tab_b = Tab::new(101, "https://b.example", default_isolation_mode());
        assert_ne!(tab_a.backend_id, tab_b.backend_id);

        if default_isolation_mode() == IsolationMode::Isolated {
            assert_ne!(tab_a.session_id, tab_b.session_id);

            let session_a = browser().get_session(&tab_a.session_id).unwrap();
            session_a.set_cookie("k".to_string(), "a".to_string());
            let session_b = browser().get_session(&tab_b.session_id).unwrap();
            assert_eq!(session_b.get_cookie("k"), None);
        }
    }
}
