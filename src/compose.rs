//! Turns validated content nodes into *section plans*: ordered blocks with content, typography,
//! physical size constraints (derived from real text extents and OpenType GPOS kerning at the design
//! viewport) and visual mass. The layout phases only ever see `layout::arrange::Block` and mass numbers.

use crate::color::apca;
use crate::color::palette::{ColorRole, Palette};
use crate::context::{ActionType, ContentNode, FontPersonality, MediaKind, Physics, Role, VisualAnchor};
use crate::ir::token::{compute_typography, metrics_family, ComputedTypography, Scale, SpacingSet, TextRole};
use crate::layout::arrange::Block;
use crate::layout::space::{AxisPolicy, Grammar};
use crate::layout::visual_mass;
use crate::text::metrics_data;

#[derive(Debug, Clone)]
pub enum BlockContent {
    Claim { text: String },
    Support { text: String },
    Action { kind: ActionType, label: String, placeholder: String, href: String, input_type: String },
    Metric { value: String, label: String },
    Feature { title: String, body: Option<String> },
    /// A navigational or footer link (`nav`, `footer`).
    Link { label: String, href: String },
    /// A pricing tier (`pricing`): plan name, price, optional billing note and description.
    Tier { name: String, price: String, note: Option<String>, body: Option<String> },
    /// A testimonial quote (`testimonial`).
    Quote { text: String, attribution: String, role: Option<String> },
    /// An interactive WebGPU / shader surface with explicit aspect ratio and visual mass.
    Canvas { shader: String, aspect_ratio: f64, label: String, caption: Option<String> },
    /// A photograph (`<picture>/<img>`) or video (`<video>`) surface with LCP/CLS optimization and temporal mass.
    MediaAsset {
        kind: MediaKind,
        src: String,
        fallback_src: Option<String>,
        poster: Option<String>,
        aspect_ratio: f64,
        alt: String,
        caption: Option<String>,
        priority_eager: bool,
    },
}

#[derive(Debug, Clone)]
pub struct PlanBlock {
    pub layout: Block,
    pub content: BlockContent,
    /// Typography per text part: Claim [t], Support [t], Action [button, input], Metric [value, label], Feature [title, body].
    pub typos: Vec<ComputedTypography>,
    pub mass: f64,
    pub measure_ch: Option<f64>,
}

