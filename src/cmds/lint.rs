use crate::log;
use crate::pipeline::{capture, token_estimate, Ctx};
use anyhow::Result;
use std::collections::BTreeMap;

pub fn run(args: &[String], _ctx: &Ctx) -> Result<()> {
    let args: Vec<&str> = args
        .iter()
        .skip_while(|a| a.as_str() == "--")
        .map(String::as_str)
        .collect();
    let full: Vec<String> = args.iter().map(|&s| s.to_string()).collect();
    let raw = capture(&full)?;

    // Group errors/warnings by rule name
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    let mut file_errors: Vec<String> = Vec::new();

    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        // eslint: "  path/file.ts:10:5  rule/name  message"
        // ruff:   "path/file.py:10:5: E501 message"
        // biome:  "path/file.ts:10:5 lint/rule ━━━"
        if let Some(rule) = extract_rule(t) {
            *by_rule.entry(rule).or_default() += 1;
        } else if t.contains("error") || t.contains("warning") {
            file_errors.push(t.to_string());
        }
    }

    let mut out_lines: Vec<String> = Vec::new();
    for (rule, count) in &by_rule {
        out_lines.push(format!("{rule}: {count}"));
    }
    for e in file_errors.iter().take(10) {
        out_lines.push(e.clone());
    }
    if out_lines.is_empty() {
        out_lines.push("ok (no issues)".into());
    }

    let out = out_lines.join("\n");
    emit("lint", &raw, &out);
    println!("{out}");
    Ok(())
}

pub fn tsc(args: &[String], _ctx: &Ctx) -> Result<()> {
    let mut full: Vec<String> = vec!["tsc".to_string()];
    full.extend(args.iter().skip_while(|a| a.as_str() == "--").cloned());
    if !full.iter().any(|a| a == "--noEmit") {
        full.push("--noEmit".into());
    }

    let raw = capture(&full)?;

    // Group errors by file
    let mut by_file: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in raw.lines() {
        // "src/foo.ts(10,5): error TS2345: ..."
        if let Some(paren) = line.find('(') {
            let file = &line[..paren];
            if file.ends_with(".ts") || file.ends_with(".tsx") {
                by_file
                    .entry(file.to_string())
                    .or_default()
                    .push(line.to_string());
            }
        }
    }

    let mut out_lines: Vec<String> = Vec::new();
    for (file, errors) in &by_file {
        out_lines.push(format!("{file}: {} error(s)", errors.len()));
        for e in errors.iter().take(3) {
            // Just the message part
            let msg = e.splitn(2, "error TS").nth(1).unwrap_or(e);
            out_lines.push(format!("  TS{}", &msg[..msg.len().min(80)]));
        }
        if errors.len() > 3 {
            out_lines.push(format!("  … {} more", errors.len() - 3));
        }
    }
    if out_lines.is_empty() {
        out_lines.push("ok (no type errors)".into());
    }

    let out = out_lines.join("\n");
    emit("tsc", &raw, &out);
    println!("{out}");
    Ok(())
}

fn extract_rule(line: &str) -> Option<String> {
    // ruff: "E501", "W291" pattern after colon-space
    // eslint: rule name after two spaces at end
    let parts: Vec<&str> = line.split_whitespace().collect();
    for part in &parts {
        if part.len() >= 4
            && part
                .chars()
                .next()
                .map(|c| c.is_ascii_uppercase())
                .unwrap_or(false)
            && part
                .chars()
                .skip(1)
                .all(|c| c.is_ascii_digit() || c.is_alphanumeric())
        {
            return Some(part.to_string());
        }
        if part.contains('/') && !part.contains("://") {
            return Some(part.to_string());
        }
    }
    None
}

fn emit(cmd: &str, raw: &str, filtered: &str) {
    log::append(&log::Entry {
        cmd,
        raw_tokens: token_estimate(raw),
        filtered_tokens: token_estimate(filtered),
    });
}
