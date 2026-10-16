//! Style backends.
//!
//! The layout IR is backend-agnostic: the renderer computes, for every element, both a Tailwind
//! utility string and a plain-CSS declaration block, then hands them to a [`StyleSink`]. The
//! Tailwind sink keeps the utilities; the CSS sink mints content-addressed class names into a
//! generated stylesheet. Both produce byte-reproducible output for identical input.

use crate::color::oklch::trim_num;
use crate::context::fnv1a64;
use crate::ir::ast::DocumentTokens;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Tailwind,
    Css,
}

impl Backend {
    pub fn parse(s: &str) -> Option<Backend> {
        match s {
            "tailwind" => Some(Backend::Tailwind),
            "css" | "plain_css" | "vanilla" => Some(Backend::Css),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Backend::Tailwind => "tailwind",
            Backend::Css => "css",
        }
    }

    /// Whether a document-mode result must load the Tailwind browser build from a CDN.
    pub fn needs_tailwind_cdn(self) -> bool {
        self == Backend::Tailwind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakpoint {
    /// 640px, matching Tailwind's `sm:`.
    Sm,
    /// 1024px, matching Tailwind's `lg:`.
    Lg,
}

impl Breakpoint {
    pub fn min_width_px(self) -> u32 {
        match self {
            Breakpoint::Sm => 640,
            Breakpoint::Lg => 1024,
        }
    }
}

/// One element's styling in backend-neutral form: a Tailwind utility string plus the equivalent
/// plain-CSS declarations (base, per-breakpoint, and `:focus-visible`).
#[derive(Debug, Clone, Default)]
pub struct Style {
    pub tailwind: String,
    pub base: String,
    pub responsive: Vec<(Breakpoint, String)>,
    pub hover: String,
    pub focus: String,
}

impl Style {
    pub fn tw(tailwind: impl Into<String>) -> Style {
        Style { tailwind: tailwind.into(), base: String::new(), responsive: Vec::new(), hover: String::new(), focus: String::new() }
    }

    pub fn css(&mut self, decls: impl Into<String>) -> &mut Style {
        self.base.push_str(&decls.into());
        self
    }

    pub fn at(&mut self, bp: Breakpoint, decls: impl Into<String>) -> &mut Style {
        self.responsive.push((bp, decls.into()));
        self
    }

    /// Declarations applied only under `:hover`.
    pub fn on_hover(&mut self, decls: impl Into<String>) -> &mut Style {
        self.hover.push_str(&decls.into());
        self
    }

    /// Declarations applied only under `:focus-visible`.
    pub fn focus(&mut self, decls: impl Into<String>) -> &mut Style {
        self.focus.push_str(&decls.into());
        self
    }

    /// Signature used by the CSS backend to name the class. Stable across runs.
    fn signature(&self) -> String {
        let mut s = self.base.clone();
        for (bp, d) in &self.responsive {
            s.push_str(&format!("|{}|{}", bp.min_width_px(), d));
        }
        s.push_str("|hover|");
        s.push_str(&self.hover);
        s.push_str("|focus|");
        s.push_str(&self.focus);
        s
    }
}

pub trait StyleSink {
    /// Returns the `class` attribute value for an element that carries several styles. Both
    /// backends compose by concatenation: Tailwind joins utility strings, the CSS backend joins
    /// class names.
    fn classes(&mut self, styles: &[Style]) -> String {
        let mut parts: Vec<String> = Vec::new();
        for s in styles {
            let c = self.class(s);
            if !c.is_empty() {
                parts.push(c);
            }
        }
        parts.join(" ")
    }
    /// Returns the `class` attribute value for a single style.
    fn class(&mut self, style: &Style) -> String;
    /// CSS that must be embedded in the document head (empty for the Tailwind backend).
    fn stylesheet(&self) -> String;
    /// Whether the head should inline a stylesheet at all.
    fn emits_stylesheet(&self) -> bool;
}

/// Emits Tailwind utilities verbatim; the document loads Tailwind itself.
pub struct TailwindSink;

impl StyleSink for TailwindSink {
    fn class(&mut self, style: &Style) -> String {
        style.tailwind.clone()
    }
    fn stylesheet(&self) -> String {
        String::new()
    }
    fn emits_stylesheet(&self) -> bool {
        false
    }
}

/// Emits content-addressed classes and collects the matching plain-CSS rules.
/// When `clean` (De-PLE / Eject mode) is enabled, emits `:root` CSS Custom Properties
/// and uses clean `ui-<8-hex>` class names instead of `ple-<16-hex>`.
#[derive(Default)]
pub struct CssSink {
    /// class name -> full rule text (base rule + media blocks), in first-seen order.
    rules: Vec<(String, String)>,
    seen: BTreeSet<String>,
    clean: bool,
    root_vars: String,
}

impl CssSink {
    pub fn new() -> CssSink {
        CssSink::default()
    }

    pub fn new_with_mode(clean: bool, tokens: Option<&DocumentTokens>) -> CssSink {
        let root_vars = match (clean, tokens) {
            (true, Some(t)) => format!(
                ":root{{\n  --bg:{};\n  --surface:{};\n  --border:{};\n  --text:{};\n  --text-secondary:{};\n  --accent:{};\n  --accent-text:{};\n  --font-family:{};\n  --radius:{}px;\n}}\n",
                t.bg.css(),
                t.surface.css(),
                t.border.css(),
                t.text.css(),
                t.text_secondary.css(),
                t.accent.css(),
                t.accent_text.css(),
                t.font_family.replace('_', " "),
                trim_num(t.radius_px, 1),
            ),
            _ => String::new(),
        };
        CssSink { rules: Vec::new(), seen: BTreeSet::new(), clean, root_vars }
    }
}

/// Minimal user-agent reset for the plain-CSS backend.
///
/// The Tailwind backend gets this from Tailwind's preflight; without it, `box-sizing` defaults to
/// `content-box`, so a `width:100%` section that also carries horizontal padding overflows its
/// parent by exactly that padding. Heading and paragraph margins are normalised too, so the two
/// backends lay out identically.
pub const CSS_RESET: &str = "\
*,::before,::after{box-sizing:border-box}
html{-webkit-text-size-adjust:100%}
body{margin:0}
h1,h2,h3,h4,p,figure,figcaption,blockquote,dl,dt,dd,ol,ul{margin:0;padding:0}
a{color:inherit}
input,button{font:inherit}
canvas{display:block;max-width:100%}
";

impl StyleSink for CssSink {
    fn class(&mut self, style: &Style) -> String {
        if style.base.is_empty() && style.responsive.is_empty() && style.hover.is_empty() && style.focus.is_empty() {
            // No declarations (e.g. an element that only carries a Tailwind-only utility).
            return String::new();
        }
        let sig = style.signature();
        let hash = fnv1a64(sig.as_bytes());
        let name = if self.clean {
            format!("ui-{:08x}", (hash & 0xFFFF_FFFF) as u32)
        } else {
            format!("ple-{:016x}", hash)
        };
        if !self.seen.contains(&name) {
            let mut rule = format!(".{} {{{}}}", name, style.base);
            if !style.hover.is_empty() {
                rule.push_str(&format!(".{}:hover {{{}}}", name, style.hover));
            }
            if !style.focus.is_empty() {
                rule.push_str(&format!(".{}:focus-visible {{{}}}", name, style.focus));
            }
            for (bp, decls) in &style.responsive {
                rule.push_str(&format!(
                    "@media (min-width: {}px) {{ .{} {{{}}} }}",
                    bp.min_width_px(),
                    name,
                    decls
                ));
            }
            self.seen.insert(name.clone());
            self.rules.push((name.clone(), rule));
        }
        name
    }

    fn stylesheet(&self) -> String {
        let mut out = String::from(CSS_RESET);
        if !self.root_vars.is_empty() {
            out.push_str(&self.root_vars);
        }
        out.push('\n');
        for (_, rule) in &self.rules {
            out.push_str(rule);
            out.push('\n');
        }
        out
    }

    fn emits_stylesheet(&self) -> bool {
        true
    }
}
