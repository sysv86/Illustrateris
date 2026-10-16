//! IR -> semantic HTML.
//!
//! One traversal renders both backends: for each element the renderer assembles a backend-neutral
//! [`Style`] list (typography, measure, placement, flex/grid) and asks the active [`StyleSink`] for
//! the `class` attribute value. Every piece of user text goes through [`escape`], every tag comes
//! from a closed enum, and `href` values have already been scheme-checked by the context parser.

use super::backend::{Backend, Style, StyleSink, TailwindSink};
use super::css;
use crate::color::oklch::trim_num;
use crate::context::{ActionType, MediaKind};
use crate::ir::ast::*;

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    /// Include the Tailwind v4 browser build so a `document` renders when opened directly.
    pub tailwind_cdn: bool,
    pub backend: Backend,
    /// Clean De-PLE / Eject mode: omits generator metadata/comments, emits `:root` CSS Custom
    /// Properties, and replaces `ple-<16hex>` classes with `ui-<8hex>`.
    pub clean: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions { tailwind_cdn: true, backend: Backend::Tailwind, clean: false }
    }
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

struct Writer {
    out: String,
    depth: usize,
}

impl Writer {
    fn new(depth: usize) -> Writer {
        Writer { out: String::new(), depth }
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn open(&mut self, s: &str) {
        self.line(s);
        self.depth += 1;
    }

    fn close(&mut self, s: &str) {
        self.depth = self.depth.saturating_sub(1);
        self.line(s);
    }
}

fn placement_style(p: Option<&GridPlacement>) -> Vec<Style> {
    match p {
        Some(p) => vec![css::placement_style(p)],
        None => Vec::new(),
    }
}

fn mime_for_image(url: &str) -> &'static str {
    let lower = url.to_ascii_lowercase();
    if lower.ends_with(".avif") {
        "image/avif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".png") {
        "image/png"
    } else {
        "image/jpeg"
    }
}

fn mime_for_video(url: &str) -> &'static str {
    let lower = url.to_ascii_lowercase();
    if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".mov") {
        "video/quicktime"
    } else {
        "video/mp4"
    }
}

