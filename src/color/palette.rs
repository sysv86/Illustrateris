//! Procedural palette synthesis with guaranteed contrast and continuous substrate/harmony divergence.
//!
//! No predefined palettes. Hue comes from the profiler; substrate lightness and harmonic accent
//! angles are derived continuously from `[precision, warmth, drama, chromatic_energy]` so different
//! domains never share a cookie-cutter ground or accent relationship; every foreground role is then
//! pushed along the L axis (constant C and H) until its APCA |Lc| target holds against *every*
//! background it can appear on.

use super::apca::{lc, LcTargets};
use super::oklch::{gamut_map, to_hex, to_rgb8, Oklch, Rgb8};
use crate::context::Physics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorRole {
    Bg,
    Surface,
    Border,
    Text,
    TextSecondary,
    Accent,
    AccentText,
}

/// A colour that is in sRGB gamut and quantized to the canonical emit precision.
#[derive(Debug, Clone, Copy)]
pub struct Tok {
    pub oklch: Oklch,
    pub rgb: Rgb8,
}

impl Tok {
    pub fn new(o: Oklch) -> Tok {
        let m = gamut_map(o.quantized()).quantized();
        Tok { oklch: m, rgb: to_rgb8(m) }
    }

    pub fn hex(&self) -> String {
        to_hex(self.rgb)
    }
}

#[derive(Debug, Clone)]
pub struct ContrastCheck {
    pub name: String,
    pub fg: String,
    pub bg: String,
    pub lc: f64,
    pub target: f64,
    pub passed: bool,
    pub adjusted: bool,
}

#[derive(Debug, Clone)]
pub struct Palette {
    pub dark: bool,
    pub hue: f64,
    pub accent_hue: f64,
    pub bg: Tok,
    pub surface: Tok,
    pub border: Tok,
    pub text: Tok,
    pub text_secondary: Tok,
    pub accent: Tok,
    pub accent_text: Tok,
    pub checks: Vec<ContrastCheck>,
}

impl Palette {
    pub fn tok(&self, role: ColorRole) -> &Tok {
        match role {
            ColorRole::Bg => &self.bg,
            ColorRole::Surface => &self.surface,
            ColorRole::Border => &self.border,
            ColorRole::Text => &self.text,
            ColorRole::TextSecondary => &self.text_secondary,
            ColorRole::Accent => &self.accent,
            ColorRole::AccentText => &self.accent_text,
        }
    }

    /// Sections alternate between `bg` and `surface`.
    pub fn section_bg(&self, index: usize) -> &Tok {
        if index % 2 == 0 {
            &self.bg
        } else {
            &self.surface
        }
    }

    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }
}

