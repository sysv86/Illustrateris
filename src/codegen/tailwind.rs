//! IR style structs -> Tailwind utility class strings.
//!
//! Arbitrary values always carry an explicit type hint (`text-[length:...]`, `bg-[color:...]`) because
//! `text-[...]` and `border-[...]` are ambiguous between size and colour. Spaces inside arbitrary
//! values are underscores. Output targets Tailwind v3.4+ and v4.

use crate::color::oklch::trim_num;
use crate::ir::ast::*;
use crate::ir::token::{weight_class, ComputedTypography, DynamicStep};

pub fn join(parts: &[String]) -> String {
    parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join(" ")
}

pub fn typography_classes(t: &ComputedTypography, with_color: bool) -> String {
    let mut v: Vec<String> = vec![
        format!("text-[length:{}]", t.font_size_clamp),
        weight_class(t.weight).to_string(),
        format!("leading-[{}]", trim_num(t.line_height, 3)),
        format!("tracking-[{}em]", trim_num(t.tracking_em, 3)),
    ];
    if with_color {
        v.push(format!("text-[color:{}]", t.color.css_tw()));
    }
    if t.uppercase {
        v.push("uppercase".to_string());
    }
    if t.tabular {
        v.push("tabular-nums".to_string());
    }
    if t.balance {
        v.push("text-balance".to_string());
    }
    join(&v)
}

pub fn section_classes(s: &SectionStyle) -> String {
    let mut v: Vec<String> = vec![
        "relative".into(),
        "w-full".into(),
        "overflow-hidden".into(),
        "flex".into(),
        "flex-col".into(),
        format!("font-[family-name:{}]", s.font_family),
        format!("bg-[color:{}]", s.bg.css_tw()),
        format!("text-[color:{}]", s.fg.css_tw()),
        format!("px-[{}]", s.pad_x.css(true)),
        format!("py-[{}]", s.pad_y.css(true)),
    ];
    if let Some(h) = s.min_height_dvh {
        v.push(format!("min-h-[{}dvh]", h));
    }
    if let Some(b) = &s.border_top {
        v.push("border-t".into());
        v.push(format!("border-[color:{}]", b.css_tw()));
    }
    join(&v)
}

fn track_css(t: &GridTrack) -> String {
    match t {
        GridTrack::Auto => "auto".to_string(),
        GridTrack::Fr(f) => format!("minmax(0,{}fr)", trim_num(*f, 2)),
        GridTrack::MinContentFr(f) => format!("minmax(min-content,{}fr)", trim_num(*f, 2)),
    }
}

pub fn grid_classes(tag: ContainerTag, tracks: &[GridTrack], gap: &DynamicStep, row_gap: &DynamicStep) -> String {
    let mut v: Vec<String> = vec![
        "grid".into(),
        "grid-cols-12".into(),
        "flex-1".into(),
        format!("gap-x-[{}]", gap.css(true)),
        format!("gap-y-[{}]", row_gap.css(true)),
    ];
    if tag == ContainerTag::Dl {
        v.push("m-0".into());
    }
    if !tracks.is_empty() {
        let tpl: Vec<String> = tracks.iter().map(track_css).collect();
        v.push(format!("lg:grid-rows-[{}]", tpl.join("_")));
    }
    join(&v)
}

fn indexed(prefix: &str, n: u32, max_named: u32) -> String {
    if n <= max_named {
        format!("{}-{}", prefix, n)
    } else {
        format!("{}-[{}]", prefix, n)
    }
}

pub fn placement_classes(p: &GridPlacement) -> String {
    let align = match p.align_self {
        Alignment::Start => "lg:self-start",
        Alignment::Center => "lg:self-center",
        Alignment::End => "lg:self-end",
        Alignment::Baseline => "lg:self-baseline",
        Alignment::Stretch => "lg:self-stretch",
    };
    let v: Vec<String> = vec![
        "col-span-12".to_string(),
        format!("lg:{}", indexed("col-start", p.col_start, 13)),
        format!("lg:{}", indexed("col-span", p.col_span, 12)),
        format!("lg:{}", indexed("row-start", p.row_start, 7)),
        format!("lg:{}", indexed("row-span", p.row_span, 6)),
        align.to_string(),
    ];
    join(&v)
}

pub fn flex_classes(direction: Direction, alignment: Alignment, spacing: &DynamicStep, card: Option<&CardSurface>) -> String {
    let dir = match direction {
        Direction::Row => "flex-row",
        Direction::Column => "flex-col",
        Direction::ColumnReverse => "flex-col-reverse",
    };
    let items = match alignment {
        Alignment::Start => "items-start",
        Alignment::Center => "items-center",
        Alignment::End => "items-end",
        Alignment::Baseline => "items-baseline",
        Alignment::Stretch => "items-stretch",
    };
    let mut parts = vec![
        "flex".to_string(),
        dir.to_string(),
        items.to_string(),
        format!("gap-[{}]", spacing.css(true)),
    ];
    if let Some(c) = card {
        parts.push("border".to_string());
        parts.push(format!("bg-[color:{}]", c.bg.css_tw()));
        parts.push(format!("border-[color:{}]", c.border.css_tw()));
        parts.push(format!("hover:border-[color:{}]", c.hover_border.css_tw()));
        parts.push(format!("rounded-[{}px]", trim_num(c.radius_px, 1)));
        parts.push(format!("p-[{}]", c.pad.css(true)));
        parts.push("transition-transform hover:-translate-y-0.5".to_string());
    }
    join(&parts)
}

