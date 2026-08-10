//! Overlay scrollbar geometry and css-scrollbars-1 style accessors for
//! [`Node`]. Geometry is shared between painting (blitz-paint) and thumb
//! hit-testing so the two cannot drift.

use blitz_traits::node_id::NodeId;
use kurbo::Rect as KurboRect;
use taffy::AbsoluteAxis;
use web_time::Duration;

use super::Node;

/// How long overlay scrollbars stay fully opaque after their last activity
/// (a scroll, or the pointer leaving the thumb), and how long the fade-out
/// takes. Chromium's overlay values. No longer drives visible fading itself
/// (`BaseDocument::scrollbar_opacity` is now a flat `1.0` — live user
/// preference: scrollbars stay visible whenever there's scrollable content,
/// not just while/just after scrolling) but `scrollbar_activity`/
/// `show_scrollbars`/`scrollbars_animating` still track it, so reverting to
/// overlay behavior later is a one-function change, not rebuilding this.
pub(crate) const FADE_DELAY: Duration = Duration::from_millis(500);
pub(crate) const FADE_DURATION: Duration = Duration::from_millis(200);

// Sized up from Chromium's overlay defaults (10px/6px thick, 32px minimum
// thumb length) per live user feedback — thicker and longer for better
// visibility/grabbability, now that these persist rather than only flashing
// in briefly. Bumped a second time ("bigger and more prominent than this")
// specifically for the always-visible viewport (whole-page) scrollbar.
pub(crate) const THUMB_THICKNESS: f64 = 20.0;
const THIN_THUMB_THICKNESS: f64 = 13.0;
const THUMB_MARGIN: f64 = 2.0;
// The floor the proportional thumb-length formula (viewport_len^2 /
// (viewport_len + scroll_extent)) can shrink to — matters most exactly
// where the user flagged it: small viewports/short tracks, where that
// formula alone would otherwise produce a barely-visible, hard-to-grab
// sliver for a page with a lot of scrollable content.
const MIN_THUMB_LENGTH: f64 = 64.0;

/// Overlay scrollbar thumb geometry for one axis, in (unscaled) CSS px
/// relative to `port`'s own origin — shared by `Node::scrollbar_thumb` (a
/// node's own `overflow: auto/scroll`) and the Himalayas viewport-scrollbar
/// patch (`BaseDocument::viewport_scrollbar_thumb` in `document.rs`, for
/// ordinary whole-page scrolling, which isn't tied to any node's own
/// `overflow` at all — see that method's doc comment). Pulled out from what
/// used to be `Node::scrollbar_thumb`'s body so both call sites share
/// exactly one copy of the thumb-sizing math instead of two that could
/// silently drift apart.
///
/// `scroll_extent` is the *extra* scrollable distance beyond one viewport
/// (i.e. `content_length - viewport_len`), not total content length — same
/// convention `taffy::Layout::scroll_width()/scroll_height()` already uses,
/// which is what makes reusing this for both callers possible: the viewport
/// caller just computes that same quantity from document-level metrics
/// instead of a node's own layout.
pub(crate) fn thumb_rect_for(
    axis: AbsoluteAxis,
    port: KurboRect,
    scroll_extent: f64,
    scroll_offset: f64,
    thickness: f64,
) -> Option<KurboRect> {
    if scroll_extent <= 0.5 {
        return None;
    }

    let (viewport_len, scroll_offset) = match axis {
        AbsoluteAxis::Horizontal => (port.width(), scroll_offset),
        AbsoluteAxis::Vertical => (port.height(), scroll_offset),
    };
    let thumb_len = (viewport_len * viewport_len / (viewport_len + scroll_extent))
        .max(MIN_THUMB_LENGTH)
        .min(viewport_len);
    let progress = (scroll_offset / scroll_extent).clamp(0.0, 1.0);
    // Round a sub-pixel displacement up to a whole pixel so any nonzero
    // scroll visibly moves the thumb off the origin.
    let thumb_start = match progress * (viewport_len - thumb_len) {
        start if start > 0.0 && start < 1.0 => 1.0,
        start => start,
    };

    Some(match axis {
        AbsoluteAxis::Horizontal => KurboRect::new(
            port.x0 + thumb_start,
            port.y1 - THUMB_MARGIN - thickness,
            port.x0 + thumb_start + thumb_len,
            port.y1 - THUMB_MARGIN,
        ),
        AbsoluteAxis::Vertical => KurboRect::new(
            port.x1 - THUMB_MARGIN - thickness,
            port.y0 + thumb_start,
            port.x1 - THUMB_MARGIN,
            port.y0 + thumb_start + thumb_len,
        ),
    })
}

