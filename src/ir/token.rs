//! Design tokens: harmonic scale, quantized spacing, fluid sizes and computed typography.
//!
//!   S_n = S_0 * r^n,  r in {1.250, 1.333, 1.414, 1.618}
//!
//! Spacing values are quantized to the base grid unit (4px or 8px). Type sizes sit on the same
//! harmonic scale and are emitted as fluid `clamp()` values interpolated between two viewport widths.

use crate::color::oklch::{trim_num, Oklch};
use crate::color::palette::ColorRole;
use crate::context::{FontPersonality, Physics};

pub const VW_MIN: f64 = 320.0;
pub const VW_MAX: f64 = 1440.0;
pub const REM_PX: f64 = 16.0;

/// A length that grows linearly with the viewport between `VW_MIN` and `VW_MAX`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicStep {
    pub min_px: f64,
    /// Slope in vw units.
    pub preferred_vw: f64,
    /// Intercept in px.
    pub offset_px: f64,
    pub max_px: f64,
}

impl DynamicStep {
    pub fn new(min_px: f64, max_px: f64) -> DynamicStep {
        let max_px = max_px.max(min_px);
        let slope = (max_px - min_px) / (VW_MAX - VW_MIN);
        DynamicStep { min_px, max_px, preferred_vw: slope * 100.0, offset_px: min_px - slope * VW_MIN }
    }

    pub fn fixed(px: f64) -> DynamicStep {
        DynamicStep::new(px, px)
    }

    pub fn at_viewport(&self, vw: f64) -> f64 {
        (self.offset_px + self.preferred_vw * vw / 100.0).clamp(self.min_px, self.max_px)
    }

