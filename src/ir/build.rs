//! Lowers section plans + chosen arrangements into the typed layout tree.

use crate::color::oklch::Oklch;
use crate::color::palette::Palette;
use crate::compose::{BlockContent, SectionPlan};
use crate::context::{Context, Physics, Role};
use crate::ir::ast::*;
use crate::ir::token::{font_stack, DynamicStep, Scale, SpacingSet};
use crate::layout::search::Chosen;
use crate::layout::visual_mass::{centroid_x, centroid_y};

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[allow(clippy::too_many_arguments)] // single internal lowering entry point; grouping into a struct would only add indirection at the one call site.
pub fn build_document(
    ctx: &Context,
    physics: &Physics,
    palette: &Palette,
    scale: &Scale,
    spacing: &SpacingSet,
    plans: &[SectionPlan],
    chosen: &[Chosen],
    generator: &str,
) -> Document {
    let mut sections: Vec<LayoutNode> = Vec::new();
    let mut h1_taken = false;
    let mut first_claim: Option<String> = None;
    let mut any_atmosphere = false;

    for (i, (plan, ch)) in plans.iter().zip(chosen.iter()).enumerate() {
        let arr = &ch.arrangement;
        let has_min_h = plan.min_height_dvh.is_some();
        let use_dl = plan.role == Role::ProofMatrix && !plan.has_header;
        let mut children: Vec<(GridPlacement, LayoutNode)> = Vec::new();
        let mut labelled: Option<String> = None;
        let mut has_canvas = false;

        let card_surface = CardSurface {
            bg: palette.section_bg(i + 1).oklch,
            border: palette.border.oklch,
            hover_border: palette.accent.oklch,
            radius_px: physics.radius_px,
            pad: DynamicStep::fixed(scale.space_near(20.0)),
            hover_ms: physics.modalities.hover_ms,
        };

        for (bi, pb) in plan.blocks.iter().enumerate() {
            let p = &arr.placements[bi];
            let at_bottom = has_min_h && p.y > 1e-3 && (p.y + p.h) >= arr.height - 1e-3;
            let anchors_bottom = matches!(pb.content, BlockContent::Action { .. } | BlockContent::Metric { .. });
            let align = if at_bottom && anchors_bottom { Alignment::End } else { Alignment::Start };
            let placement = GridPlacement {
                col_start: p.col_start,
                col_span: p.col_span,
                row_start: p.row_start,
                row_span: p.row_span,
                align_self: align,
            };

            let node = match &pb.content {
                BlockContent::Claim { text } => {
                    let is_heading = plan.role.claim_is_heading();
                    let tag = if !is_heading {
                        HtmlTag::P
                    } else if h1_taken {
                        HtmlTag::H2
                    } else {
                        HtmlTag::H1
                    };
                    if is_heading {
                        h1_taken = true;
                    }
                    let id = format!("{}-title", plan.id);
                    if is_heading && labelled.is_none() {
                        labelled = Some(id.clone());
                    }
                    if is_heading && first_claim.is_none() {
                        first_claim = Some(text.clone());
                    }
                    LayoutNode::TextLeaf {
                        id: if is_heading { Some(id) } else { None },
                        tag,
                        value: text.clone(),
                        typography: pb.typos[0].clone(),
                        max_ch: None,
                    }
                }
                BlockContent::Support { text } => LayoutNode::TextLeaf {
                    id: None,
                    tag: HtmlTag::P,
                    value: text.clone(),
                    typography: pb.typos[0].clone(),
                    max_ch: pb.measure_ch,
                },
                BlockContent::Action { kind, label, placeholder, href, input_type } => LayoutNode::ActionLeaf {
                    id: format!("{}-action", plan.id),
                    kind: *kind,
                    label: label.clone(),
                    placeholder: placeholder.clone(),
                    href: href.clone(),
                    input_type: input_type.clone(),
                    style: ActionStyle {
                        accent: palette.accent.oklch,
                        accent_text: palette.accent_text.oklch,
                        field_bg: palette.section_bg(i + 1).oklch,
                        field_fg: palette.text.oklch,
                        field_border: palette.border.oklch,
                        placeholder: palette.text_secondary.oklch,
                        ring: palette.accent.oklch,
                        radius_px: physics.radius_px,
                        pad_x: DynamicStep::fixed(scale.space_near(24.0)),
                        pad_y: DynamicStep::fixed(scale.space_near(14.0)),
                        hover_ms: physics.modalities.hover_ms,
                        button_typo: pb.typos[0].clone(),
                        input_typo: pb.typos[1].clone(),
                    },
                },
                BlockContent::Metric { value, label } => {
                    let value_leaf_tag = if use_dl { HtmlTag::Dd } else { HtmlTag::P };
                    let label_leaf_tag = if use_dl { HtmlTag::Dt } else { HtmlTag::P };
                    let value_leaf = LayoutNode::TextLeaf {
                        id: None,
                        tag: value_leaf_tag,
                        value: value.clone(),
                        typography: pb.typos[0].clone(),
                        max_ch: None,
                    };
                    let label_leaf = LayoutNode::TextLeaf {
                        id: None,
                        tag: label_leaf_tag,
                        value: label.clone(),
                        typography: pb.typos[1].clone(),
                        max_ch: None,
                    };
                    let (direction, kids) = if use_dl {
                        (Direction::ColumnReverse, vec![label_leaf, value_leaf])
                    } else {
                        (Direction::Column, vec![value_leaf, label_leaf])
                    };
                    LayoutNode::FlexRow {
                        tag: FlexTag::Div,
                        direction,
                        alignment: Alignment::Start,
                        spacing: DynamicStep::fixed(scale.space_near(8.0)),
                        card: None,
                        children: kids,
                    }
                }
                BlockContent::Feature { title, body } => {
                    let mut kids = vec![LayoutNode::TextLeaf {
                        id: None,
                        tag: HtmlTag::H3,
                        value: title.clone(),
                        typography: pb.typos[0].clone(),
                        max_ch: None,
                    }];
                    if let Some(b) = body {
                        kids.push(LayoutNode::TextLeaf {
                            id: None,
                            tag: HtmlTag::P,
                            value: b.clone(),
                            typography: pb.typos[1].clone(),
                            max_ch: pb.measure_ch,
                        });
                    }
                    let use_card = plan.role == Role::FeatureList && physics.density >= 0.55;
                    LayoutNode::FlexRow {
                        tag: FlexTag::Article,
                        direction: Direction::Column,
                        alignment: Alignment::Start,
                        spacing: DynamicStep::fixed(scale.space_near(12.0)),
                        card: if use_card { Some(card_surface) } else { None },
                        children: kids,
                    }
                }
                BlockContent::Link { label, href } => LayoutNode::LinkLeaf {
                    href: href.clone(),
                    label: label.clone(),
                    typography: pb.typos[0].clone(),
                },
                BlockContent::Tier { name, price, note, body } => {
                    let mut row = vec![LayoutNode::TextLeaf {
                        id: None,
                        tag: HtmlTag::P,
                        value: price.clone(),
                        typography: pb.typos[1].clone(),
                        max_ch: None,
                    }];
                    if let Some(n) = note {
                        row.push(LayoutNode::TextLeaf {
                            id: None,
                            tag: HtmlTag::P,
                            value: n.clone(),
                            typography: pb.typos[2].clone(),
                            max_ch: None,
                        });
                    }
                    let mut kids = vec![
                        LayoutNode::TextLeaf {
                            id: None,
                            tag: HtmlTag::H3,
                            value: name.clone(),
                            typography: pb.typos[0].clone(),
                            max_ch: None,
                        },
                        LayoutNode::FlexRow {
                            tag: FlexTag::Div,
                            direction: Direction::Row,
                            alignment: Alignment::Baseline,
                            spacing: DynamicStep::fixed(scale.space_near(8.0)),
                            card: None,
                            children: row,
                        },
                    ];
                    if let Some(b) = body {
                        kids.push(LayoutNode::TextLeaf {
                            id: None,
                            tag: HtmlTag::P,
                            value: b.clone(),
                            typography: pb.typos[2].clone(),
                            max_ch: pb.measure_ch,
                        });
                    }
                    LayoutNode::FlexRow {
                        tag: FlexTag::Article,
                        direction: Direction::Column,
                        alignment: Alignment::Start,
                        spacing: DynamicStep::fixed(scale.space_near(12.0)),
                        card: Some(card_surface),
                        children: kids,
                    }
                }
                BlockContent::Quote { text, attribution, role } => {
                    let attr = match role {
                        Some(r) if !r.trim().is_empty() => format!("{} \u{2014} {}", attribution, r),
                        _ => attribution.clone(),
                    };
                    let kids = vec![
                        LayoutNode::TextLeaf {
                            id: None,
                            tag: HtmlTag::Blockquote,
                            value: text.clone(),
                            typography: pb.typos[0].clone(),
                            max_ch: pb.measure_ch,
                        },
                        LayoutNode::TextLeaf {
                            id: None,
                            tag: HtmlTag::Cite,
                            value: attr,
                            typography: pb.typos[1].clone(),
                            max_ch: None,
                        },
                    ];
                    LayoutNode::FlexRow {
                        tag: FlexTag::Article,
                        direction: Direction::Column,
                        alignment: Alignment::Start,
                        spacing: DynamicStep::fixed(scale.space_near(12.0)),
                        card: None,
                        children: kids,
                    }
                }
                BlockContent::Canvas { shader, aspect_ratio, label, caption } => {
                    has_canvas = true;
                    LayoutNode::CanvasLeaf {
                        id: format!("{}-canvas-{}", plan.id, bi + 1),
                        shader: shader.clone(),
                        label: label.clone(),
                        caption: caption.clone(),
                        caption_typo: pb.typos[0].clone(),
                        style: CanvasStyle {
                            surface: palette.section_bg(i + 1).oklch,
                            border: palette.border.oklch,
                            accent: palette.accent.oklch,
                            radius_px: physics.radius_px,
                            aspect_ratio: *aspect_ratio,
                        },
                    }
                }
                BlockContent::MediaAsset { kind, src, fallback_src, poster, aspect_ratio, alt, caption, priority_eager } => {
                    LayoutNode::MediaLeaf {
                        id: format!("{}-media-{}", plan.id, bi + 1),
                        kind: *kind,
                        src: src.clone(),
                        fallback_src: fallback_src.clone(),
                        poster: poster.clone(),
                        alt: alt.clone(),
                        caption: caption.clone(),
                        caption_typo: pb.typos[0].clone(),
                        priority_eager: *priority_eager,
                        style: CanvasStyle {
                            surface: palette.section_bg(i + 1).oklch,
                            border: palette.border.oklch,
                            accent: palette.accent.oklch,
                            radius_px: physics.radius_px,
                            aspect_ratio: *aspect_ratio,
                        },
                    }
                }
            };
            children.push((placement, node));
        }

        let atmosphere = if matches!(plan.role, Role::Hero | Role::Cta)
            && !has_canvas
            && physics.modalities.noisy_gradient_score >= 0.38
        {
            any_atmosphere = true;
            let masses: Vec<f64> = plan.blocks.iter().map(|b| b.mass).collect();
            let (cx, _) = centroid_x(arr, &masses);
            let (cy, _) = centroid_y(arr, &masses);
            let glow_l = if palette.dark {
                (palette.bg.oklch.l + 0.08).min(0.32)
            } else {
                (palette.bg.oklch.l - 0.04).max(0.88)
            };
            let glow_c = (palette.accent.oklch.c * 0.45).clamp(0.015, 0.09);
            let glow = Oklch::new(glow_l, glow_c, palette.accent.oklch.h).quantized();
            Some(AtmosphereStyle {
                focal_x_pct: round2((cx * 100.0).clamp(18.0, 82.0)),
                focal_y_pct: round2((cy * 100.0).clamp(18.0, 82.0)),
                glow,
                glow_alpha: if palette.dark { 0.22 } else { 0.14 },
            })
        } else {
            None
        };

        let tracks: Vec<GridTrack> = if has_min_h {
            arr.row_heights.iter().map(|h| GridTrack::MinContentFr(round2(*h))).collect()
        } else {
            Vec::new()
        };
        let grid = LayoutNode::AsymmetricGrid {
            tag: if use_dl { ContainerTag::Dl } else { ContainerTag::Div },
            columns: 12,
            tracks,
            gap: spacing.gap_x,
            row_gap: spacing.gap_y,
            children,
        };
        sections.push(LayoutNode::Section {
            id: plan.id.clone(),
            labelled_by: labelled,
            style: SectionStyle {
                bg: palette.section_bg(i).oklch,
                fg: palette.text.oklch,
                border_top: if i > 0 { Some(palette.border.oklch) } else { None },
                pad_x: spacing.pad_x,
                pad_y: spacing.pad_y,
                min_height_dvh: plan.min_height_dvh,
                font_family: font_stack(physics.personality).to_string(),
                atmosphere,
            },
            children: vec![grid],
        });
    }

    let title = ctx
        .intent
        .title
        .clone()
        .or(first_claim)
        .unwrap_or_else(|| "Untitled".to_string());
    Document {
        lang: ctx.intent.lang.clone(),
        title,
        dark: palette.dark,
        page_bg: palette.bg.oklch,
        tokens: DocumentTokens {
            bg: palette.bg.oklch,
            surface: palette.surface.oklch,
            border: palette.border.oklch,
            text: palette.text.oklch,
            text_secondary: palette.text_secondary.oklch,
            accent: palette.accent.oklch,
            accent_text: palette.accent_text.oklch,
            font_family: font_stack(physics.personality).to_string(),
            radius_px: physics.radius_px,
            grain_frequency: physics.modalities.grain_frequency,
            grain_opacity: physics.modalities.grain_opacity,
            emit_grain: any_atmosphere,
        },
        sections,
        generator: generator.to_string(),
    }
}
