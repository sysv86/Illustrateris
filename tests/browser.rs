//! End-to-end verification that the generated documents actually render correctly in a browser.
//!
//! This test is opt-in (`PLE_BROWSER=1`) because it needs a real Chromium via Python Playwright,
//! which the default hermetic `cargo test` run must not depend on. It closes the "the output was
//! never opened in a browser" gap: it renders the full 10-role example with the plain-CSS backend
//! and checks, in a real engine, that
//!
//!   * the page never scrolls horizontally at 1440 / 768 / 375 px,
//!   * every `<section>` contains its descendants,
//!   * no console/page errors are raised,
//!   * and the engine's per-glyph width estimates match the browser's measured widths.
//!
//! Run it with:
//!   PLE_BROWSER=1 CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/ple_target cargo test --test browser

use ple_engine::codegen::backend::Backend;
use ple_engine::json::Value;
use ple_engine::{compile, json, EmitMode, EngineConfig};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Largest tolerated relative difference between an engine estimate and the browser measurement,
/// in em units (so it is independent of the rendered font size).
const MAX_WIDTH_ERROR: f64 = 0.08;
/// How many text runs must be matched before the comparison is considered meaningful.
const MIN_MATCHED_RUNS: usize = 40;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn python() -> Option<String> {
    for cand in ["python3", "python"] {
        if Command::new(cand).arg("-c").arg("import playwright").output().map(|o| o.status.success()).unwrap_or(false) {
            return Some(cand.to_string());
        }
    }
    None
}

fn f64_of(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}

fn str_of(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(str::to_string)
}

