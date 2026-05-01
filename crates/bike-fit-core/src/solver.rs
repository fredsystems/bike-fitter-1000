//! The fit solver.
//!
//! Given a [`Frame`], a target bar point in BB coords, and a [`Cockpit`]
//! configuration, find the (stem, spacer-stack) combination that lands the
//! stem's far clamp face closest to the target.
//!
//! The search space is small (a few dozen stems × a few hundred reachable
//! spacer totals), so we brute-force it. Reported results include the
//! residual error vector so the UI can show "1.3 mm forward / 0.4 mm low"
//! style feedback.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{geometry, Cockpit, FitProfile, Frame, Point, Spacer, SpacerCatalog, Stem};

/// A single recommended bike build: which stem, which spacer breakdown,
/// where it lands, and how far off the target it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Build {
    pub stem: Stem,
    pub spacer_stack: SpacerStack,
    /// Where the stem's far clamp face actually lands in BB coords.
    pub achieved_bar_position: Point,
    /// `achieved_bar_position - target`. Positive x means we ended up forward
    /// of the target; positive y means above.
    pub residual: Point,
}

impl Build {
    /// Magnitude of the residual error in millimeters.
    pub fn error_mm(&self) -> f64 {
        self.residual.length()
    }
}

/// A spacer stack: a list of `(spacer_sku, count)` entries totaling the
/// `total_mm` height. The breakdown is one valid way to assemble the stack;
/// other equivalent breakdowns may exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpacerStack {
    pub total_mm: u16,
    /// Pairs of (spacer SKU, count). Sum of `sku.height_mm * count` equals
    /// `total_mm`.
    pub breakdown: Vec<(Spacer, u16)>,
}

#[derive(Debug, Clone, Error, PartialEq)]
pub enum BuildError {
    #[error("cockpit has no candidate stems")]
    NoStems,
    #[error("spacer catalog is empty")]
    NoSpacers,
}

/// Find the best (stem, spacer-stack) combination for `target`.
pub fn solve_for_target(
    frame: &Frame,
    target: Point,
    cockpit: &Cockpit,
) -> Result<Build, BuildError> {
    let stems = cockpit.candidate_stems();
    if stems.is_empty() {
        return Err(BuildError::NoStems);
    }
    let spacers = cockpit.spacers();
    if spacers.spacers.is_empty() {
        return Err(BuildError::NoSpacers);
    }
    let top_cap = cockpit.headset_top_cap_mm();
    let totals = spacers.reachable_totals_mm();

    let mut best: Option<(f64, Build)> = None;

    for &stem in &stems {
        for &total in &totals {
            let pos = geometry::stem_clamp_face(frame, top_cap, f64::from(total), stem);
            let residual = pos - target;
            let err = residual.length();
            if best.as_ref().is_none_or(|(b, _)| err < *b) {
                let stack = SpacerStack {
                    total_mm: total,
                    breakdown: greedy_breakdown(spacers, total),
                };
                best = Some((
                    err,
                    Build {
                        stem,
                        spacer_stack: stack,
                        achieved_bar_position: pos,
                        residual,
                    },
                ));
            }
        }
    }

    Ok(best
        .expect("totals always contains 0 and stems is non-empty")
        .1)
}

/// Solve a complete fit profile against a frame: just delegates to
/// [`solve_for_target`] using `profile.bar_target`.
pub fn solve_for_profile(
    frame: &Frame,
    profile: &FitProfile,
    cockpit: &Cockpit,
) -> Result<Build, BuildError> {
    solve_for_target(frame, profile.bar_target, cockpit)
}

