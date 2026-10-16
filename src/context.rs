//! Phase 0 + 1: context deserialization / validation and the Semantic Profiler.
//!
//! The base model passes only entities and context markers. Everything physical
//! (scale ratio, grid unit, weights, measure, hue, theme, and sensory modality calculus)
//! is derived continuously here without static templates.

use crate::json::{self, JsonError, Value};
use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ContextError {
    Json(JsonError),
    Invalid(String),
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextError::Json(e) => write!(f, "{}", e),
            ContextError::Invalid(m) => write!(f, "invalid context: {}", m),
        }
    }
}

impl std::error::Error for ContextError {}

impl From<JsonError> for ContextError {
    fn from(e: JsonError) -> Self {
        ContextError::Json(e)
    }
}

fn invalid<T, S: Into<String>>(msg: S) -> Result<T, ContextError> {
    Err(ContextError::Invalid(msg.into()))
}

// ---------------------------------------------------------------------------
// Context model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePref {
    Auto,
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Nav,
    Hero,
    ProofMatrix,
    FeatureList,
    Narrative,
    Media,
    Pricing,
    Testimonial,
    Faq,
    Cta,
    Footer,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "nav" | "navigation" | "navbar" => Some(Role::Nav),
            "hero" => Some(Role::Hero),
            "proof_matrix" | "proof" | "stats" => Some(Role::ProofMatrix),
            "feature_list" | "feature_grid" | "features" => Some(Role::FeatureList),
            "narrative" | "text" => Some(Role::Narrative),
            "media" | "canvas" | "webgpu" | "showcase" | "gallery" | "video" => Some(Role::Media),
            "pricing" => Some(Role::Pricing),
            "testimonial" | "testimonials" | "quote" => Some(Role::Testimonial),
            "faq" | "accordion" => Some(Role::Faq),
            "cta" => Some(Role::Cta),
            "footer" => Some(Role::Footer),
            _ => None,
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Role::Nav => "nav",
            Role::Hero => "hero",
            Role::ProofMatrix => "proof",
            Role::FeatureList => "features",
            Role::Narrative => "narrative",
            Role::Media => "media",
            Role::Pricing => "pricing",
            Role::Testimonial => "testimonial",
            Role::Faq => "faq",
            Role::Cta => "cta",
            Role::Footer => "footer",
        }
    }

    /// Roles whose `primary_claim` is a page/section heading and may become the single `<h1>`.
    pub fn claim_is_heading(self) -> bool {
        !matches!(self, Role::Nav | Role::Footer)
    }

    /// Human-readable list used in validation error messages.
    pub const ALL_NAMES: &'static str =
        "nav|hero|proof_matrix|feature_list|narrative|media|pricing|testimonial|faq|cta|footer";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    None,
    TransactionalInput,
    PrimaryButton,
    SecondaryLink,
}

