//! IR style structs -> plain-CSS declaration blocks (the non-Tailwind backend).
//!
//! Every declaration list produced here is the exact equivalent of the Tailwind utilities in
//! `tailwind.rs`, so the two backends render identically. Values use the same tokens (fluid
//! `clamp()`, quantized spacing, OKLCH colours) but with real spaces and no utility syntax.

use super::backend::{Breakpoint, Style};
use crate::color::oklch::trim_num;
use crate::ir::ast::*;
use crate::ir::token::{ComputedTypography, DynamicStep};

/// Font stacks carry underscores where Tailwind needs spaces; plain CSS wants the real spaces.
fn css_font_family(stack: &str) -> String {
    stack.replace('_', " ")
}

fn decl(prop: &str, value: &str) -> String {
    format!("{}:{};", prop, value)
}

pub fn typography_decls(t: &ComputedTypography, with_color: bool) -> String {
    let mut s = String::new();
    s.push_str(&decl("font-size", &t.size.css(false)));
    s.push_str(&decl("font-weight", &t.weight.to_string()));
    s.push_str(&decl("line-height", &trim_num(t.line_height, 3)));
    s.push_str(&decl("letter-spacing", &format!("{}em", trim_num(t.tracking_em, 3))));
    if with_color {
        s.push_str(&decl("color", &t.color.css()));
    }
    if t.uppercase {
        s.push_str(&decl("text-transform", "uppercase"));
    }
    if t.tabular {
        s.push_str(&decl("font-variant-numeric", "tabular-nums"));
    }
    if t.balance {
        s.push_str(&decl("text-wrap", "balance"));
    }
    s
}

pub fn measure_decls(max_ch: Option<f64>) -> String {
    match max_ch {
        Some(ch) => decl("max-width", &format!("{}ch", trim_num(ch, 1))),
        None => String::new(),
    }
}

/// Backend-neutral style for a text element: typography (+ optional colour) and measure cap.
pub fn typography_style(t: &ComputedTypography, with_color: bool) -> Style {
    let mut st = Style::tw(super::tailwind::typography_classes(t, with_color));
    st.css(typography_decls(t, with_color));
    st
}

pub fn measure_style(max_ch: Option<f64>) -> Style {
    let mut st = Style::tw(super::tailwind::measure_class(max_ch));
    st.css(measure_decls(max_ch));
    st
}

pub fn section_style(s: &SectionStyle) -> Style {
    let mut st = Style::tw(super::tailwind::section_classes(s));
    st.css(section_decls(s));
    st
}

pub fn section_decls(s: &SectionStyle) -> String {
    let mut out = String::new();
    out.push_str("position:relative;width:100%;overflow:hidden;display:flex;flex-direction:column;");
    out.push_str(&decl("font-family", &css_font_family(&s.font_family)));
    if let Some(atm) = &s.atmosphere {
        out.push_str(&decl("background-color", &s.bg.css()));
        out.push_str(&decl(
            "background-image",
            &format!(
                "radial-gradient(115% 90% at {}% {}%, {} 0%, {} 58%, transparent 100%)",
                trim_num(atm.focal_x_pct, 1),
                trim_num(atm.focal_y_pct, 1),
                atm.glow.css(),
                s.bg.css()
            ),
        ));
    } else {
        out.push_str(&decl("background", &s.bg.css()));
    }
    out.push_str(&decl("color", &s.fg.css()));
    out.push_str(&decl("padding", &format!("{} {}", s.pad_y.css(false), s.pad_x.css(false))));
    if let Some(h) = s.min_height_dvh {
        out.push_str(&decl("min-height", &format!("{}dvh", h)));
    }
    if let Some(b) = &s.border_top {
        out.push_str(&format!("border-top:1px solid {};", b.css()));
    }
    out
}

fn track_css(t: &GridTrack) -> String {
    match t {
        GridTrack::Auto => "auto".to_string(),
        GridTrack::Fr(f) => format!("minmax(0,{}fr)", trim_num(*f, 2)),
        GridTrack::MinContentFr(f) => format!("minmax(min-content,{}fr)", trim_num(*f, 2)),
    }
}

