use crate::Document;
use blitz_traits::events::{
    BlitzKeyEvent, BlitzPointerEvent, BlitzPointerId, BlitzWheelEvent, DomEvent, DomEventData,
    EventState, Point, PointerCoords, UiEvent,
};
use blitz_traits::node_id::NodeId;
use keyboard_types::Key;
use std::collections::VecDeque;

/// Himalayas patch: keyboard shortcuts the shell chrome needs to see
/// regardless of which sub-document currently holds focus — new tab, close
/// tab, refresh, focus address bar, switch tab by number, Escape. Without
/// this, a loaded page's own web-view (which holds focus for essentially
/// all of normal browsing) would swallow every one of these before the
/// outer RSX shell (driven by ordinary Dioxus event dispatch, only reached
/// via the fallthrough path below) ever got a chance to handle them. Named
/// browser-chrome actions, not "any key a web page's own script might also
/// want" — deliberately narrow, so normal in-page typing/shortcuts
/// (including a page's own Cmd/Ctrl+K-style command palettes) aren't
/// silently stolen.
fn is_reserved_browser_shortcut(event: &UiEvent) -> bool {
    fn is_reserved(data: &BlitzKeyEvent) -> bool {
        if data.modifiers.meta() || data.modifiers.ctrl() {
            return matches!(
                &data.key,
                Key::Character(c) if matches!(c.as_str(), "t" | "T" | "w" | "W" | "r" | "R" | "l" | "L" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            );
        }
        matches!(data.key, Key::Escape)
    }
    match event {
        UiEvent::KeyDown(data) | UiEvent::KeyUp(data) => is_reserved(data),
        _ => false,
    }
}

/// Translate a pointer event's page/client coordinates into a sub-document's
/// local coordinate space, given the offset of the `<web-view>` element that
/// hosts it in the outer document. Part of the Himalayas sub-document event
/// forwarding patch — see the comment in `EventDriver::handle_ui_event`.
fn shift_pointer_event(event: &UiEvent, offset_x: f32, offset_y: f32) -> UiEvent {
    fn shift(mut data: BlitzPointerEvent, offset_x: f32, offset_y: f32) -> BlitzPointerEvent {
        data.coords.page_x -= offset_x;
        data.coords.page_y -= offset_y;
        data.coords.client_x -= offset_x;
        data.coords.client_y -= offset_y;
        data
    }
    fn shift_wheel(mut data: BlitzWheelEvent, offset_x: f32, offset_y: f32) -> BlitzWheelEvent {
        data.coords.page_x -= offset_x;
        data.coords.page_y -= offset_y;
        data.coords.client_x -= offset_x;
        data.coords.client_y -= offset_y;
        data
    }
    match event {
        UiEvent::PointerDown(d) => UiEvent::PointerDown(shift(d.clone(), offset_x, offset_y)),
        UiEvent::PointerUp(d) => UiEvent::PointerUp(shift(d.clone(), offset_x, offset_y)),
        UiEvent::PointerMove(d) => UiEvent::PointerMove(shift(d.clone(), offset_x, offset_y)),
        UiEvent::PointerCancel(d) => UiEvent::PointerCancel(shift(d.clone(), offset_x, offset_y)),
        UiEvent::Wheel(d) => UiEvent::Wheel(shift_wheel(d.clone(), offset_x, offset_y)),
        other => other.clone(),
    }
}

pub trait EventHandler {
    fn handle_event(
        &mut self,
        chain: &[NodeId],
        event: &mut DomEvent,
        doc: &mut dyn Document,
        event_state: &mut EventState,
    );
}

pub struct NoopEventHandler;
impl EventHandler for NoopEventHandler {
    fn handle_event(
        &mut self,
        _chain: &[NodeId],
        _event: &mut DomEvent,
        _doc: &mut dyn Document,
        _event_state: &mut EventState,
    ) {
        // Do nothing
    }
}

pub struct EventDriver<'doc, Handler: EventHandler> {
    doc: &'doc mut dyn Document,
    handler: Handler,
    queue: VecDeque<DomEvent>,
}

