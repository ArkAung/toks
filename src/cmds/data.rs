use crate::log;
use crate::pipeline::{
    capture, token_estimate, CollapseBlank, Ctx, Dedup, Pipeline, StripAnsi, Truncate,
};
use anyhow::Result;

pub fn json_schema(file: &Option<String>, _ctx: &Ctx) -> Result<()> {
    let raw = match file {
        Some(f) => std::fs::read_to_string(f)?,
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    // Heuristic: show structure, not values
    // Replace string values with "...", numbers with 0, booleans with false
    let schema = simplify_json(&raw);
    emit("json", &raw, &schema);
    println!("{schema}");
    Ok(())
}

/// Naively replace JSON values with placeholders to show structure only.
fn simplify_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() / 2);
    let mut in_string = false;
    let mut in_value_string = false;
    let mut after_colon = false;
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_string => {
                in_string = true;
                if after_colon {
                    // This is a value string — suppress content
                    in_value_string = true;
                    out.push('"');
                    // drain until closing quote
                    let mut escaped = false;
                    for c2 in chars.by_ref() {
                        if escaped {
                            escaped = false;
                            continue;
                        }
                        if c2 == '\\' {
                            escaped = true;
                            continue;
                        }
                        if c2 == '"' {
                            break;
                        }
                    }
                    out.push_str("\"…\"");
                    in_string = false;
                    after_colon = false;
                } else {
                    out.push(c);
                }
            }
            '"' if in_string && !in_value_string => {
                in_string = false;
                out.push(c);
            }
            ':' if !in_string => {
                after_colon = true;
                out.push(c);
            }
            ',' | '\n' | '{' | '}' | '[' | ']' => {
                after_colon = false;
                out.push(c);
            }
            _ if after_colon && !in_string => {
                // Numeric or boolean value — skip and emit placeholder
                let mut token = String::from(c);
                while let Some(&nc) = chars.peek() {
                    if nc == ',' || nc == '\n' || nc == '}' || nc == ']' {
                        break;
                    }
                    token.push(chars.next().unwrap());
                }
                let t = token.trim();
                if t == "true" || t == "false" {
                    out.push_str(t);
                } else if t == "null" {
                    out.push_str("null");
                } else {
                    out.push('0');
                }
                after_colon = false;
            }
            _ => {
                out.push(c);
            }
        }
    }
    out
}

pub fn log_dedup(args: &[String], _ctx: &Ctx) -> Result<()> {
    let args: Vec<&str> = args
        .iter()
        .skip_while(|a| a.as_str() == "--")
        .map(String::as_str)
        .collect();

    let raw = if args.is_empty() {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        capture(&args.iter().map(|&s| s.to_string()).collect::<Vec<_>>())?
    };

    let pipeline = Pipeline::new()
        .push(StripAnsi)
        .push(Dedup)
        .push(CollapseBlank)
        .push(Truncate { max: 100, tail: 20 });

    let (out, _, _) = pipeline.run(&raw);
    emit("log", &raw, &out);
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
