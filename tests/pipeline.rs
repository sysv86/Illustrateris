use ple_engine::codegen::backend::Backend;
use ple_engine::{compile, json, EmitMode, EngineConfig};

fn example(name: &str) -> String {
    let path = format!("{}/examples/{}.json", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {}", path, e))
}

fn cfg() -> EngineConfig {
    EngineConfig { candidates: 24, ..EngineConfig::default() }
}

#[test]
fn all_examples_compile_with_passing_contrast() {
    for name in ["fintech", "editorial", "playful", "product-platform", "webgpu-flagship"] {
        let out = compile(&example(name), &cfg()).unwrap_or_else(|e| panic!("{}: {}", name, e));
        let palette = out.report.get("palette").unwrap();
        assert_eq!(
            palette.get("all_checks_passed").and_then(|v| v.as_bool()),
            Some(true),
            "{}: palette checks failed: {}",
            name,
            palette.to_json_pretty()
        );
        assert!(out.html.contains("<h1"), "{}: no h1", name);
        assert!(out.html.contains("<section"), "{}: no section", name);
        assert!(out.html.contains("grid-cols-12"), "{}: no grid", name);
    }
}

/// The ordered sequence of element tags, ignoring attributes and text. Used to prove that the two
/// styling backends emit the same semantic structure.
fn tag_sequence(html: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let body = match html.find("<body") {
        Some(i) => &html[i..],
        None => html,
    };
    let mut rest = body;
    while let Some(i) = rest.find('<') {
        let tail = &rest[i..];
        let Some(end) = tail.find('>') else { break };
        let inner = &tail[1..end];
        rest = &tail[end + 1..];
        if inner.starts_with('!') || inner.starts_with('/') {
            continue;
        }
        let name: String = inner
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        if !name.is_empty() {
            tags.push(name);
        }
    }
    tags
}

#[test]
fn both_backends_emit_identical_semantic_structure() {
    let src = example("product-platform");
    let mut tw = cfg();
    tw.backend = Backend::Tailwind;
    let mut css = cfg();
    css.backend = Backend::Css;
    css.tailwind_cdn = false;
    let a = compile(&src, &tw).unwrap();
    let b = compile(&src, &css).unwrap();
    let ta = tag_sequence(&a.html);
    let tb = tag_sequence(&b.html);
    assert!(!ta.is_empty());
    assert_eq!(ta, tb, "the Tailwind and CSS backends produced different element structures");
}

#[test]
fn clean_eject_mode_strips_ple_fingerprints_and_emits_root_tokens() {
    let src = example("fintech");
    let mut c = cfg();
    c.backend = Backend::Css;
    c.tailwind_cdn = false;
    c.clean = true;
    let out = compile(&src, &c).unwrap();
    assert!(!out.html.contains("ple-engine"), "clean mode must not emit ple-engine generator meta or comments");
    assert!(!out.html.contains("class=\"ple-"), "clean mode must not emit ple-* content-addressed classes");
    assert!(out.html.contains(":root{"), "clean mode must emit :root CSS Custom Properties");
    assert!(out.html.contains("ui-"), "clean mode must emit ui-* classes");
}

#[test]
fn webgpu_canvas_and_media_roles_render_semantic_figure() {
    let out = compile(&example("webgpu-flagship"), &cfg()).unwrap();
    assert!(out.html.contains("<figure"), "missing figure container for WebGPU canvas");
    assert!(out.html.contains("<canvas"), "missing canvas element");
    assert!(out.html.contains("data-webgpu-shader=\"sdf_raymarch\""), "missing shader attribute");
}

#[test]
fn photo_and_video_media_render_picture_and_video_elements() {
    let src = r#"{
      "intent": {"domain": "cinema_optics", "tone": "cinematic_editorial"},
      "content_tree": [
        {
          "role": "hero",
          "primary_claim": "Anamorphic Optical Precision",
          "visual_anchor": "photo",
          "media_src": "/assets/hero.avif",
          "fallback_src": "/assets/hero.webp",
          "aspect_ratio": 1.5
        },
        {
          "role": "media",
          "primary_claim": "Live Sensor Capture",
          "items": [
            {
              "title": "6K RAW Telemetry Loop",
              "media_src": "/assets/loop-av1.mp4",
              "fallback_src": "/assets/loop-vp9.webm",
              "poster": "/assets/poster.avif",
              "aspect_ratio": 1.778
            }
          ]
        }
      ]
    }"#;
    let out = compile(src, &cfg()).unwrap();
    assert!(out.html.contains("<picture>"), "hero photo must render <picture>");
    assert!(out.html.contains("type=\"image/avif\""), "hero photo must include AVIF source");
    assert!(out.html.contains("fetchpriority=\"high\""), "hero photo must set fetchpriority=high for LCP");
    assert!(out.html.contains("<video"), "media video must render <video>");
    assert!(out.html.contains("autoplay muted loop playsinline"), "video must include autoplay muted loop playsinline");
    assert!(out.html.contains("poster=\"/assets/poster.avif\""), "video must include poster attribute");
}