impl<'doc, Handler: EventHandler> EventDriver<'doc, Handler> {
    pub fn new(doc: &'doc mut dyn Document, handler: Handler) -> Self {
        EventDriver {
            doc,
            handler,
            queue: VecDeque::with_capacity(4),
        }
    }

    pub fn handle_pointer_move(&mut self, event: &BlitzPointerEvent) -> Option<NodeId> {
        let mut doc = self.doc.inner_mut();

        let prev_hover_node_id = doc.hover_node_id;
        let changed = doc.set_hover_to(event.page_x(), event.page_y());
        let hover_node_id = doc.hover_node_id;

        drop(doc);

        if !changed {
            return prev_hover_node_id;
        }

        let doc = self.doc.inner();
        let mut old_chain = prev_hover_node_id
            .map(|id| doc.node_chain(id))
            .unwrap_or_default();
        let mut new_chain = hover_node_id
            .map(|id| doc.node_chain(id))
            .unwrap_or_default();
        old_chain.reverse();
        new_chain.reverse();

        // Find the difference in the node chain of the last hovered objected and the newest
        let old_len = old_chain.len();
        let new_len = new_chain.len();

        let first_difference_index = old_chain
            .iter()
            .zip(&new_chain)
            .position(|(old, new)| old != new)
            .unwrap_or_else(|| old_len.min(new_len));

        drop(doc);

        let is_mouse = event.is_mouse();

        if let Some(target) = prev_hover_node_id {
            self.handle_dom_event(DomEvent::new(
                target,
                DomEventData::PointerOut(event.clone()),
            ));
            if is_mouse {
                self.handle_dom_event(DomEvent::new(target, DomEventData::MouseOut(event.clone())));
            }

            // Send an mouseleave event to all old elements on the chain
            for node_id in old_chain
                .get(first_difference_index..)
                .unwrap_or(&[])
                .iter()
            {
                self.handle_dom_event(DomEvent::new(
                    *node_id,
                    DomEventData::PointerLeave(event.clone()),
                ));
                if is_mouse {
                    self.handle_dom_event(DomEvent::new(
                        *node_id,
                        DomEventData::MouseLeave(event.clone()),
                    ));
                }
            }
        }

        if let Some(target) = hover_node_id {
            self.handle_dom_event(DomEvent::new(
                target,
                DomEventData::PointerOver(event.clone()),
            ));

            if is_mouse {
                self.handle_dom_event(DomEvent::new(
                    target,
                    DomEventData::MouseOver(event.clone()),
                ));
            }

            // Send an mouseenter event to all new elements on the chain
            for node_id in new_chain
                .get(first_difference_index..)
                .unwrap_or(&[])
                .iter()
            {
                self.handle_dom_event(DomEvent::new(
                    *node_id,
                    DomEventData::PointerEnter(event.clone()),
                ));

                if is_mouse {
                    self.handle_dom_event(DomEvent::new(
                        *node_id,
                        DomEventData::MouseEnter(event.clone()),
                    ));
                }
            }
        }

        hover_node_id
    }