    /// CSS value. With `tailwind = true` spaces become underscores for arbitrary-value classes.
    pub fn css(&self, tailwind: bool) -> String {
        if (self.max_px - self.min_px).abs() < 1e-9 {
            return format!("{}rem", trim_num(self.max_px / REM_PX, 4));
        }
        let sp = if tailwind { "_" } else { " " };
        let (sign, off) = if self.offset_px >= 0.0 { ("+", self.offset_px) } else { ("-", -self.offset_px) };
        format!(
            "clamp({}rem,{}rem{sp}{sign}{sp}{}vw,{}rem)",
            trim_num(self.min_px / REM_PX, 4),
            trim_num(off / REM_PX, 4),
            trim_num(self.preferred_vw, 3),
            trim_num(self.max_px / REM_PX, 4),
            sp = sp,
            sign = sign
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Scale {
    pub ratio: f64,
    pub base_unit: f64,
    pub space_s0: f64,
}

fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

impl Scale {
    pub fn new(p: &Physics) -> Scale {
        Scale { ratio: p.scale_ratio, base_unit: p.base_unit, space_s0: p.base_unit * 2.0 }
    }

    pub fn quantize(&self, px: f64) -> f64 {
        ((px / self.base_unit).round() * self.base_unit).max(self.base_unit)
    }

    pub fn space(&self, n: i32) -> f64 {
        self.quantize(self.space_s0 * self.ratio.powi(n))
    }

    pub fn space_step_for(&self, target_px: f64) -> i32 {
        ((target_px.max(1.0) / self.space_s0).ln() / self.ratio.ln()).round() as i32
    }

    pub fn space_near(&self, target_px: f64) -> f64 {
        self.space(self.space_step_for(target_px))
    }

    pub fn type_step_for(&self, target_rem: f64) -> i32 {
        (target_rem.max(0.1).ln() / self.ratio.ln()).round() as i32
    }

    pub fn type_rem(&self, n: i32) -> f64 {
        round3(self.ratio.powi(n))
    }

    pub fn type_rem_near(&self, target_rem: f64) -> f64 {
        self.type_rem(self.type_step_for(target_rem))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpacingSet {
    pub pad_x: DynamicStep,
    pub pad_y: DynamicStep,
    pub gap_x: DynamicStep,
    pub gap_y: DynamicStep,
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

impl SpacingSet {
    pub fn new(p: &Physics, s: &Scale) -> SpacingSet {
        let d = p.density;
        let pad_x_max = lerp(80.0, 40.0, d);
        let pad_y_max = lerp(176.0, 72.0, d);
        let gap_x_max = lerp(48.0, 24.0, d);
        let gap_y_max = lerp(64.0, 32.0, d);
        SpacingSet {
            pad_x: DynamicStep::new(s.space_near(24.0), s.space_near(pad_x_max)),
            pad_y: DynamicStep::new(s.space_near(pad_y_max * 0.4), s.space_near(pad_y_max)),
            gap_x: DynamicStep::new(s.space_near(16.0), s.space_near(gap_x_max)),
            gap_y: DynamicStep::new(s.space_near(24.0), s.space_near(gap_y_max)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRole {
    Display,
    Metric,
    Heading,
    Subheading,
    Lead,
    Body,
    Label,
    Button,
}

#[derive(Debug, Clone)]
pub struct ComputedTypography {
    pub role: TextRole,
    pub font_family: String,
    /// Raw fluid size in px; each backend formats it (Tailwind arbitrary value or plain CSS).
    pub size: DynamicStep,
    /// Tailwind-ready fluid size (`clamp(...)` with underscores) or a fixed rem value.
    pub font_size_clamp: String,
    pub size_min_rem: f64,
    pub size_max_rem: f64,
    pub line_height: f64,
    pub tracking_em: f64,
    pub weight: u16,
    pub color: Oklch,
    pub color_role: ColorRole,
    pub uppercase: bool,
    pub tabular: bool,
    pub balance: bool,
}

/// Font stacks without quotes or literal spaces so they survive Tailwind arbitrary values
/// (underscores become spaces). Advances in `text::metrics_data` are measured from these exact
/// reference faces, so the extent estimates match what a browser renders.
pub fn font_stack(p: FontPersonality) -> &'static str {
    crate::text::metrics_data::css_stack(match p {
        FontPersonality::Sans => crate::text::metrics_data::Family::Sans,
        FontPersonality::Serif => crate::text::metrics_data::Family::Serif,
        FontPersonality::Mono => crate::text::metrics_data::Family::Mono,
    })
}

pub fn metrics_family(p: FontPersonality) -> crate::text::metrics_data::Family {
    match p {
        FontPersonality::Sans => crate::text::metrics_data::Family::Sans,
        FontPersonality::Serif => crate::text::metrics_data::Family::Serif,
        FontPersonality::Mono => crate::text::metrics_data::Family::Mono,
    }
}

/// Analytic leading. Body-like roles use  LH = 1 + 0.55 / sqrt(size_rem)  (the draft's invariant).
/// Display roles use a tighter family so a 6rem headline lands near 0.94 instead of 1.22.
pub fn line_height_for(role: TextRole, size_rem: f64) -> f64 {
    let s = size_rem.max(0.5).sqrt();
    let lh = match role {
        TextRole::Display | TextRole::Metric => 0.78 + 0.40 / s,
        TextRole::Heading => 0.90 + 0.45 / s,
        TextRole::Subheading => 1.00 + 0.45 / s,
        _ => 1.00 + 0.55 / s,
    };
    (lh.max(0.9) * 1000.0).round() / 1000.0
}

pub fn tracking_for(role: TextRole, size_rem: f64) -> f64 {
    let t = match role {
        TextRole::Display | TextRole::Metric => -(0.010 + 0.035 * ((size_rem - 1.5) / 4.0).clamp(0.0, 1.0)),
        TextRole::Heading => -0.020,
        TextRole::Subheading => -0.010,
        TextRole::Label => 0.080,
        TextRole::Button => 0.010,
        _ => 0.0,
    };
    (t * 1000.0).round() / 1000.0
}

pub fn weight_class(w: u16) -> &'static str {
    match ((w as f64 / 100.0).round() as i32).clamp(1, 9) {
        1 => "font-thin",
        2 => "font-extralight",
        3 => "font-light",
        4 => "font-normal",
        5 => "font-medium",
        6 => "font-semibold",
        7 => "font-bold",
        8 => "font-extrabold",
        _ => "font-black",
    }
}

pub fn compute_typography(
    role: TextRole,
    physics: &Physics,
    scale: &Scale,
    size_mult: f64,
    color: Oklch,
    color_role: ColorRole,
) -> ComputedTypography {
    let dw = physics.display_weight as i32;
    let near = |t: f64| scale.type_rem_near(t);
    let (max_rem, min_rem, weight, uppercase, tabular, balance): (f64, f64, i32, bool, bool, bool) = match role {
        TextRole::Display => {
            let max = near(physics.display_max_rem * size_mult).max(2.5);
            (max, (max * 0.45).max(2.0).min(max), dw, false, false, true)
        }
        TextRole::Metric => {
            let max = near(physics.display_max_rem * size_mult * 0.72).max(2.0);
            (max, (max * 0.55).max(1.75).min(max), (dw - 200).max(300), false, true, false)
        }
        TextRole::Heading => {
            let max = near(2.75).clamp(1.75, 3.5);
            (max, (max * 0.62).max(1.5).min(max), dw, false, false, true)
        }
        TextRole::Subheading => {
            let max = near(1.5).clamp(1.125, 1.75);
            (max, (max * 0.85).max(1.0).min(max), 500, false, false, false)
        }
        TextRole::Lead => {
            let max = near(1.25).clamp(1.125, 1.5);
            (max, (max * 0.9).max(1.0).min(max), 400, false, false, false)
        }
        TextRole::Body => (1.0, 1.0, 400, false, false, false),
        TextRole::Label => {
            let max = near(0.8125).clamp(0.75, 0.9);
            (max, max, 500, true, false, false)
        }
        TextRole::Button => (1.0, 1.0, 600, false, false, false),
    };
    let size = DynamicStep::new(min_rem * REM_PX, max_rem * REM_PX);
    let clamp_css = size.css(true);
    ComputedTypography {
        role,
        font_family: font_stack(physics.personality).to_string(),
        size,
        font_size_clamp: clamp_css,
        size_min_rem: min_rem,
        size_max_rem: max_rem,
        line_height: line_height_for(role, max_rem),
        tracking_em: tracking_for(role, max_rem),
        weight: weight.clamp(100, 900) as u16,
        color,
        color_role,
        uppercase,
        tabular,
        balance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_leading_is_tight() {
        let lh = line_height_for(TextRole::Display, 6.2);
        assert!((lh - 0.94).abs() < 0.01, "lh = {}", lh);
    }

    #[test]
    fn body_leading_matches_invariant() {
        let lh = line_height_for(TextRole::Body, 1.0);
        assert!((lh - 1.55).abs() < 1e-9);
    }

    #[test]
    fn dynamic_step_endpoints() {
        let d = DynamicStep::new(24.0, 80.0);
        assert!((d.at_viewport(VW_MIN) - 24.0).abs() < 1e-9);
        assert!((d.at_viewport(VW_MAX) - 80.0).abs() < 1e-9);
        assert!(d.css(true).starts_with("clamp(1.5rem,"));
        assert!(d.css(true).ends_with(",5rem)"));
        assert!(!d.css(true).contains(' '));
    }

    #[test]
    fn fixed_step_is_plain_rem() {
        assert_eq!(DynamicStep::fixed(16.0).css(true), "1rem");
    }
}
