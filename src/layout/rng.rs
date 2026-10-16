//! Deterministic PRNG (SplitMix64). The whole engine is a pure function of (context, seed, config).

#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

pub fn mix(a: u64, b: u64) -> u64 {
    let mut z = a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(17);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub fn derive_seed(seed: u64, a: u64) -> u64 {
    mix(seed, a.wrapping_add(0xA5A5_5A5A_1234_5678))
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform integer in [lo, hi). Returns `lo` when the range is empty.
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo) as u64) as usize
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// Index sampled proportionally to `weights` (non-positive weights are never chosen unless all are).
    pub fn weighted(&mut self, weights: &[f64]) -> usize {
        let total: f64 = weights.iter().map(|w| w.max(0.0)).sum();
        if total <= 0.0 || weights.is_empty() {
            return 0;
        }
        let mut x = self.next_f64() * total;
        for (i, w) in weights.iter().enumerate() {
            let w = w.max(0.0);
            if x < w {
                return i;
            }
            x -= w;
        }
        weights.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn f64_in_unit_interval() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let x = r.next_f64();
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn weighted_respects_zero() {
        let mut r = Rng::new(9);
        for _ in 0..200 {
            assert_eq!(r.weighted(&[0.0, 1.0, 0.0]), 1);
        }
    }
}