    pub fn handle_ui_event(&mut self, event: UiEvent) {
        let doc = self.doc.inner();

        let mut should_clear_hover = false;
        let mut hover_node_id = doc.hover_node_id;
        let focussed_node_id = doc.focus_node_id;
        drop(doc);

        // Update document input state (hover, focus, active, etc)
        match &event {
            UiEvent::PointerMove(event) => {
                hover_node_id = self.handle_pointer_move(event);
            }
            UiEvent::PointerDown(event) => {
                hover_node_id = self.handle_pointer_move(event);
                let mut doc = self.doc.inner_mut();
                doc.active_node();
                doc.set_mousedown_node_id(hover_node_id);
            }
            UiEvent::PointerUp(event) => {
                hover_node_id = self.handle_pointer_move(event);
                let mut doc = self.doc.inner_mut();
                doc.unactive_node();

                if event.is_primary && matches!(event.id, BlitzPointerId::Finger(_)) {
                    should_clear_hover = true;
                }
            }
            UiEvent::PointerCancel(event) => {
                hover_node_id = self.handle_pointer_move(event);
                let mut doc = self.doc.inner_mut();
                doc.unactive_node();

                if event.is_primary && matches!(event.id, BlitzPointerId::Finger(_)) {
                    should_clear_hover = true;
                }
            }
            _ => {}
        };

        let target = match event {
            UiEvent::PointerMove(_) => hover_node_id,
            UiEvent::PointerUp(_) => hover_node_id,
            UiEvent::PointerDown(_) => hover_node_id,
            UiEvent::PointerCancel(_) => hover_node_id,
            UiEvent::Wheel(_) => hover_node_id,
            UiEvent::KeyUp(_) => focussed_node_id,
            UiEvent::KeyDown(_) => focussed_node_id,
            UiEvent::Ime(_) => focussed_node_id,
            UiEvent::AppleStandardKeybinding(_) => focussed_node_id,
        };
        let target = target.unwrap_or_else(|| self.doc.inner().root_element().id);

        // --- Himalayas patch: forward pointer + keyboard events into sub-documents ---
        // Upstream, `<web-view>`/iframe sub-documents are painted (blitz-paint)
        // and laid out (resolve.rs) but never receive events: `sub_document_nodes`
        // has no readers anywhere in the event-dispatch path. Without this, a
        // sub-document's own `Document::handle_ui_event` override (e.g. a custom
        // scripting handler) is unreachable via real clicks, and typing into a
        // form field inside a sub-document does nothing. See
        // docs/NATIVE_RENDERING_PLAN.md in the Himalayas Browser repo.
        let has_subdoc = self
            .doc
            .inner()
            .get_node(target)
            .is_some_and(|n| n.subdoc().is_some());
        if has_subdoc {
            if matches!(
                event,
                UiEvent::PointerDown(_) | UiEvent::PointerUp(_) | UiEvent::PointerMove(_) | UiEvent::PointerCancel(_)
            ) {
                let rect = self.doc.inner().get_client_bounding_rect(target);
                if let Some(rect) = rect {
                    let offset_x = rect.x as f32;
                    let offset_y = rect.y as f32;
                    let forwarded = shift_pointer_event(&event, offset_x, offset_y);
                    let mut doc = self.doc.inner_mut();
                    // A pointer-down on the sub-document's mount point becomes
                    // the *outer* document's notion of "focus" too, so a
                    // keyboard event arriving afterwards (which targets
                    // `focussed_node_id`, computed above, not a screen
                    // position) still resolves to this same sub-document and
                    // gets forwarded below — without this, focus silently
                    // stays wherever it was before the click (often nowhere),
                    // and all subsequent typing goes nowhere.
                    if matches!(event, UiEvent::PointerDown(_)) {
                        doc.set_focus_to(target);
                    }
                    if let Some(sub_doc) = doc.get_node_mut(target).and_then(|n| n.subdoc_mut()) {
                        sub_doc.handle_ui_event(forwarded);
                    }
                }
                return;
            }
            if matches!(event, UiEvent::Wheel(_)) {
                // Without this, a wheel event over a sub-document's mount
                // point fell through to the *outer* document's own default
                // action below (`UiEvent::Wheel` match arm further down),
                // scrolling the outer chrome's content container instead of
                // the page's own internal scroll offset — the page itself
                // never actually received the scroll, which is what made
                // scrolling loaded pages feel broken/unresponsive rather
                // than just visually different.
                let rect = self.doc.inner().get_client_bounding_rect(target);
                if let Some(rect) = rect {
                    let forwarded = shift_pointer_event(&event, rect.x as f32, rect.y as f32);
                    let mut doc = self.doc.inner_mut();
                    if let Some(sub_doc) = doc.get_node_mut(target).and_then(|n| n.subdoc_mut()) {
                        sub_doc.handle_ui_event(forwarded);
                    }
                }
                return;
            }
            if matches!(event, UiEvent::KeyDown(_) | UiEvent::KeyUp(_) | UiEvent::Ime(_))
                && !is_reserved_browser_shortcut(&event)
            {
                // No coordinate translation needed — keyboard/IME events are
                // targeted by focus, not screen position. The sub-document's
                // own `focussed_node_id` (set as a side effect of forwarding
                // pointer-downs, above) determines the real target within it.
                let mut doc = self.doc.inner_mut();
                if let Some(sub_doc) = doc.get_node_mut(target).and_then(|n| n.subdoc_mut()) {
                    sub_doc.handle_ui_event(event);
                }
                return;
            }
            // Reserved shortcuts fall through instead of returning: without
            // this, a loaded page's own web-view — which holds focus for
            // essentially all of normal browsing — would swallow every
            // Cmd/Ctrl+T-style browser shortcut before the shell chrome
            // (RSX, driven by normal Dioxus event dispatch on the *outer*
            // document) ever saw it. See `is_reserved_browser_shortcut`.
        }
        // --- end patch ---

        match event {
            UiEvent::PointerMove(data) => {
                self.handle_pointer_event(
                    target,
                    data,
                    DomEventData::PointerMove,
                    Some(DomEventData::MouseMove),
                    DomEventData::TouchMove,
                );
            }
            UiEvent::PointerUp(data) => {
                self.handle_pointer_event(
                    target,
                    data,
                    DomEventData::PointerUp,
                    Some(DomEventData::MouseUp),
                    DomEventData::TouchEnd,
                );
            }
            UiEvent::PointerDown(data) => {
                self.handle_pointer_event(
                    target,
                    data,
                    DomEventData::PointerDown,
                    Some(DomEventData::MouseDown),
                    DomEventData::TouchStart,
                );
            }
            UiEvent::PointerCancel(data) => {
                // `pointercancel` has no mouse-compatibility event, but does
                // generate a `touchcancel` for touch-like inputs.
                self.handle_pointer_event(
                    target,
                    data,
                    DomEventData::PointerCancel,
                    None::<fn(BlitzPointerEvent) -> DomEventData>,
                    DomEventData::TouchCancel,
                );
            }
            UiEvent::Wheel(data) => {
                self.handle_dom_event(DomEvent::new(target, DomEventData::Wheel(data)))
            }
            UiEvent::KeyUp(data) => {
                self.handle_dom_event(DomEvent::new(target, DomEventData::KeyUp(data)))
            }
            UiEvent::KeyDown(data) => {
                self.handle_dom_event(DomEvent::new(target, DomEventData::KeyDown(data)))
            }
            UiEvent::Ime(data) => {
                self.handle_dom_event(DomEvent::new(target, DomEventData::Ime(data)))
            }
            UiEvent::AppleStandardKeybinding(data) => {
                let mut dom_event =
                    DomEvent::new(target, DomEventData::AppleStandardKeybinding(data));
                self.run_default_action(&mut dom_event);
                self.process_queue();
            }
        };

        // Update document input state (hover, focus, active, etc)
        if should_clear_hover {
            self.doc.inner_mut().clear_hover();
        }
    }