impl PlanBlock {
    pub fn kind_name(&self) -> &'static str {
        match &self.content {
            BlockContent::Claim { .. } => "claim",
            BlockContent::Support { .. } => "support",
            BlockContent::Action { .. } => "action",
            BlockContent::Metric { .. } => "metric",
            BlockContent::Feature { .. } => "feature",
            BlockContent::Link { .. } => "link",
            BlockContent::Tier { .. } => "tier",
            BlockContent::Quote { .. } => "quote",
            BlockContent::Canvas { .. } => "canvas",
            BlockContent::MediaAsset { kind: MediaKind::Video, .. } => "video",
            BlockContent::MediaAsset { .. } => "image",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SectionPlan {
    pub id: String,
    pub role: Role,
    pub blocks: Vec<PlanBlock>,
    pub grammar: Grammar,
    pub height_units: f64,
    pub min_height_dvh: Option<u32>,
    pub bg_index: usize,
    pub has_header: bool,
}

/// Column geometry of the design viewport; converts pixel extents into column counts.
#[derive(Debug, Clone, Copy)]
pub struct GridMetrics {
    pub col_px: f64,
    pub gap_px: f64,
}

impl GridMetrics {
    pub fn new(vw: f64, sp: &SpacingSet) -> GridMetrics {
        let pad = sp.pad_x.at_viewport(vw);
        let gap = sp.gap_x.at_viewport(vw);
        let col_px = ((vw - 2.0 * pad - 11.0 * gap) / 12.0).max(40.0);
        GridMetrics { col_px, gap_px: gap }
    }

    /// Smallest whole number of columns whose span is at least `px` wide, clamped to 1..=12.
    pub fn cols_for(&self, px: f64) -> f64 {
        (((px + self.gap_px) / (self.col_px + self.gap_px)).ceil()).clamp(1.0, 12.0)
    }
}

/// Width in px of `s` laid out on a single line at the block's largest fluid size, using the real
/// per-glyph advances and OpenType GPOS kerning pairs of the reference face for the personality.
fn run_width(s: &str, t: &ComputedTypography, p: FontPersonality) -> f64 {
    let fam = metrics_family(p);
    let fs = t.size_max_rem * 16.0;
    let upper = if t.uppercase { s.to_uppercase() } else { s.to_string() };
    let tabular_digit = if t.tabular {
        Some(metrics_data::advance_em(fam, '0', t.weight))
    } else {
        None
    };
    let mut em_sum = 0.0;
    let mut prev: Option<char> = None;
    for ch in upper.chars() {
        let is_tab_digit = tabular_digit.is_some() && ch.is_ascii_digit();
        let adv = match tabular_digit {
            Some(d) if ch.is_ascii_digit() => d,
            _ => metrics_data::advance_em(fam, ch, t.weight),
        };
        let kern = match prev {
            Some(pr) if !is_tab_digit && !(tabular_digit.is_some() && pr.is_ascii_digit()) => {
                metrics_data::kern_em(fam, pr, ch, t.weight)
            }
            _ => 0.0,
        };
        em_sum += (adv + kern + t.tracking_em).max(0.0);
        prev = Some(ch);
    }
    em_sum * fs
}

/// (single-line width, longest unbreakable word width) in px at the largest fluid size.
pub fn text_extent(text: &str, t: &ComputedTypography, p: FontPersonality) -> (f64, f64) {
    let total = run_width(text, t, p);
    let longest = text
        .split_whitespace()
        .map(|w| run_width(w, t, p))
        .fold(0.0f64, f64::max);
    (total, longest)
}

struct Builder<'a> {
    physics: &'a Physics,
    scale: &'a Scale,
    palette: &'a Palette,
    gm: GridMetrics,
    bg_index: usize,
}

impl<'a> Builder<'a> {
    fn typo(&self, role: TextRole, mult: f64, color: ColorRole) -> ComputedTypography {
        compute_typography(role, self.physics, self.scale, mult, self.palette.tok(color).oklch, color)
    }

    fn text_mass(&self, t: &ComputedTypography, text: &str) -> f64 {
        let fs = t.size_max_rem * 16.0;
        let (total, _) = text_extent(text, t, self.physics.personality);
        let ink = total * (t.line_height * fs);
        let bg = self.palette.section_bg(self.bg_index).rgb;
        let fg = self.palette.tok(t.color_role).rgb;
        visual_mass::mass(ink, apca::lc(fg, bg).abs(), t.weight)
    }

    fn claim(&self, text: &str, role: TextRole, mult: f64, gweight: f64, max_cols: f64) -> PlanBlock {
        let t = self.typo(role, mult, ColorRole::Text);
        let (total, longest) = text_extent(text, &t, self.physics.personality);
        let min = self.gm.cols_for(longest * 1.03);
        let max = max_cols.max(min).min(12.0);
        let pref = self.gm.cols_for(total / 3.0 * 1.08).max(min).min(max);
        let ideal = self.gm.cols_for(total * 1.05).max(pref).min(max);
        let mass = self.text_mass(&t, text);
        PlanBlock {
            layout: Block { min_cols: min, pref_cols: pref, ideal_max_cols: ideal, max_cols: max, min_h: 0.5, weight: gweight },
            content: BlockContent::Claim { text: text.to_string() },
            typos: vec![t],
            mass,
            measure_ch: None,
        }
    }