/// Greedy breakdown of `total_mm` into spacer SKUs from `catalog`, taking the
/// largest first. Always succeeds when `total_mm` came from
/// `catalog.reachable_totals_mm()`, since that set is exactly the reachable
/// sums of non-negative integer combinations of the SKUs.
///
/// Note: greedy isn't the *minimum-spacer-count* breakdown for arbitrary SKU
/// sets, but with the default `[3, 5, 10, 20]` it is. We can swap in a
/// proper coin-change DP if a user supplies a pathological catalog.
fn greedy_breakdown(catalog: &SpacerCatalog, total_mm: u16) -> Vec<(Spacer, u16)> {
    let mut remaining = total_mm;
    // Sort SKUs descending by height.
    let mut skus: Vec<Spacer> = catalog.spacers.clone();
    skus.sort_by_key(|s| std::cmp::Reverse(s.height_mm));

    let mut out = Vec::new();
    for sku in skus {
        if sku.height_mm == 0 {
            continue;
        }
        let count = remaining / sku.height_mm;
        if count > 0 {
            out.push((sku, count));
            remaining -= count * sku.height_mm;
        }
        if remaining == 0 {
            break;
        }
    }
    // Fallback: if greedy somehow failed (it shouldn't for our defaults),
    // try a small DP. This keeps the function total even on weird catalogs.
    if remaining > 0 {
        if let Some(dp) = coin_change_breakdown(catalog, total_mm) {
            return dp;
        }
        // Truly unreachable — return what we have, the caller will see
        // a mismatched total. We assert via tests that this branch isn't
        // hit for standard catalogs.
    }
    out
}