    pub fn handle_dom_event(&mut self, event: DomEvent) {
        self.queue.push_back(event);
        self.process_queue();
    }

    fn handle_pointer_event(
        &mut self,
        target: NodeId,
        data: BlitzPointerEvent,
        make_ptr_data: impl FnOnce(BlitzPointerEvent) -> DomEventData,
        make_mouse_data: Option<impl FnOnce(BlitzPointerEvent) -> DomEventData>,
        make_touch_data: impl FnOnce(BlitzPointerEvent) -> DomEventData,
    ) {
        let mut ptr_event = DomEvent::new(target, make_ptr_data(data.clone()));
        let mut event_state = EventState::default();
        event_state = self.run_handler_event(&mut ptr_event, event_state);

        // Generate the corresponding compatibility event (mouse events for the
        // mouse, touch events for fingers and pen/stylus input) and expose it to
        // script. The default action is always run on the pointer event so that
        // the shell layer and default actions remain pointer-based.
        //
        // `pointercancel` has no mouse equivalent, so `make_mouse_data` is `None`
        // in that case and no mouse event is generated.
        if !event_state.is_cancelled() {
            if data.is_mouse() {
                if let Some(make_mouse_data) = make_mouse_data {
                    let mut mouse_event = DomEvent::new(target, make_mouse_data(data));
                    event_state = self.run_handler_event(&mut mouse_event, event_state);
                }
            } else if data.is_finger() || data.is_pen() {
                let mut touch_event = DomEvent::new(target, make_touch_data(data));
                event_state = self.run_handler_event(&mut touch_event, event_state);
            }
        }

        if !event_state.is_cancelled() {
            self.run_default_action(&mut ptr_event);
        }
        self.process_queue();
    }

