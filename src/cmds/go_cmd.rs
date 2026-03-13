use crate::log;
use crate::pipeline::{capture, token_estimate, CollapseBlank, FilterNoise, Pipeline, StripAnsi, Truncate, Ctx};
use anyhow::Result;

pub fn run(args: &[String], _ctx: &Ctx) -> Result<()> {
    let args: Vec<&str> = args.iter().skip_while(|a| a.as_str() == "--").map(String::as_str).collect();
    let full: Vec<String> = std::iter::once("go").chain(args.iter().copied()).map(String::from).collect();
    let raw = capture(&full)?;

    let subcmd = args.first().copied().unwrap_or("go");
    let pipeline = Pipeline::new()
        .push(StripAnsi)
        .push(FilterNoise { patterns: vec!["# command-line-arguments"] })
        .push(CollapseBlank)
        .push(Truncate { max: 100, tail: 15 });

    let (out, _, _) = pipeline.run(&raw);
    emit(&format!("go {subcmd}"), &raw, &out);
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