/// Coin-change DP fallback for breaking `total_mm` into the catalog's SKUs.
/// Returns `None` if `total_mm` isn't reachable.
fn coin_change_breakdown(catalog: &SpacerCatalog, total_mm: u16) -> Option<Vec<(Spacer, u16)>> {
    let n = usize::from(total_mm);
    let mut prev: Vec<Option<Spacer>> = vec![None; n + 1];
    let mut reached = vec![false; n + 1];
    reached[0] = true;
    for i in 0..=n {
        if !reached[i] {
            continue;
        }
        for &sku in &catalog.spacers {
            let h = usize::from(sku.height_mm);
            if h == 0 {
                continue;
            }
            let j = i + h;
            if j <= n && !reached[j] {
                reached[j] = true;
                prev[j] = Some(sku);
            }
        }
    }
    if !reached[n] {
        return None;
    }
    // Walk back from n collecting SKUs.
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<u16, u16> = BTreeMap::new();
    let mut cur = n;
    while cur > 0 {
        let sku = prev[cur]?;
        *counts.entry(sku.height_mm).or_insert(0) += 1;
        cur -= usize::from(sku.height_mm);
    }
    Some(
        counts
            .into_iter()
            .rev()
            .map(|(h, c)| (Spacer { height_mm: h }, c))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::WheelSize;

    fn aeroad_2xs() -> Frame {
        Frame {
            manufacturer: "Canyon".into(),
            model: "Aeroad CF SLX 7".into(),
            size_label: "2XS".into(),
            year: Some(2025),
            stack_mm: 498.0,
            reach_mm: 372.0,
            head_tube_angle_deg: 70.0,
            head_tube_length_mm: 88.0,
            seat_tube_angle_deg: 73.5,
            seat_tube_length_mm: 441.0,
            top_tube_effective_mm: 516.0,
            bb_drop_mm: 70.0,
            chainstay_mm: 410.0,
            fork_rake_mm: 40.6,
            front_center_horizontal_mm: Some(571.0),
            wheel_size: WheelSize::Iso622,
            tire_width_mm: 28.0,
        }
    }

    #[test]
    fn greedy_breakdown_default_catalog_sums_to_total() {
        let cat = SpacerCatalog::default_set();
        for total in cat.reachable_totals_mm() {
            let bd = greedy_breakdown(&cat, total);
            let sum: u16 = bd.iter().map(|(s, c)| s.height_mm * c).sum();
            assert_eq!(sum, total, "breakdown {bd:?} doesn't sum to {total}");
        }
    }

    #[test]
    fn round_trip_recovers_exact_combo() {
        // Pick a stem & spacer combo, compute the bar target it produces,
        // then ask the solver to recover it. Residual should be ~0.
        let frame = aeroad_2xs();
        let cockpit = Cockpit::default_traditional();
        let chosen_stem = Stem {
            length_mm: 100.0,
            angle_deg: -6.0,
        };
        let chosen_total = 30u16;
        let target = geometry::stem_clamp_face(
            &frame,
            cockpit.headset_top_cap_mm(),
            f64::from(chosen_total),
            chosen_stem,
        );

        let build = solve_for_target(&frame, target, &cockpit).unwrap();

        assert_eq!(build.stem, chosen_stem);
        assert_eq!(build.spacer_stack.total_mm, chosen_total);
        assert!(
            build.error_mm() < 1e-9,
            "residual was {} mm",
            build.error_mm()
        );
    }

    #[test]
    fn solver_residual_bounded_for_near_target() {
        // Offset an exactly-reachable target by 1mm. The solver should land
        // within ~1mm (it can pick any near-tied combo; we don't pin which).
        let frame = aeroad_2xs();
        let cockpit = Cockpit::default_traditional();
        let stem = Stem {
            length_mm: 100.0,
            angle_deg: -6.0,
        };
        let exact = geometry::stem_clamp_face(&frame, 5.0, 30.0, stem);
        let target = exact + Point::new(1.0, 0.0);

        let build = solve_for_target(&frame, target, &cockpit).unwrap();
        // Best combo can't be worse than the original combo's 1mm residual.
        assert!(
            build.error_mm() <= 1.0 + 1e-9,
            "expected ≤1mm residual, got {} mm with stem {:?} stack {}",
            build.error_mm(),
            build.stem,
            build.spacer_stack.total_mm,
        );
        // Sanity: residual vector matches achieved - target.
        let recomputed = build.achieved_bar_position - target;
        assert!((recomputed.x - build.residual.x).abs() < 1e-12);
        assert!((recomputed.y - build.residual.y).abs() < 1e-12);
    }

    #[test]
    fn solver_searches_all_stems_and_spacer_totals() {
        // Big offset target — solver should still return the closest combo,
        // not one specific to a particular stem.
        let frame = aeroad_2xs();
        let cockpit = Cockpit::default_traditional();
        // A target far above the frame; closest is the longest stem at +17°
        // with the most spacers.
        let target = Point::new(500.0, 700.0);
        let build = solve_for_target(&frame, target, &cockpit).unwrap();
        // The solver should pick a high-rise long stem.
        assert!(
            build.stem.angle_deg > 0.0,
            "expected positive angle, got {}",
            build.stem.angle_deg
        );
        assert!(
            build.stem.length_mm >= 100.0,
            "expected long stem, got {}",
            build.stem.length_mm
        );
    }

    #[test]
    fn empty_stem_catalog_errors() {
        let frame = aeroad_2xs();
        let cockpit = Cockpit::Traditional {
            stems: crate::StemCatalog { stems: vec![] },
            spacers: SpacerCatalog::default_set(),
            headset_top_cap_mm: 5.0,
        };
        let err = solve_for_target(&frame, Point::new(400.0, 550.0), &cockpit).unwrap_err();
        assert_eq!(err, BuildError::NoStems);
    }

    #[test]
    fn solve_for_profile_delegates_to_target() {
        let frame = aeroad_2xs();
        let cockpit = Cockpit::default_traditional();
        let stem = Stem {
            length_mm: 110.0,
            angle_deg: -6.0,
        };
        let target = geometry::stem_clamp_face(&frame, 5.0, 20.0, stem);
        let profile = FitProfile::new("rider", Point::new(-50.0, 700.0), target);

        let build = solve_for_profile(&frame, &profile, &cockpit).unwrap();
        assert_eq!(build.stem, stem);
        assert_eq!(build.spacer_stack.total_mm, 20);
    }
}
