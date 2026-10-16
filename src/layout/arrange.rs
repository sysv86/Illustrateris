//! Phase 4: turns a BSP topology into integer 12-column placements by solving a constraint system.
//!
//! Variables: x, y, w, h for every BSP node (columns for x/w, abstract row units for y/h).
//!
//! REQUIRED   tree structure (children tile the parent exactly), root rectangle, leaf min width / min height
//! STRONG     leaf max width (measure), leaf preferred minimum width (line-count target)
//! MEDIUM     split ratio targets (golden-section family), alignment of sibling horizontal cuts
//! WEAK       leaf "ideal max" width (do not grow far beyond a single line)
//!
//! The continuous solution is then quantized top-down: every vertical cut is rounded to a whole column
//! (clamped so each subtree keeps its required minimum), so columns never overlap or leave gaps.

use super::solver::{Expr, Rel, Solution, SolveError, Solver, Strength, Var};
use super::space::{Axis, Bsp};

#[derive(Debug, Clone)]
pub struct Block {
    /// Required minimum width in columns (longest unbreakable word, control minimums).
    pub min_cols: f64,
    /// Preferred minimum width in columns (e.g. headline <= 3 lines).
    pub pref_cols: f64,
    /// Soft upper bound: wider than this is wasted (single line).
    pub ideal_max_cols: f64,
    /// Strong upper bound: reading measure.
    pub max_cols: f64,
    /// Required minimum height in row units.
    pub min_h: f64,
    /// Grammar weight: heavier blocks receive the larger side of a split.
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub block: usize,
    /// 1-based CSS grid line.
    pub col_start: u32,
    pub col_span: u32,
    /// 1-based CSS grid line.
    pub row_start: u32,
    pub row_span: u32,
    pub y: f64,
    pub h: f64,
}