#[test]
fn new_roles_render_their_semantic_elements() {
    let out = compile(&example("product-platform"), &cfg()).unwrap();
    assert!(out.html.contains("<a href=\"#product\""), "nav link missing");
    assert!(out.html.contains("<a href=\"https://example.com/status\""), "footer link missing");
    assert!(out.html.contains("<blockquote"), "quote missing");
    assert!(out.html.contains("<cite"), "quote attribution missing");
    assert!(out.html.contains("$49") && out.html.contains("Enterprise"));
    for role in ["nav", "hero", "proof", "features", "narrative", "testimonial", "pricing", "faq", "cta", "footer"] {
        assert!(out.html.contains(&format!("-{}-", role)) || out.html.contains(&format!("-{}\"", role)),
            "missing section for role {role}");
    }
}

#[test]
fn unsafe_link_schemes_are_rejected() {
    let js = r#"{"content_tree":[{"role":"nav","items":[{"title":"x","href":"javascript:alert(1)"}]}]}"#;
    assert!(compile(js, &cfg()).is_err(), "javascript: href must be rejected");
    let data = r#"{"content_tree":[{"role":"nav","items":[{"title":"x","href":"  JaVaScRiPt:alert(1)"}]}]}"#;
    assert!(compile(data, &cfg()).is_err(), "obfuscated javascript: href must be rejected");
    let ok = r#"{"content_tree":[{"role":"nav","items":[{"title":"Docs","href":"/docs"},{"title":"Mail","href":"mailto:a@b.c"}]}]}"#;
    assert!(compile(ok, &cfg()).is_ok(), "relative and mailto hrefs must be accepted");
    let bad_action = r#"{"content_tree":[{"role":"cta","primary_claim":"go","action_type":"primary_button","action_label":"Go","action_href":"javascript:x"}]}"#;
    assert!(compile(bad_action, &cfg()).is_err(), "unsafe action_href must be rejected");
    let bad_media = r#"{"content_tree":[{"role":"media","primary_claim":"x","media_src":"javascript:alert(1)"}]}"#;
    assert!(compile(bad_media, &cfg()).is_err(), "unsafe media_src must be rejected");
}

#[test]
fn role_specific_requirements_are_enforced() {
    let p = r#"{"content_tree":[{"role":"pricing","items":[{"title":"Team"}]}]}"#;
    let e = compile(p, &cfg()).unwrap_err().to_string();
    assert!(e.contains("metric"), "unexpected: {e}");
    let t = r#"{"content_tree":[{"role":"testimonial","items":[{"body":"great"}]}]}"#;
    let e = compile(t, &cfg()).unwrap_err().to_string();
    assert!(e.contains("title"), "unexpected: {e}");
    let n = r#"{"content_tree":[{"role":"nav","items":[{"title":"Docs"}]}]}"#;
    let e = compile(n, &cfg()).unwrap_err().to_string();
    assert!(e.contains("href"), "unexpected: {e}");
}