pub fn measure_class(max_ch: Option<f64>) -> String {
    match max_ch {
        Some(ch) => format!("max-w-[{}ch]", trim_num(ch, 1)),
        None => String::new(),
    }
}

fn focus_ring(s: &ActionStyle) -> String {
    format!(
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[color:{}]",
        s.ring.css_tw()
    )
}

pub fn button_classes(s: &ActionStyle) -> String {
    join(&[
        "inline-flex items-center justify-center whitespace-nowrap select-none no-underline".to_string(),
        format!("font-[family-name:{}]", s.button_typo.font_family),
        typography_classes(&s.button_typo, false),
        format!("bg-[color:{}]", s.accent.css_tw()),
        format!("text-[color:{}]", s.accent_text.css_tw()),
        format!("px-[{}]", s.pad_x.css(true)),
        format!("py-[{}]", s.pad_y.css(true)),
        format!("rounded-[{}px]", trim_num(s.radius_px, 1)),
        "transition-[filter] hover:brightness-110".to_string(),
        focus_ring(s),
    ])
}

pub fn input_classes(s: &ActionStyle) -> String {
    join(&[
        "min-w-0 w-full flex-1 border".to_string(),
        format!("font-[family-name:{}]", s.input_typo.font_family),
        typography_classes(&s.input_typo, false),
        format!("bg-[color:{}]", s.field_bg.css_tw()),
        format!("text-[color:{}]", s.field_fg.css_tw()),
        format!("placeholder:text-[color:{}]", s.placeholder.css_tw()),
        format!("border-[color:{}]", s.field_border.css_tw()),
        format!("px-[{}]", s.pad_x.css(true)),
        format!("py-[{}]", s.pad_y.css(true)),
        format!("rounded-[{}px]", trim_num(s.radius_px, 1)),
        focus_ring(s),
    ])
}

pub fn form_classes() -> String {
    "flex w-full flex-col gap-3 sm:flex-row".to_string()
}

pub fn link_classes(s: &ActionStyle) -> String {
    join(&[
        "inline-flex items-center gap-2 underline underline-offset-4".to_string(),
        format!("font-[family-name:{}]", s.button_typo.font_family),
        typography_classes(&s.button_typo, false),
        format!("text-[color:{}]", s.field_fg.css_tw()),
        format!("decoration-[color:{}]", s.accent.css_tw()),
        focus_ring(s),
    ])
}

/// Inline `nav` / `footer` link: no underline, coloured text, own font family.
pub fn nav_link_classes(t: &ComputedTypography) -> String {
    join(&[
        "no-underline hover:opacity-80 transition-opacity".to_string(),
        format!("font-[family-name:{}]", t.font_family),
        typography_classes(t, true),
    ])
}

pub fn canvas_figure_classes() -> String {
    "flex flex-col gap-2 w-full m-0".to_string()
}

pub fn canvas_surface_classes(s: &CanvasStyle) -> String {
    join(&[
        "block w-full border overflow-hidden".to_string(),
        format!("aspect-[{}]", trim_num(s.aspect_ratio, 3)),
        format!("bg-[color:{}]", s.surface.css_tw()),
        format!("border-[color:{}]", s.border.css_tw()),
        format!("rounded-[{}px]", trim_num(s.radius_px, 1)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_class_shape() {
        let p = GridPlacement { col_start: 1, col_span: 8, row_start: 1, row_span: 1, align_self: Alignment::Start };
        let c = placement_classes(&p);
        assert!(c.starts_with("col-span-12 lg:col-start-1 lg:col-span-8 lg:row-start-1 lg:row-span-1"));
    }

    #[test]
    fn large_indices_use_arbitrary_values() {
        let p = GridPlacement { col_start: 1, col_span: 4, row_start: 9, row_span: 7, align_self: Alignment::End };
        let c = placement_classes(&p);
        assert!(c.contains("lg:row-start-[9]") && c.contains("lg:row-span-[7]"));
    }

    #[test]
    fn track_template_joins_with_underscores() {
        let t = grid_classes(
            ContainerTag::Div,
            &[GridTrack::MinContentFr(5.0), GridTrack::MinContentFr(3.0)],
            &DynamicStep::new(16.0, 32.0),
            &DynamicStep::new(16.0, 32.0),
        );
        assert!(t.contains("lg:grid-rows-[minmax(min-content,5fr)_minmax(min-content,3fr)]"));
    }
}