fn render_node(w: &mut Writer, sink: &mut dyn StyleSink, node: &LayoutNode, placement: Option<&GridPlacement>) {
    match node {
        LayoutNode::Section { id, labelled_by, style, children } => {
            let class = sink.classes(&[css::section_style(style)]);
            let mut attrs = format!("id=\"{}\" class=\"{}\"", escape(id), escape(&class));
            if let Some(l) = labelled_by {
                attrs.push_str(&format!(" aria-labelledby=\"{}\"", escape(l)));
            }
            if style.atmosphere.is_some() {
                attrs.push_str(" data-atmosphere=\"oklch-grain\"");
            }
            w.open(&format!("<section {}>", attrs));
            for c in children {
                render_node(w, sink, c, None);
            }
            w.close("</section>");
        }
        LayoutNode::AsymmetricGrid { tag, tracks, gap, row_gap, children, .. } => {
            let name = match tag {
                ContainerTag::Div => "div",
                ContainerTag::Dl => "dl",
            };
            let mut styles = vec![css::grid_style(*tag, tracks, gap, row_gap)];
            styles.extend(placement_style(placement));
            let class = sink.classes(&styles);
            w.open(&format!("<{} class=\"{}\">", name, escape(&class)));
            for (p, c) in children {
                render_node(w, sink, c, Some(p));
            }
            w.close(&format!("</{}>", name));
        }
        LayoutNode::FlexRow { tag, direction, alignment, spacing, card, children } => {
            let name = match tag {
                FlexTag::Div => "div",
                FlexTag::Article => "article",
            };
            let mut styles = vec![css::flex_style(*direction, *alignment, spacing, card.as_ref())];
            styles.extend(placement_style(placement));
            let class = sink.classes(&styles);
            w.open(&format!("<{} class=\"{}\">", name, escape(&class)));
            for c in children {
                render_node(w, sink, c, None);
            }
            w.close(&format!("</{}>", name));
        }
        LayoutNode::TextLeaf { id, tag, value, typography, max_ch } => {
            let mut styles = vec![css::typography_style(typography, true), css::measure_style(*max_ch)];
            styles.extend(placement_style(placement));
            let class = sink.classes(&styles);
            let id_attr = match id {
                Some(i) => format!(" id=\"{}\"", escape(i)),
                None => String::new(),
            };
            let name = tag.name();
            w.line(&format!("<{}{} class=\"{}\">{}</{}>", name, id_attr, escape(&class), escape(value), name));
        }
        LayoutNode::LinkLeaf { href, label, typography } => {
            let mut styles = vec![css::nav_link_style(typography)];
            styles.extend(placement_style(placement));
            let class = sink.classes(&styles);
            w.line(&format!("<a href=\"{}\" class=\"{}\">{}</a>", escape(href), escape(&class), escape(label)));
        }
        LayoutNode::ActionLeaf { id, kind, label, placeholder, href, input_type, style } => {
            let class = sink.classes(&placement_style(placement));
            w.open(&format!("<div class=\"{}\">", escape(&class)));
            match kind {
                ActionType::TransactionalInput => {
                    let field_id = format!("{}-field", id);
                    let (name, autocomplete) = match input_type.as_str() {
                        "email" => ("email", "email"),
                        "tel" => ("phone", "tel"),
                        "url" => ("url", "url"),
                        _ => ("value", "off"),
                    };
                    let form_class = sink.classes(&[css::form_style()]);
                    w.open(&format!(
                        "<form class=\"{}\" action=\"{}\" method=\"post\">",
                        escape(&form_class),
                        escape(href)
                    ));
                    let input_class = sink.classes(&[css::input_style(style)]);
                    let button_class = sink.classes(&[css::button_style(style)]);
                    let placeholder = if placeholder.is_empty() { label.as_str() } else { placeholder.as_str() };
                    w.line(&format!("<label class=\"sr-only\" for=\"{}\">{}</label>", escape(&field_id), escape(label)));
                    w.line(&format!(
                        "<input id=\"{}\" class=\"{}\" type=\"{}\" name=\"{}\" placeholder=\"{}\" autocomplete=\"{}\" required>",
                        escape(&field_id),
                        escape(&input_class),
                        escape(input_type),
                        name,
                        escape(placeholder),
                        autocomplete
                    ));
                    w.line(&format!(
                        "<button type=\"submit\" class=\"{}\">{}</button>",
                        escape(&button_class),
                        escape(label)
                    ));
                    w.close("</form>");
                }
                ActionType::PrimaryButton => {
                    let button_class = sink.classes(&[css::button_style(style)]);
                    w.line(&format!("<a href=\"{}\" class=\"{}\">{}</a>", escape(href), escape(&button_class), escape(label)));
                }
                ActionType::SecondaryLink => {
                    let link_class = sink.classes(&[css::link_style(style)]);
                    let join_cap = if style.radius_px <= 5.0 { "square" } else { "round" };
                    w.line(&format!(
                        "<a href=\"{}\" class=\"{}\">{}<svg width=\"16\" height=\"16\" viewBox=\"0 0 16 16\" fill=\"none\" aria-hidden=\"true\"><path d=\"M3 8H13M9 4L13 8L9 12\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"{}\" stroke-linejoin=\"miter\"/></svg></a>",
                        escape(href),
                        escape(&link_class),
                        escape(label),
                        join_cap
                    ));
                }
                ActionType::None => {}
            }
            w.close("</div>");
        }
        LayoutNode::CanvasLeaf { id, shader, label, caption, caption_typo, style } => {
            let mut fig_styles = vec![css::canvas_figure_style()];
            fig_styles.extend(placement_style(placement));
            let fig_class = sink.classes(&fig_styles);
            let canvas_class = sink.classes(&[css::canvas_surface_style(style)]);
            w.open(&format!("<figure class=\"{}\">", escape(&fig_class)));
            w.line(&format!(
                "<canvas id=\"{}\" class=\"{}\" data-webgpu-shader=\"{}\" data-aspect-ratio=\"{}\" role=\"img\" aria-label=\"{}\"></canvas>",
                escape(id),
                escape(&canvas_class),
                escape(shader),
                trim_num(style.aspect_ratio, 3),
                escape(label)
            ));
            if let Some(cap) = caption {
                let cap_class = sink.classes(&[css::typography_style(caption_typo, true)]);
                w.line(&format!("<figcaption class=\"{}\">{}</figcaption>", escape(&cap_class), escape(cap)));
            }
            w.close("</figure>");
        }
        LayoutNode::MediaLeaf { id, kind, src, fallback_src, poster, alt, caption, caption_typo, priority_eager, style } => {
            let mut fig_styles = vec![css::canvas_figure_style()];
            fig_styles.extend(placement_style(placement));
            let fig_class = sink.classes(&fig_styles);
            let media_class = sink.classes(&[css::canvas_surface_style(style)]);
            w.open(&format!("<figure class=\"{}\">", escape(&fig_class)));
            match kind {
                MediaKind::Video => {
                    let poster_attr = match poster {
                        Some(p) => format!(" poster=\"{}\"", escape(p)),
                        None => String::new(),
                    };
                    w.open(&format!(
                        "<video id=\"{}\" class=\"{}\" autoplay muted loop playsinline preload=\"metadata\"{} aria-label=\"{}\">",
                        escape(id),
                        escape(&media_class),
                        poster_attr,
                        escape(alt)
                    ));
                    w.line(&format!("<source src=\"{}\" type=\"{}\">", escape(src), mime_for_video(src)));
                    if let Some(fb) = fallback_src {
                        w.line(&format!("<source src=\"{}\" type=\"{}\">", escape(fb), mime_for_video(fb)));
                    }
                    w.close("</video>");
                }
                _ => {
                    let load_attrs = if *priority_eager {
                        "fetchpriority=\"high\" loading=\"eager\" decoding=\"async\""
                    } else {
                        "loading=\"lazy\" decoding=\"async\""
                    };
                    w.open("<picture>");
                    if let Some(fb) = fallback_src {
                        w.line(&format!("<source srcset=\"{}\" type=\"{}\">", escape(src), mime_for_image(src)));
                        w.line(&format!(
                            "<img id=\"{}\" class=\"{}\" src=\"{}\" alt=\"{}\" {}>",
                            escape(id),
                            escape(&media_class),
                            escape(fb),
                            escape(alt),
                            load_attrs
                        ));
                    } else {
                        w.line(&format!(
                            "<img id=\"{}\" class=\"{}\" src=\"{}\" alt=\"{}\" {}>",
                            escape(id),
                            escape(&media_class),
                            escape(src),
                            escape(alt),
                            load_attrs
                        ));
                    }
                    w.close("</picture>");
                }
            }
            if let Some(cap) = caption {
                let cap_class = sink.classes(&[css::typography_style(caption_typo, true)]);
                w.line(&format!("<figcaption class=\"{}\">{}</figcaption>", escape(&cap_class), escape(cap)));
            }
            w.close("</figure>");
        }
    }
}

