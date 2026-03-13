use crate::log;
use crate::pipeline::{capture, token_estimate, CollapseBlank, Dedup, Pipeline, StripAnsi, Truncate, Ctx};
use anyhow::Result;

pub fn run(args: &[String], _ctx: &Ctx) -> Result<()> {
    let args: Vec<&str> = args.iter().skip_while(|a| a.as_str() == "--").map(String::as_str).collect();
    let full: Vec<String> = std::iter::once("docker").chain(args.iter().copied()).map(String::from).collect();
    let raw = capture(&full)?;

    let subcmd = args.first().copied().unwrap_or("");

    let out = match subcmd {
        "ps" => compact_table(&raw, &["CONTAINER ID", "IMAGE", "STATUS", "PORTS", "NAMES"]),
        "images" => compact_table(&raw, &["REPOSITORY", "TAG", "SIZE"]),
        "logs" => {
            let pipeline = Pipeline::new()
                .push(StripAnsi)
                .push(Dedup)
                .push(CollapseBlank)
                .push(Truncate { max: 60, tail: 15 });
            let (o, _, _) = pipeline.run(&raw);
            o
        }
        _ => {
            let pipeline = Pipeline::new()
                .push(StripAnsi)
                .push(CollapseBlank)
                .push(Truncate::default());
            let (o, _, _) = pipeline.run(&raw);
            o
        }
    };

    emit(&format!("docker {subcmd}"), &raw, &out);
    println!("{out}");
    Ok(())
}

/// Keep only the most useful columns from docker tabular output.
fn compact_table(raw: &str, _keep_cols: &[&str]) -> String {
    // Simple approach: keep header + data lines, strip empty/noise lines
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .take(30)
        .collect::<Vec<_>>()
        .join("\n")
}

fn emit(cmd: &str, raw: &str, filtered: &str) {
    log::append(&log::Entry {
        cmd,
        raw_tokens: token_estimate(raw),
        filtered_tokens: token_estimate(filtered),
    });
}