    fn process_queue(&mut self) {
        while let Some(mut event) = self.queue.pop_front() {
            let event_state = self.run_handler_event(&mut event, EventState::default());
            if !event_state.is_cancelled() {
                self.run_default_action(&mut event);
            }
        }
    }

    fn adjust_element_coords(
        &self,
        target: NodeId,
        coords: &PointerCoords,
        element: &mut Point<f32>,
    ) {
        if let Some(rect) = self.doc.inner().get_client_bounding_rect(target) {
            element.x = coords.client_x - rect.x as f32;
            element.y = coords.client_y - rect.y as f32;
        }
    }

    fn run_handler_event(
        &mut self,
        event: &mut DomEvent,
        initial_event_state: EventState,
    ) -> EventState {
        let chain = if event.bubbles {
            let doc = self.doc.inner();
            doc.node_chain(event.target)
        } else {
            vec![event.target]
        };

        match &mut event.data {
            DomEventData::PointerMove(data)
            | DomEventData::PointerDown(data)
            | DomEventData::PointerUp(data)
            | DomEventData::PointerCancel(data)
            | DomEventData::PointerEnter(data)
            | DomEventData::PointerLeave(data)
            | DomEventData::PointerOver(data)
            | DomEventData::PointerOut(data)
            | DomEventData::MouseMove(data)
            | DomEventData::MouseDown(data)
            | DomEventData::MouseUp(data)
            | DomEventData::MouseEnter(data)
            | DomEventData::MouseLeave(data)
            | DomEventData::MouseOver(data)
            | DomEventData::MouseOut(data)
            | DomEventData::TouchStart(data)
            | DomEventData::TouchEnd(data)
            | DomEventData::TouchMove(data)
            | DomEventData::TouchCancel(data)
            | DomEventData::Click(data)
            | DomEventData::ContextMenu(data)
            | DomEventData::DoubleClick(data) => {
                self.adjust_element_coords(event.target, &data.coords, &mut data.element)
            }
            DomEventData::Wheel(data) => {
                self.adjust_element_coords(event.target, &data.coords, &mut data.element)
            }
            _ => {}
        }

        let mut event_state = initial_event_state;
        self.handler
            .handle_event(&chain, event, self.doc, &mut event_state);

        event_state
    }

    fn run_default_action(&mut self, event: &mut DomEvent) {
        let mut doc = self.doc.inner_mut();
        doc.handle_dom_event(event, |new_evt| self.queue.push_back(new_evt));
    }
}