pub fn grid_style(tag: ContainerTag, tracks: &[GridTrack], gap: &DynamicStep, row_gap: &DynamicStep) -> Style {
    let mut st = Style::tw(super::tailwind::grid_classes(tag, tracks, gap, row_gap));
    st.css("display:grid;grid-template-columns:repeat(12,minmax(0,1fr));flex:1 1 0%;");
    st.css(decl("column-gap", &gap.css(false)));
    st.css(decl("row-gap", &row_gap.css(false)));
    if tag == ContainerTag::Dl {
        st.css("margin:0;");
    }
    if !tracks.is_empty() {
        let tpl: Vec<String> = tracks.iter().map(track_css).collect();
        st.at(Breakpoint::Lg, decl("grid-template-rows", &tpl.join(" ")));
    }
    st
}

fn align_value(a: Alignment) -> &'static str {
    match a {
        Alignment::Start => "flex-start",
        Alignment::Center => "center",
        Alignment::End => "flex-end",
        Alignment::Baseline => "baseline",
        Alignment::Stretch => "stretch",
    }
}

pub fn placement_style(p: &GridPlacement) -> Style {
    let mut st = Style::tw(super::tailwind::placement_classes(p));
    // Mobile first: every child spans the full 12 columns.
    st.css("grid-column:1 / -1;");
    st.at(
        Breakpoint::Lg,
        format!(
            "grid-column:{} / span {};grid-row:{} / span {};align-self:{};",
            p.col_start,
            p.col_span,
            p.row_start,
            p.row_span,
            align_value(p.align_self)
        ),
    );
    st
}

pub fn flex_decls(direction: Direction, alignment: Alignment, spacing: &DynamicStep, card: Option<&CardSurface>) -> String {
    let dir = match direction {
        Direction::Row => "row",
        Direction::Column => "column",
        Direction::ColumnReverse => "column-reverse",
    };
    let mut s = String::new();
    s.push_str("display:flex;");
    s.push_str(&decl("flex-direction", dir));
    s.push_str(&decl("align-items", align_value(alignment)));
    s.push_str(&decl("gap", &spacing.css(false)));
    if let Some(c) = card {
        s.push_str(&decl("background-color", &c.bg.css()));
        s.push_str(&format!("border:1px solid {};", c.border.css()));
        s.push_str(&decl("border-radius", &format!("{}px", trim_num(c.radius_px, 1))));
        s.push_str(&decl("padding", &c.pad.css(false)));
        s.push_str(&format!(
            "transition:transform {}ms cubic-bezier(0.22,1,0.36,1),border-color {}ms cubic-bezier(0.22,1,0.36,1);",
            c.hover_ms, c.hover_ms
        ));
    }
    s
}

pub fn flex_style(direction: Direction, alignment: Alignment, spacing: &DynamicStep, card: Option<&CardSurface>) -> Style {
    let mut st = Style::tw(super::tailwind::flex_classes(direction, alignment, spacing, card));
    st.css(flex_decls(direction, alignment, spacing, card));
    if let Some(c) = card {
        st.on_hover(format!("transform:translate3d(0,-2px,0);border-color:{};", c.hover_border.css()));
    }
    st
}

fn focus_ring_decls(s: &ActionStyle) -> String {
    format!("outline:2px solid {};outline-offset:2px;", s.ring.css())
}

/// Form controls do not inherit `font-family` from their ancestors in any browser, so every
/// interactive control sets it explicitly from its own typography.
fn family_decl(t: &ComputedTypography) -> String {
    decl("font-family", &css_font_family(&t.font_family))
}