fn render_sections(sink: &mut dyn StyleSink, doc: &Document) -> String {
    let mut w = Writer::new(0);
    for s in &doc.sections {
        render_node(&mut w, sink, s, None);
    }
    w.out
}

/// Renders every section with the chosen backend and returns `(body_html, stylesheet)`.
fn render_body(backend: Backend, clean: bool, doc: &Document) -> (String, String) {
    let mut tw_sink = TailwindSink;
    let mut css_sink = super::backend::CssSink::new_with_mode(clean, Some(&doc.tokens));
    let (body, css) = match backend {
        Backend::Tailwind => {
            let body = render_sections(&mut tw_sink, doc);
            (body, String::new())
        }
        Backend::Css => {
            let body = render_sections(&mut css_sink, doc);
            let css = css_sink.stylesheet();
            (body, css)
        }
    };
    (body, css)
}

/// Sections only, preceded by a deterministic provenance comment (unless `clean` is enabled).
pub fn render_fragment(doc: &Document, backend: Backend) -> String {
    render_fragment_with_options(doc, &RenderOptions { tailwind_cdn: false, backend, clean: false })
}

pub fn render_fragment_with_options(doc: &Document, opts: &RenderOptions) -> String {
    let (body, css) = render_body(opts.backend, opts.clean, doc);
    let mut w = Writer::new(0);
    if !opts.clean {
        w.line(&format!("<!-- {} -->", doc.generator.replace("--", "- -")));
    }
    let mut out = w.out;
    if !css.is_empty() {
        out.push_str(&format!("<style>\n{}</style>\n", css));
    }
    out.push_str(&body);
    out
}

