use crate::log;
use crate::pipeline::{capture, token_estimate, CollapseBlank, FilterNoise, Pipeline, StripAnsi, Truncate, Ctx};
use anyhow::Result;

const CARGO_NOISE: &[&str] = &[
    "Compiling ",
    "   Compiling",
    "    Finished",
    "    Blocking",
    "   Blocking",
    "Downloading ",
    "Downloaded ",
    "   Updating",
    "    Locking",
    "     Running",
    "    Checking",
];

pub fn run(args: &[String], ctx: &Ctx) -> Result<()> {
    let args: Vec<&str> = args.iter()
        .skip_while(|a| a.as_str() == "--")
        .map(String::as_str)
        .collect();

    match args.first().copied().unwrap_or("") {
        "test" => cargo_test(&args, ctx),
        "build" | "b" => cargo_build(&args, ctx),
        "clippy" => cargo_clippy(&args, ctx),
        _ => generic_cargo(&args, ctx),
    }
}

fn cargo_test(args: &[&str], _ctx: &Ctx) -> Result<()> {
    let full: Vec<String> = std::iter::once("cargo").chain(args.iter().copied()).map(String::from).collect();
    let raw = capture(&full)?;

    // Count tests and failures from cargo test output
    let mut total = 0usize;
    let mut failures = 0usize;
    let mut fail_lines: Vec<String> = Vec::new();

    for line in raw.lines() {
        if line.starts_with("test ") && (line.ends_with("... ok") || line.ends_with("... FAILED") || line.ends_with("... ignored")) {
            total += 1;
            if line.ends_with("... FAILED") {
                failures += 1;
                fail_lines.push(line.to_string());
            }
        }
    }

    let mut out_lines = Vec::new();
    if total > 0 {
        if failures == 0 {
            out_lines.push(format!("ok {total} tests passed"));
        } else {
            out_lines.push(format!("FAILED: {failures}/{total} tests"));
            out_lines.extend(fail_lines);

            // Include panic / assertion messages
            let mut in_failure = false;
            for line in raw.lines() {
                if line.contains("---- ") && line.contains(" stdout ----") { in_failure = true; }
                if in_failure {
                    out_lines.push(line.to_string());
                    if line.trim().is_empty() { in_failure = false; }
                }
            }
        }
    } else {
        // Fallback: just show errors
        let pipeline = Pipeline::new()
            .push(StripAnsi)
            .push(FilterNoise { patterns: CARGO_NOISE.to_vec() })
            .push(CollapseBlank)
            .push(Truncate { max: 100, tail: 20 });
        let (o, _, _) = pipeline.run(&raw);
        out_lines.push(o);
    }

    let out = out_lines.join("\n");
    emit("cargo test", &raw, &out);
    println!("{out}");
    Ok(())
}

fn cargo_build(args: &[&str], _ctx: &Ctx) -> Result<()> {
    let full: Vec<String> = std::iter::once("cargo").chain(args.iter().copied()).map(String::from).collect();
    let raw = capture(&full)?;

    let pipeline = Pipeline::new()
        .push(StripAnsi)
        .push(FilterNoise { patterns: CARGO_NOISE.to_vec() })
        .push(CollapseBlank)
        .push(Truncate { max: 60, tail: 10 });

    let (out, _, _) = pipeline.run(&raw);
    let out = if out.trim().is_empty() { "ok".to_string() } else { out };
    emit("cargo build", &raw, &out);
    println!("{out}");
    Ok(())
}

fn cargo_clippy(args: &[&str], _ctx: &Ctx) -> Result<()> {
    let full: Vec<String> = std::iter::once("cargo").chain(args.iter().copied()).map(String::from).collect();
    let raw = capture(&full)?;

    // Group by lint rule
    let mut warnings: std::collections::BTreeMap<String, usize> = Default::default();
    let mut errors: Vec<String> = Vec::new();

    for line in raw.lines() {
        let t = line.trim();
        if t.starts_with("warning:") {
            let rule = t.splitn(2, "warning: ").nth(1).unwrap_or("(unknown)");
            *warnings.entry(rule.to_string()).or_default() += 1;
        } else if t.starts_with("error") {
            errors.push(t.to_string());
        }
    }

    let mut out_lines = Vec::new();
    if !errors.is_empty() {
        for e in &errors { out_lines.push(e.clone()); }
    }
    for (rule, count) in &warnings {
        if *count > 1 {
            out_lines.push(format!("warning: {rule} (×{count})"));
        } else {
            out_lines.push(format!("warning: {rule}"));
        }
    }
    if out_lines.is_empty() {
        out_lines.push("ok (no warnings)".into());
    }

    let out = out_lines.join("\n");
    emit("cargo clippy", &raw, &out);
    println!("{out}");
    Ok(())
}

fn generic_cargo(args: &[&str], _ctx: &Ctx) -> Result<()> {
    let full: Vec<String> = std::iter::once("cargo").chain(args.iter().copied()).map(String::from).collect();
    let raw = capture(&full)?;
    let pipeline = Pipeline::new()
        .push(StripAnsi)
        .push(FilterNoise { patterns: CARGO_NOISE.to_vec() })
        .push(CollapseBlank)
        .push(Truncate::default());
    let (out, _, _) = pipeline.run(&raw);
    emit("cargo", &raw, &out);
    println!("{out}");
    Ok(())
}

fn emit(cmd: &str, raw: &str, filtered: &str) {
    log::append(&log::Entry {
        cmd,
        raw_tokens: token_estimate(raw),
        filtered_tokens: token_estimate(filtered),
    });
}