    fn support(&self, text: &str, role: TextRole) -> PlanBlock {
        let t = self.typo(role, 1.0, ColorRole::TextSecondary);
        let fs = t.size_max_rem * 16.0;
        let factor = if role == TextRole::Lead { 0.8 } else { 1.0 };
        let measure_ch = self.physics.measure_ch * factor;
        let measure_px = measure_ch * 0.55 * fs;
        let (total, longest) = text_extent(text, &t, self.physics.personality);
        let min = self.gm.cols_for(longest * 1.05).max(3.0);
        let max = self.gm.cols_for(measure_px * 1.1).max(min);
        let ideal = self.gm.cols_for(measure_px).max(min).min(max);
        let pref = self.gm.cols_for(total / 5.0).max(min).min(ideal);
        let mass = self.text_mass(&t, text);
        PlanBlock {
            layout: Block { min_cols: min, pref_cols: pref, ideal_max_cols: ideal, max_cols: max, min_h: 0.5, weight: 1.0 },
            content: BlockContent::Support { text: text.to_string() },
            typos: vec![t],
            mass,
            measure_ch: Some(measure_ch),
        }
    }

    fn action(&self, node: &ContentNode) -> PlanBlock {
        let bt = self.typo(TextRole::Button, 1.0, ColorRole::AccentText);
        let it = self.typo(TextRole::Body, 1.0, ColorRole::Text);
        let input_type = node.action_input_type.clone().unwrap_or_else(|| "email".to_string());
        let label = node
            .action_label
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Get started".to_string());
        let placeholder = node
            .action_placeholder
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| if input_type == "email" { "Work email".to_string() } else { "Enter a value".to_string() });
        let href = node.action_href.clone().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "#".to_string());
        let (min, pref, ideal, max) = match node.action_type {
            ActionType::TransactionalInput => (4.0, 5.0, 6.0, 7.0),
            _ => (2.0, 3.0, 3.0, 5.0),
        };
        let fs = bt.size_max_rem * 16.0;
        let label_w = run_width(&label, &bt, self.physics.personality);
        let pad_adv = metrics_data::average_advance_em(metrics_family(self.physics.personality)) * fs;
        let ink = (label_w + 4.0 * pad_adv) * (fs * 3.0);
        let bg = self.palette.section_bg(self.bg_index).rgb;
        let lc = apca::lc(self.palette.accent.rgb, bg).abs();
        let mass = visual_mass::mass(ink, lc, bt.weight);
        PlanBlock {
            layout: Block { min_cols: min, pref_cols: pref, ideal_max_cols: ideal, max_cols: max, min_h: 0.5, weight: 1.2 },
            content: BlockContent::Action { kind: node.action_type, label, placeholder, href, input_type },
            typos: vec![bt, it],
            mass,
            measure_ch: None,
        }
    }

    fn metric(&self, value: &str, label: &str) -> PlanBlock {
        let mt = self.typo(TextRole::Metric, 1.0, ColorRole::Text);
        let lt = self.typo(TextRole::Label, 1.0, ColorRole::TextSecondary);
        let (vtot, _) = text_extent(value, &mt, self.physics.personality);
        let (ltot, _) = text_extent(label, &lt, self.physics.personality);
        let min = self.gm.cols_for(vtot.max(ltot) * 1.05);
        let mass = self.text_mass(&mt, value) + self.text_mass(&lt, label);
        PlanBlock {
            layout: Block {
                min_cols: min,
                pref_cols: min,
                ideal_max_cols: (min + 1.0).min(12.0),
                max_cols: (min + 3.0).min(12.0),
                min_h: 0.5,
                weight: 1.5,
            },
            content: BlockContent::Metric { value: value.to_string(), label: label.to_string() },
            typos: vec![mt, lt],
            mass,
            measure_ch: None,
        }
    }

    fn feature(&self, title: &str, body: Option<&str>) -> PlanBlock {
        let tt = self.typo(TextRole::Subheading, 1.0, ColorRole::Text);
        let bt = self.typo(TextRole::Body, 1.0, ColorRole::TextSecondary);
        let (_, tlong) = text_extent(title, &tt, self.physics.personality);
        let (blong, mut mass) = match body {
            Some(b) => {
                let (_, l) = text_extent(b, &bt, self.physics.personality);
                (l, self.text_mass(&bt, b))
            }
            None => (0.0, 0.0),
        };
        mass += self.text_mass(&tt, title);
        let measure_px = self.physics.measure_ch * 0.55 * bt.size_max_rem * 16.0;
        let min = self.gm.cols_for(blong.max(tlong) * 1.05).max(3.0);
        let max = self.gm.cols_for(measure_px * 1.05).max(min);
        let ideal = self.gm.cols_for(measure_px * 0.85).max(min).min(max);
        PlanBlock {
            layout: Block { min_cols: min, pref_cols: min, ideal_max_cols: ideal, max_cols: max, min_h: 0.5, weight: 1.5 },
            content: BlockContent::Feature { title: title.to_string(), body: body.map(|s| s.to_string()) },
            typos: vec![tt, bt],
            mass,
            measure_ch: Some(self.physics.measure_ch),
        }
    }

    /// A navigational / footer link: label with the body face, constrained by its own extent.
    fn link(&self, label: &str, href: &str) -> PlanBlock {
        let t = self.typo(TextRole::Button, 1.0, ColorRole::TextSecondary);
        let (total, longest) = text_extent(label, &t, self.physics.personality);
        let min = self.gm.cols_for(longest * 1.05).max(2.0);
        let mass = self.text_mass(&t, label);
        PlanBlock {
            layout: Block {
                min_cols: min,
                pref_cols: min,
                ideal_max_cols: (min + 1.0).min(12.0),
                max_cols: self.gm.cols_for(total * 1.08).max(min).min(12.0),
                min_h: 0.5,
                weight: 1.0,
            },
            content: BlockContent::Link { label: label.to_string(), href: href.to_string() },
            typos: vec![t],
            mass,
            measure_ch: None,
        }
    }

    /// A pricing tier: plan name, price and an optional description.
    fn tier(&self, name: &str, price: &str, note: Option<&str>, body: Option<&str>) -> PlanBlock {
        let nt = self.typo(TextRole::Subheading, 1.0, ColorRole::Text);
        let pt = self.typo(TextRole::Metric, 1.0, ColorRole::Text);
        let bt = self.typo(TextRole::Body, 1.0, ColorRole::TextSecondary);
        let (npw, _) = text_extent(name, &nt, self.physics.personality);
        let (ppw, _) = text_extent(price, &pt, self.physics.personality);
        let (bpw, blong) = match body {
            Some(b) => {
                let (w, l) = text_extent(b, &bt, self.physics.personality);
                (w, l)
            }
            None => (0.0, 0.0),
        };
        let mut mass = self.text_mass(&pt, price) + self.text_mass(&nt, name);
        if let Some(b) = body {
            mass += self.text_mass(&bt, b);
        }
        if let Some(n) = note {
            mass += self.text_mass(&bt, n);
        }
        // A tier must fit name and price side by side.
        let min = self.gm.cols_for((npw + ppw) * 0.72 * 1.05).max(self.gm.cols_for(blong * 1.05)).max(3.0);
        let max = self.gm.cols_for(self.physics.measure_ch * 0.55 * bt.size_max_rem * 16.0 * 1.05).max(min).min(12.0);
        let ideal = self.gm.cols_for(bpw.max(npw + ppw) * 0.9 + 40.0).max(min).min(max);
        PlanBlock {
            layout: Block { min_cols: min, pref_cols: min, ideal_max_cols: ideal, max_cols: max, min_h: 0.5, weight: 1.5 },
            content: BlockContent::Tier {
                name: name.to_string(),
                price: price.to_string(),
                note: note.map(|s| s.to_string()),
                body: body.map(|s| s.to_string()),
            },
            typos: vec![nt, pt, bt],
            mass,
            measure_ch: Some(self.physics.measure_ch),
        }
    }

    /// A testimonial: quote text with an attribution line.
    fn quote(&self, text: &str, attribution: &str, role: Option<&str>) -> PlanBlock {
        let qt = self.typo(TextRole::Lead, 1.0, ColorRole::Text);
        let at = self.typo(TextRole::Label, 1.0, ColorRole::TextSecondary);
        let (_, qlong) = text_extent(text, &qt, self.physics.personality);
        let (alw, _) = text_extent(attribution, &at, self.physics.personality);
        let mut mass = self.text_mass(&qt, text) + self.text_mass(&at, attribution);
        if let Some(r) = role {
            mass += self.text_mass(&at, r);
        }
        let min = self.gm.cols_for(qlong * 1.05).max(self.gm.cols_for(alw * 1.05)).max(3.0);
        let max = self.gm.cols_for(self.physics.measure_ch * 0.8 * 0.55 * qt.size_max_rem * 16.0 * 1.1).max(min).min(12.0);
        let ideal = self.gm.cols_for(self.physics.measure_ch * 0.8 * 0.55 * qt.size_max_rem * 16.0).max(min).min(max);
        PlanBlock {
            layout: Block { min_cols: min, pref_cols: min, ideal_max_cols: ideal, max_cols: max, min_h: 0.5, weight: 1.0 },
            content: BlockContent::Quote {
                text: text.to_string(),
                attribution: attribution.to_string(),
                role: role.map(|s| s.to_string()),
            },
            typos: vec![qt, at],
            mass,
            measure_ch: Some(self.physics.measure_ch * 0.8),
        }
    }

    /// An interactive WebGPU / shader surface with area-derived visual mass.
    fn canvas(&self, shader: &str, aspect_ratio: f64, label: &str, caption: Option<&str>, hero_embed: bool) -> PlanBlock {
        let ct = self.typo(TextRole::Label, 1.0, ColorRole::TextSecondary);
        let aspect = aspect_ratio.clamp(0.25, 4.0);
        let (min_cols, pref_cols, ideal_cols, max_cols, gweight) = if hero_embed {
            (4.0, 5.0, 6.0, 7.0, 2.2)
        } else {
            (4.0, 6.0, 8.0, 12.0, 2.4)
        };
        let span_px = pref_cols * self.gm.col_px + (pref_cols - 1.0) * self.gm.gap_px;
        let height_px = (span_px / aspect).clamp(180.0, 720.0);
        let effective_ink = span_px * height_px * 0.35;
        let bg = self.palette.section_bg(self.bg_index).rgb;
        let lc = apca::lc(self.palette.accent.rgb, bg).abs().max(45.0);
        let mut mass = visual_mass::mass(effective_ink, lc, 500);
        if let Some(c) = caption {
            mass += self.text_mass(&ct, c);
        }
        PlanBlock {
            layout: Block {
                min_cols,
                pref_cols,
                ideal_max_cols: ideal_cols,
                max_cols,
                min_h: 1.5,
                weight: gweight,
            },
            content: BlockContent::Canvas {
                shader: shader.to_string(),
                aspect_ratio: aspect,
                label: label.to_string(),
                caption: caption.map(|s| s.to_string()),
            },
            typos: vec![ct],
            mass,
            measure_ch: None,
        }
    }

    /// A photograph (`<picture>/<img>`) or video (`<video>`) surface with temporal visual mass.
    #[allow(clippy::too_many_arguments)]
    fn media_asset(
        &self,
        kind: MediaKind,
        src: &str,
        fallback_src: Option<&str>,
        poster: Option<&str>,
        aspect_ratio: f64,
        alt: &str,
        caption: Option<&str>,
        hero_embed: bool,
    ) -> PlanBlock {
        let ct = self.typo(TextRole::Label, 1.0, ColorRole::TextSecondary);
        let aspect = aspect_ratio.clamp(0.25, 4.0);
        let (min_cols, pref_cols, ideal_cols, max_cols, gweight) = if hero_embed {
            (4.0, 5.0, 6.0, 7.0, if kind == MediaKind::Video { 2.5 } else { 2.1 })
        } else {
            (4.0, 6.0, 8.0, 12.0, if kind == MediaKind::Video { 2.6 } else { 2.2 })
        };
        let span_px = pref_cols * self.gm.col_px + (pref_cols - 1.0) * self.gm.gap_px;
        let height_px = (span_px / aspect).clamp(180.0, 720.0);
        // Video has 1.35x higher perceptual foveal pull than a static photograph due to temporal motion.
        let density_factor = if kind == MediaKind::Video { 0.48 } else { 0.35 };
        let effective_ink = span_px * height_px * density_factor;
        let bg = self.palette.section_bg(self.bg_index).rgb;
        let lc = apca::lc(self.palette.text.rgb, bg).abs().max(60.0);
        let mut mass = visual_mass::mass(effective_ink, lc, 500);
        if let Some(c) = caption {
            mass += self.text_mass(&ct, c);
        }
        PlanBlock {
            layout: Block {
                min_cols,
                pref_cols,
                ideal_max_cols: ideal_cols,
                max_cols,
                min_h: 1.5,
                weight: gweight,
            },
            content: BlockContent::MediaAsset {
                kind,
                src: src.to_string(),
                fallback_src: fallback_src.map(|s| s.to_string()),
                poster: poster.map(|s| s.to_string()),
                aspect_ratio: aspect,
                alt: alt.to_string(),
                caption: caption.map(|s| s.to_string()),
                priority_eager: self.bg_index == 0 || hero_embed,
            },
            typos: vec![ct],
            mass,
            measure_ch: None,
        }
    }
}

