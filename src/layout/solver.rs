//! Linear constraint solver with Cassowary-style constraint strengths.
//!
//! Cassowary is a simplex method over linear equalities / inequalities in which every non-required
//! constraint carries a weighted error term that is minimized. This module implements exactly that
//! formulation as a dense two-phase simplex (Bland's anti-cycling rule):
//!
//!   * variables are free reals (split internally as x = p - n, p,n >= 0),
//!   * REQUIRED constraints must hold exactly (phase I proves feasibility),
//!   * STRONG / MEDIUM / WEAK constraints get non-negative error variables with weights
//!     1e6 / 1e3 / 1 (times a per-constraint weight) and the weighted error sum is minimized (phase II).
//!
//! Problem sizes here are tiny (tens of variables), so a dense tableau is the simpler and more
//! predictable choice than Cassowary's incremental sparse tableau; the semantics are the same
//! except that it re-solves from scratch instead of incrementally.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Var(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rel {
    Eq,
    Le,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Required,
    Strong,
    Medium,
    Weak,
}

impl Strength {
    fn base_weight(self) -> f64 {
        match self {
            Strength::Required => 0.0,
            Strength::Strong => 1.0e6,
            Strength::Medium => 1.0e3,
            Strength::Weak => 1.0,
        }
    }
}

/// Linear expression: sum(coeff * var) + constant.
#[derive(Debug, Clone, Default)]
pub struct Expr {
    pub terms: Vec<(Var, f64)>,
    pub constant: f64,
}

impl Expr {
    pub fn new() -> Expr {
        Expr::default()
    }

    pub fn term(mut self, v: Var, k: f64) -> Expr {
        self.terms.push((v, k));
        self
    }

    pub fn constant(mut self, c: f64) -> Expr {
        self.constant += c;
        self
    }
}

#[derive(Debug, Clone)]
struct Constraint {
    expr: Expr,
    rel: Rel,
    strength: Strength,
    weight: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Solver {
    n_vars: usize,
    constraints: Vec<Constraint>,
}

#[derive(Debug, Clone)]
pub struct Solution {
    pub values: Vec<f64>,
    /// Weighted sum of all soft-constraint errors.
    pub cost: f64,
    /// Raw error per constraint (0 for required or satisfied constraints), in insertion order.
    pub violations: Vec<f64>,
}

impl Solution {
    pub fn value(&self, v: Var) -> f64 {
        self.values[v.0]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveError {
    Infeasible,
    Unbounded,
    IterationLimit,
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolveError::Infeasible => write!(f, "required constraints are infeasible"),
            SolveError::Unbounded => write!(f, "problem is unbounded"),
            SolveError::IterationLimit => write!(f, "simplex iteration limit reached"),
        }
    }
}

impl std::error::Error for SolveError {}

struct Row {
    coeffs: Vec<(usize, f64)>,
    rhs: f64,
}

impl Solver {
    pub fn new() -> Solver {
        Solver::default()
    }

    pub fn new_var(&mut self) -> Var {
        let v = Var(self.n_vars);
        self.n_vars += 1;
        v
    }

    pub fn var_count(&self) -> usize {
        self.n_vars
    }

    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    /// Adds `expr rel 0` with the given strength. `weight` scales the strength's base weight.
    pub fn add(&mut self, expr: Expr, rel: Rel, strength: Strength, weight: f64) {
        self.constraints.push(Constraint { expr, rel, strength, weight });
    }