/// Moves `start` along L (constant C, H) toward `toward_light` until min |Lc| over `bgs` >= target.
/// Returns (token, adjusted, achieved min |Lc|). If the target is unreachable even at the L extreme,
/// the extreme is returned and the achieved value is below target.
fn enforce(start: Oklch, bgs: &[Rgb8], target: f64, toward_light: bool) -> (Tok, bool, f64) {
    let eval = |l: f64| -> (Tok, f64) {
        let t = Tok::new(Oklch::new(l.clamp(0.0, 1.0), start.c, start.h));
        let mut min = f64::MAX;
        for bg in bgs {
            let v = lc(t.rgb, *bg).abs();
            if v < min {
                min = v;
            }
        }
        (t, min)
    };

    let (t0, lc0) = eval(start.l);
    if lc0 >= target {
        return (t0, false, lc0);
    }
    let ext = if toward_light { 1.0 } else { 0.0 };
    let (te, lce) = eval(ext);
    if lce < target {
        return (te, true, lce);
    }
    let mut lo = 0.0f64;
    let mut hi = 1.0f64;
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        let (_, v) = eval(start.l + (ext - start.l) * mid);
        if v >= target {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let mut l = start.l + (ext - start.l) * hi;
    let mut res = eval(l);
    let step = if toward_light { 0.001 } else { -0.001 };
    let mut guard = 0;
    while res.1 < target && guard < 1000 {
        l += step;
        res = eval(l);
        guard += 1;
    }
    (res.0, true, res.1)
}

pub fn synthesize(physics: &Physics, targets: &LcTargets) -> Palette {
    let dark = physics.theme_dark;
    let hue = physics.hue;
    let energy = physics.chromatic_energy;
    let drama = physics.tone.drama.clamp(0.0, 1.0);
    let warmth = physics.tone.warmth.clamp(0.0, 1.0);
    let precision = physics.tone.precision.clamp(0.0, 1.0);

    // Continuous substrate depth: high-drama dark themes sink toward deep obsidian (L ~ 0.115),
    // while technical calm sits at graphite (L ~ 0.155). Warm light themes sit on tactile stone/ivory
    // (L ~ 0.974), while high-precision light themes sit on crisp optical white (L ~ 0.990).
    let (bg_l, surface_l, bg_c, surface_c) = if dark {
        let l_base = (0.155 - 0.040 * drama + 0.010 * warmth).clamp(0.110, 0.160);
        let l_surf = (l_base + 0.036 + 0.010 * drama).clamp(0.150, 0.205);
        let c = 0.010 + 0.042 * energy;
        (l_base, l_surf, c, c * 1.15)
    } else {
        let l_base = (0.990 - 0.018 * warmth).clamp(0.968, 0.992);
        let l_surf = (l_base - 0.028 - 0.008 * drama).clamp(0.935, 0.965);
        let c = 0.003 + 0.014 * energy + 0.006 * warmth;
        (l_base, l_surf, c, c * 1.55)
    };
    let bg = Tok::new(Oklch::new(bg_l, bg_c, hue));
    let surface = Tok::new(Oklch::new(surface_l, surface_c, hue));
    let bgs = [bg.rgb, surface.rgb];
    let toward_light = dark;

    // Text ------------------------------------------------------------------------------------
    let text_start = if dark { Oklch::new(0.96, 0.006, hue) } else { Oklch::new(0.18, 0.020, hue) };
    let (text, text_adj, _) = enforce(text_start, &bgs, targets.body, toward_light);

    let sec_start = if dark { Oklch::new(0.72, 0.012, hue) } else { Oklch::new(0.42, 0.020, hue) };
    let (text_secondary, sec_adj, _) = enforce(sec_start, &bgs, targets.secondary, toward_light);

    // Border (hairline) -----------------------------------------------------------------------
    let border_start = if dark { Oklch::new(0.28, 0.015, hue) } else { Oklch::new(0.88, 0.010, hue) };
    let (border, border_adj, _) = enforce(border_start, &bgs, targets.hairline, toward_light);

    // Continuous harmonic accent angle:
    // Warm/organic domains use analogous shifts (+25..+65 deg); high-drama or high-precision domains
    // use split-complementary / triadic shifts (+110..+185 deg) scaled by chromatic_energy.
    let harmony_shift = if warmth >= 0.65 && precision < 0.55 {
        28.0 + 52.0 * energy
    } else if precision >= 0.75 {
        95.0 * energy + 25.0 * drama
    } else {
        150.0 * energy + 20.0 * (drama - warmth)
    };
    let accent_hue = (hue + harmony_shift).rem_euclid(360.0);
    let accent_c = 0.05 + 0.17 * energy;
    let accent_start = if dark { Oklch::new(0.82, accent_c, accent_hue) } else { Oklch::new(0.50, accent_c, accent_hue) };
    let (accent_a, adj_a, _) = enforce(accent_start, &bgs, targets.ui_component, toward_light);

    // Accent text: pick whichever of near-black / near-white contrasts more, then push the accent
    // fill away from it until the button-text target holds.
    let dark_txt = Tok::new(Oklch::new(0.16, 0.020, accent_hue));
    let light_txt = Tok::new(Oklch::new(0.985, 0.005, accent_hue));
    let lc_dark = lc(dark_txt.rgb, accent_a.rgb).abs();
    let lc_light = lc(light_txt.rgb, accent_a.rgb).abs();
    let (accent_text, text_is_dark) = if lc_dark >= lc_light { (dark_txt, true) } else { (light_txt, false) };
    let (accent, adj_b, _) = enforce(accent_a.oklch, &[accent_text.rgb], targets.secondary, text_is_dark);

    // Checks ----------------------------------------------------------------------------------
    let mut checks: Vec<ContrastCheck> = Vec::new();
    {
        let mut push = |name: &str, fg: &Tok, bgt: &Tok, target: f64, adjusted: bool| {
            let v = lc(fg.rgb, bgt.rgb).abs();
            checks.push(ContrastCheck {
                name: name.to_string(),
                fg: fg.oklch.css(),
                bg: bgt.oklch.css(),
                lc: (v * 100.0).round() / 100.0,
                target,
                passed: v + 1e-9 >= target,
                adjusted,
            });
        };
        push("text/bg", &text, &bg, targets.body, text_adj);
        push("text/surface", &text, &surface, targets.body, text_adj);
        push("display(text)/bg", &text, &bg, targets.display, false);
        push("display(text)/surface", &text, &surface, targets.display, false);
        push("text-secondary/bg", &text_secondary, &bg, targets.secondary, sec_adj);
        push("text-secondary/surface", &text_secondary, &surface, targets.secondary, sec_adj);
        push("border/bg", &border, &bg, targets.hairline, border_adj);
        push("border/surface", &border, &surface, targets.hairline, border_adj);
        push("accent/bg", &accent, &bg, targets.ui_component, adj_a || adj_b);
        push("accent/surface", &accent, &surface, targets.ui_component, adj_a || adj_b);
        push("accent-text/accent", &accent_text, &accent, targets.secondary, adj_b);
    }

    Palette {
        dark,
        hue,
        accent_hue,
        bg,
        surface,
        border,
        text,
        text_secondary,
        accent,
        accent_text,
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{profile, Intent, ThemePref};

    fn intent(tone: &str, theme: ThemePref) -> Intent {
        Intent {
            domain: "fintech_infrastructure".into(),
            tone: tone.into(),
            density: 0.5,
            asymmetry: 0.5,
            chromatic_energy: 0.3,
            theme,
            seed: None,
            hue: None,
            lang: "en".into(),
            title: None,
        }
    }

    #[test]
    fn dark_palette_meets_targets() {
        let p = profile(&intent("high_assurance_technical", ThemePref::Dark), 0.15);
        let pal = synthesize(&p, &LcTargets::default());
        for c in &pal.checks {
            assert!(c.passed, "failed: {:?}", c);
        }
    }

    #[test]
    fn light_palette_meets_targets() {
        let p = profile(&intent("playful_friendly", ThemePref::Light), 0.15);
        let pal = synthesize(&p, &LcTargets::default());
        for c in &pal.checks {
            assert!(c.passed, "failed: {:?}", c);
        }
    }
}