impl ActionType {
    pub fn parse(s: &str) -> Option<ActionType> {
        match s {
            "none" => Some(ActionType::None),
            "transactional_input" => Some(ActionType::TransactionalInput),
            "primary_button" => Some(ActionType::PrimaryButton),
            "secondary_link" => Some(ActionType::SecondaryLink),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualAnchor {
    None,
    MonumentalType,
    MonumentalMetric,
    Quiet,
    WebGpuCanvas,
    Photo,
    Video,
}

impl VisualAnchor {
    pub fn parse(s: &str) -> Option<VisualAnchor> {
        match s {
            "none" => Some(VisualAnchor::None),
            "monumental_type" => Some(VisualAnchor::MonumentalType),
            "monumental_metric" => Some(VisualAnchor::MonumentalMetric),
            "quiet" => Some(VisualAnchor::Quiet),
            "webgpu_canvas" | "shader_canvas" | "interactive_canvas" => Some(VisualAnchor::WebGpuCanvas),
            "photo" | "image" => Some(VisualAnchor::Photo),
            "video" | "reel" => Some(VisualAnchor::Video),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Canvas,
    Image,
    Video,
}

impl MediaKind {
    pub fn parse(s: &str) -> Option<MediaKind> {
        match s {
            "canvas" | "webgpu" | "shader" => Some(MediaKind::Canvas),
            "image" | "photo" | "picture" => Some(MediaKind::Image),
            "video" | "reel" | "loop" => Some(MediaKind::Video),
            _ => None,
        }
    }

    pub fn infer_from_src(src: Option<&str>) -> MediaKind {
        let Some(s) = src else { return MediaKind::Canvas };
        let lower = s.to_ascii_lowercase();
        if lower.ends_with(".mp4") || lower.ends_with(".webm") || lower.ends_with(".mov") {
            MediaKind::Video
        } else {
            MediaKind::Image
        }
    }
}

#[derive(Debug, Clone)]
pub struct Intent {
    pub domain: String,
    pub tone: String,
    pub density: f64,
    pub asymmetry: f64,
    pub chromatic_energy: f64,
    pub theme: ThemePref,
    pub seed: Option<u64>,
    pub hue: Option<f64>,
    pub lang: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Item {
    pub metric: Option<String>,
    pub label: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
    /// Link target for `nav` / `footer` link items.
    pub href: Option<String>,
    /// Optional WebGPU shader identifier for `media` / `showcase` items.
    pub shader: Option<String>,
    /// Optional primary media URL (`.avif`, `.webp`, `.mp4`, `.webm`) for photo/video items.
    pub media_src: Option<String>,
    /// Optional fallback/secondary media URL (e.g. `.webp` fallback for `.avif`, or `.mp4` fallback for `.webm`).
    pub fallback_src: Option<String>,
    /// Optional poster image URL (`.avif` / `.webp`) for `<video>` items.
    pub poster: Option<String>,
    /// Optional explicit media kind (`canvas` | `image` | `video`).
    pub media_kind: Option<MediaKind>,
    /// Optional aspect ratio (`width / height`, e.g. `1.777` for 16:9) for canvas/media items.
    pub aspect_ratio: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ContentNode {
    pub role: Role,
    pub id: String,
    pub primary_claim: Option<String>,
    pub supporting_text: Option<String>,
    pub action_type: ActionType,
    pub action_label: Option<String>,
    pub action_placeholder: Option<String>,
    pub action_href: Option<String>,
    pub action_input_type: Option<String>,
    pub visual_anchor: VisualAnchor,
    pub shader: Option<String>,
    pub media_src: Option<String>,
    pub fallback_src: Option<String>,
    pub poster: Option<String>,
    pub media_kind: Option<MediaKind>,
    pub aspect_ratio: Option<f64>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub intent: Intent,
    pub content_tree: Vec<ContentNode>,
    /// Canonical (sorted-key, compact) serialization of the input; the default seed derives from it.
    pub canonical: String,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

const MAX_SECTIONS: usize = 16;
const MAX_PROOF_ITEMS: usize = 6;
const MAX_FEATURE_ITEMS: usize = 8;
const MAX_LINK_ITEMS: usize = 8;
const MAX_PRICING_ITEMS: usize = 4;
const MAX_QUOTE_ITEMS: usize = 4;
const MAX_FAQ_ITEMS: usize = 12;
const MAX_MEDIA_ITEMS: usize = 4;
const MAX_TEXT_LEN: usize = 2000;
const MAX_HREF_LEN: usize = 512;

fn opt_str(v: &Value, key: &str, path: &str) -> Result<Option<String>, ContextError> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Str(s)) => {
            if s.chars().count() > MAX_TEXT_LEN {
                invalid(format!("{}.{} is longer than {} characters", path, key, MAX_TEXT_LEN))
            } else {
                Ok(Some(s.clone()))
            }
        }
        Some(_) => invalid(format!("{}.{} must be a string", path, key)),
    }
}

fn opt_num(v: &Value, key: &str, path: &str) -> Result<Option<f64>, ContextError> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Num(n)) => Ok(Some(*n)),
        Some(_) => invalid(format!("{}.{} must be a number", path, key)),
    }
}

fn unit_num(v: &Value, key: &str, path: &str, default: f64) -> Result<f64, ContextError> {
    Ok(opt_num(v, key, path)?.unwrap_or(default).clamp(0.0, 1.0))
}

fn parse_seed(v: &Value) -> Result<Option<u64>, ContextError> {
    match v.get("seed") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Num(n)) => {
            if *n >= 0.0 && n.fract() == 0.0 && *n < 9.007_199_254_740_992e15 {
                Ok(Some(*n as u64))
            } else {
                invalid("intent.seed must be a non-negative integer below 2^53 (use a string for larger values)")
            }
        }
        Some(Value::Str(s)) => match s.trim().parse::<u64>() {
            Ok(n) => Ok(Some(n)),
            Err(_) => invalid("intent.seed string must be a decimal u64"),
        },
        Some(_) => invalid("intent.seed must be a number or a decimal string"),
    }
}