/// A specific scrollbar: one axis of one scroll container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarRef {
    pub node_id: NodeId,
    pub axis: AbsoluteAxis,
}

/// The computed value of `scrollbar-width` (css-scrollbars-1). A local
/// mirror of the stylo type, which isn't exposed to the servo engine yet
/// (servo/stylo#413).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollbarWidth {
    #[default]
    Auto,
    Thin,
    None,
}

/// The computed value of `scrollbar-color` (css-scrollbars-1). A local
/// mirror of the stylo type, which isn't exposed to the servo engine yet
/// (servo/stylo#413). Colors are fully resolved (no `currentColor`).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ScrollbarColor {
    #[default]
    Auto,
    Colors {
        thumb: style::color::AbsoluteColor,
        track: style::color::AbsoluteColor,
    },
}

impl Node {
    /// The node's used `scrollbar-width`.
    pub fn scrollbar_width(&self) -> ScrollbarWidth {
        // TODO: read the computed style once stylo exposes scrollbar-width
        // to the servo engine (servo/stylo#413):
        // match self.primary_styles().map(|s| s.clone_scrollbar_width()) { .. }
        ScrollbarWidth::Auto
    }

    /// The node's used `scrollbar-color`.
    pub fn scrollbar_color(&self) -> ScrollbarColor {
        // TODO: read the computed style once stylo exposes scrollbar-color
        // to the servo engine (servo/stylo#413), resolving the colors
        // against the element's `color`:
        // self.primary_styles().map(|s| s.clone_scrollbar_color()) { .. }
        ScrollbarColor::Auto
    }

    /// Whether the node shows an overlay scrollbar in the given axis:
    /// always for `overflow: scroll`, only when the content overflows for
    /// `overflow: auto`, never otherwise — and never when
    /// `scrollbar-width: none`.
    pub fn wants_scrollbar(&self, axis: AbsoluteAxis) -> bool {
        use style::values::computed::Overflow;
        let Some(style) = self.primary_styles() else {
            return false;
        };
        if self.scrollbar_width() == ScrollbarWidth::None {
            return false;
        }
        let (overflow, scroll_extent) = match axis {
            AbsoluteAxis::Horizontal => (
                style.clone_overflow_x(),
                self.final_layout().scroll_width() as f64,
            ),
            AbsoluteAxis::Vertical => (
                style.clone_overflow_y(),
                self.final_layout().scroll_height() as f64,
            ),
        };
        match overflow {
            Overflow::Scroll => true,
            Overflow::Auto => scroll_extent > 0.5,
            _ => false,
        }
    }

    /// The scrollport (padding box) in (unscaled) CSS px relative to the
    /// node's border-box origin. Taffy has content-box helpers but none for
    /// the padding box.
    fn scrollport(&self) -> KurboRect {
        let layout = self.final_layout();
        KurboRect::new(
            layout.border.left as f64,
            layout.border.top as f64,
            layout.size.width as f64 - layout.border.right as f64,
            layout.size.height as f64 - layout.border.bottom as f64,
        )
    }

    /// Geometry of the overlay scrollbar thumb for the given axis, in
    /// (unscaled) CSS px relative to the node's border-box origin. `None`
    /// if there is no scrollable overflow in that axis.
    pub fn scrollbar_thumb(&self, axis: AbsoluteAxis) -> Option<KurboRect> {
        let layout = self.final_layout();
        let scroll_extent = match axis {
            AbsoluteAxis::Horizontal => layout.scroll_width() as f64,
            AbsoluteAxis::Vertical => layout.scroll_height() as f64,
        };
        let thickness = match self.scrollbar_width() {
            ScrollbarWidth::Thin => THIN_THUMB_THICKNESS,
            _ => THUMB_THICKNESS,
        };
        let port = self.scrollport();
        let scroll_offset = match axis {
            AbsoluteAxis::Horizontal => self.scroll_offset().x,
            AbsoluteAxis::Vertical => self.scroll_offset().y,
        };
        thumb_rect_for(axis, port, scroll_extent, scroll_offset, thickness)
    }