#[derive(Debug, Clone)]
pub struct CutStat {
    pub axis: Axis,
    pub target_ratio: f64,
    pub actual_ratio: f64,
    /// Area share of the node being cut (page-normalized); used to weight imbalance.
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct Arrangement {
    pub placements: Vec<Placement>,
    /// Height of each row track in row units (sums to `height`).
    pub row_heights: Vec<f64>,
    /// Weighted soft-constraint cost from the solver.
    pub cost: f64,
    pub cuts: Vec<CutStat>,
    pub cols: u32,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrangeError {
    Infeasible,
    Solver(String),
}

#[derive(Debug, Clone, Copy)]
enum FKind {
    Leaf(usize),
    Split { axis: Axis, ratio: f64, first: usize, second: usize },
}

fn flatten(t: &Bsp, out: &mut Vec<FKind>) -> usize {
    let idx = out.len();
    out.push(FKind::Leaf(0));
    match t {
        Bsp::Leaf(b) => {
            out[idx] = FKind::Leaf(*b);
        }
        Bsp::Split { axis, ratio, first, second } => {
            let f = flatten(first, out);
            let s = flatten(second, out);
            out[idx] = FKind::Split { axis: *axis, ratio: *ratio, first: f, second: s };
        }
    }
    idx
}

const X: usize = 0;
const Y: usize = 1;
const W: usize = 2;
const H: usize = 3;

fn add(s: &mut Solver, terms: &[(Var, f64)], constant: f64, rel: Rel, st: Strength, weight: f64) {
    let mut e = Expr::new();
    for &(v, k) in terms {
        e = e.term(v, k);
    }
    s.add(e.constant(constant), rel, st, weight);
}

fn min_cols_sub(idx: usize, nodes: &[FKind], blocks: &[Block]) -> u32 {
    match nodes[idx] {
        FKind::Leaf(b) => blocks[b].min_cols.ceil().max(1.0) as u32,
        FKind::Split { axis, first, second, .. } => {
            let a = min_cols_sub(first, nodes, blocks);
            let b = min_cols_sub(second, nodes, blocks);
            match axis {
                Axis::Vertical => a + b,
                Axis::Horizontal => a.max(b),
            }
        }
    }
}

pub fn arrange(tree: &Bsp, blocks: &[Block], cols: u32, height: f64) -> Result<Arrangement, ArrangeError> {
    let mut nodes: Vec<FKind> = Vec::new();
    flatten(tree, &mut nodes);

    let mut s = Solver::new();
    let vars: Vec<[Var; 4]> = (0..nodes.len())
        .map(|_| [s.new_var(), s.new_var(), s.new_var(), s.new_var()])
        .collect();

    let cols_f = cols as f64;
    let root = vars[0];
    add(&mut s, &[(root[X], 1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
    add(&mut s, &[(root[Y], 1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
    add(&mut s, &[(root[W], 1.0)], -cols_f, Rel::Eq, Strength::Required, 1.0);
    add(&mut s, &[(root[H], 1.0)], -height, Rel::Eq, Strength::Required, 1.0);

    for idx in 0..nodes.len() {
        match nodes[idx] {
            FKind::Leaf(b) => {
                let blk = &blocks[b];
                let n = vars[idx];
                add(&mut s, &[(n[W], 1.0)], -blk.min_cols, Rel::Ge, Strength::Required, 1.0);
                add(&mut s, &[(n[H], 1.0)], -blk.min_h, Rel::Ge, Strength::Required, 1.0);
                add(&mut s, &[(n[W], 1.0)], -blk.max_cols, Rel::Le, Strength::Strong, 1.0);
                add(&mut s, &[(n[W], 1.0)], -blk.pref_cols, Rel::Ge, Strength::Strong, 1.0);
                add(&mut s, &[(n[W], 1.0)], -blk.ideal_max_cols, Rel::Le, Strength::Weak, 1.0);
            }
            FKind::Split { axis, ratio, first, second } => {
                let n = vars[idx];
                let a = vars[first];
                let b = vars[second];
                match axis {
                    Axis::Vertical => {
                        add(&mut s, &[(a[X], 1.0), (n[X], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(a[Y], 1.0), (n[Y], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(a[H], 1.0), (n[H], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(b[Y], 1.0), (n[Y], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(b[H], 1.0), (n[H], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(b[X], 1.0), (a[X], -1.0), (a[W], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(a[W], 1.0), (b[W], 1.0), (n[W], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(a[W], 1.0), (n[W], -ratio)], 0.0, Rel::Eq, Strength::Medium, 1.0);
                        // Tidy grid: if both children are horizontal splits, align their cut lines.
                        if let (
                            FKind::Split { axis: Axis::Horizontal, first: af, .. },
                            FKind::Split { axis: Axis::Horizontal, first: bf, .. },
                        ) = (nodes[first], nodes[second])
                        {
                            add(&mut s, &[(vars[af][H], 1.0), (vars[bf][H], -1.0)], 0.0, Rel::Eq, Strength::Medium, 1.0);
                        }
                    }
                    Axis::Horizontal => {
                        add(&mut s, &[(a[X], 1.0), (n[X], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(a[W], 1.0), (n[W], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(a[Y], 1.0), (n[Y], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(b[X], 1.0), (n[X], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(b[W], 1.0), (n[W], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(b[Y], 1.0), (a[Y], -1.0), (a[H], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(a[H], 1.0), (b[H], 1.0), (n[H], -1.0)], 0.0, Rel::Eq, Strength::Required, 1.0);
                        add(&mut s, &[(a[H], 1.0), (n[H], -ratio)], 0.0, Rel::Eq, Strength::Medium, 1.0);
                    }
                }
            }
        }
    }

    let sol = match s.solve() {
        Ok(sol) => sol,
        Err(SolveError::Infeasible) => return Err(ArrangeError::Infeasible),
        Err(e) => return Err(ArrangeError::Solver(e.to_string())),
    };

    let mut placements: Vec<Placement> = Vec::new();
    let mut cuts: Vec<CutStat> = Vec::new();
    place(0, 0, cols, cols, height, &nodes, &vars, &sol, blocks, &mut placements, &mut cuts)?;
    placements.sort_by_key(|p| p.block);

    // Row tracks: every distinct horizontal boundary of every leaf becomes a track edge.
    let tol = 1e-4;
    let mut bounds: Vec<f64> = vec![0.0, height];
    for p in &placements {
        bounds.push(p.y.clamp(0.0, height));
        bounds.push((p.y + p.h).clamp(0.0, height));
    }
    bounds.sort_by(|a, b| a.total_cmp(b));
    let mut uniq: Vec<f64> = Vec::new();
    for b in bounds {
        let is_new = match uniq.last() {
            Some(l) => b - *l > tol,
            None => true,
        };
        if is_new {
            uniq.push(b);
        }
    }
    let needs_push = match uniq.last() {
        Some(l) => (height - *l).abs() > tol,
        None => true,
    };
    if needs_push {
        uniq.push(height);
    } else if let Some(last) = uniq.last_mut() {
        *last = height;
    }
    let row_heights: Vec<f64> = uniq.windows(2).map(|w| w[1] - w[0]).collect();
    for p in placements.iter_mut() {
        let start = nearest(&uniq, p.y);
        let end = nearest(&uniq, p.y + p.h);
        p.row_start = start as u32 + 1;
        p.row_span = (end.saturating_sub(start)).max(1) as u32;
    }

    Ok(Arrangement { placements, row_heights, cost: sol.cost, cuts, cols, height })
}

fn nearest(sorted: &[f64], v: f64) -> usize {
    let mut best = 0usize;
    let mut best_d = f64::INFINITY;
    for (i, b) in sorted.iter().enumerate() {
        let d = (b - v).abs();
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

#[allow(clippy::too_many_arguments)]
fn place(
    idx: usize,
    x: u32,
    w: u32,
    cols_total: u32,
    height: f64,
    nodes: &[FKind],
    vars: &[[Var; 4]],
    sol: &Solution,
    blocks: &[Block],
    out: &mut Vec<Placement>,
    cuts: &mut Vec<CutStat>,
) -> Result<(), ArrangeError> {
    if w < min_cols_sub(idx, nodes, blocks) {
        return Err(ArrangeError::Infeasible);
    }
    match nodes[idx] {
        FKind::Leaf(b) => {
            out.push(Placement {
                block: b,
                col_start: x + 1,
                col_span: w,
                row_start: 0,
                row_span: 0,
                y: sol.value(vars[idx][Y]),
                h: sol.value(vars[idx][H]),
            });
            Ok(())
        }
        FKind::Split { axis, ratio, first, second } => {
            let nh = sol.value(vars[idx][H]);
            let weight = (w as f64 / cols_total as f64) * (nh / height).max(0.0);
            match axis {
                Axis::Vertical => {
                    let nw = sol.value(vars[idx][W]);
                    let aw = sol.value(vars[first][W]);
                    let rho = if nw > 1e-9 { (aw / nw).clamp(0.0, 1.0) } else { ratio };
                    let min_a = min_cols_sub(first, nodes, blocks);
                    let min_b = min_cols_sub(second, nodes, blocks);
                    if min_a + min_b > w {
                        return Err(ArrangeError::Infeasible);
                    }
                    let mut ai = (rho * w as f64).round() as i64;
                    ai = ai.max(min_a as i64).min((w - min_b) as i64);
                    let ai = ai as u32;
                    cuts.push(CutStat { axis, target_ratio: ratio, actual_ratio: ai as f64 / w as f64, weight });
                    place(first, x, ai, cols_total, height, nodes, vars, sol, blocks, out, cuts)?;
                    place(second, x + ai, w - ai, cols_total, height, nodes, vars, sol, blocks, out, cuts)
                }
                Axis::Horizontal => {
                    let ah = sol.value(vars[first][H]);
                    let actual = if nh > 1e-9 { (ah / nh).clamp(0.0, 1.0) } else { ratio };
                    cuts.push(CutStat { axis, target_ratio: ratio, actual_ratio: actual, weight });
                    place(first, x, w, cols_total, height, nodes, vars, sol, blocks, out, cuts)?;
                    place(second, x, w, cols_total, height, nodes, vars, sol, blocks, out, cuts)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::space::stack_tree;

    fn blk(min: f64, pref: f64, ideal: f64, max: f64) -> Block {
        Block { min_cols: min, pref_cols: pref, ideal_max_cols: ideal, max_cols: max, min_h: 0.5, weight: 1.0 }
    }

    #[test]
    fn vertical_split_follows_ratio() {
        let tree = Bsp::Split {
            axis: Axis::Vertical,
            ratio: 0.667,
            first: Box::new(Bsp::Leaf(0)),
            second: Box::new(Bsp::Leaf(1)),
        };
        let blocks = vec![blk(3.0, 3.0, 12.0, 12.0), blk(3.0, 3.0, 12.0, 12.0)];
        let a = arrange(&tree, &blocks, 12, 6.0).unwrap();
        assert_eq!(a.placements[0].col_span + a.placements[1].col_span, 12);
        assert_eq!(a.placements[0].col_span, 8);
        assert_eq!(a.placements[1].col_start, 9);
    }

    #[test]
    fn infeasible_when_minimums_exceed_columns() {
        let tree = Bsp::Split {
            axis: Axis::Vertical,
            ratio: 0.5,
            first: Box::new(Bsp::Leaf(0)),
            second: Box::new(Bsp::Leaf(1)),
        };
        let blocks = vec![blk(7.0, 7.0, 12.0, 12.0), blk(7.0, 7.0, 12.0, 12.0)];
        assert_eq!(arrange(&tree, &blocks, 12, 6.0).unwrap_err(), ArrangeError::Infeasible);
    }

    #[test]
    fn stack_is_always_feasible() {
        let blocks = vec![blk(4.0, 6.0, 12.0, 12.0), blk(3.0, 4.0, 8.0, 10.0), blk(2.0, 3.0, 4.0, 6.0)];
        let a = arrange(&stack_tree(3), &blocks, 12, 6.0).unwrap();
        for p in &a.placements {
            assert_eq!(p.col_start, 1);
        }
        assert_eq!(a.row_heights.len(), 3);
        let sum: f64 = a.row_heights.iter().sum();
        assert!((sum - 6.0).abs() < 1e-6);
    }

    #[test]
    fn min_width_is_respected_after_rounding() {
        let tree = Bsp::Split {
            axis: Axis::Vertical,
            ratio: 0.15,
            first: Box::new(Bsp::Leaf(0)),
            second: Box::new(Bsp::Leaf(1)),
        };
        let blocks = vec![blk(4.0, 4.0, 12.0, 12.0), blk(3.0, 3.0, 12.0, 12.0)];
        let a = arrange(&tree, &blocks, 12, 6.0).unwrap();
        assert!(a.placements[0].col_span >= 4);
    }
}