fn parse_intent(v: Option<&Value>) -> Result<Intent, ContextError> {
    let empty = Value::Obj(Default::default());
    let v = v.unwrap_or(&empty);
    if v.as_object().is_none() {
        return invalid("intent must be an object");
    }
    let theme = match opt_str(v, "theme", "intent")?.as_deref() {
        None | Some("auto") => ThemePref::Auto,
        Some("dark") => ThemePref::Dark,
        Some("light") => ThemePref::Light,
        Some(other) => return invalid(format!("intent.theme must be auto|dark|light, got '{}'", other)),
    };
    let hue = opt_num(v, "hue", "intent")?.map(|h| h.rem_euclid(360.0));
    Ok(Intent {
        domain: opt_str(v, "domain", "intent")?.unwrap_or_else(|| "general".to_string()),
        tone: opt_str(v, "tone", "intent")?.unwrap_or_else(|| "neutral".to_string()),
        density: unit_num(v, "density", "intent", 0.5)?,
        asymmetry: unit_num(v, "asymmetry", "intent", 0.4)?,
        chromatic_energy: unit_num(v, "chromatic_energy", "intent", 0.3)?,
        theme,
        seed: parse_seed(v)?,
        hue,
        lang: opt_str(v, "lang", "intent")?.unwrap_or_else(|| "en".to_string()),
        title: opt_str(v, "title", "intent")?,
    })
}

fn valid_shader_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

fn parse_aspect(v: &Value, path: &str) -> Result<Option<f64>, ContextError> {
    match opt_num(v, "aspect_ratio", path)? {
        None => Ok(None),
        Some(a) if (0.25..=4.0).contains(&a) => Ok(Some(a)),
        Some(_) => invalid(format!("{}.aspect_ratio must be in 0.25..=4.0", path)),
    }
}

fn check_safe_url(opt: &Option<String>, field_path: &str) -> Result<(), ContextError> {
    if let Some(u) = opt {
        if u.chars().count() > MAX_HREF_LEN {
            return invalid(format!("{} is longer than {} characters", field_path, MAX_HREF_LEN));
        }
        if !safe_href(u) {
            return invalid(format!(
                "{} uses an unsafe URL scheme (allowed: http, https, mailto, tel, or a relative path)",
                field_path
            ));
        }
    }
    Ok(())
}

fn parse_media_kind(v: &Value, path: &str) -> Result<Option<MediaKind>, ContextError> {
    match opt_str(v, "media_kind", path)? {
        None => Ok(None),
        Some(s) => match MediaKind::parse(&s) {
            Some(k) => Ok(Some(k)),
            None => invalid(format!("{}.media_kind '{}' is unknown (canvas|image|video)", path, s)),
        },
    }
}

fn parse_item(v: &Value, path: &str) -> Result<Item, ContextError> {
    if v.as_object().is_none() {
        return invalid(format!("{} must be an object", path));
    }
    let shader = opt_str(v, "shader", path)?;
    if let Some(sh) = &shader {
        if !valid_shader_id(sh) {
            return invalid(format!("{}.shader must match [A-Za-z0-9_.-]{{1,64}}", path));
        }
    }
    let media_src = opt_str(v, "media_src", path)?.or(opt_str(v, "src", path)?);
    let fallback_src = opt_str(v, "fallback_src", path)?;
    let poster = opt_str(v, "poster", path)?;
    check_safe_url(&media_src, &format!("{}.media_src", path))?;
    check_safe_url(&fallback_src, &format!("{}.fallback_src", path))?;
    check_safe_url(&poster, &format!("{}.poster", path))?;
    Ok(Item {
        metric: opt_str(v, "metric", path)?,
        label: opt_str(v, "label", path)?,
        title: opt_str(v, "title", path)?,
        body: opt_str(v, "body", path)?,
        href: opt_str(v, "href", path)?,
        shader,
        media_src,
        fallback_src,
        poster,
        media_kind: parse_media_kind(v, path)?,
        aspect_ratio: parse_aspect(v, path)?,
    })
}

/// Rejects link targets that would execute script when followed. Values with no scheme (relative
/// paths, fragments) are always allowed; an absolute URL must use a safe scheme.
fn safe_href(raw: &str) -> bool {
    let s: String = raw.chars().filter(|c| !c.is_whitespace() && !c.is_control()).collect::<String>().to_lowercase();
    let scheme_end = match s.find(':') {
        Some(i) => i,
        None => return true,
    };
    // A ':' before any '/', '?' or '#' introduces a scheme.
    if s[..scheme_end].contains('/') || s[..scheme_end].contains('?') || s[..scheme_end].contains('#') {
        return true;
    }
    let scheme = &s[..scheme_end];
    !scheme.is_empty() && matches!(scheme, "http" | "https" | "mailto" | "tel")
}

fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 48
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn parse_node(v: &Value, idx: usize) -> Result<ContentNode, ContextError> {
    let path = format!("content_tree[{}]", idx);
    if v.as_object().is_none() {
        return invalid(format!("{} must be an object", path));
    }
    let role_s = match opt_str(v, "role", &path)? {
        Some(r) => r,
        None => return invalid(format!("{}.role is required", path)),
    };
    let role = match Role::parse(&role_s) {
        Some(r) => r,
        None => {
            return invalid(format!(
                "{}.role '{}' is unknown ({})",
                path, role_s, Role::ALL_NAMES
            ))
        }
    };
    let id = match opt_str(v, "id", &path)? {
        Some(id) => {
            if !valid_id(&id) {
                return invalid(format!("{}.id must match [A-Za-z0-9_-]{{1,48}}", path));
            }
            id
        }
        None => format!("s{}-{}", idx + 1, role.slug()),
    };
    let action_type = match opt_str(v, "action_type", &path)? {
        None => ActionType::None,
        Some(s) => match ActionType::parse(&s) {
            Some(a) => a,
            None => {
                return invalid(format!(
                    "{}.action_type '{}' is unknown (none|transactional_input|primary_button|secondary_link)",
                    path, s
                ))
            }
        },
    };
    let visual_anchor = match opt_str(v, "visual_anchor", &path)? {
        None => VisualAnchor::None,
        Some(s) => match VisualAnchor::parse(&s) {
            Some(a) => a,
            None => {
                return invalid(format!(
                    "{}.visual_anchor '{}' is unknown (none|monumental_type|monumental_metric|quiet|webgpu_canvas|photo|video)",
                    path, s
                ))
            }
        },
    };
    let shader = opt_str(v, "shader", &path)?;
    if let Some(sh) = &shader {
        if !valid_shader_id(sh) {
            return invalid(format!("{}.shader must match [A-Za-z0-9_.-]{{1,64}}", path));
        }
    }
    let media_src = opt_str(v, "media_src", &path)?.or(opt_str(v, "src", &path)?);
    let fallback_src = opt_str(v, "fallback_src", &path)?;
    let poster = opt_str(v, "poster", &path)?;
    check_safe_url(&media_src, &format!("{}.media_src", path))?;
    check_safe_url(&fallback_src, &format!("{}.fallback_src", path))?;
    check_safe_url(&poster, &format!("{}.poster", path))?;
    let media_kind = parse_media_kind(v, &path)?;
    let aspect_ratio = parse_aspect(v, &path)?;
    let input_type = opt_str(v, "action_input_type", &path)?;
    if let Some(t) = &input_type {
        let ok = ["email", "text", "tel", "number", "url", "search"].contains(&t.as_str());
        if !ok {
            return invalid(format!("{}.action_input_type must be email|text|tel|number|url|search", path));
        }
    }
    let mut items = Vec::new();
    match v.get("items") {
        None | Some(Value::Null) => {}
        Some(Value::Arr(a)) => {
            for (i, it) in a.iter().enumerate() {
                items.push(parse_item(it, &format!("{}.items[{}]", path, i))?);
            }
        }
        Some(_) => return invalid(format!("{}.items must be an array", path)),
    }
    for (i, it) in items.iter().enumerate() {
        check_safe_url(&it.href, &format!("{}.items[{}].href", path, i))?;
    }
    let action_href = opt_str(v, "action_href", &path)?;
    check_safe_url(&action_href, &format!("{}.action_href", path))?;
    let node = ContentNode {
        role,
        id,
        primary_claim: opt_str(v, "primary_claim", &path)?,
        supporting_text: opt_str(v, "supporting_text", &path)?,
        action_type,
        action_label: opt_str(v, "action_label", &path)?,
        action_placeholder: opt_str(v, "action_placeholder", &path)?,
        action_href,
        action_input_type: input_type,
        visual_anchor,
        shader,
        media_src,
        fallback_src,
        poster,
        media_kind,
        aspect_ratio,
        items,
    };
    validate_node(&node, &path)?;
    Ok(node)
}

fn non_blank(o: &Option<String>) -> bool {
    o.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
}

fn validate_items(n: &ContentNode, path: &str, min: usize, max: usize, what: &str) -> Result<(), ContextError> {
    if n.items.len() < min || n.items.len() > max {
        return invalid(format!("{}.items must contain {min}..{max} entries for {what}", path));
    }
    Ok(())
}

