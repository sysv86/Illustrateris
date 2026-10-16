//! APCA-W3C contrast (SAPC 0.0.98G-4g constants).
//!
//! Note: the first concept draft used simplified constants (and an invalid single-exponent
//! formula). This is the published algorithm, including soft black clamp, polarity-dependent
//! exponents and low-contrast clipping.

use super::oklch::Rgb8;

pub const MAIN_TRC: f64 = 2.4;
pub const S_RCO: f64 = 0.2126729;
pub const S_GCO: f64 = 0.7151522;
pub const S_BCO: f64 = 0.0721750;

pub const NORM_BG: f64 = 0.56;
pub const NORM_TXT: f64 = 0.57;
pub const REV_TXT: f64 = 0.62;
pub const REV_BG: f64 = 0.65;

pub const BLK_THRS: f64 = 0.022;
pub const BLK_CLMP: f64 = 1.414;
pub const SCALE_BOW: f64 = 1.14;
pub const SCALE_WOB: f64 = 1.14;
pub const LO_BOW_OFFSET: f64 = 0.027;
pub const LO_WOB_OFFSET: f64 = 0.027;
pub const DELTA_Y_MIN: f64 = 0.0005;
pub const LO_CLIP: f64 = 0.1;

/// Minimum |Lc| per role. Applied against every background a role can appear on.
#[derive(Debug, Clone, Copy)]
pub struct LcTargets {
    /// Body text <= 16px.
    pub body: f64,
    /// Secondary text, labels, button text.
    pub secondary: f64,
    /// Large display type.
    pub display: f64,
    /// Non-text UI components (button fill vs page).
    pub ui_component: f64,
    /// Hairlines / dividers.
    pub hairline: f64,
}

impl Default for LcTargets {
    fn default() -> Self {
        LcTargets { body: 90.0, secondary: 75.0, display: 60.0, ui_component: 45.0, hairline: 12.0 }
    }
}

/// Screen luminance Y from 8-bit sRGB (APCA uses a pure 2.4 power curve, not the piecewise sRGB EOTF).
pub fn luminance_y(rgb: Rgb8) -> f64 {
    let f = |v: u8| (v as f64 / 255.0).powf(MAIN_TRC);
    S_RCO * f(rgb[0]) + S_GCO * f(rgb[1]) + S_BCO * f(rgb[2])
}

/// Signed Lc: positive = dark text on light background, negative = light text on dark background.
pub fn lc_from_y(txt_y: f64, bg_y: f64) -> f64 {
    if txt_y.is_nan() || bg_y.is_nan() || txt_y < 0.0 || bg_y < 0.0 || txt_y > 1.1 || bg_y > 1.1 {
        return 0.0;
    }
    let txt = if txt_y > BLK_THRS { txt_y } else { txt_y + (BLK_THRS - txt_y).powf(BLK_CLMP) };
    let bg = if bg_y > BLK_THRS { bg_y } else { bg_y + (BLK_THRS - bg_y).powf(BLK_CLMP) };
    if (bg - txt).abs() < DELTA_Y_MIN {
        return 0.0;
    }
    let out = if bg > txt {
        let sapc = (bg.powf(NORM_BG) - txt.powf(NORM_TXT)) * SCALE_BOW;
        if sapc < LO_CLIP {
            0.0
        } else {
            sapc - LO_BOW_OFFSET
        }
    } else {
        let sapc = (bg.powf(REV_BG) - txt.powf(REV_TXT)) * SCALE_WOB;
        if sapc > -LO_CLIP {
            0.0
        } else {
            sapc + LO_WOB_OFFSET
        }
    };
    out * 100.0
}

pub fn lc(txt: Rgb8, bg: Rgb8) -> f64 {
    lc_from_y(luminance_y(txt), luminance_y(bg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_values() {
        let bow = lc([0, 0, 0], [255, 255, 255]);
        assert!((bow - 106.04).abs() < 0.1, "black on white = {}", bow);
        let wob = lc([255, 255, 255], [0, 0, 0]);
        assert!((wob + 107.88).abs() < 0.1, "white on black = {}", wob);
    }

    #[test]
    fn identical_colours_have_zero_contrast() {
        assert_eq!(lc([120, 120, 120], [120, 120, 120]), 0.0);
    }

    #[test]
    fn polarity_signs() {
        assert!(lc([20, 20, 20], [240, 240, 240]) > 0.0);
        assert!(lc([240, 240, 240], [20, 20, 20]) < 0.0);
    }
}
