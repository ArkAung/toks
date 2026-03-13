use crate::log;
use crate::pipeline::{capture, token_estimate, CollapseBlank, FilterNoise, KeepOnly, Pipeline, StripAnsi, Truncate, Dedup, Ctx};
use anyhow::Result;

// ---------------------------------------------------------------------------
// Generic test runner: show failures only
// ---------------------------------------------------------------------------

pub fn run(args: &[String], _ctx: &Ctx) -> Result<()> {
    let args: Vec<&str> = args.iter().skip_while(|a| a.as_str() == "--").map(String::as_str).collect();
    let full: Vec<String> = args.iter().map(|&s| s.to_string()).collect();
    let raw = capture(&full)?;

    let pipeline = Pipeline::new()
        .push(StripAnsi)
        .push(KeepOnly { patterns: vec!["fail", "error", "panic", "assert", "FAIL", "ERROR"] })
        .push(Dedup)
        .push(CollapseBlank)
        .push(Truncate { max: 80, tail: 15 });

    let (out, _, _) = pipeline.run(&raw);
    let out = if out.trim().is_empty() {
        "ok (all tests passed)".to_string()
    } else {
        out
    };
    emit("test", &raw, &out);
    println!("{out}");
    Ok(())
}

/// Show stderr/errors only from any command.
pub fn err_only(args: &[String], _ctx: &Ctx) -> Result<()> {
    use std::process::Command;
    let args: Vec<&str> = args.iter().skip_while(|a| a.as_str() == "--").map(String::as_str).collect();
    if args.is_empty() { anyhow::bail!("no command given"); }

    let output = Command::new(args[0]).args(&args[1..]).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let raw = stderr.clone();

    let pipeline = Pipeline::new()
        .push(StripAnsi)
        .push(FilterNoise { patterns: vec!["npm warn", "yarn warn"] })
        .push(Dedup)
        .push(CollapseBlank)
        .push(Truncate { max: 100, tail: 20 });

    let (out, _, _) = pipeline.run(&raw);
    emit("err", &raw, &out);
    if !out.trim().is_empty() { println!("{out}"); }
    Ok(())
}

fn emit(cmd: &str, raw: &str, filtered: &str) {
    log::append(&log::Entry {
        cmd,
        raw_tokens: token_estimate(raw),
        filtered_tokens: token_estimate(filtered),
    });
}