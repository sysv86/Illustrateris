//! `ple` command line interface.

use ple_engine::codegen::backend::Backend;
use ple_engine::{compile, json, EmitMode, EngineConfig, ENGINE_VERSION};
use std::io::{Read, Write};
use std::process::exit;

const USAGE: &str = "\
ple - Procedural Layout Engine

USAGE:
  ple compile <input.json|-> [options]
  ple eject   <input.json|-> [options]   emit clean De-PLE HTML + :root CSS tokens (no ple-* fingerprints)
  ple palette <input.json|->             print the synthesized palette + contrast checks as JSON
  ple version
  ple help

OPTIONS (compile / eject):
  -o, --out <file>        write HTML to <file> (default: stdout)
      --emit <mode>       document (default) | fragment
      --report <file|->   write the JSON report to <file>, or to stderr with '-'
      --candidates <n>    BSP candidates per section (default 64)
      --seed <u64>        override the seed (default: derived from the input)
      --max-ecc <x>       max centre-of-mass eccentricity, 0.01..0.5 (default 0.15)
      --viewport <px>     design viewport in px, 480..4000 (default 1440)
      --backend <name>    styling backend: tailwind (default) | css
      --clean, --eject    strip ple-engine fingerprints, emit :root tokens & ui-* classes
      --no-cdn            do not include the Tailwind browser build in document mode

Input '-' reads stdin.
";

fn die(msg: &str, code: i32) -> ! {
    eprintln!("ple: {}", msg);
    exit(code);
}

fn read_input(path: &str) -> String {
    let mut s = String::new();
    if path == "-" {
        if let Err(e) = std::io::stdin().read_to_string(&mut s) {
            die(&format!("cannot read stdin: {}", e), 1);
        }
    } else {
        match std::fs::read_to_string(path) {
            Ok(t) => s = t,
            Err(e) => die(&format!("cannot read '{}': {}", path, e), 1),
        }
    }
    s
}

struct Opts {
    input: String,
    out: Option<String>,
    report: Option<String>,
    cfg: EngineConfig,
}

fn parse_opts(args: &[String]) -> Opts {
    let mut input: Option<String> = None;
    let mut out: Option<String> = None;
    let mut report: Option<String> = None;
    let mut cfg = EngineConfig::default();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let mut value = |name: &str| -> String {
            i += 1;
            match args.get(i) {
                Some(v) => v.clone(),
                None => die(&format!("{} needs a value", name), 2),
            }
        };
        match a {
            "-o" | "--out" => out = Some(value("--out")),
            "--report" => report = Some(value("--report")),
            "--emit" => {
                cfg.emit = match value("--emit").as_str() {
                    "document" => EmitMode::Document,
                    "fragment" => EmitMode::Fragment,
                    other => die(&format!("unknown emit mode '{}'", other), 2),
                }
            }
            "--candidates" => match value("--candidates").parse::<usize>() {
                Ok(n) if (1..=2048).contains(&n) => cfg.candidates = n,
                _ => die("--candidates must be an integer in 1..=2048", 2),
            },
            "--seed" => match value("--seed").parse::<u64>() {
                Ok(n) => cfg.seed = Some(n),
                Err(_) => die("--seed must be a u64", 2),
            },
            "--max-ecc" => match value("--max-ecc").parse::<f64>() {
                Ok(x) if (0.01..=0.5).contains(&x) => cfg.max_eccentricity = x,
                _ => die("--max-ecc must be in 0.01..=0.5", 2),
            },
            "--viewport" => match value("--viewport").parse::<f64>() {
                Ok(x) if (480.0..=4000.0).contains(&x) => cfg.viewport_px = x,
                _ => die("--viewport must be in 480..=4000", 2),
            },
            "--no-cdn" => cfg.tailwind_cdn = false,
            "--clean" | "--eject" => cfg.clean = true,
            "--backend" => {
                let v = value("--backend");
                match Backend::parse(&v) {
                    Some(b) => cfg.backend = b,
                    None => die(&format!("unknown backend '{}' (tailwind|css)", v), 2),
                }
            }
            s if s.starts_with('-') && s != "-" => die(&format!("unknown option '{}'", s), 2),
            s => {
                if input.is_some() {
                    die("only one input file is allowed", 2);
                }
                input = Some(s.to_string());
            }
        }
        i += 1;
    }
    let input = match input {
        Some(p) => p,
        None => die("missing input file (use '-' for stdin)", 2),
    };
    Opts { input, out, report, cfg }
}

fn write_file(path: &str, data: &str) {
    if let Err(e) = std::fs::write(path, data) {
        die(&format!("cannot write '{}': {}", path, e), 1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = match args.first() {
        Some(c) => c.as_str(),
        None => {
            eprint!("{}", USAGE);
            exit(2);
        }
    };
    match cmd {
        "help" | "-h" | "--help" => {
            print!("{}", USAGE);
        }
        "version" | "-V" | "--version" => {
            println!("ple {}", ENGINE_VERSION);
        }
        "compile" | "eject" | "palette" => {
            let mut opts = parse_opts(&args[1..]);
            if cmd == "eject" {
                opts.cfg.clean = true;
                opts.cfg.backend = Backend::Css;
                opts.cfg.tailwind_cdn = false;
            }
            let src = read_input(&opts.input);
            let compiled = match compile(&src, &opts.cfg) {
                Ok(c) => c,
                Err(e) => die(&e.to_string(), 1),
            };
            if cmd == "palette" {
                match compiled.report.get("palette") {
                    Some(p) => println!("{}", p.to_json_pretty()),
                    None => die("internal error: report has no palette", 1),
                }
                return;
            }
            match &opts.out {
                Some(p) => write_file(p, &compiled.html),
                None => {
                    let stdout = std::io::stdout();
                    let mut lock = stdout.lock();
                    if lock.write_all(compiled.html.as_bytes()).is_err() {
                        exit(1);
                    }
                }
            }
            let pretty = compiled.report.to_json_pretty();
            match &opts.report {
                Some(p) if p == "-" => eprintln!("{}", pretty),
                Some(p) => write_file(p, &pretty),
                None => {}
            }
            let empty: Vec<json::Value> = Vec::new();
            let warnings = match compiled.report.get("warnings").and_then(|w| w.as_array()) {
                Some(w) => w,
                None => &empty,
            };
            let ms = compiled.report.get("timing_ms").and_then(|v| v.as_f64()).unwrap_or(0.0);
            eprintln!("ple: ok, {:.1} ms, {} warning(s)", ms, warnings.len());
            for w in warnings {
                if let Some(s) = w.as_str() {
                    eprintln!("  warning: {}", s);
                }
            }
        }
        other => {
            eprintln!("ple: unknown command '{}'\n", other);
            eprint!("{}", USAGE);
            exit(2);
        }
    }
}