#[test]
fn document_renders_and_matches_engine_estimates_in_a_browser() {
    if std::env::var("PLE_BROWSER").as_deref() != Ok("1") {
        eprintln!("skipping browser verification; set PLE_BROWSER=1 to enable");
        return;
    }
    let Some(python) = python() else {
        panic!("PLE_BROWSER=1 but no python with the playwright package was found");
    };
    let root = repo_root();

    // --- compile the full example with the self-contained CSS backend ---------------------------
    let src = std::fs::read_to_string(root.join("examples/product-platform.json")).expect("read example");
    let cfg = EngineConfig {
        emit: EmitMode::Document,
        backend: Backend::Css,
        tailwind_cdn: false,
        candidates: 24,
        ..EngineConfig::default()
    };
    let compiled = compile(&src, &cfg).expect("compile");

    let tmp = std::env::temp_dir().join(format!("ple-browser-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create temp dir");
    let html_path = tmp.join("page.html");
    std::fs::write(&html_path, &compiled.html).expect("write html");
    let json_out = tmp.join("geometry.json");

    // --- render and measure in headless Chromium -------------------------------------------------
    let mut cmd = Command::new(&python);
    cmd.arg(root.join("verify/browser_verify.py"))
        .arg("--html").arg(&html_path)
        .arg("--viewport").arg("1440,768,375")
        .arg("--json").arg(&json_out);
    for d in ["sans", "serif", "mono"] {
        cmd.arg("--fonts-dir").arg(root.join("fonts").join(d));
    }
    let out = cmd.output().expect("run browser_verify.py");
    assert!(
        json_out.exists(),
        "harness produced no report (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let geo: Value = json::parse(&std::fs::read_to_string(&json_out).unwrap()).expect("parse geometry json");

    // --- structural assertions -------------------------------------------------------------------
    assert_eq!(geo.get("ok").and_then(|v| v.as_bool()), Some(true), "harness reported ok=false");
    let console_errors = geo.get("console_errors").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    assert!(console_errors.is_empty(), "console errors: {:?}", console_errors);

    // --- estimate-vs-reality comparison ----------------------------------------------------------
    let texts = compiled.report.get("texts").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let viewports = geo.get("viewports").and_then(|v| v.as_array()).cloned().expect("viewports");

    let mut matched = 0usize;
    let mut worst = 0.0f64;
    let mut worst_desc = String::new();

    for vp in &viewports {
        let width = f64_of(vp, "width").unwrap_or(0.0);
        assert_eq!(vp.get("horizontal_overflow").and_then(|v| v.as_bool()), Some(false),
            "width {width}: horizontal overflow of {:?}px", f64_of(vp, "overflow_px"));
        for sec in vp.get("sections").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
            assert_eq!(sec.get("contained").and_then(|v| v.as_bool()), Some(true),
                "width {width}: section {:?} escapes by {:?}px", str_of(&sec, "id"), f64_of(&sec, "overflow_px"));
        }
        // Sections are `overflow: hidden`, so a block that outgrows its own box is clipped instead of
        // reported as page overflow. Unbreakable tokens (long identifiers, paths, URLs) used
        // to be cut off this way at 320-375px; assert that nothing overflows its own box.
        let overflowing = vp.get("overflowing_elements").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        assert!(overflowing.is_empty(),
            "width {width}: {} element(s) overflow their own box (clipped content): {:?}",
            overflowing.len(), overflowing.first());

        let measured = vp.get("texts").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        for r in &texts {
            let Some(text) = str_of(r, "text") else { continue };
            let Some(est) = f64_of(r, "estimated_single_line_px") else { continue };
            let Some(est_fs) = f64_of(r, "font_size_px") else { continue };
            if est <= 0.0 || est_fs <= 0.0 {
                continue;
            }
            let Some(m) = measured.iter().find(|m| str_of(m, "text").as_deref() == Some(text.as_str())) else {
                continue;
            };
            // Elements that carry inline element children (action icons) measure text + icon in
            // the browser; the engine only estimates the text run, so they are not comparable.
            if m.get("has_element_children").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }
            let (Some(meas_w), Some(meas_fs)) = (f64_of(m, "single_line_width_px"), f64_of(m, "font_size_px")) else {
                continue;
            };
            if meas_w <= 0.0 || meas_fs <= 0.0 {
                continue;
            }
            let err = ((est / est_fs) - (meas_w / meas_fs)).abs() / (meas_w / meas_fs);
            matched += 1;
            if err > worst {
                worst = err;
                worst_desc = format!(
                    "width {width} {} {:?}: estimate {:.3}em vs measured {:.3}em",
                    str_of(r, "tag").unwrap_or_default(),
                    text,
                    est / est_fs,
                    meas_w / meas_fs
                );
            }
        }
    }

    eprintln!("browser check: {matched} text runs matched, worst relative width error {:.2}%", worst * 100.0);
    assert!(matched >= MIN_MATCHED_RUNS, "only {matched} text runs matched; comparison is not meaningful");
    assert!(
        worst <= MAX_WIDTH_ERROR,
        "font-metric estimate off by {:.2}% (> {:.0}%): {worst_desc}",
        worst * 100.0,
        MAX_WIDTH_ERROR * 100.0
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn tailwind_backend_document_is_self_describing() {
    // Hermetic: no browser needed. Guards the contract the CDN build relies on.
    let root = repo_root();
    let src = std::fs::read_to_string(root.join("examples/product-platform.json")).unwrap();
    let cfg = EngineConfig { emit: EmitMode::Document, backend: Backend::Tailwind, candidates: 24, ..Default::default() };
    let c = compile(&src, &cfg).unwrap();
    assert!(c.html.starts_with("<!doctype html>"));
    assert!(c.html.contains("@tailwindcss/browser@4"), "tailwind document must load the browser build");
    assert!(!c.html.contains("class=\"ple-"), "tailwind backend must not emit content-addressed CSS classes");
}

#[test]
fn css_backend_document_is_self_contained_and_deterministic() {
    let root = repo_root();
    let src = std::fs::read_to_string(root.join("examples/product-platform.json")).unwrap();
    let cfg = EngineConfig { emit: EmitMode::Document, backend: Backend::Css, tailwind_cdn: false, seed: Some(7), candidates: 24, ..Default::default() };
    let a = compile(&src, &cfg).unwrap();
    let b = compile(&src, &cfg).unwrap();
    assert_eq!(a.html, b.html, "CSS backend output must be byte-deterministic");
    assert!(a.html.contains("box-sizing:border-box"), "missing reset: width:100% + padding would overflow");
    assert!(!a.html.contains("@tailwindcss"), "CSS backend must not depend on the Tailwind CDN");
    assert!(a.html.contains("<style>") && a.html.contains("ple-"), "CSS backend must inline a stylesheet");
    assert_eq!(a.report.get("backend").and_then(|v| v.as_str()), Some("css"));

    // A fragment must carry its own stylesheet, otherwise embedded sections are unstyled.
    let mut fcfg = cfg;
    fcfg.emit = EmitMode::Fragment;
    let f = compile(&src, &fcfg).unwrap();
    assert!(f.html.trim_start().starts_with("<!--"), "fragment must begin with the provenance comment");
    assert!(!f.html.contains("<!doctype"), "fragment must not contain a document shell");
    assert!(f.html.contains("<style>"), "CSS fragment must inline its stylesheet");
}

#[test]
fn every_css_class_referenced_is_defined() {
    // A class used in the markup but never emitted into the stylesheet would silently render unstyled.
    let root = repo_root();
    for name in ["product-platform.json", "fintech.json", "editorial.json", "playful.json"] {
        let src = std::fs::read_to_string(root.join("examples").join(name)).unwrap();
        let cfg = EngineConfig { emit: EmitMode::Document, backend: Backend::Css, tailwind_cdn: false, candidates: 24, ..Default::default() };
        let c = compile(&src, &cfg).unwrap();
        let head_end = c.html.find("</head>").expect("head");
        let head = &c.html[..head_end];
        let body = &c.html[head_end..];
        for token in body.split("class=\"").skip(1) {
            let Some(class_attr) = token.split('"').next() else { continue };
            for cls in class_attr.split_whitespace() {
                if cls.starts_with("ple-") {
                    assert!(head.contains(&format!(".{cls} ")) || head.contains(&format!(".{cls}{{")),
                        "{name}: class {cls} is used but not defined in the stylesheet");
                }
            }
        }
    }
}

#[test]
fn source_assets_are_present() {
    // The verification harness and the font-metric tables must ship together with the crate.
    let root: &Path = &repo_root();
    for rel in [
        "verify/browser_verify.py",
        "tools/gen_font_metrics.py",
        "fonts/SOURCES.md",
    ] {
        assert!(root.join(rel).exists(), "missing required asset: {rel}");
    }
    assert!(root.join("fonts").join("sans").exists());
    assert!(root.join("fonts").join("serif").exists());
    assert!(root.join("fonts").join("mono").exists());
}
