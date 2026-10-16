//! Read-only inspection of the built IR.
//!
//! `collect_texts` walks a [`Document`] and returns every rendered text run together with the
//! exact typography the codegen will use. The JSON report embeds this list so the estimated width
//! of each run can be checked against what a browser actually measures.

use crate::ir::ast::*;
use crate::ir::token::ComputedTypography;

#[derive(Debug, Clone)]
pub struct TextRecord {
    /// HTML tag the run is rendered with (`h1`, `p`, `a`, ...).
    pub tag: &'static str,
    /// The exact text content (already the string that is escaped into the document).
    pub text: String,
    pub typography: ComputedTypography,
}

fn walk(node: &LayoutNode, out: &mut Vec<TextRecord>) {
    match node {
        LayoutNode::Section { children, .. } => {
            for c in children {
                walk(c, out);
            }
        }
        LayoutNode::AsymmetricGrid { children, .. } => {
            for (_, c) in children {
                walk(c, out);
            }
        }
        LayoutNode::FlexRow { children, .. } => {
            for c in children {
                walk(c, out);
            }
        }
        LayoutNode::TextLeaf { tag, value, typography, .. } => out.push(TextRecord {
            tag: tag.name(),
            text: value.clone(),
            typography: typography.clone(),
        }),
        LayoutNode::LinkLeaf { label, typography, .. } => out.push(TextRecord {
            tag: "a",
            text: label.clone(),
            typography: typography.clone(),
        }),
        LayoutNode::ActionLeaf { kind, label, style, .. } => {
            if *kind != crate::context::ActionType::None {
                out.push(TextRecord {
                    tag: if *kind == crate::context::ActionType::PrimaryButton { "a" } else { "button" },
                    text: label.clone(),
                    typography: style.button_typo.clone(),
                });
            }
        }
        LayoutNode::CanvasLeaf { caption, caption_typo, .. } | LayoutNode::MediaLeaf { caption, caption_typo, .. } => {
            if let Some(c) = caption {
                out.push(TextRecord {
                    tag: "figcaption",
                    text: c.clone(),
                    typography: caption_typo.clone(),
                });
            }
        }
    }
}

/// Every rendered text run, in DOM order.
pub fn collect_texts(doc: &Document) -> Vec<TextRecord> {
    let mut out = Vec::new();
    for s in &doc.sections {
        walk(s, &mut out);
    }
    out
}
