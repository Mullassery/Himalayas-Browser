//! `srcset`/`sizes` responsive image selection — a Himalayas patch, not
//! upstream Blitz (confirmed absent from the vendored commit before this:
//! zero references to `srcset` anywhere in the tree). See
//! docs/NATIVE_RENDERING_PLAN.md, "Media Optimization Engine" / Himalayas
//! Adaptive Web Rendering, for the product framing this came from.
//!
//! Scope: real, standards-track `<img srcset sizes>` selection — this is
//! the mechanism the web platform already has for exactly this ("let the
//! browser pick the best-fit image for the viewport/DPR instead of always
//! fetching whatever `src=` points to"), not a novel Himalayas protocol.
//! `<picture>`/`<source>` (which layer media-query-based art direction on
//! top of the same idea) are a separate, larger follow-up — this covers the
//! much more common plain `<img srcset>` case.

#[derive(Debug, Clone, Copy, PartialEq)]
enum Descriptor {
    /// `800w` — the candidate's intrinsic width in px.
    Width(u32),
    /// `2x` (or no descriptor at all, which defaults to `1x` per spec) —
    /// the candidate's pixel density.
    Density(f32),
}

#[derive(Debug, Clone, PartialEq)]
struct Candidate {
    url: String,
    descriptor: Descriptor,
}

/// Parse a `srcset` attribute value into `(url, descriptor)` candidates.
/// Each comma-separated entry is `url [descriptor]`. A `srcset` is supposed
/// to use one descriptor kind consistently (all `w` or all `x`); this
/// doesn't enforce that — it just reads whatever each entry says, and
/// `select_srcset_candidate` picks a selection strategy based on whichever
/// kind is present.
fn parse_srcset(value: &str) -> Vec<Candidate> {
    value
        .split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            let mut parts = entry.split_whitespace();
            let url = parts.next()?.to_string();
            let descriptor = match parts.next() {
                Some(d) if d.ends_with('w') => d[..d.len() - 1].parse::<u32>().ok().map(Descriptor::Width)?,
                Some(d) if d.ends_with('x') => d[..d.len() - 1].parse::<f32>().ok().map(Descriptor::Density)?,
                Some(_) => return None, // unrecognized descriptor — skip rather than guess
                None => Descriptor::Density(1.0),
            };
            Some(Candidate { url, descriptor })
        })
        .collect()
}

/// Evaluate a `sizes` attribute against the viewport width (in CSS px),
/// returning the resolved width the image is expected to render at — the
/// value `select_srcset_candidate` compares `w`-descriptor candidates
/// against. Handles the overwhelmingly common real-world forms:
/// comma-separated `(max-width: Npx) Length` / `(min-width: Npx) Length` /
/// bare `Length` entries, `Length` as `Npx` or `Nvw`. The first entry whose
/// condition matches (or that has no condition at all — per spec, an
/// unconditional entry always "matches" and evaluation stops there
/// regardless of position) wins. Anything this can't parse (`calc()`,
/// compound conditions, other units, entirely missing `sizes`) falls
/// through to the spec default of `100vw`, rather than guessing.
pub(crate) fn eval_sizes(value: Option<&str>, viewport_width_px: f32) -> f32 {
    let default = viewport_width_px; // 100vw
    let Some(value) = value else { return default };

    for entry in value.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        if let Some(rest) = entry.strip_prefix('(') {
            let Some(close) = rest.find(')') else { continue };
            let condition = &rest[..close];
            let length = rest[close + 1..].trim();
            if eval_media_condition(condition, viewport_width_px) {
                if let Some(px) = parse_length_px(length, viewport_width_px) {
                    return px;
                }
            }
            // Condition present but didn't match (or length didn't parse) — next entry.
        } else if let Some(px) = parse_length_px(entry, viewport_width_px) {
            return px; // unconditional entry: spec says stop here
        }
    }

    default
}

fn eval_media_condition(condition: &str, viewport_width_px: f32) -> bool {
    let condition = condition.trim();
    if let Some(px_str) = condition.strip_prefix("max-width:") {
        return parse_length_px(px_str.trim(), viewport_width_px).is_some_and(|max| viewport_width_px <= max);
    }
    if let Some(px_str) = condition.strip_prefix("min-width:") {
        return parse_length_px(px_str.trim(), viewport_width_px).is_some_and(|min| viewport_width_px >= min);
    }
    false
}

fn parse_length_px(value: &str, viewport_width_px: f32) -> Option<f32> {
    let value = value.trim();
    if let Some(n) = value.strip_suffix("px") {
        return n.trim().parse::<f32>().ok();
    }
    if let Some(n) = value.strip_suffix("vw") {
        return n.trim().parse::<f32>().ok().map(|vw| vw / 100.0 * viewport_width_px);
    }
    None
}

