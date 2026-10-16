//! Visual mass, 2D centre of gravity (horizontal + vertical) and entropy damping.
//!
//! mass   M = InkArea * (|Lc| / 100) * (Weight / 400)
//! InkArea is the estimated area covered by glyphs (chars * advance * line box) or canvas surface,
//! not the raw grid cell: a cell holding a 6rem headline or active WebGPU surface outweighs an
//! equally large cell holding a caption.

use super::arrange::{Arrangement, CutStat};
use super::space::Axis;

pub fn mass(ink_px2: f64, lc_abs: f64, weight: u16) -> f64 {
    ink_px2.max(0.0) * (lc_abs / 100.0).max(0.0) * (weight as f64 / 400.0)
}

/// Mass-weighted horizontal centroid (0..1 of page width) and total mass.
pub fn centroid_x(arr: &Arrangement, masses: &[f64]) -> (f64, f64) {
    let mut total = 0.0;
    let mut acc = 0.0;
    for p in &arr.placements {
        let m = masses.get(p.block).copied().unwrap_or(0.0);
        let cx = ((p.col_start as f64 - 1.0) + p.col_span as f64 / 2.0) / arr.cols.max(1) as f64;
        total += m;
        acc += m * cx;
    }
    if total <= 0.0 {
        (0.5, 0.0)
    } else {
        (acc / total, total)
    }
}

/// Mass-weighted vertical centroid (0..1 of section height) and total mass.
pub fn centroid_y(arr: &Arrangement, masses: &[f64]) -> (f64, f64) {
    let mut total = 0.0;
    let mut acc = 0.0;
    let h_total = arr.height.max(1e-6);
    for p in &arr.placements {
        let m = masses.get(p.block).copied().unwrap_or(0.0);
        let cy = ((p.y + 0.5 * p.h) / h_total).clamp(0.0, 1.0);
        total += m;
        acc += m * cy;
    }
    if total <= 0.0 {
        (0.5, 0.0)
    } else {
        (acc / total, total)
    }
}

/// 2D centroid `(cx, cy, total_mass)` normalized to `[0, 1] x [0, 1]`.
pub fn centroid_2d(arr: &Arrangement, masses: &[f64]) -> (f64, f64, f64) {
    let (cx, total) = centroid_x(arr, masses);
    let (cy, _) = centroid_y(arr, masses);
    (cx, cy, total)
}

pub fn eccentricity(cx: f64) -> f64 {
    (cx - 0.5).abs()
}

/// Vertical eccentricity from the optical vertical centre (0.48 of section height).
pub fn eccentricity_y(cy: f64) -> f64 {
    (cy - 0.48).abs()
}

/// Imbalance factor in [0,1].
///
/// o = min(1, |ratio - 0.5| / 0.25) per vertical cut (golden split ~0.47, 0.7 split = 0.8, 0.75+ = 1);
/// factor = 0.7 * area-weighted mean(o) + 0.3 * min(1, eccentricity / 0.3).
/// A section without vertical cuts (pure stack) has a cut term of 0.
pub fn imbalance_factor(cuts: &[CutStat], ecc: f64) -> f64 {
    let mut wsum = 0.0;
    let mut acc = 0.0;
    for c in cuts {
        if c.axis == Axis::Vertical {
            let o = ((c.actual_ratio - 0.5).abs() / 0.25).min(1.0);
            wsum += c.weight;
            acc += c.weight * o;
        }
    }
    let cut_term = if wsum > 0.0 { acc / wsum } else { 0.0 };
    (0.7 * cut_term + 0.3 * (ecc / 0.3).min(1.0)).clamp(0.0, 1.0)
}

/// Previous-section factor above which the next section is forced into structural calm.
pub const DAMPING_THRESHOLD: f64 = 0.6;
pub const CALM_RANGE: (f64, f64) = (0.2, 0.4);

/// Entropy damping: after a high-imbalance section the next one is clamped to the calm range;
/// otherwise the allowed range is the intent's asymmetry +- 0.2.
pub fn allowed_range(prev: Option<f64>, target: f64) -> (f64, f64) {
    match prev {
        Some(p) if p > DAMPING_THRESHOLD => CALM_RANGE,
        _ => ((target - 0.2).max(0.0), (target + 0.2).min(1.0)),
    }
}

pub fn range_distance(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo - v
    } else if v > hi {
        v - hi
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::Placement;

    fn arr(places: Vec<(u32, u32)>) -> Arrangement {
        Arrangement {
            placements: places
                .into_iter()
                .enumerate()
                .map(|(i, (s, w))| Placement { block: i, col_start: s, col_span: w, row_start: 1, row_span: 1, y: 0.0, h: 1.0 })
                .collect(),
            row_heights: vec![1.0],
            cost: 0.0,
            cuts: vec![],
            cols: 12,
            height: 1.0,
        }
    }

    #[test]
    fn centred_mass_has_zero_eccentricity() {
        let a = arr(vec![(1, 6), (7, 6)]);
        let (cx, _) = centroid_x(&a, &[1.0, 1.0]);
        assert!(eccentricity(cx) < 1e-9);
    }

    #[test]
    fn heavy_left_block_pulls_centroid() {
        let a = arr(vec![(1, 4), (9, 4)]);
        let (cx, _) = centroid_x(&a, &[10.0, 1.0]);
        assert!(cx < 0.5);
    }

    #[test]
    fn vertical_centroid_tracks_row_positions() {
        let a = Arrangement {
            placements: vec![
                Placement { block: 0, col_start: 1, col_span: 12, row_start: 1, row_span: 1, y: 0.0, h: 2.0 },
                Placement { block: 1, col_start: 1, col_span: 12, row_start: 2, row_span: 1, y: 2.0, h: 2.0 },
            ],
            row_heights: vec![2.0, 2.0],
            cost: 0.0,
            cuts: vec![],
            cols: 12,
            height: 4.0,
        };
        let (cy_even, _) = centroid_y(&a, &[1.0, 1.0]);
        assert!((cy_even - 0.5).abs() < 1e-9);
        let (cy_top, _) = centroid_y(&a, &[10.0, 1.0]);
        assert!(cy_top < 0.35);
    }

    #[test]
    fn damping_forces_calm_range() {
        assert_eq!(allowed_range(Some(0.8), 0.6), CALM_RANGE);
        let (lo, hi) = allowed_range(Some(0.3), 0.6);
        assert!((lo - 0.4).abs() < 1e-9 && (hi - 0.8).abs() < 1e-9);
    }

    #[test]
    fn stack_has_zero_cut_term() {
        assert_eq!(imbalance_factor(&[], 0.0), 0.0);
    }
}
