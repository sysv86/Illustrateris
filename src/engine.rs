//! Pipeline orchestration: Context JSON -> physics -> palette -> plans -> arrangements -> IR -> HTML.
//! The result is a pure function of (input, config): same bytes in, same bytes out.

use crate::codegen::backend::Backend;
use crate::codegen::html::{render_document, render_fragment_with_options, RenderOptions};
use crate::color::apca::LcTargets;
use crate::color::palette::{synthesize, Palette};
use crate::compose::{plan_section, GridMetrics, SectionPlan};
use crate::context::{fnv1a64, parse_context, profile, ContextError, FontPersonality, HueSource, Physics};
use crate::ir::ast::Document;
use crate::ir::build::build_document;
use crate::ir::token::{Scale, SpacingSet};
use crate::json::{self, Value};
use crate::layout::arrange::Block;
use crate::layout::rng::derive_seed;
use crate::layout::search::{search, Chosen, SearchInput};
use crate::layout::visual_mass::{allowed_range, centroid_x, centroid_y, eccentricity, eccentricity_y};
use std::fmt;

pub const ENGINE_NAME: &str = "ple-engine";
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitMode {
    Document,
    Fragment,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// BSP candidates generated and scored per section.
    pub candidates: usize,
    /// Maximum horizontal eccentricity of the visual centre of mass (fraction of page width).
    pub max_eccentricity: f64,
    /// Design viewport used to turn text extents into column counts.
    pub viewport_px: f64,
    pub emit: EmitMode,
    pub tailwind_cdn: bool,
    /// Styling backend: Tailwind utility classes (default) or a self-contained plain-CSS stylesheet.
    pub backend: Backend,
    /// Clean / De-PLE mode: strips `ple-engine` provenance tags/comments, emits `:root` CSS Custom
    /// Properties, and replaces `ple-<16hex>` classes with `ui-<8hex>`.
    pub clean: bool,
    /// Overrides `intent.seed` and the context-derived seed.
    pub seed: Option<u64>,
    pub targets: LcTargets,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            candidates: 64,
            max_eccentricity: 0.15,
            viewport_px: 1440.0,
            emit: EmitMode::Document,
            tailwind_cdn: true,
            backend: Backend::Tailwind,
            clean: false,
            seed: None,
            targets: LcTargets::default(),
        }
    }
}