/// Pick the best-fit `srcset` candidate for `target_width_px` (from
/// `eval_sizes`) and `dpr` (device pixel ratio, `Viewport::hidpi_scale`).
/// `None` for an empty/unparseable `srcset` — the caller falls back to the
/// plain `src`.
///
/// Width descriptors: the smallest candidate wide enough for
/// `target_width_px * dpr`, falling back to the largest available if none
/// are big enough (never intentionally serve something *smaller* than
/// requested — that's a quality regression the whole point of this feature
/// is to avoid triggering, not cause). Density descriptors: closest to
/// `dpr`.
pub(crate) fn select_srcset_candidate(value: &str, target_width_px: f32, dpr: f32) -> Option<String> {
    let candidates = parse_srcset(value);
    if candidates.is_empty() {
        return None;
    }

    let has_width_descriptors = candidates.iter().any(|c| matches!(c.descriptor, Descriptor::Width(_)));
    if has_width_descriptors {
        let needed = target_width_px * dpr;
        let mut sorted: Vec<&Candidate> =
            candidates.iter().filter(|c| matches!(c.descriptor, Descriptor::Width(_))).collect();
        sorted.sort_by_key(|c| match c.descriptor {
            Descriptor::Width(w) => w,
            Descriptor::Density(_) => 0,
        });
        sorted
            .iter()
            .find(|c| matches!(c.descriptor, Descriptor::Width(w) if w as f32 >= needed))
            .or_else(|| sorted.last())
            .map(|c| c.url.clone())
    } else {
        candidates
            .into_iter()
            .min_by(|a, b| {
                let da = match a.descriptor {
                    Descriptor::Density(d) => (d - dpr).abs(),
                    Descriptor::Width(_) => f32::MAX,
                };
                let db = match b.descriptor {
                    Descriptor::Density(d) => (d - dpr).abs(),
                    Descriptor::Width(_) => f32::MAX,
                };
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|c| c.url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_width_descriptors() {
        let candidates = parse_srcset("small.jpg 480w, medium.jpg 800w, large.jpg 1600w");
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].url, "small.jpg");
        assert_eq!(candidates[0].descriptor, Descriptor::Width(480));
        assert_eq!(candidates[2].descriptor, Descriptor::Width(1600));
    }

    #[test]
    fn parses_density_descriptors_and_defaults_missing_to_1x() {
        let candidates = parse_srcset("base.jpg, base-2x.jpg 2x, base-3x.jpg 3x");
        assert_eq!(candidates[0].descriptor, Descriptor::Density(1.0));
        assert_eq!(candidates[1].descriptor, Descriptor::Density(2.0));
        assert_eq!(candidates[2].descriptor, Descriptor::Density(3.0));
    }

    #[test]
    fn sizes_defaults_to_100vw_when_absent() {
        assert_eq!(eval_sizes(None, 1000.0), 1000.0);
    }

    #[test]
    fn sizes_evaluates_max_width_condition() {
        let sizes = "(max-width: 600px) 480px, 800px";
        // Narrow viewport: condition matches, use 480px.
        assert_eq!(eval_sizes(Some(sizes), 400.0), 480.0);
        // Wide viewport: condition doesn't match, fall through to unconditional 800px.
        assert_eq!(eval_sizes(Some(sizes), 1200.0), 800.0);
    }

    #[test]
    fn sizes_evaluates_vw_lengths() {
        // 50vw at a 1000px viewport should resolve to 500px.
        assert_eq!(eval_sizes(Some("50vw"), 1000.0), 500.0);
    }

    #[test]
    fn select_width_descriptor_picks_smallest_sufficient_candidate() {
        let srcset = "small.jpg 480w, medium.jpg 800w, large.jpg 1600w";
        // Need ~640px (320 CSS px * 2 DPR) — 800w is the smallest that covers it.
        assert_eq!(select_srcset_candidate(srcset, 320.0, 2.0), Some("medium.jpg".to_string()));
    }

    #[test]
    fn select_width_descriptor_falls_back_to_largest_when_nothing_is_big_enough() {
        let srcset = "small.jpg 480w, medium.jpg 800w";
        assert_eq!(select_srcset_candidate(srcset, 2000.0, 2.0), Some("medium.jpg".to_string()));
    }

    #[test]
    fn select_density_descriptor_picks_closest_to_dpr() {
        let srcset = "base.jpg 1x, base-2x.jpg 2x, base-3x.jpg 3x";
        assert_eq!(select_srcset_candidate(srcset, 100.0, 2.0), Some("base-2x.jpg".to_string()));
    }

    #[test]
    fn select_returns_none_for_empty_srcset() {
        assert_eq!(select_srcset_candidate("", 100.0, 1.0), None);
        assert_eq!(select_srcset_candidate("   ", 100.0, 1.0), None);
    }
}