pub fn button_style(s: &ActionStyle) -> Style {
    let mut st = Style::tw(super::tailwind::button_classes(s));
    let mut out = String::new();
    out.push_str("display:inline-flex;align-items:center;justify-content:center;white-space:nowrap;");
    out.push_str("user-select:none;text-decoration:none;");
    out.push_str(&family_decl(&s.button_typo));
    out.push_str(&typography_decls(&s.button_typo, false));
    out.push_str(&decl("background-color", &s.accent.css()));
    out.push_str(&decl("color", &s.accent_text.css()));
    out.push_str(&decl("padding", &format!("{} {}", s.pad_y.css(false), s.pad_x.css(false))));
    out.push_str(&decl("border-radius", &format!("{}px", trim_num(s.radius_px, 1))));
    out.push_str(&format!(
        "transition:filter {}ms cubic-bezier(0.22,1,0.36,1),transform {}ms cubic-bezier(0.22,1,0.36,1);",
        s.hover_ms, s.hover_ms
    ));
    st.css(out);
    st.on_hover("filter:brightness(1.1);transform:translate3d(0,-1px,0);");
    st.focus(focus_ring_decls(s));
    st
}

pub fn input_style(s: &ActionStyle) -> Style {
    let mut st = Style::tw(super::tailwind::input_classes(s));
    let mut out = String::new();
    out.push_str("min-width:0;width:100%;flex:1 1 0%;border-width:1px;border-style:solid;");
    out.push_str(&family_decl(&s.input_typo));
    out.push_str(&typography_decls(&s.input_typo, false));
    out.push_str(&decl("background-color", &s.field_bg.css()));
    out.push_str(&decl("color", &s.field_fg.css()));
    out.push_str(&decl("border-color", &s.field_border.css()));
    out.push_str(&decl("padding", &format!("{} {}", s.pad_y.css(false), s.pad_x.css(false))));
    out.push_str(&decl("border-radius", &format!("{}px", trim_num(s.radius_px, 1))));
    st.css(out);
    st.focus(focus_ring_decls(s));
    st
}

pub fn form_style() -> Style {
    let mut st = Style::tw(super::tailwind::form_classes());
    st.css("display:flex;width:100%;flex-direction:column;gap:0.75rem;");
    st.at(Breakpoint::Sm, "flex-direction:row;");
    st
}

pub fn link_style(s: &ActionStyle) -> Style {
    let mut st = Style::tw(super::tailwind::link_classes(s));
    let mut out = String::new();
    out.push_str("display:inline-flex;align-items:center;gap:0.5rem;");
    out.push_str("text-decoration-line:underline;text-underline-offset:4px;");
    out.push_str(&family_decl(&s.button_typo));
    out.push_str(&typography_decls(&s.button_typo, false));
    out.push_str(&decl("color", &s.field_fg.css()));
    out.push_str(&decl("text-decoration-color", &s.accent.css()));
    out.push_str(&format!("transition:opacity {}ms cubic-bezier(0.22,1,0.36,1);", s.hover_ms));
    st.css(out);
    st.on_hover("opacity:0.85;");
    st.focus(focus_ring_decls(s));
    st
}

/// Inline link styling (`nav` / `footer` links).
pub fn nav_link_style(t: &ComputedTypography) -> Style {
    let mut st = Style::tw(super::tailwind::nav_link_classes(t));
    let mut out = family_decl(t);
    out.push_str(&typography_decls(t, true));
    out.push_str("text-decoration:none;transition:opacity 150ms ease;");
    st.css(out);
    st.on_hover("opacity:0.8;");
    st.focus(format!("outline:2px solid {};outline-offset:2px;", t.color.css()));
    st
}

pub fn canvas_figure_style() -> Style {
    let mut st = Style::tw(super::tailwind::canvas_figure_classes());
    st.css("display:flex;flex-direction:column;gap:0.5rem;width:100%;margin:0;");
    st
}

pub fn canvas_surface_style(s: &CanvasStyle) -> Style {
    let mut st = Style::tw(super::tailwind::canvas_surface_classes(s));
    let mut out = String::new();
    out.push_str("display:block;width:100%;border-width:1px;border-style:solid;overflow:hidden;");
    out.push_str(&decl("aspect-ratio", &trim_num(s.aspect_ratio, 3)));
    out.push_str(&decl("background-color", &s.surface.css()));
    out.push_str(&decl("border-color", &s.border.css()));
    out.push_str(&decl("border-radius", &format!("{}px", trim_num(s.radius_px, 1))));
    st.css(out);
    st
}