    pub fn solve(&self) -> Result<Solution, SolveError> {
        let nv = self.n_vars;
        let mut ncols = 2 * nv;
        let mut rows: Vec<Row> = Vec::with_capacity(self.constraints.len());
        let mut cost_entries: Vec<(usize, f64)> = Vec::new();
        let mut err_cols: Vec<Vec<usize>> = Vec::with_capacity(self.constraints.len());

        for c in &self.constraints {
            let mut coeffs: Vec<(usize, f64)> = Vec::with_capacity(c.expr.terms.len() * 2 + 2);
            for &(v, k) in &c.expr.terms {
                coeffs.push((2 * v.0, k));
                coeffs.push((2 * v.0 + 1, -k));
            }
            let rhs = -c.expr.constant;
            let mut errs: Vec<usize> = Vec::new();
            let w = c.strength.base_weight() * c.weight.max(0.0);
            match (c.strength, c.rel) {
                (Strength::Required, Rel::Eq) => {}
                (Strength::Required, Rel::Le) => {
                    // expr + s = rhs  (s >= 0)  =>  expr <= rhs
                    coeffs.push((ncols, 1.0));
                    ncols += 1;
                }
                (Strength::Required, Rel::Ge) => {
                    // expr - s = rhs  (s >= 0)  =>  expr >= rhs
                    coeffs.push((ncols, -1.0));
                    ncols += 1;
                }
                (_, Rel::Eq) => {
                    // expr + e- - e+ = rhs, minimize w (e- + e+)
                    coeffs.push((ncols, 1.0));
                    coeffs.push((ncols + 1, -1.0));
                    errs.push(ncols);
                    errs.push(ncols + 1);
                    cost_entries.push((ncols, w));
                    cost_entries.push((ncols + 1, w));
                    ncols += 2;
                }
                (_, Rel::Le) => {
                    // expr + s - e = rhs, minimize w e
                    coeffs.push((ncols, 1.0));
                    coeffs.push((ncols + 1, -1.0));
                    errs.push(ncols + 1);
                    cost_entries.push((ncols + 1, w));
                    ncols += 2;
                }
                (_, Rel::Ge) => {
                    // expr - s + e = rhs, minimize w e
                    coeffs.push((ncols, -1.0));
                    coeffs.push((ncols + 1, 1.0));
                    errs.push(ncols + 1);
                    cost_entries.push((ncols + 1, w));
                    ncols += 2;
                }
            }
            rows.push(Row { coeffs, rhs });
            err_cols.push(errs);
        }

        let m = rows.len();
        let total = ncols + m; // structural + slack/error + artificial
        let mut a = vec![vec![0.0f64; total + 1]; m];
        let mut basis = vec![0usize; m];
        for (i, row) in rows.iter().enumerate() {
            let sign = if row.rhs < 0.0 { -1.0 } else { 1.0 };
            for &(j, k) in &row.coeffs {
                a[i][j] += sign * k;
            }
            a[i][total] = sign * row.rhs;
            a[i][ncols + i] = 1.0;
            basis[i] = ncols + i;
        }

        // Phase I: minimize the sum of artificial variables.
        let mut cost1 = vec![0.0f64; total];
        cost1[ncols..].fill(1.0);
        run_simplex(&mut a, &mut basis, &cost1, total, 1e-9)?;
        let mut infeasibility = 0.0;
        for i in 0..m {
            if basis[i] >= ncols {
                infeasibility += a[i][total];
            }
        }
        if infeasibility > 1e-6 {
            return Err(SolveError::Infeasible);
        }
        // Drive zero-level artificials out of the basis where possible; rows with no structural
        // entry are redundant and keep their artificial at zero (it can never re-enter).
        for i in 0..m {
            if basis[i] >= ncols {
                if let Some(j) = a[i][..ncols].iter().position(|v| v.abs() > 1e-9) {
                    pivot(&mut a, &mut basis, i, j);
                }
            }
        }

        // Phase II: minimize weighted soft-constraint error.
        let mut cost2 = vec![0.0f64; total];
        let mut max_cost = 1.0f64;
        for &(j, w) in &cost_entries {
            cost2[j] += w;
            if w > max_cost {
                max_cost = w;
            }
        }
        run_simplex(&mut a, &mut basis, &cost2, ncols, 1e-9 * max_cost)?;

        let mut x = vec![0.0f64; total];
        for i in 0..m {
            x[basis[i]] = a[i][total];
        }
        let values: Vec<f64> = (0..nv).map(|v| x[2 * v] - x[2 * v + 1]).collect();
        let mut cost = 0.0;
        for &(j, w) in &cost_entries {
            cost += w * x[j];
        }
        let violations: Vec<f64> = err_cols
            .iter()
            .map(|cols| cols.iter().map(|&j| x[j]).sum::<f64>())
            .collect();
        Ok(Solution { values, cost, violations })
    }
}

fn pivot(a: &mut [Vec<f64>], basis: &mut [usize], row: usize, col: usize) {
    let width = a[row].len();
    let p = a[row][col];
    for v in a[row].iter_mut() {
        *v /= p;
    }
    let pivot_row = a[row].clone();
    for (r, rrow) in a.iter_mut().enumerate() {
        if r == row {
            continue;
        }
        let f = rrow[col];
        if f != 0.0 {
            for c in 0..width {
                rrow[c] -= f * pivot_row[c];
            }
            rrow[col] = 0.0;
            let last = width - 1;
            if rrow[last] < 0.0 && rrow[last] > -1e-10 {
                rrow[last] = 0.0;
            }
        }
    }
    basis[row] = col;
}

/// Primal simplex on an existing feasible tableau. Only columns < `enter_limit` may enter.
fn run_simplex(
    a: &mut [Vec<f64>],
    basis: &mut [usize],
    cost: &[f64],
    enter_limit: usize,
    eps_cost: f64,
) -> Result<(), SolveError> {
    let m = a.len();
    if m == 0 {
        return Ok(());
    }
    let rhs = a[0].len() - 1;
    let max_iter = 50_000usize.max(200 * (m + enter_limit));
    for _ in 0..max_iter {
        let cb: Vec<f64> = basis.iter().map(|&b| cost[b]).collect();
        let mut enter: Option<usize> = None;
        for j in 0..enter_limit {
            let mut r = cost[j];
            for i in 0..m {
                r -= cb[i] * a[i][j];
            }
            if r < -eps_cost {
                enter = Some(j);
                break; // Bland's rule: lowest index
            }
        }
        let j = match enter {
            Some(j) => j,
            None => return Ok(()),
        };
        let mut leave: Option<usize> = None;
        let mut best = f64::INFINITY;
        for i in 0..m {
            let aij = a[i][j];
            if aij > 1e-9 {
                let ratio = a[i][rhs] / aij;
                match leave {
                    None => {
                        leave = Some(i);
                        best = ratio;
                    }
                    Some(l) => {
                        if ratio < best - 1e-12 || ((ratio - best).abs() <= 1e-12 && basis[i] < basis[l]) {
                            leave = Some(i);
                            if ratio < best {
                                best = ratio;
                            }
                        }
                    }
                }
            }
        }
        match leave {
            None => return Err(SolveError::Unbounded),
            Some(i) => pivot(a, basis, i, j),
        }
    }
    Err(SolveError::IterationLimit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn required_bounds_and_soft_target() {
        let mut s = Solver::new();
        let x = s.new_var();
        s.add(Expr::new().term(x, 1.0), Rel::Ge, Strength::Required, 1.0); // x >= 0
        s.add(Expr::new().term(x, 1.0).constant(-12.0), Rel::Le, Strength::Required, 1.0); // x <= 12
        s.add(Expr::new().term(x, 1.0).constant(-20.0), Rel::Eq, Strength::Weak, 1.0); // want x = 20
        let sol = s.solve().unwrap();
        assert!(approx(sol.value(x), 12.0), "x = {}", sol.value(x));
    }

    #[test]
    fn strong_beats_weak() {
        let mut s = Solver::new();
        let x = s.new_var();
        s.add(Expr::new().term(x, 1.0).constant(-3.0), Rel::Le, Strength::Strong, 1.0); // x <= 3
        s.add(Expr::new().term(x, 1.0).constant(-5.0), Rel::Ge, Strength::Weak, 1.0); // x >= 5
        let sol = s.solve().unwrap();
        assert!(approx(sol.value(x), 3.0), "x = {}", sol.value(x));
    }

    #[test]
    fn detects_infeasible() {
        let mut s = Solver::new();
        let x = s.new_var();
        s.add(Expr::new().term(x, 1.0).constant(-1.0), Rel::Ge, Strength::Required, 1.0); // x >= 1
        s.add(Expr::new().term(x, 1.0).constant(-0.0), Rel::Le, Strength::Required, 1.0); // x <= 0
        assert_eq!(s.solve().unwrap_err(), SolveError::Infeasible);
    }

    #[test]
    fn two_variable_split() {
        // a + b = 12, a >= 3, b >= 3, soft a = 0.618 * 12
        let mut s = Solver::new();
        let a = s.new_var();
        let b = s.new_var();
        s.add(Expr::new().term(a, 1.0).term(b, 1.0).constant(-12.0), Rel::Eq, Strength::Required, 1.0);
        s.add(Expr::new().term(a, 1.0).constant(-3.0), Rel::Ge, Strength::Required, 1.0);
        s.add(Expr::new().term(b, 1.0).constant(-3.0), Rel::Ge, Strength::Required, 1.0);
        s.add(Expr::new().term(a, 1.0).constant(-0.618 * 12.0), Rel::Eq, Strength::Medium, 1.0);
        let sol = s.solve().unwrap();
        assert!(approx(sol.value(a), 0.618 * 12.0));
        assert!(approx(sol.value(a) + sol.value(b), 12.0));
    }

    #[test]
    fn negative_values_supported() {
        let mut s = Solver::new();
        let x = s.new_var();
        s.add(Expr::new().term(x, 1.0).constant(5.0), Rel::Eq, Strength::Required, 1.0); // x = -5
        let sol = s.solve().unwrap();
        assert!(approx(sol.value(x), -5.0));
    }
}