impl EngineConfig {
    /// Applies a JSON options object: candidates, max_eccentricity, viewport, emit, cdn, backend, clean/eject, seed.
    pub fn apply_json(&mut self, v: &Value) -> Result<(), String> {
        let obj = match v.as_object() {
            Some(o) => o,
            None => return Err("options must be a JSON object".to_string()),
        };
        for (k, val) in obj {
            match k.as_str() {
                "candidates" => {
                    let n = val.as_f64().ok_or("candidates must be a number")?;
                    if !(1.0..=2048.0).contains(&n) {
                        return Err("candidates must be in 1..=2048".to_string());
                    }
                    self.candidates = n as usize;
                }
                "max_eccentricity" => {
                    let n = val.as_f64().ok_or("max_eccentricity must be a number")?;
                    if !(0.01..=0.5).contains(&n) {
                        return Err("max_eccentricity must be in 0.01..=0.5".to_string());
                    }
                    self.max_eccentricity = n;
                }
                "viewport" => {
                    let n = val.as_f64().ok_or("viewport must be a number")?;
                    if !(480.0..=4000.0).contains(&n) {
                        return Err("viewport must be in 480..=4000".to_string());
                    }
                    self.viewport_px = n;
                }
                "emit" => match val.as_str() {
                    Some("document") => self.emit = EmitMode::Document,
                    Some("fragment") => self.emit = EmitMode::Fragment,
                    _ => return Err("emit must be \"document\" or \"fragment\"".to_string()),
                },
                "cdn" => {
                    self.tailwind_cdn = val.as_bool().ok_or("cdn must be a boolean")?;
                }
                "backend" => match val.as_str().and_then(Backend::parse) {
                    Some(b) => self.backend = b,
                    None => return Err("backend must be \"tailwind\" or \"css\"".to_string()),
                },
                "clean" | "eject" => {
                    self.clean = val.as_bool().ok_or("clean/eject must be a boolean")?;
                }
                "seed" => {
                    self.seed = Some(match val {
                        Value::Num(n) if *n >= 0.0 && n.fract() == 0.0 => *n as u64,
                        Value::Str(s) => s.trim().parse::<u64>().map_err(|_| "seed string must be a decimal u64".to_string())?,
                        _ => return Err("seed must be a non-negative integer or decimal string".to_string()),
                    });
                }
                other => return Err(format!("unknown option '{}'", other)),
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum EngineError {
    Context(ContextError),
    Layout(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::Context(e) => write!(f, "{}", e),
            EngineError::Layout(m) => write!(f, "layout error: {}", m),
        }
    }
}

impl std::error::Error for EngineError {}

#[derive(Debug, Clone)]
pub struct Compiled {
    pub html: String,
    pub report: Value,
}

#[cfg(not(target_arch = "wasm32"))]
struct Clock(std::time::Instant);
#[cfg(not(target_arch = "wasm32"))]
impl Clock {
    fn start() -> Clock {
        Clock(std::time::Instant::now())
    }
    fn ms(&self) -> f64 {
        self.0.elapsed().as_secs_f64() * 1000.0
    }
}
#[cfg(target_arch = "wasm32")]
struct Clock;
#[cfg(target_arch = "wasm32")]
impl Clock {
    fn start() -> Clock {
        Clock
    }
    fn ms(&self) -> f64 {
        0.0
    }
}

pub fn compile(src: &str, cfg: &EngineConfig) -> Result<Compiled, EngineError> {
    let clock = Clock::start();
    let ctx = parse_context(src).map_err(EngineError::Context)?;

    // Phase 1: semantic profiler
    let physics = profile(&ctx.intent, cfg.max_eccentricity);
    let seed = cfg.seed.or(ctx.intent.seed).unwrap_or_else(|| fnv1a64(ctx.canonical.as_bytes()));

    // Phase 2: colour
    let palette = synthesize(&physics, &cfg.targets);

    // Tokens
    let scale = Scale::new(&physics);
    let spacing = SpacingSet::new(&physics, &scale);
    let gm = GridMetrics::new(cfg.viewport_px, &spacing);

    // Plans (content -> blocks with real size constraints)
    let mut plans: Vec<SectionPlan> = Vec::new();
    for (i, node) in ctx.content_tree.iter().enumerate() {
        plans.push(plan_section(i, node, &physics, &scale, &palette, gm).map_err(EngineError::Layout)?);
    }

    // Phases 3 + 4: spatial grammar, solver, candidate search with entropy damping across sections
    let mut chosen: Vec<Chosen> = Vec::new();
    let mut ranges: Vec<(f64, f64)> = Vec::new();
    let mut prev: Option<f64> = None;
    for (i, plan) in plans.iter().enumerate() {
        let blocks: Vec<Block> = plan.blocks.iter().map(|b| b.layout.clone()).collect();
        let masses: Vec<f64> = plan.blocks.iter().map(|b| b.mass).collect();
        let allowed = allowed_range(prev, physics.asymmetry);
        let input = SearchInput {
            blocks: &blocks,
            masses: &masses,
            grammar: plan.grammar,
            cols: 12,
            height: plan.height_units,
            asymmetry: physics.asymmetry,
            candidates: cfg.candidates,
            seed: derive_seed(seed, 1000 + i as u64),
            max_ecc: cfg.max_eccentricity,
            allowed,
        };
        let c = search(&input);
        prev = Some(c.factor);
        ranges.push(allowed);
        chosen.push(c);
    }

    // Phase 5: IR + codegen
    let generator = format!("{} {} seed={}", ENGINE_NAME, ENGINE_VERSION, seed);
    let doc = build_document(&ctx, &physics, &palette, &scale, &spacing, &plans, &chosen, &generator);
    let render_opts = RenderOptions { tailwind_cdn: cfg.tailwind_cdn, backend: cfg.backend, clean: cfg.clean };
    let html = match cfg.emit {
        EmitMode::Document => render_document(&doc, &render_opts),
        EmitMode::Fragment => render_fragment_with_options(&doc, &render_opts),
    };

    let report = build_report(&doc, &physics, &palette, &plans, &chosen, &ranges, seed, cfg, clock.ms());
    Ok(Compiled { html, report })
}

fn hue_source_json(h: &HueSource) -> Value {
    match h {
        HueSource::Override => json::string("override"),
        HueSource::Hash => json::string("domain_hash"),
        HueSource::Semantic(keys) => json::obj(vec![(
            "semantic",
            json::arr(keys.iter().map(|k| json::string(k.as_str())).collect()),
        )]),
    }
}

fn tok_json(t: &crate::color::palette::Tok) -> Value {
    json::obj(vec![("oklch", json::string(t.oklch.css())), ("hex", json::string(t.hex()))])
}

#[allow(clippy::too_many_arguments)]
fn build_report(
    doc: &Document,
    physics: &Physics,
    palette: &Palette,
    plans: &[SectionPlan],
    chosen: &[Chosen],
    ranges: &[(f64, f64)],
    seed: u64,
    cfg: &EngineConfig,
    elapsed_ms: f64,
) -> Value {
    let mut warnings: Vec<Value> = Vec::new();
    for c in &palette.checks {
        if !c.passed {
            warnings.push(json::string(format!(
                "contrast check '{}' failed: |Lc| {:.2} < {:.0}",
                c.name, c.lc, c.target
            )));
        }
    }

    let mut page_acc = 0.0;
    let mut page_acc_y = 0.0;
    let mut page_total = 0.0;
    let mut sections: Vec<Value> = Vec::new();
    for (i, (plan, ch)) in plans.iter().zip(chosen.iter()).enumerate() {
        let masses: Vec<f64> = plan.blocks.iter().map(|b| b.mass).collect();
        let (cx, total) = centroid_x(&ch.arrangement, &masses);
        let (cy, _) = centroid_y(&ch.arrangement, &masses);
        page_acc += cx * total;
        page_acc_y += cy * total;
        page_total += total;
        for w in &ch.warnings {
            warnings.push(json::string(format!("{}: {}", plan.id, w)));
        }
        let placements: Vec<Value> = ch
            .arrangement
            .placements
            .iter()
            .map(|p| {
                json::obj(vec![
                    ("block", json::int(p.block)),
                    ("kind", json::string(plan.blocks[p.block].kind_name())),
                    ("col_start", json::int(p.col_start as usize)),
                    ("col_span", json::int(p.col_span as usize)),
                    ("row_start", json::int(p.row_start as usize)),
                    ("row_span", json::int(p.row_span as usize)),
                ])
            })
            .collect();
        sections.push(json::obj(vec![
            ("id", json::string(plan.id.as_str())),
            ("role", json::string(plan.role.slug())),
            ("background", json::string(if plan.bg_index % 2 == 0 { "bg" } else { "surface" })),
            ("candidates_tried", json::int(ch.tried)),
            ("candidates_feasible", json::int(ch.feasible)),
            ("chosen_candidate", json::int(ch.candidate_index)),
            ("fallback_stack", json::boolean(ch.fallback)),
            ("imbalance_factor", json::num(ch.factor)),
            ("allowed_range", json::arr(vec![json::num(ranges[i].0), json::num(ranges[i].1)])),
            ("eccentricity", json::num(ch.eccentricity)),
            ("centroid_y", json::num(ch.centroid_y)),
            ("eccentricity_y", json::num(ch.eccentricity_y)),
            ("penalty", json::num(ch.penalty)),
            ("solver_cost", json::num(ch.arrangement.cost)),
            ("row_heights", json::arr(ch.arrangement.row_heights.iter().map(|h| json::num(*h)).collect())),
            ("placements", json::arr(placements)),
        ]));
    }
    let page_cx = if page_total > 0.0 { page_acc / page_total } else { 0.5 };
    let page_cy = if page_total > 0.0 { page_acc_y / page_total } else { 0.5 };
    let page_ecc = eccentricity(page_cx);
    let page_ecc_y = eccentricity_y(page_cy);
    if page_ecc > cfg.max_eccentricity {
        warnings.push(json::string(format!(
            "page-level eccentricity {:.3} exceeds the limit {:.3}",
            page_ecc, cfg.max_eccentricity
        )));
    }

    let checks: Vec<Value> = palette
        .checks
        .iter()
        .map(|c| {
            json::obj(vec![
                ("name", json::string(c.name.as_str())),
                ("fg", json::string(c.fg.as_str())),
                ("bg", json::string(c.bg.as_str())),
                ("lc", json::num(c.lc)),
                ("target", json::num(c.target)),
                ("passed", json::boolean(c.passed)),
                ("adjusted", json::boolean(c.adjusted)),
            ])
        })
        .collect();

    let personality = match physics.personality {
        FontPersonality::Sans => "sans",
        FontPersonality::Serif => "serif",
        FontPersonality::Mono => "mono",
    };

    // Exact per-text estimates, in DOM order, so a browser measurement can be compared against
    // what the engine predicted for the same run.
    let texts: Vec<Value> = crate::ir::inspect::collect_texts(doc)
        .into_iter()
        .map(|r| {
            let (single, longest) = crate::compose::text_extent(&r.text, &r.typography, physics.personality);
            json::obj(vec![
                ("tag", json::string(r.tag)),
                ("text", json::string(r.text)),
                ("family", json::string(r.typography.font_family.clone())),
                ("font_size_px", json::num(r.typography.size_max_rem * 16.0)),
                ("weight", json::int(r.typography.weight as usize)),
                ("estimated_single_line_px", json::num(single)),
                ("estimated_longest_word_px", json::num(longest)),
            ])
        })
        .collect();

    let m = &physics.modalities;
    let modalities_json = json::obj(vec![
        (
            "noisy_gradient",
            json::obj(vec![
                ("recommended", json::boolean(m.noisy_gradient_score >= 0.38)),
                ("score", json::num(m.noisy_gradient_score)),
                ("grain_frequency", json::num(m.grain_frequency)),
                ("grain_opacity", json::num(m.grain_opacity)),
            ]),
        ),
        (
            "webgpu_shader",
            json::obj(vec![
                ("recommended", json::boolean(m.webgpu_shader_score >= 0.52)),
                ("score", json::num(m.webgpu_shader_score)),
            ]),
        ),
        (
            "micro_animation",
            json::obj(vec![
                ("spring_omega_rad_s", json::num(m.spring_omega)),
                ("hover_ms", json::int(m.hover_ms as usize)),
            ]),
        ),
        (
            "svg_iconography",
            json::obj(vec![
                ("recommended", json::boolean(m.svg_icon_score >= 0.40)),
                ("score", json::num(m.svg_icon_score)),
                ("stroke_width_px", json::num(m.svg_stroke_px)),
                ("sharp_joins", json::boolean(m.svg_sharp_joins)),
            ]),
        ),
    ]);

    json::obj(vec![
        ("engine", json::obj(vec![("name", json::string(ENGINE_NAME)), ("version", json::string(ENGINE_VERSION))])),
        ("seed", json::string(seed.to_string())),
        ("timing_ms", json::num(elapsed_ms)),
        (
            "profile",
            json::obj(vec![
                ("precision", json::num(physics.tone.precision)),
                ("warmth", json::num(physics.tone.warmth)),
                ("drama", json::num(physics.tone.drama)),
                ("formality", json::num(physics.tone.formality)),
                ("font_personality", json::string(personality)),
                ("theme", json::string(if physics.theme_dark { "dark" } else { "light" })),
                ("hue", json::num(physics.hue)),
                ("hue_source", hue_source_json(&physics.hue_source)),
                ("scale_ratio", json::num(physics.scale_ratio)),
                ("base_unit_px", json::num(physics.base_unit)),
                ("display_max_rem", json::num(physics.display_max_rem)),
                ("display_weight", json::int(physics.display_weight as usize)),
                ("measure_ch", json::num(physics.measure_ch)),
                ("aesthetic_modalities", modalities_json),
            ]),
        ),
        (
            "palette",
            json::obj(vec![
                ("bg", tok_json(&palette.bg)),
                ("surface", tok_json(&palette.surface)),
                ("border", tok_json(&palette.border)),
                ("text", tok_json(&palette.text)),
                ("text_secondary", tok_json(&palette.text_secondary)),
                ("accent", tok_json(&palette.accent)),
                ("accent_text", tok_json(&palette.accent_text)),
                ("all_checks_passed", json::boolean(palette.all_passed())),
                ("checks", json::arr(checks)),
            ]),
        ),
        ("sections", json::arr(sections)),
        ("texts", json::arr(texts)),
        ("backend", json::string(cfg.backend.name())),
        ("clean", json::boolean(cfg.clean)),
        (
            "page",
            json::obj(vec![
                ("centroid_x", json::num(page_cx)),
                ("centroid_y", json::num(page_cy)),
                ("eccentricity", json::num(page_ecc)),
                ("eccentricity_y", json::num(page_ecc_y)),
                ("within_limit", json::boolean(page_ecc <= cfg.max_eccentricity)),
            ]),
        ),
        ("warnings", json::arr(warnings)),
    ])
}
