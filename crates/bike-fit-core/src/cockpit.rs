//! Cockpit components: stems, spacers, and cockpit kinds.
//!
//! See `AGENTS.md` §4.4 for the distinction between cockpit kinds.

use serde::{Deserialize, Serialize};

/// A stem SKU: a length plus an angle.
///
/// `angle_deg` is measured relative to the line perpendicular to the steerer
/// (industry convention). Positive rises above the perpendicular, negative
/// drops below it. A "−6° stem" — the most common default — has
/// `angle_deg = -6.0`.
///
/// Flipping a stem inverts the sign of `angle_deg`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stem {
    pub length_mm: f64,
    pub angle_deg: f64,
}

/// A spacer SKU: a single height in millimeters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Spacer {
    pub height_mm: u16,
}

/// A list of stem SKUs the solver may choose from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemCatalog {
    pub stems: Vec<Stem>,
}

impl StemCatalog {
    /// Default catalog: lengths 70–130 mm in 10 mm steps × angles ±6°, ±17°.
    /// Matches what the major manufacturers offer (Zipp, Pro, Deda, etc.).
    pub fn default_traditional() -> Self {
        let lengths = [70.0, 80.0, 90.0, 100.0, 110.0, 120.0, 130.0];
        let angles = [-17.0, -6.0, 6.0, 17.0];
        let mut stems = Vec::with_capacity(lengths.len() * angles.len());
        for &l in &lengths {
            for &a in &angles {
                stems.push(Stem {
                    length_mm: l,
                    angle_deg: a,
                });
            }
        }
        Self { stems }
    }
}

/// A list of spacer SKUs the solver may combine, plus the maximum allowed
/// total stack height. The solver enumerates all non-negative integer
/// combinations of these SKUs whose sum is ≤ `max_stack_mm`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpacerCatalog {
    pub spacers: Vec<Spacer>,
    pub max_stack_mm: u16,
}

impl SpacerCatalog {
    /// Default catalog: 3, 5, 10, 20 mm SKUs, 60 mm max stack.
    pub fn default_set() -> Self {
        Self {
            spacers: vec![
                Spacer { height_mm: 3 },
                Spacer { height_mm: 5 },
                Spacer { height_mm: 10 },
                Spacer { height_mm: 20 },
            ],
            max_stack_mm: 60,
        }
    }

    /// Enumerate every reachable total spacer height (in mm) up to the max,
    /// sorted ascending and deduplicated. Always includes 0 (no spacers).
    pub fn reachable_totals_mm(&self) -> Vec<u16> {
        let max = self.max_stack_mm;
        let mut reachable = std::collections::BTreeSet::new();
        reachable.insert(0u16);
        // Iterative BFS over reachable sums.
        let mut frontier: Vec<u16> = vec![0];
        while let Some(s) = frontier.pop() {
            for sp in &self.spacers {
                let next = s.saturating_add(sp.height_mm);
                if next <= max && reachable.insert(next) {
                    frontier.push(next);
                }
            }
        }
        reachable.into_iter().collect()
    }
}

/// An integrated-cockpit SKU: a single bonded part where length, angle, bar
/// reach, and bar drop are all properties of the part. The user picks one of
/// these; the solver doesn't combine them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IntegratedSku {
    pub length_mm: f64,
    pub angle_deg: f64,
    pub bar_reach_mm: f64,
    pub bar_drop_mm: f64,
}

impl IntegratedSku {
    /// View this integrated SKU as a stem for the solver.
    pub fn as_stem(&self) -> crate::Stem {
        crate::Stem {
            length_mm: self.length_mm,
            angle_deg: self.angle_deg,
        }
    }
}

/// The cockpit configuration the solver searches over. See `AGENTS.md` §4.4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Cockpit {
    /// Separate stem (length × angle from a catalog) plus a separate bar.
    /// Spacers freely combinable.
    Traditional {
        stems: StemCatalog,
        spacers: SpacerCatalog,
        headset_top_cap_mm: f64,
    },
    /// Aero stem (length is the only variable, angle is fixed by the part)
    /// bolted to a separate bar. Spacers normal.
    AeroStem {
        stems: StemCatalog,
        spacers: SpacerCatalog,
        headset_top_cap_mm: f64,
    },
    /// Fully integrated cockpit. Length & angle are properties of the SKU;
    /// the user picks a SKU. Spacers may still be in play depending on the
    /// part — modeled as a normal spacer catalog here.
    Integrated {
        skus: Vec<IntegratedSku>,
        spacers: SpacerCatalog,
        headset_top_cap_mm: f64,
    },
}

impl Cockpit {
    /// Default traditional cockpit: standard stem catalog, standard spacer
    /// set, 5 mm top cap.
    pub fn default_traditional() -> Self {
        Self::Traditional {
            stems: StemCatalog::default_traditional(),
            spacers: SpacerCatalog::default_set(),
            headset_top_cap_mm: 5.0,
        }
    }

    pub fn headset_top_cap_mm(&self) -> f64 {
        match self {
            Self::Traditional {
                headset_top_cap_mm, ..
            }
            | Self::AeroStem {
                headset_top_cap_mm, ..
            }
            | Self::Integrated {
                headset_top_cap_mm, ..
            } => *headset_top_cap_mm,
        }
    }

    pub fn spacers(&self) -> &SpacerCatalog {
        match self {
            Self::Traditional { spacers, .. }
            | Self::AeroStem { spacers, .. }
            | Self::Integrated { spacers, .. } => spacers,
        }
    }

    /// Stems the solver should search over.
    pub fn candidate_stems(&self) -> Vec<Stem> {
        match self {
            Self::Traditional { stems, .. } | Self::AeroStem { stems, .. } => stems.stems.clone(),
            Self::Integrated { skus, .. } => skus.iter().map(IntegratedSku::as_stem).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_traditional_catalog_has_28_stems() {
        let c = StemCatalog::default_traditional();
        // 7 lengths × 4 angles.
        assert_eq!(c.stems.len(), 28);
    }

    #[test]
    fn default_spacer_set_includes_zero_and_max() {
        let s = SpacerCatalog::default_set();
        let totals = s.reachable_totals_mm();
        assert_eq!(totals[0], 0, "zero must always be reachable");
        let max = *totals.last().unwrap();
        assert!(max <= s.max_stack_mm, "{max} > {}", s.max_stack_mm);
        // We should be able to hit 60 mm exactly with 3×20, 6×10, etc.
        assert!(totals.contains(&60));
    }

    #[test]
    fn reachable_totals_are_sorted_unique() {
        let s = SpacerCatalog::default_set();
        let totals = s.reachable_totals_mm();
        for w in totals.windows(2) {
            assert!(w[0] < w[1], "not sorted/unique at {w:?}");
        }
    }

    #[test]
    fn reachable_totals_include_small_increments() {
        // With a 3 mm SKU available, we should be able to make 3, 6, 9, 13, ...
        let s = SpacerCatalog::default_set();
        let totals = s.reachable_totals_mm();
        for t in [3, 5, 6, 8, 10, 13, 15, 20, 25, 30] {
            assert!(
                totals.contains(&t),
                "expected {t} mm to be reachable; got {totals:?}"
            );
        }
    }
}
