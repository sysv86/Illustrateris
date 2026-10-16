//! Spatial grammar: stochastic recursive binary space partition (BSP) over an *ordered* list of blocks.
//!
//! Blocks keep reading order: every split cuts the ordered list into two contiguous groups, so DOM order,
//! visual order and focus order stay identical (WCAG 1.3.2).
//!
//! The grammar only proposes a topology plus target ratios. Whether it is realizable (min widths,
//! measure limits, 12-column quantization) is decided by the solver in `arrange.rs`.

use super::rng::Rng;

/// `Vertical` = the cut line is vertical (children sit side by side); `Horizontal` = children are stacked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone)]
pub enum Bsp {
    Leaf(usize),
    Split { axis: Axis, ratio: f64, first: Box<Bsp>, second: Box<Bsp> },
}

impl Bsp {
    pub fn leaf_count(&self) -> usize {
        match self {
            Bsp::Leaf(_) => 1,
            Bsp::Split { first, second, .. } => first.leaf_count() + second.leaf_count(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPolicy {
    Free,
    VerticalOnly,
    HorizontalOnly,
}

#[derive(Debug, Clone, Copy)]
pub struct Grammar {
    pub policy: AxisPolicy,
    /// If true the first block is a header: the first cut is a horizontal band cut right after it.
    pub header_band: bool,
    /// If true, group sizes follow leaf counts (equal-width items) and asymmetry only perturbs them.
    pub equalize: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GenParams {
    pub asymmetry: f64,
    pub grammar: Grammar,
    pub width: f64,
    pub height: f64,
}

/// Golden-section family of split ratios (plus the plain halves and a few musical ratios).
const RATIOS: [f64; 7] = [0.5, 0.618, 0.382, 0.667, 0.333, 0.75, 0.25];

pub fn generate(rng: &mut Rng, weights: &[f64], params: &GenParams) -> Bsp {
    let ids: Vec<usize> = (0..weights.len()).collect();
    if ids.is_empty() {
        return Bsp::Leaf(0);
    }
    gen_node(rng, &ids, params.width, params.height, true, weights, params)
}

fn gen_node(rng: &mut Rng, ids: &[usize], w: f64, h: f64, top: bool, weights: &[f64], params: &GenParams) -> Bsp {
    if ids.len() == 1 {
        return Bsp::Leaf(ids[0]);
    }
    let n = ids.len();
    let g = params.grammar;
    let header = top && g.header_band;

    let axis = if header {
        Axis::Horizontal
    } else {
        match g.policy {
            AxisPolicy::VerticalOnly => Axis::Vertical,
            AxisPolicy::HorizontalOnly => Axis::Horizontal,
            AxisPolicy::Free => {
                let aspect = if h > 0.0 { w / h } else { 2.0 };
                let p_vertical = if aspect >= 1.8 {
                    0.85
                } else if aspect >= 1.0 {
                    0.60
                } else {
                    0.25
                };
                if rng.chance(p_vertical) {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                }
            }
        }
    };

    let k = if header {
        1
    } else if g.equalize {
        let mid = (n / 2).max(1);
        if n % 2 == 1 && rng.chance(0.5) {
            (mid + 1).min(n - 1)
        } else {
            mid
        }
    } else {
        rng.range_usize(1, n)
    };
    let (left, right) = ids.split_at(k);

    let ratio = choose_ratio(rng, left, right, weights, params, header);
    let (w1, h1, w2, h2) = match axis {
        Axis::Vertical => (w * ratio, h, w * (1.0 - ratio), h),
        Axis::Horizontal => (w, h * ratio, w, h * (1.0 - ratio)),
    };
    let first = gen_node(rng, left, w1, h1, false, weights, params);
    let second = gen_node(rng, right, w2, h2, false, weights, params);
    Bsp::Split { axis, ratio, first: Box::new(first), second: Box::new(second) }
}

fn choose_ratio(rng: &mut Rng, left: &[usize], right: &[usize], weights: &[f64], params: &GenParams, header: bool) -> f64 {
    if header {
        return 0.28 + 0.08 * rng.next_f64();
    }
    let a = params.asymmetry.clamp(0.0, 1.0);
    let probs = [
        (1.25 - a).max(0.05),
        0.4 + a,
        0.4 + a,
        0.2 + 0.5 * a,
        0.2 + 0.5 * a,
        0.8 * a + 0.02,
        0.8 * a + 0.02,
    ];
    let mut ratio = RATIOS[rng.weighted(&probs)];
    // Larger share goes to the heavier group (visual-mass-aware ratio orientation).
    let wl: f64 = left.iter().map(|&i| weights[i]).sum();
    let wr: f64 = right.iter().map(|&i| weights[i]).sum();
    if (ratio - 0.5).abs() > 1e-9 {
        let want_first_larger = wl >= wr;
        let first_larger = ratio > 0.5;
        if want_first_larger != first_larger && rng.chance(0.85) {
            ratio = 1.0 - ratio;
        }
    }
    if params.grammar.equalize {
        let count_ratio = left.len() as f64 / (left.len() + right.len()) as f64;
        ratio = count_ratio + (ratio - 0.5) * a * 0.5;
    }
    ratio.clamp(0.15, 0.85)
}

/// Single-column fallback: every block full width, stacked in order.
pub fn stack_tree(n: usize) -> Bsp {
    fn go(lo: usize, n: usize) -> Bsp {
        if lo + 1 >= n {
            Bsp::Leaf(lo)
        } else {
            let remaining = (n - lo) as f64;
            Bsp::Split {
                axis: Axis::Horizontal,
                ratio: 1.0 / remaining,
                first: Box::new(Bsp::Leaf(lo)),
                second: Box::new(go(lo + 1, n)),
            }
        }
    }
    go(0, n.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_all_leaves_in_order() {
        let weights = vec![3.0, 1.0, 1.2, 1.0];
        let params = GenParams {
            asymmetry: 0.6,
            grammar: Grammar { policy: AxisPolicy::Free, header_band: false, equalize: false },
            width: 12.0,
            height: 8.0,
        };
        for seed in 0..50u64 {
            let mut rng = Rng::new(seed);
            let t = generate(&mut rng, &weights, &params);
            assert_eq!(t.leaf_count(), 4);
            let mut order = Vec::new();
            collect(&t, &mut order);
            assert_eq!(order, vec![0, 1, 2, 3]);
        }
    }

    fn collect(t: &Bsp, out: &mut Vec<usize>) {
        match t {
            Bsp::Leaf(i) => out.push(*i),
            Bsp::Split { first, second, .. } => {
                collect(first, out);
                collect(second, out);
            }
        }
    }

    #[test]
    fn stack_tree_shape() {
        assert_eq!(stack_tree(1).leaf_count(), 1);
        assert_eq!(stack_tree(5).leaf_count(), 5);
    }
}