#[test]
fn interactive_states_are_preserved_across_backends() {
    let src = example("product-platform");
    let mut tw = cfg();
    tw.backend = Backend::Tailwind;
    let mut css = cfg();
    css.backend = Backend::Css;
    css.tailwind_cdn = false;
    let a = compile(&src, &tw).unwrap();
    let b = compile(&src, &css).unwrap();
    assert!(a.html.contains("hover:opacity-80") && a.html.contains("hover:brightness-110"));
    assert!(b.html.contains(":hover {opacity:0.8;}"), "nav link hover missing in CSS backend");
    assert!(b.html.contains(":hover {filter:brightness(1.1);transform:translate3d(0,-1px,0);}"), "button hover missing in CSS backend");
    assert!(b.html.contains(":focus-visible {outline:"), "focus ring missing in CSS backend");
}

#[test]
fn output_is_deterministic() {
    let a = compile(&example("fintech"), &cfg()).unwrap();
    let b = compile(&example("fintech"), &cfg()).unwrap();
    assert_eq!(a.html, b.html);
}

#[test]
fn seed_changes_output_but_stays_valid() {
    let mut c1 = cfg();
    c1.seed = Some(1);
    let mut c2 = cfg();
    c2.seed = Some(987654321);
    let a = compile(&example("playful"), &c1).unwrap();
    let b = compile(&example("playful"), &c2).unwrap();
    assert!(a.html.contains("<h1") && b.html.contains("<h1"));
}

#[test]
fn placements_stay_inside_the_grid() {
    let out = compile(&example("fintech"), &cfg()).unwrap();
    let sections = out.report.get("sections").and_then(|s| s.as_array()).unwrap();
    for s in sections {
        for p in s.get("placements").and_then(|p| p.as_array()).unwrap() {
            let start = p.get("col_start").and_then(|v| v.as_f64()).unwrap();
            let span = p.get("col_span").and_then(|v| v.as_f64()).unwrap();
            assert!(start >= 1.0 && span >= 1.0 && start + span <= 13.0, "bad placement {:?}", p);
        }
    }
}

#[test]
fn user_text_is_escaped() {
    let src = r#"{"intent":{"domain":"x","tone":"calm"},"content_tree":[{"role":"hero","primary_claim":"<script>alert(1)</script> & \"q\""}]}"#;
    let out = compile(src, &cfg()).unwrap();
    assert!(!out.html.contains("<script>alert"));
    assert!(out.html.contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; &quot;q&quot;"));
}

#[test]
fn fragment_mode_has_no_document_shell() {
    let mut c = cfg();
    c.emit = EmitMode::Fragment;
    let out = compile(&example("editorial"), &c).unwrap();
    assert!(!out.html.contains("<!doctype"));
    assert!(out.html.starts_with("<!--"));
}

#[test]
fn invalid_input_is_rejected_with_message() {
    assert!(compile("{", &cfg()).is_err());
    assert!(compile(r#"{"content_tree":[]}"#, &cfg()).is_err());
    let e = compile(r#"{"content_tree":[{"role":"hero"}]}"#, &cfg()).unwrap_err();
    assert!(e.to_string().contains("primary_claim"));
}

#[test]
fn report_is_valid_json() {
    let out = compile(&example("fintech"), &cfg()).unwrap();
    let text = out.report.to_json_pretty();
    assert!(json::parse(&text).is_ok());
}

#[test]
fn json_api_roundtrip() {
    let s = ple_engine::compile_to_json(&example("editorial"), Some(r#"{"candidates":8,"emit":"fragment"}"#));
    let v = json::parse(&s).unwrap();
    assert_eq!(v.get("ok").and_then(|b| b.as_bool()), Some(true));
    let bad = ple_engine::compile_to_json("{", None);
    assert_eq!(json::parse(&bad).unwrap().get("ok").and_then(|b| b.as_bool()), Some(false));
}
