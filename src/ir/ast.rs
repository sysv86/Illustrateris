//! Typed layout IR. The codegen can only emit what this tree can express: tags come from closed
//! enums and text is escaped at the single point where it is written, so output validity follows
//! from the types plus the escaping function, not from template discipline.

use super::token::{ComputedTypography, DynamicStep};
use crate::color::oklch::Oklch;
use crate::context::{ActionType, MediaKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
    Baseline,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Row,
    Column,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridTrack {
    Auto,
    Fr(f64),
    /// `minmax(min-content, Nfr)`: proportional, but never smaller than the content.
    MinContentFr(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerTag {
    Div,
    Dl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexTag {
    Div,
    Article,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlTag {
    H1,
    H2,
    H3,
    P,
    Span,
    Dt,
    Dd,
    Blockquote,
    Cite,
    Figcaption,
}

impl HtmlTag {
    pub fn name(self) -> &'static str {
        match self {
            HtmlTag::H1 => "h1",
            HtmlTag::H2 => "h2",
            HtmlTag::H3 => "h3",
            HtmlTag::P => "p",
            HtmlTag::Span => "span",
            HtmlTag::Dt => "dt",
            HtmlTag::Dd => "dd",
            HtmlTag::Blockquote => "blockquote",
            HtmlTag::Cite => "cite",
            HtmlTag::Figcaption => "figcaption",
        }
    }
}

/// 1-based CSS grid lines on the 12-column desktop grid. On small screens every child spans all columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridPlacement {
    pub col_start: u32,
    pub col_span: u32,
    pub row_start: u32,
    pub row_span: u32,
    pub align_self: Alignment,
}

/// Perceptual OKLCH atmospheric light field anchored at the section's visual mass centroid `(cx, cy)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtmosphereStyle {
    pub focal_x_pct: f64,
    pub focal_y_pct: f64,
    pub glow: Oklch,
    pub glow_alpha: f64,
}

/// Tactile card surface with 1px structural border, radius, and micro-interaction hover lift.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardSurface {
    pub bg: Oklch,
    pub border: Oklch,
    pub hover_border: Oklch,
    pub radius_px: f64,
    pub pad: DynamicStep,
    pub hover_ms: u16,
}

#[derive(Debug, Clone)]
pub struct SectionStyle {
    pub bg: Oklch,
    pub fg: Oklch,
    pub border_top: Option<Oklch>,
    pub pad_x: DynamicStep,
    pub pad_y: DynamicStep,
    pub min_height_dvh: Option<u32>,
    pub font_family: String,
    pub atmosphere: Option<AtmosphereStyle>,
}

#[derive(Debug, Clone)]
pub struct ActionStyle {
    pub accent: Oklch,
    pub accent_text: Oklch,
    pub field_bg: Oklch,
    pub field_fg: Oklch,
    pub field_border: Oklch,
    pub placeholder: Oklch,
    pub ring: Oklch,
    pub radius_px: f64,
    pub pad_x: DynamicStep,
    pub pad_y: DynamicStep,
    pub hover_ms: u16,
    pub button_typo: ComputedTypography,
    pub input_typo: ComputedTypography,
}

#[derive(Debug, Clone)]
pub struct CanvasStyle {
    pub surface: Oklch,
    pub border: Oklch,
    pub accent: Oklch,
    pub radius_px: f64,
    pub aspect_ratio: f64,
}

#[allow(clippy::large_enum_variant)] // IR nodes are allocated per element, never in a flat array; boxing a variant would churn every constructor and match site.
#[derive(Debug, Clone)]
pub enum LayoutNode {
    Section {
        id: String,
        /// Id of the heading element that labels the section (aria-labelledby).
        labelled_by: Option<String>,
        style: SectionStyle,
        children: Vec<LayoutNode>,
    },
    FlexRow {
        tag: FlexTag,
        direction: Direction,
        alignment: Alignment,
        spacing: DynamicStep,
        card: Option<CardSurface>,
        children: Vec<LayoutNode>,
    },
    AsymmetricGrid {
        tag: ContainerTag,
        columns: u32,
        tracks: Vec<GridTrack>,
        gap: DynamicStep,
        row_gap: DynamicStep,
        children: Vec<(GridPlacement, LayoutNode)>,
    },
    TextLeaf {
        id: Option<String>,
        tag: HtmlTag,
        value: String,
        typography: ComputedTypography,
        /// Reading measure cap in `ch`.
        max_ch: Option<f64>,
    },
    /// A text link. `href` has already been validated by the context parser (no script schemes).
    LinkLeaf {
        href: String,
        label: String,
        typography: ComputedTypography,
    },
    ActionLeaf {
        id: String,
        kind: ActionType,
        label: String,
        placeholder: String,
        href: String,
        input_type: String,
        style: ActionStyle,
    },
    /// An accessible WebGPU / shader `<figure><canvas>` container with optional `<figcaption>`.
    CanvasLeaf {
        id: String,
        shader: String,
        label: String,
        caption: Option<String>,
        caption_typo: ComputedTypography,
        style: CanvasStyle,
    },
    /// A responsive photograph (`<picture>/<img>`) or muted video loop (`<video>`) with zero CLS.
    MediaLeaf {
        id: String,
        kind: MediaKind,
        src: String,
        fallback_src: Option<String>,
        poster: Option<String>,
        alt: String,
        caption: Option<String>,
        caption_typo: ComputedTypography,
        priority_eager: bool,
        style: CanvasStyle,
    },
}

#[derive(Debug, Clone)]
pub struct DocumentTokens {
    pub bg: Oklch,
    pub surface: Oklch,
    pub border: Oklch,
    pub text: Oklch,
    pub text_secondary: Oklch,
    pub accent: Oklch,
    pub accent_text: Oklch,
    pub font_family: String,
    pub radius_px: f64,
    pub grain_frequency: f64,
    pub grain_opacity: f64,
    pub emit_grain: bool,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub lang: String,
    pub title: String,
    pub dark: bool,
    pub page_bg: Oklch,
    pub tokens: DocumentTokens,
    pub sections: Vec<LayoutNode>,
    pub generator: String,
}
