//! Candidate search: generate K BSP topologies, solve each, reject infeasible ones, score the rest
//! against the anti-chaos invariants, keep the best. Deterministic for a given seed.

use super::arrange::{arrange, ArrangeError, Arrangement, Block, Placement};
use super::rng::{derive_seed, Rng};
use super::space::{generate, stack_tree, wrap_tree, Bsp, GenParams, Grammar};
use super::visual_mass::{centroid_x, centroid_y, eccentricity, eccentricity_y, imbalance_factor, range_distance};

pub struct SearchInput<'a> {
    pub blocks: &'a [Block],
    /// Precomputed visual mass per block (depends on palette and section background, not on layout).
    pub masses: &'a [f64],
    pub grammar: Grammar,
    pub cols: u32,
    pub height: f64,
    pub asymmetry: f64,
    pub candidates: usize,
    pub seed: u64,
    pub max_ecc: f64,
    /// Allowed imbalance range from entropy damping.
    pub allowed: (f64, f64),
}

/// Rows tried when a section cannot be realized under its preferred axis policy.
const MAX_WRAP_ROWS: usize = 4;

#[derive(Debug, Clone)]
pub struct Chosen {
    pub arrangement: Arrangement,
    pub tree: Bsp,
    pub candidate_index: usize,
    pub tried: usize,
    pub feasible: usize,
    pub eccentricity: f64,
    pub centroid_y: f64,
    pub eccentricity_y: f64,
    pub factor: f64,
    pub penalty: f64,
    /// True when no generated candidate was realizable and the single-column stack was used.
    pub fallback: bool,
    pub warnings: Vec<String>,
}

fn score(arr: &Arrangement, ecc: f64, ecc_y: f64, factor: f64, inp: &SearchInput) -> f64 {
    let ecc_pen = if ecc > inp.max_ecc {
        1000.0 * (ecc - inp.max_ecc) + 50.0 * ecc
    } else {
        10.0 * ecc
    };
    let vert_pen = if arr.row_heights.len() >= 2 && ecc_y > 0.35 {
        15.0 * (ecc_y - 0.35)
    } else {
        0.0
    };
    let range_pen = if inp.blocks.len() >= 2 {
        200.0 * range_distance(factor, inp.allowed.0, inp.allowed.1)
    } else {
        0.0
    };
    let target_pen = if inp.blocks.len() >= 2 { 5.0 * (factor - inp.asymmetry).abs() } else { 0.0 };
    let cost_pen = arr.cost / 100.0;
    ecc_pen + vert_pen + range_pen + target_pen + cost_pen
}

fn manual_stack(blocks: &[Block], cols: u32, height: f64) -> Arrangement {
    let n = blocks.len().max(1);
    let h = height / n as f64;
    let placements: Vec<Placement> = (0..blocks.len())
        .map(|i| Placement {
            block: i,
            col_start: 1,
            col_span: cols,
            row_start: i as u32 + 1,
            row_span: 1,
            y: h * i as f64,
            h,
        })
        .collect();
    Arrangement { placements, row_heights: vec![h; n], cost: 0.0, cuts: vec![], cols, height }
}

pub fn search(inp: &SearchInput) -> Chosen {
    let chosen = search_grammar(inp, inp.grammar);
    if !chosen.fallback {
        return chosen;
    }
    // A restrictive axis policy can make a section unrealizable even though it wraps cleanly: a nav
    // with more links than fit side by side on one 12-column row has no all-vertical solution, yet
    // it is trivially solvable as two rows. Before degrading to the full-width stack (which is not a
    // nav at all), try ordered wraps — row 1 = blocks 0..k, row 2 = the rest — so reading order and
    // focus order are preserved.
    let mut best = chosen;
    for rows in 2..=inp.blocks.len().min(MAX_WRAP_ROWS) {
        let tree = wrap_tree(inp.blocks.len(), rows);
        let Ok(arr) = arrange(&tree, inp.blocks, inp.cols, inp.height) else { continue };
        let (cx, _) = centroid_x(&arr, inp.masses);
        let (cy, _) = centroid_y(&arr, inp.masses);
        let ecc = eccentricity(cx);
        let ecc_y = eccentricity_y(cy);
        let factor = imbalance_factor(&arr.cuts, ecc);
        let penalty = score(&arr, ecc, ecc_y, factor, inp);
        if best.fallback || penalty < best.penalty - 1e-12 {
            best = Chosen {
                arrangement: arr,
                tree,
                candidate_index: 0,
                tried: inp.candidates.max(1),
                feasible: 1,
                eccentricity: ecc,
                centroid_y: cy,
                eccentricity_y: ecc_y,
                factor,
                penalty,
                fallback: false,
                warnings: Vec::new(),
            };
        }
    }
    best
}