pub fn render_document(doc: &Document, opts: &RenderOptions) -> String {
    let backend = opts.backend;
    let (body_html, css) = render_body(backend, opts.clean, doc);

    let scheme = if doc.dark { "dark" } else { "light" };
    let mut w = Writer::new(0);
    w.line("<!doctype html>");
    w.open(&format!("<html lang=\"{}\" style=\"color-scheme: {}\">", escape(&doc.lang), scheme));
    w.open("<head>");
    w.line("<meta charset=\"utf-8\">");
    w.line("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    w.line(&format!("<title>{}</title>", escape(&doc.title)));
    w.line(&format!("<meta name=\"color-scheme\" content=\"{}\">", scheme));
    if !opts.clean {
        w.line(&format!("<meta name=\"generator\" content=\"{}\">", escape(&doc.generator)));
    }
    if opts.tailwind_cdn && backend.needs_tailwind_cdn() {
        w.line("<script src=\"https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4\"></script>");
    }
    let mut base_style = format!(
        "html{{background:{};-webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}}img,video{{display:block;max-width:100%;object-fit:cover}}",
        doc.page_bg.css()
    );
    if doc.tokens.emit_grain {
        base_style.push_str(
            "section[data-atmosphere=\"oklch-grain\"]::before{content:\"\";position:absolute;inset:0;pointer-events:none;filter:url(#ui-grain)}",
        );
    }
    w.line(&format!("<style>{}</style>", base_style));
    if !css.is_empty() {
        w.line("<style>");
        for line in css.lines() {
            w.line(line);
        }
        w.line("</style>");
    }
    w.close("</head>");
    w.open("<body class=\"m-0\">");
    if doc.tokens.emit_grain {
        w.line(&format!(
            "<svg width=\"0\" height=\"0\" style=\"position:absolute;pointer-events:none\" aria-hidden=\"true\"><defs><filter id=\"ui-grain\" x=\"0%\" y=\"0%\" width=\"100%\" height=\"100%\"><feTurbulence type=\"fractalNoise\" baseFrequency=\"{}\" numOctaves=\"3\" stitchTiles=\"stitch\" result=\"n\"/><feColorMatrix type=\"saturate\" values=\"0\" in=\"n\" result=\"m\"/><feComponentTransfer in=\"m\"><feFuncA type=\"linear\" slope=\"{}\"/></feComponentTransfer></filter></defs></svg>",
            trim_num(doc.tokens.grain_frequency, 2),
            trim_num(doc.tokens.grain_opacity, 4)
        ));
    }
    w.open("<main>");
    let mut out = std::mem::take(&mut w.out);
    out.push_str(&body_html);
    let mut w = Writer { out, depth: 2 };
    w.close("</main>");
    w.close("</body>");
    w.close("</html>");
    w.out
}

/// Small helper used by tests and callers that only need a stateless Tailwind sink.
pub fn tailwind_sink() -> TailwindSink {
    TailwindSink
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_all_dangerous_characters() {
        assert_eq!(escape("<a href=\"x\">&'</a>"), "&lt;a href=&quot;x&quot;&gt;&amp;&#39;&lt;/a&gt;");
    }
}
