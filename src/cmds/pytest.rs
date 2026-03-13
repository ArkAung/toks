use crate::log;
use crate::pipeline::{capture, token_estimate, CollapseBlank, FilterNoise, Pipeline, StripAnsi, Truncate, Ctx};
use anyhow::Result;

// ---------------------------------------------------------------------------
// pytest
// ---------------------------------------------------------------------------

pub fn run(args: &[String], _ctx: &Ctx) -> Result<()> {
    let mut full: Vec<String> = vec!["pytest".to_string()];
    full.extend(args.iter().cloned());

    let raw = capture(&full)?;

    // Parse pytest summary line: "5 passed, 2 failed"
    let summary = raw.lines().rev()
        .find(|l| l.contains("passed") || l.contains("failed") || l.contains("error"))
        .map(|l| l.trim().to_string());

    let mut out_lines = Vec::new();
    if let Some(s) = &summary {
        out_lines.push(s.clone());
    }

    // Add failing test names
    let mut in_failure = false;
    for line in raw.lines() {
        if line.starts_with("FAILED ") || line.starts_with("ERROR ") {
            out_lines.push(line.to_string());
        }
        if line.contains("_ FAILED _") || line.contains("_ ERROR _") {
            in_failure = true;
        }
        if in_failure && (line.starts_with("E ") || line.starts_with("AssertionError")) {
            out_lines.push(format!("  {}", line.trim()));
        }
        if in_failure && line.trim().is_empty() {
            in_failure = false;
        }
    }

    if out_lines.is_empty() {
        let pipeline = Pipeline::new()
            .push(StripAnsi)
            .push(FilterNoise { patterns: vec!["platform ", "rootdir:", "plugins:", "collected "] })
            .push(CollapseBlank)
            .push(Truncate { max: 60, tail: 10 });
        let (o, _, _) = pipeline.run(&raw);
        out_lines.push(o);
    }

    let out = out_lines.join("\n");
    emit("pytest", &raw, &out);
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