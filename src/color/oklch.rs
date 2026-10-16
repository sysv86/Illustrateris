//! OKLab / OKLCH <-> sRGB conversions and chroma-reduction gamut mapping.
//! Matrices from Bjorn Ottosson's reference implementation.

pub type Rgb8 = [u8; 3];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    /// Hue in degrees, [0, 360).
    pub h: f64,
}

pub fn trim_num(x: f64, decimals: usize) -> String {
    let mut s = format!("{:.*}", decimals, x);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".to_string();
    }
    s
}

fn round_to(x: f64, decimals: i32) -> f64 {
    let k = 10f64.powi(decimals);
    (x * k).round() / k
}

impl Oklch {
    pub fn new(l: f64, c: f64, h: f64) -> Oklch {
        Oklch { l, c: c.max(0.0), h: h.rem_euclid(360.0) }
    }

    /// Canonical precision used everywhere a colour is emitted or measured:
    /// L, C at 3 decimals, H at 1 decimal. Contrast is always computed on the quantized value,
    /// so the number the engine verifies is the number the browser receives.
    pub fn quantized(self) -> Oklch {
        let mut h = round_to(self.h.rem_euclid(360.0), 1);
        if h >= 360.0 {
            h = 0.0;
        }
        Oklch {
            l: round_to(self.l.clamp(0.0, 1.0), 3),
            c: round_to(self.c.max(0.0), 3),
            h,
        }
    }

    pub fn css(&self) -> String {
        format!("oklch({} {} {})", trim_num(self.l, 3), trim_num(self.c, 3), trim_num(self.h, 1))
    }

    /// Same as `css` but with underscores for use inside Tailwind arbitrary values.
    pub fn css_tw(&self) -> String {
        self.css().replace(' ', "_")
    }
}

/// OKLCH -> linear sRGB (may be out of [0,1]).
pub fn to_linear(c: &Oklch) -> [f64; 3] {
    let hr = c.h.to_radians();
    let a = c.c * hr.cos();
    let b = c.c * hr.sin();
    let l_ = c.l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = c.l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = c.l - 0.0894841775 * a - 1.2914855480 * b;
    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;
    [
        4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3,
        -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3,
        -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3,
    ]
}

/// Linear sRGB -> OKLCH.
pub fn from_linear(rgb: [f64; 3]) -> Oklch {
    let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();
    let ll = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let aa = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let bb = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;
    let c = aa.hypot(bb);
    let h = if c < 1e-6 { 0.0 } else { bb.atan2(aa).to_degrees().rem_euclid(360.0) };
    Oklch { l: ll, c, h }
}

pub fn srgb_encode(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.0031308 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

pub fn srgb_decode(x: f64) -> f64 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

pub fn in_gamut(lin: [f64; 3]) -> bool {
    lin.iter().all(|v| *v >= -1e-6 && *v <= 1.0 + 1e-6)
}

/// Gamut mapping by chroma reduction at constant L and H (binary search).
/// Deliberately simple: palette colours are low-chroma, so the CSS Color 4
/// JND-clip refinement would not change any result.
pub fn gamut_map(c: Oklch) -> Oklch {
    if c.l >= 1.0 {
        return Oklch { l: 1.0, c: 0.0, h: 0.0 };
    }
    if c.l <= 0.0 {
        return Oklch { l: 0.0, c: 0.0, h: 0.0 };
    }
    if in_gamut(to_linear(&c)) {
        return c;
    }
    let mut lo = 0.0f64;
    let mut hi = c.c;
    for _ in 0..28 {
        let mid = 0.5 * (lo + hi);
        let t = Oklch { l: c.l, c: mid, h: c.h };
        if in_gamut(to_linear(&t)) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Oklch { l: c.l, c: lo, h: c.h }
}

pub fn to_rgb8(c: Oklch) -> Rgb8 {
    let m = gamut_map(c);
    let lin = to_linear(&m);
    let enc = |v: f64| (srgb_encode(v) * 255.0).round().clamp(0.0, 255.0) as u8;
    [enc(lin[0]), enc(lin[1]), enc(lin[2])]
}

pub fn from_rgb8(rgb: Rgb8) -> Oklch {
    let lin = [
        srgb_decode(rgb[0] as f64 / 255.0),
        srgb_decode(rgb[1] as f64 / 255.0),
        srgb_decode(rgb[2] as f64 / 255.0),
    ];
    from_linear(lin)
}

pub fn to_hex(rgb: Rgb8) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_and_black() {
        assert_eq!(to_rgb8(Oklch::new(1.0, 0.0, 0.0)), [255, 255, 255]);
        assert_eq!(to_rgb8(Oklch::new(0.0, 0.0, 0.0)), [0, 0, 0]);
    }

    #[test]
    fn roundtrip_rgb() {
        for rgb in [[12u8, 200, 90], [255, 0, 0], [128, 128, 128], [3, 3, 40]] {
            let back = to_rgb8(from_rgb8(rgb));
            for i in 0..3 {
                assert!((back[i] as i32 - rgb[i] as i32).abs() <= 1, "{:?} -> {:?}", rgb, back);
            }
        }
    }

    #[test]
    fn gamut_mapping_reduces_chroma() {
        let wild = Oklch::new(0.6, 0.45, 150.0);
        let m = gamut_map(wild);
        assert!(m.c < wild.c);
        assert!(in_gamut(to_linear(&m)));
    }

    #[test]
    fn css_format() {
        assert_eq!(Oklch::new(0.14, 0.02, 260.0).css(), "oklch(0.14 0.02 260)");
        assert_eq!(Oklch::new(0.14, 0.02, 260.0).css_tw(), "oklch(0.14_0.02_260)");
    }
}