fn non_blank(o: &Option<String>) -> Option<&str> {
    match o {
        Some(s) if !s.trim().is_empty() => Some(s.as_str()),
        _ => None,
    }
}

pub fn plan_section(
    index: usize,
    node: &ContentNode,
    physics: &Physics,
    scale: &Scale,
    palette: &Palette,
    gm: GridMetrics,
) -> Result<SectionPlan, String> {
    let b = Builder { physics, scale, palette, gm, bg_index: index };
    let claim = non_blank(&node.primary_claim);
    let support = non_blank(&node.supporting_text);
    let mut blocks: Vec<PlanBlock> = Vec::new();
    let mut has_header = false;
    let grammar;
    let height_units;
    let mut min_height_dvh: Option<u32> = None;

    match node.role {
        Role::Hero => {
            let mult = match node.visual_anchor {
                VisualAnchor::MonumentalType => 1.15,
                VisualAnchor::MonumentalMetric => 0.9,
                VisualAnchor::Quiet => 0.8,
                VisualAnchor::WebGpuCanvas | VisualAnchor::Photo | VisualAnchor::Video => 0.95,
                VisualAnchor::None => 1.0,
            };
            let c = claim.ok_or_else(|| format!("{}: hero needs primary_claim", node.id))?;
            let has_hero_visual = matches!(
                node.visual_anchor,
                VisualAnchor::WebGpuCanvas | VisualAnchor::Photo | VisualAnchor::Video
            ) || node.shader.is_some()
                || node.media_src.is_some();
            let max_claim_cols = if has_hero_visual { 8.0 } else { 12.0 };
            blocks.push(b.claim(c, TextRole::Display, mult, 3.0, max_claim_cols));
            if let Some(s) = support {
                blocks.push(b.support(s, TextRole::Lead));
            }
            if node.action_type != ActionType::None {
                blocks.push(b.action(node));
            }
            if let Some(msrc) = non_blank(&node.media_src) {
                let kind = node.media_kind.unwrap_or_else(|| match node.visual_anchor {
                    VisualAnchor::Video => MediaKind::Video,
                    VisualAnchor::Photo => MediaKind::Image,
                    _ => MediaKind::infer_from_src(Some(msrc)),
                });
                let aspect = node.aspect_ratio.unwrap_or(1.333);
                blocks.push(b.media_asset(
                    kind,
                    msrc,
                    non_blank(&node.fallback_src),
                    non_blank(&node.poster),
                    aspect,
                    c,
                    None,
                    true,
                ));
            } else if node.visual_anchor == VisualAnchor::WebGpuCanvas || node.shader.is_some() {
                let sh = non_blank(&node.shader).unwrap_or("sdf_field");
                let aspect = node.aspect_ratio.unwrap_or(1.333);
                blocks.push(b.canvas(sh, aspect, c, None, true));
            }
            grammar = Grammar { policy: AxisPolicy::Free, header_band: false, equalize: false };
            height_units = 8.0;
            min_height_dvh = Some(100);
        }
        Role::Media => {
            if let Some(c) = claim {
                blocks.push(b.claim(c, TextRole::Heading, 1.0, 2.0, 10.0));
                has_header = true;
            }
            if let Some(s) = support {
                blocks.push(b.support(s, TextRole::Lead));
            }
            if node.items.is_empty() {
                let aspect = node.aspect_ratio.unwrap_or(1.778);
                let lbl = claim.unwrap_or("Interactive media surface");
                if let Some(msrc) = non_blank(&node.media_src) {
                    let kind = node.media_kind.unwrap_or_else(|| MediaKind::infer_from_src(Some(msrc)));
                    blocks.push(b.media_asset(
                        kind,
                        msrc,
                        non_blank(&node.fallback_src),
                        non_blank(&node.poster),
                        aspect,
                        lbl,
                        None,
                        false,
                    ));
                } else {
                    let sh = non_blank(&node.shader).unwrap_or("webgpu_surface");
                    blocks.push(b.canvas(sh, aspect, lbl, None, false));
                }
            } else {
                for it in &node.items {
                    let aspect = it.aspect_ratio.or(node.aspect_ratio).unwrap_or(1.778);
                    let lbl = non_blank(&it.title).or(claim).unwrap_or("Interactive media surface");
                    if let Some(msrc) = non_blank(&it.media_src) {
                        let kind = it
                            .media_kind
                            .or(node.media_kind)
                            .unwrap_or_else(|| MediaKind::infer_from_src(Some(msrc)));
                        blocks.push(b.media_asset(
                            kind,
                            msrc,
                            non_blank(&it.fallback_src).or_else(|| non_blank(&node.fallback_src)),
                            non_blank(&it.poster).or_else(|| non_blank(&node.poster)),
                            aspect,
                            lbl,
                            non_blank(&it.body),
                            false,
                        ));
                    } else {
                        let sh = non_blank(&it.shader).or_else(|| non_blank(&node.shader)).unwrap_or("webgpu_surface");
                        blocks.push(b.canvas(sh, aspect, lbl, non_blank(&it.body), false));
                    }
                }
            }
            if node.action_type != ActionType::None {
                blocks.push(b.action(node));
            }
            grammar = Grammar { policy: AxisPolicy::Free, header_band: has_header, equalize: node.items.len() > 1 };
            height_units = if has_header { 7.0 } else { 6.0 };
        }
        Role::ProofMatrix => {
            if let Some(c) = claim {
                blocks.push(b.claim(c, TextRole::Subheading, 1.0, 1.0, 8.0));
                has_header = true;
            }
            for it in &node.items {
                let v = non_blank(&it.metric).ok_or_else(|| format!("{}: metric missing", node.id))?;
                let l = non_blank(&it.label).ok_or_else(|| format!("{}: label missing", node.id))?;
                blocks.push(b.metric(v, l));
            }
            grammar = Grammar { policy: AxisPolicy::VerticalOnly, header_band: has_header, equalize: true };
            height_units = if has_header { 4.0 } else { 3.0 };
        }
        Role::FeatureList => {
            if let Some(c) = claim {
                blocks.push(b.claim(c, TextRole::Heading, 1.0, 2.0, 10.0));
                has_header = true;
            }
            for it in &node.items {
                let t = non_blank(&it.title).ok_or_else(|| format!("{}: title missing", node.id))?;
                blocks.push(b.feature(t, non_blank(&it.body)));
            }
            grammar = Grammar { policy: AxisPolicy::Free, header_band: has_header, equalize: true };
            height_units = if has_header { 7.0 } else { 6.0 };
        }
        Role::Narrative | Role::Cta => {
            if let Some(c) = claim {
                blocks.push(b.claim(c, TextRole::Heading, 1.0, 2.0, 10.0));
            }
            if let Some(s) = support {
                blocks.push(b.support(s, TextRole::Lead));
            }
            if node.action_type != ActionType::None {
                blocks.push(b.action(node));
            }
            grammar = Grammar { policy: AxisPolicy::Free, header_band: false, equalize: false };
            height_units = 4.0;
        }
        Role::Nav => {
            if let Some(c) = claim {
                blocks.push(b.claim(c, TextRole::Subheading, 1.0, 2.0, 6.0));
            }
            for it in &node.items {
                let l = non_blank(&it.title).ok_or_else(|| format!("{}: link label missing", node.id))?;
                let h = non_blank(&it.href).ok_or_else(|| format!("{}: link href missing", node.id))?;
                blocks.push(b.link(l, h));
            }
            if node.action_type != ActionType::None {
                blocks.push(b.action(node));
            }
            grammar = Grammar { policy: AxisPolicy::VerticalOnly, header_band: false, equalize: false };
            height_units = 2.0;
        }
        Role::Footer => {
            if let Some(c) = claim {
                blocks.push(b.claim(c, TextRole::Subheading, 1.0, 2.0, 6.0));
                has_header = true;
            }
            for it in &node.items {
                let l = non_blank(&it.title).ok_or_else(|| format!("{}: link label missing", node.id))?;
                let h = non_blank(&it.href).ok_or_else(|| format!("{}: link href missing", node.id))?;
                blocks.push(b.link(l, h));
            }
            if let Some(s) = support {
                blocks.push(b.support(s, TextRole::Body));
            }
            if node.action_type != ActionType::None {
                blocks.push(b.action(node));
            }
            grammar = Grammar { policy: AxisPolicy::Free, header_band: has_header, equalize: true };
            height_units = 4.0;
        }
        Role::Pricing => {
            if let Some(c) = claim {
                blocks.push(b.claim(c, TextRole::Heading, 1.0, 2.0, 10.0));
                has_header = true;
            }
            for it in &node.items {
                let name = non_blank(&it.title).ok_or_else(|| format!("{}: plan name missing", node.id))?;
                let price = non_blank(&it.metric).ok_or_else(|| format!("{}: price missing", node.id))?;
                blocks.push(b.tier(name, price, non_blank(&it.label), non_blank(&it.body)));
            }
            grammar = Grammar { policy: AxisPolicy::Free, header_band: has_header, equalize: true };
            height_units = if has_header { 8.0 } else { 7.0 };
        }
        Role::Testimonial => {
            if let Some(c) = claim {
                blocks.push(b.claim(c, TextRole::Heading, 1.0, 2.0, 10.0));
                has_header = true;
            }
            for it in &node.items {
                let text = non_blank(&it.body).ok_or_else(|| format!("{}: quote body missing", node.id))?;
                let who = non_blank(&it.title).ok_or_else(|| format!("{}: quote attribution missing", node.id))?;
                blocks.push(b.quote(text, who, non_blank(&it.label)));
            }
            grammar = Grammar { policy: AxisPolicy::Free, header_band: has_header, equalize: true };
            height_units = if has_header { 6.0 } else { 5.0 };
        }
        Role::Faq => {
            if let Some(c) = claim {
                blocks.push(b.claim(c, TextRole::Heading, 1.0, 2.0, 10.0));
                has_header = true;
            }
            for it in &node.items {
                let q = non_blank(&it.title).ok_or_else(|| format!("{}: question missing", node.id))?;
                let a = non_blank(&it.body).ok_or_else(|| format!("{}: answer missing", node.id))?;
                blocks.push(b.feature(q, Some(a)));
            }
            grammar = Grammar { policy: AxisPolicy::HorizontalOnly, header_band: has_header, equalize: false };
            height_units = if has_header { 7.0 } else { 6.0 };
        }
    }

    if blocks.is_empty() {
        return Err(format!("{}: section produced no blocks", node.id));
    }
    Ok(SectionPlan {
        id: node.id.clone(),
        role: node.role,
        blocks,
        grammar,
        height_units,
        min_height_dvh,
        bg_index: index,
        has_header,
    })
}