fn validate_node(n: &ContentNode, path: &str) -> Result<(), ContextError> {
    match n.role {
        Role::Hero | Role::Cta => {
            if !non_blank(&n.primary_claim) {
                return invalid(format!("{}.primary_claim is required for role {}", path, n.role.slug()));
            }
        }
        Role::Narrative => {
            if !non_blank(&n.primary_claim) && !non_blank(&n.supporting_text) {
                return invalid(format!("{}: narrative needs primary_claim or supporting_text", path));
            }
        }
        Role::Media => {
            if !non_blank(&n.primary_claim) && !non_blank(&n.shader) && !non_blank(&n.media_src) && n.items.is_empty() {
                return invalid(format!("{}: media section needs primary_claim, shader, media_src, or items", path));
            }
            validate_items(n, path, 0, MAX_MEDIA_ITEMS, "media")?;
            for (i, it) in n.items.iter().enumerate() {
                if !non_blank(&it.title) && !non_blank(&it.shader) && !non_blank(&it.media_src) {
                    return invalid(format!("{}.items[{}] needs a non-empty title, shader, or media_src", path, i));
                }
            }
        }
        Role::ProofMatrix => {
            validate_items(n, path, 1, MAX_PROOF_ITEMS, "proof_matrix")?;
            for (i, it) in n.items.iter().enumerate() {
                if !non_blank(&it.metric) || !non_blank(&it.label) {
                    return invalid(format!("{}.items[{}] needs non-empty metric and label", path, i));
                }
            }
        }
        Role::FeatureList => {
            validate_items(n, path, 1, MAX_FEATURE_ITEMS, "feature_list")?;
            for (i, it) in n.items.iter().enumerate() {
                if !non_blank(&it.title) {
                    return invalid(format!("{}.items[{}] needs a non-empty title", path, i));
                }
            }
        }
        Role::Nav | Role::Footer => {
            if !non_blank(&n.primary_claim) && n.items.is_empty() {
                return invalid(format!("{}: {} needs primary_claim or items", path, n.role.slug()));
            }
            validate_items(n, path, 0, MAX_LINK_ITEMS, n.role.slug())?;
            for (i, it) in n.items.iter().enumerate() {
                if !non_blank(&it.title) {
                    return invalid(format!("{}.items[{}] needs a non-empty title (link label)", path, i));
                }
                if !non_blank(&it.href) {
                    return invalid(format!("{}.items[{}] needs a non-empty href", path, i));
                }
            }
        }
        Role::Pricing => {
            validate_items(n, path, 1, MAX_PRICING_ITEMS, "pricing")?;
            for (i, it) in n.items.iter().enumerate() {
                if !non_blank(&it.title) {
                    return invalid(format!("{}.items[{}] needs a non-empty title (plan name)", path, i));
                }
                if !non_blank(&it.metric) {
                    return invalid(format!("{}.items[{}] needs a non-empty metric (price)", path, i));
                }
            }
        }
        Role::Testimonial => {
            validate_items(n, path, 1, MAX_QUOTE_ITEMS, "testimonial")?;
            for (i, it) in n.items.iter().enumerate() {
                if !non_blank(&it.body) {
                    return invalid(format!("{}.items[{}] needs a non-empty body (the quote)", path, i));
                }
                if !non_blank(&it.title) {
                    return invalid(format!("{}.items[{}] needs a non-empty title (the attribution)", path, i));
                }
            }
        }
        Role::Faq => {
            validate_items(n, path, 1, MAX_FAQ_ITEMS, "faq")?;
            for (i, it) in n.items.iter().enumerate() {
                if !non_blank(&it.title) {
                    return invalid(format!("{}.items[{}] needs a non-empty title (the question)", path, i));
                }
                if !non_blank(&it.body) {
                    return invalid(format!("{}.items[{}] needs a non-empty body (the answer)", path, i));
                }
            }
        }
    }
    if n.action_type != ActionType::None
        && !matches!(
            n.role,
            Role::Hero | Role::Cta | Role::Narrative | Role::Media | Role::Nav | Role::Footer | Role::Pricing
        )
    {
        return invalid(format!(
            "{}: actions are only supported on hero, cta, narrative, media, nav, footer and pricing sections",
            path
        ));
    }
    Ok(())
}