    /// Content px scrolled per thumb px dragged, for the given axis.
    pub fn scrollbar_drag_ratio(&self, axis: AbsoluteAxis) -> f64 {
        let Some(thumb) = self.scrollbar_thumb(axis) else {
            return 0.0;
        };
        let port = self.scrollport();
        let (scroll_extent, viewport_len, thumb_len) = match axis {
            AbsoluteAxis::Horizontal => (
                self.final_layout().scroll_width() as f64,
                port.width(),
                thumb.width(),
            ),
            AbsoluteAxis::Vertical => (
                self.final_layout().scroll_height() as f64,
                port.height(),
                thumb.height(),
            ),
        };
        let track_play = viewport_len - thumb_len;
        if track_play <= 0.0 {
            return 0.0;
        }
        scroll_extent / track_play
    }

    /// The scrollbar thumb containing the given point (in this node's
    /// border-box coordinates), if any. The `scrollbars` feature's single
    /// behavioral gate: returning `None` keeps unpainted thumbs from ever
    /// claiming pointer events.
    pub(crate) fn scrollbar_at_local(&self, x: f64, y: f64) -> Option<ScrollbarRef> {
        if !cfg!(feature = "scrollbars") {
            return None;
        }
        for axis in [AbsoluteAxis::Vertical, AbsoluteAxis::Horizontal] {
            if !self.wants_scrollbar(axis) {
                continue;
            }
            if let Some(thumb) = self.scrollbar_thumb(axis)
                && thumb.contains(kurbo::Point::new(x, y))
            {
                return Some(ScrollbarRef {
                    node_id: self.id,
                    axis,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `thumb_rect_for` is the geometry both `Node::scrollbar_thumb` (a
    // node's own `overflow: auto/scroll`) and
    // `BaseDocument::viewport_scrollbar_thumb` (Himalayas patch — ordinary
    // whole-page scrolling) share, so these double as regression coverage
    // for both callers at once.

    #[test]
    fn no_thumb_when_content_fits() {
        let port = KurboRect::new(0.0, 0.0, 800.0, 600.0);
        assert_eq!(thumb_rect_for(AbsoluteAxis::Vertical, port, 0.0, 0.0, THUMB_THICKNESS), None);
    }

    #[test]
    fn thumb_sits_at_top_when_unscrolled() {
        let port = KurboRect::new(0.0, 0.0, 800.0, 600.0);
        // 600px viewport, 400px more content below it (1000px total).
        let thumb = thumb_rect_for(AbsoluteAxis::Vertical, port, 400.0, 0.0, THUMB_THICKNESS).unwrap();
        assert_eq!(thumb.y0, 0.0);
        assert!(thumb.height() < 600.0);
        // Vertical thumb hugs the right edge, inset by the margin.
        assert_eq!(thumb.x1, port.x1 - THUMB_MARGIN);
    }

    #[test]
    fn thumb_moves_down_proportionally_to_scroll_progress() {
        let port = KurboRect::new(0.0, 0.0, 800.0, 600.0);
        let at_start = thumb_rect_for(AbsoluteAxis::Vertical, port, 400.0, 0.0, THUMB_THICKNESS).unwrap();
        let halfway = thumb_rect_for(AbsoluteAxis::Vertical, port, 400.0, 200.0, THUMB_THICKNESS).unwrap();
        let at_end = thumb_rect_for(AbsoluteAxis::Vertical, port, 400.0, 400.0, THUMB_THICKNESS).unwrap();
        assert!(at_start.y0 < halfway.y0);
        assert!(halfway.y0 < at_end.y0);
        // Fully scrolled: thumb's bottom edge reaches the port's bottom edge.
        assert!((at_end.y1 - port.y1).abs() < 1.0);
    }

    #[test]
    fn thumb_length_never_shrinks_below_the_minimum() {
        let port = KurboRect::new(0.0, 0.0, 800.0, 600.0);
        // Enormous scroll extent relative to the viewport.
        let thumb = thumb_rect_for(AbsoluteAxis::Vertical, port, 1_000_000.0, 0.0, THUMB_THICKNESS).unwrap();
        assert!(thumb.height() >= MIN_THUMB_LENGTH - 1e-6);
    }
}