fn search_grammar(inp: &SearchInput, grammar: Grammar) -> Chosen {
    let weights: Vec<f64> = inp.blocks.iter().map(|b| b.weight).collect();
    let params = GenParams { asymmetry: inp.asymmetry, grammar, width: inp.cols as f64, height: inp.height };
    let tried = inp.candidates.max(1);
    let mut feasible = 0usize;
    let mut best: Option<Chosen> = None;

    for k in 0..tried {
        let mut rng = Rng::new(derive_seed(inp.seed, k as u64));
        let tree = generate(&mut rng, &weights, &params);
        let arr = match arrange(&tree, inp.blocks, inp.cols, inp.height) {
            Ok(a) => a,
            Err(_) => continue,
        };
        feasible += 1;
        let (cx, _) = centroid_x(&arr, inp.masses);
        let (cy, _) = centroid_y(&arr, inp.masses);
        let ecc = eccentricity(cx);
        let ecc_y = eccentricity_y(cy);
        let factor = imbalance_factor(&arr.cuts, ecc);
        let penalty = score(&arr, ecc, ecc_y, factor, inp);
        let better = match &best {
            Some(b) => penalty < b.penalty - 1e-12,
            None => true,
        };
        if better {
            best = Some(Chosen {
                arrangement: arr,
                tree,
                candidate_index: k,
                tried,
                feasible: 0,
                eccentricity: ecc,
                centroid_y: cy,
                eccentricity_y: ecc_y,
                factor,
                penalty,
                fallback: false,
                warnings: Vec::new(),
            });
        }
    }

    if let Some(mut b) = best {
        b.feasible = feasible;
        if b.eccentricity > inp.max_ecc {
            b.warnings.push(format!(
                "best candidate has eccentricity {:.3} above the limit {:.3}",
                b.eccentricity, inp.max_ecc
            ));
        }
        return b;
    }

    // Fallback: single column stack. Infeasible only if a block's minimum exceeds the grid, in which
    // case the manual stack ignores widths and reports it.
    let tree = stack_tree(inp.blocks.len());
    let mut warnings = vec!["no feasible candidate; used single-column stack".to_string()];
    let arrangement = match arrange(&tree, inp.blocks, inp.cols, inp.height) {
        Ok(a) => a,
        Err(e) => {
            warnings.push(match e {
                ArrangeError::Infeasible => "a block minimum width exceeds the grid".to_string(),
                ArrangeError::Solver(m) => format!("solver error: {}", m),
            });
            manual_stack(inp.blocks, inp.cols, inp.height)
        }
    };
    let (cx, _) = centroid_x(&arrangement, inp.masses);
    let (cy, _) = centroid_y(&arrangement, inp.masses);
    let ecc = eccentricity(cx);
    let ecc_y = eccentricity_y(cy);
    let factor = imbalance_factor(&arrangement.cuts, ecc);
    Chosen {
        penalty: score(&arrangement, ecc, ecc_y, factor, inp),
        arrangement,
        tree,
        candidate_index: 0,
        tried,
        feasible: 0,
        eccentricity: ecc,
        centroid_y: cy,
        eccentricity_y: ecc_y,
        factor,
        fallback: true,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::space::AxisPolicy;

    fn blk(min: f64, pref: f64, ideal: f64, max: f64, weight: f64) -> Block {
        Block { min_cols: min, pref_cols: pref, ideal_max_cols: ideal, max_cols: max, min_h: 0.5, weight }
    }

    #[test]
    fn search_is_deterministic_and_valid() {
        let blocks = vec![blk(4.0, 6.0, 10.0, 12.0, 3.0), blk(3.0, 3.0, 5.0, 6.0, 1.0), blk(4.0, 5.0, 6.0, 7.0, 1.2)];
        let masses = vec![10.0, 2.0, 3.0];
        let inp = SearchInput {
            blocks: &blocks,
            masses: &masses,
            grammar: Grammar { policy: AxisPolicy::Free, header_band: false, equalize: false },
            cols: 12,
            height: 8.0,
            asymmetry: 0.6,
            candidates: 32,
            seed: 123,
            max_ecc: 0.15,
            allowed: (0.4, 0.8),
        };
        let a = search(&inp);
        let b = search(&inp);
        assert_eq!(a.candidate_index, b.candidate_index);
        assert_eq!(a.arrangement.placements.len(), 3);
        for p in &a.arrangement.placements {
            assert!(p.col_start >= 1 && p.col_start + p.col_span <= 13);
        }
    }

    #[test]
    fn falls_back_when_nothing_fits() {
        // A block wider than the grid cannot be realized by any topology, so the manual stack (which
        // ignores widths) is the last resort.
        let blocks = vec![blk(13.0, 13.0, 12.0, 12.0, 1.0), blk(13.0, 13.0, 12.0, 12.0, 1.0)];
        let masses = vec![1.0, 1.0];
        let inp = SearchInput {
            blocks: &blocks,
            masses: &masses,
            grammar: Grammar { policy: AxisPolicy::VerticalOnly, header_band: false, equalize: true },
            cols: 12,
            height: 4.0,
            asymmetry: 0.3,
            candidates: 8,
            seed: 1,
            max_ecc: 0.15,
            allowed: (0.1, 0.5),
        };
        let c = search(&inp);
        assert!(c.fallback);
        assert_eq!(c.arrangement.placements.len(), 2);
    }

    #[test]
    fn wraps_instead_of_stacking_when_one_row_cannot_fit() {
        // Three five-column blocks cannot sit side by side on a twelve-column row (15 > 12), but they
        // wrap cleanly into two rows. The old behaviour silently emitted a full-width stack, which is
        // not a nav at all; the wrap must win.
        let blocks = vec![
            blk(5.0, 5.0, 6.0, 6.0, 1.0),
            blk(5.0, 5.0, 6.0, 6.0, 1.0),
            blk(5.0, 5.0, 6.0, 6.0, 1.0),
        ];
        let masses = vec![1.0, 1.0, 1.0];
        let inp = SearchInput {
            blocks: &blocks,
            masses: &masses,
            grammar: Grammar { policy: AxisPolicy::VerticalOnly, header_band: false, equalize: false },
            cols: 12,
            height: 4.0,
            asymmetry: 0.3,
            candidates: 16,
            seed: 7,
            max_ecc: 0.15,
            allowed: (0.2, 0.6),
        };
        let c = search(&inp);
        assert!(!c.fallback, "a wrap must be preferred over the full-width stack");
        let places = &c.arrangement.placements;
        assert!(places.len() == 3);
        let rows: std::collections::BTreeSet<u32> = places.iter().map(|p| p.row_start).collect();
        assert_eq!(rows.len(), 2, "expected exactly two wrapped rows");
        // Reading order (block index) must match visual order (row, then column).
        for w in places.windows(2) {
            let ordered = w[1].row_start > w[0].row_start
                || (w[1].row_start == w[0].row_start && w[1].col_start >= w[0].col_start);
            assert!(ordered, "reading order broken: {:?} then {:?}", w[0], w[1]);
        }
        for p in places {
            assert!(p.col_start >= 1 && p.col_start + p.col_span <= 13);
        }
    }
}