pub fn parse_context(src: &str) -> Result<Context, ContextError> {
    let root = json::parse(src)?;
    if root.as_object().is_none() {
        return invalid("root must be an object");
    }
    let intent = parse_intent(root.get("intent"))?;
    let tree = match root.get("content_tree") {
        Some(Value::Arr(a)) => a,
        _ => return invalid("content_tree must be an array"),
    };
    if tree.is_empty() || tree.len() > MAX_SECTIONS {
        return invalid(format!("content_tree must contain 1..{} sections", MAX_SECTIONS));
    }
    let mut nodes: Vec<ContentNode> = Vec::new();
    for (i, v) in tree.iter().enumerate() {
        let n = parse_node(v, i)?;
        if nodes.iter().any(|o| o.id == n.id) {
            return invalid(format!("duplicate section id '{}'", n.id));
        }
        nodes.push(n);
    }
    Ok(Context { intent, content_tree: nodes, canonical: root.to_json() })
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// ---------------------------------------------------------------------------
// Phase 1: Semantic Profiler & Continuous Aesthetic Modality Calculus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToneVector {
    pub precision: f64,
    pub warmth: f64,
    pub drama: f64,
    pub formality: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontPersonality {
    Sans,
    Serif,
    Mono,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HueSource {
    Override,
    Semantic(Vec<String>),
    Hash,
}

/// Continuous aesthetic modality calculus derived from domain physics (zero templates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModalityProfile {
    pub noisy_gradient_score: f64,
    pub grain_frequency: f64,
    pub grain_opacity: f64,
    pub webgpu_shader_score: f64,
    pub spring_omega: f64,
    pub hover_ms: u16,
    pub svg_icon_score: f64,
    pub svg_stroke_px: f64,
    pub svg_sharp_joins: bool,
}

/// Physical parameters of the composition. Every downstream phase reads only this.
#[derive(Debug, Clone)]
pub struct Physics {
    pub tone: ToneVector,
    pub personality: FontPersonality,
    pub theme_dark: bool,
    pub hue: f64,
    pub hue_source: HueSource,
    /// Harmonic ratio r in S_n = S_0 * r^n; one of {1.250, 1.333, 1.414, 1.618}.
    pub scale_ratio: f64,
    /// Base grid unit in px (4 or 8). Every spacing value is quantized to it.
    pub base_unit: f64,
    pub display_max_rem: f64,
    pub display_weight: u16,
    /// Body measure in `ch`.
    pub measure_ch: f64,
    pub radius_px: f64,
    pub density: f64,
    pub asymmetry: f64,
    pub chromatic_energy: f64,
    /// Max allowed horizontal eccentricity of the visual centre of mass (fraction of page width).
    pub max_eccentricity: f64,
    /// Derived sensory modalities (noisy gradient, WebGPU shader, micro-animation, SVG iconography).
    pub modalities: ModalityProfile,
}

// [precision, warmth, drama, formality]
const TONE_TABLE: &[(&str, [f64; 4])] = &[
    ("assurance", [0.90, 0.15, 0.25, 0.85]),
    ("technical", [0.85, 0.20, 0.30, 0.60]),
    ("institutional", [0.70, 0.30, 0.20, 0.90]),
    ("corporate", [0.60, 0.30, 0.20, 0.90]),
    ("serious", [0.70, 0.10, 0.30, 0.80]),
    ("precise", [0.95, 0.10, 0.20, 0.70]),
    ("minimal", [0.70, 0.30, 0.10, 0.60]),
    ("calm", [0.50, 0.60, 0.10, 0.50]),
    ("editorial", [0.40, 0.50, 0.70, 0.60]),
    ("luxury", [0.50, 0.30, 0.55, 0.90]),
    ("elegant", [0.45, 0.40, 0.45, 0.80]),
    ("playful", [0.20, 0.90, 0.70, 0.10]),
    ("friendly", [0.30, 0.90, 0.40, 0.20]),
    ("warm", [0.30, 0.90, 0.35, 0.30]),
    ("bold", [0.40, 0.40, 0.95, 0.40]),
    ("energetic", [0.30, 0.70, 0.90, 0.20]),
    ("creative", [0.30, 0.60, 0.80, 0.30]),
    ("brutalist", [0.60, 0.10, 1.00, 0.20]),
    ("kinetic", [0.75, 0.25, 0.85, 0.45]),
    ("spatial", [0.80, 0.20, 0.75, 0.55]),
    ("cinematic", [0.65, 0.35, 0.90, 0.65]),
];

const NEUTRAL_TONE: [f64; 4] = [0.50, 0.40, 0.40, 0.50];

const DOMAIN_TABLE: &[(&str, f64)] = &[
    ("fintech", 255.0),
    ("finance", 255.0),
    ("bank", 250.0),
    ("payment", 258.0),
    ("insurance", 235.0),
    ("legal", 250.0),
    ("security", 275.0),
    ("crypto", 295.0),
    ("infrastructure", 262.0),
    ("devtools", 270.0),
    ("developer", 270.0),
    ("cloud", 245.0),
    ("ai", 290.0),
    ("saas", 265.0),
    ("health", 175.0),
    ("medical", 190.0),
    ("wellness", 160.0),
    ("bio", 165.0),
    ("climate", 150.0),
    ("energy", 120.0),
    ("sustainab", 145.0),
    ("agri", 130.0),
    ("education", 85.0),
    ("food", 55.0),
    ("restaurant", 45.0),
    ("fashion", 350.0),
    ("retail", 25.0),
    ("commerce", 35.0),
    ("travel", 215.0),
    ("hospitality", 70.0),
    ("gaming", 310.0),
    ("entertainment", 325.0),
    ("media", 20.0),
    ("art", 340.0),
    ("design", 330.0),
    ("music", 305.0),
    ("estate", 200.0),
    ("logistics", 60.0),
    ("industrial", 50.0),
    ("graphics", 280.0),
    ("shader", 285.0),
    ("hardware", 210.0),
    ("robotics", 220.0),
    ("quantum", 260.0),
];

fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

fn key_matches(token: &str, key: &str) -> bool {
    token == key || (key.len() >= 4 && token.starts_with(key))
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn round_to_100(x: f64) -> u16 {
    ((x / 100.0).round() * 100.0).clamp(100.0, 900.0) as u16
}

pub fn profile_tone(tone: &str) -> (ToneVector, FontPersonality) {
    let toks = tokens(tone);
    let mut sum = [0.0f64; 4];
    let mut count = 0.0f64;
    for (key, vec) in TONE_TABLE {
        if toks.iter().any(|t| key_matches(t, key)) {
            for i in 0..4 {
                sum[i] += vec[i];
            }
            count += 1.0;
        }
    }
    let v = if count > 0.0 {
        [sum[0] / count, sum[1] / count, sum[2] / count, sum[3] / count]
    } else {
        NEUTRAL_TONE
    };
    let has = |k: &str| toks.iter().any(|t| key_matches(t, k));
    let personality = if has("brutalist") {
        FontPersonality::Mono
    } else if has("editorial") || has("luxury") || has("elegant") {
        FontPersonality::Serif
    } else {
        FontPersonality::Sans
    };
    (
        ToneVector { precision: v[0], warmth: v[1], drama: v[2], formality: v[3] },
        personality,
    )
}

pub fn profile_hue(domain: &str, override_hue: Option<f64>) -> (f64, HueSource) {
    if let Some(h) = override_hue {
        return (h, HueSource::Override);
    }
    let toks = tokens(domain);
    let mut matched: Vec<(String, f64)> = Vec::new();
    for (key, hue) in DOMAIN_TABLE {
        if toks.iter().any(|t| key_matches(t, key)) {
            matched.push((key.to_string(), *hue));
        }
    }
    if matched.is_empty() {
        let h = fnv1a64(domain.trim().to_lowercase().as_bytes());
        let hue = (h % 3600) as f64 / 10.0;
        return (hue, HueSource::Hash);
    }
    // circular mean
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    for (_, h) in &matched {
        sx += h.to_radians().cos();
        sy += h.to_radians().sin();
    }
    let hue = sy.atan2(sx).to_degrees().rem_euclid(360.0);
    let keys = matched.into_iter().map(|(k, _)| k).collect();
    (hue, HueSource::Semantic(keys))
}

pub fn profile_modalities(tone: &ToneVector, density: f64, energy: f64, theme_dark: bool, radius_px: f64) -> ModalityProfile {
    let p = tone.precision.clamp(0.0, 1.0);
    let w = tone.warmth.clamp(0.0, 1.0);
    let d = tone.drama.clamp(0.0, 1.0);
    let e = energy.clamp(0.0, 1.0);
    let den = density.clamp(0.0, 1.0);

    let noisy_gradient_score = (0.45 * (d * e * 2.2) + 0.30 * w + (if theme_dark { 0.25 } else { 0.12 }) * (1.0 - den * 0.4)).clamp(0.0, 1.0);
    let grain_frequency = ((0.60 + 0.30 * p - 0.10 * w) * 100.0).round() / 100.0;
    let raw_op = if theme_dark { 0.024 + 0.018 * w + 0.008 * d } else { 0.028 + 0.022 * w };
    let grain_opacity = (raw_op * 1000.0).round() / 1000.0;

    let webgpu_shader_score = (0.55 * d + 0.25 * p.max(e) + 0.20 * (1.0 - w * 0.6) - 0.15 * (den - 0.75).max(0.0)).clamp(0.0, 1.0);
    let spring_omega = ((14.0 + 8.0 * p - 4.0 * w) * 10.0).round() / 10.0;
    let hover_ms = (180.0 - 50.0 * p + 30.0 * w).round().clamp(100.0, 260.0) as u16;
    let svg_icon_score = (0.50 * p + 0.30 * den + 0.20 * d).clamp(0.0, 1.0);
    let svg_stroke_px = ((1.25 + 0.50 * d + 0.25 * p) * 20.0).round() / 20.0;
    let svg_sharp_joins = radius_px <= 5.0;

    ModalityProfile {
        noisy_gradient_score,
        grain_frequency,
        grain_opacity,
        webgpu_shader_score,
        spring_omega,
        hover_ms,
        svg_icon_score,
        svg_stroke_px,
        svg_sharp_joins,
    }
}

pub fn profile(intent: &Intent, max_eccentricity: f64) -> Physics {
    let (tone, personality) = profile_tone(&intent.tone);
    let (hue, hue_source) = profile_hue(&intent.domain, intent.hue);

    let dark_score = 0.5 * tone.precision + 0.3 * (1.0 - tone.warmth) + 0.2 * tone.drama;
    let theme_dark = match intent.theme {
        ThemePref::Dark => true,
        ThemePref::Light => false,
        ThemePref::Auto => dark_score >= 0.60,
    };

    let scale_ratio = if tone.drama < 0.30 {
        1.250
    } else if tone.drama < 0.55 {
        1.333
    } else if tone.drama < 0.80 {
        1.414
    } else {
        1.618
    };
    let base_unit = if intent.density >= 0.5 { 4.0 } else { 8.0 };
    let display_max_rem = lerp(4.5, 7.5, tone.drama);
    let display_weight = round_to_100(300.0 + 400.0 * tone.drama + 100.0 * tone.formality).clamp(300, 700);
    let measure_ch = lerp(78.0, 42.0, intent.density);
    let radius_px = (2.0 + 14.0 * tone.warmth).round();
    let modalities = profile_modalities(&tone, intent.density, intent.chromatic_energy, theme_dark, radius_px);

    Physics {
        tone,
        personality,
        theme_dark,
        hue,
        hue_source,
        scale_ratio,
        base_unit,
        display_max_rem,
        display_weight,
        measure_ch,
        radius_px,
        density: intent.density,
        asymmetry: intent.asymmetry,
        chromatic_energy: intent.chromatic_energy,
        max_eccentricity,
        modalities,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiler_fintech() {
        let (tone, p) = profile_tone("high_assurance_technical");
        assert!(tone.precision > 0.8);
        assert_eq!(p, FontPersonality::Sans);
        let (hue, src) = profile_hue("fintech_infrastructure", None);
        assert!(hue > 250.0 && hue < 270.0);
        assert!(matches!(src, HueSource::Semantic(_)));
    }

    #[test]
    fn modality_calculus_responds_to_domain_vectors() {
        let i_tech = Intent {
            domain: "graphics_shader".into(),
            tone: "kinetic_spatial_precise".into(),
            density: 0.65,
            asymmetry: 0.6,
            chromatic_energy: 0.45,
            theme: ThemePref::Dark,
            seed: None,
            hue: None,
            lang: "en".into(),
            title: None,
        };
        let p = profile(&i_tech, 0.15);
        assert!(p.modalities.webgpu_shader_score > 0.55);
        assert!(p.modalities.noisy_gradient_score > 0.35);
        assert!(p.modalities.svg_icon_score > 0.50);
    }

    #[test]
    fn hash_hue_is_deterministic() {
        let a = profile_hue("zzzz-unknown-domain", None);
        let b = profile_hue("zzzz-unknown-domain", None);
        assert_eq!(a.0, b.0);
        assert_eq!(a.1, HueSource::Hash);
    }

    #[test]
    fn parses_minimal_context() {
        let src = r#"{"intent":{"domain":"fintech","tone":"calm"},"content_tree":[{"role":"hero","primary_claim":"Hi"}]}"#;
        let c = parse_context(src).unwrap();
        assert_eq!(c.content_tree.len(), 1);
        assert_eq!(c.content_tree[0].id, "s1-hero");
    }

    #[test]
    fn parses_media_photo_and_video() {
        let src = r#"{"content_tree":[{"role":"media","primary_claim":"Reel","media_src":"/assets/hero.mp4","poster":"/assets/poster.avif","aspect_ratio":1.777}]}"#;
        let c = parse_context(src).unwrap();
        assert_eq!(c.content_tree[0].role, Role::Media);
        assert_eq!(c.content_tree[0].media_src.as_deref(), Some("/assets/hero.mp4"));
    }

    #[test]
    fn rejects_bad_role_and_missing_items() {
        assert!(parse_context(r#"{"content_tree":[{"role":"nope"}]}"#).is_err());
        assert!(parse_context(r#"{"content_tree":[{"role":"proof_matrix"}]}"#).is_err());
    }
}
