use crate::log;
use crate::pipeline::Ctx;
use crate::pipeline::{
    capture, token_estimate, CollapseBlank, Dedup, FilterNoise, Pipeline, StripAnsi, Truncate,
};
use anyhow::Result;

// ---------------------------------------------------------------------------
// Noise patterns common to all git output
// ---------------------------------------------------------------------------

const GIT_NOISE: &[&str] = &[
    "Enumerating objects:",
    "Counting objects:",
    "Compressing objects:",
    "Writing objects:",
    "Total ",
    "Delta compression",
    "remote: Counting",
    "remote: Compressing",
    "remote: Total",
    "Receiving objects:",
    "Resolving deltas:",
    "hint:",
    "Everything up-to-date",
];

// ---------------------------------------------------------------------------
// Public entry
// ---------------------------------------------------------------------------

pub fn run(args: &[String], ctx: &Ctx) -> Result<()> {
    // Strip leading "--" separator if present
    let args: Vec<&str> = args
        .iter()
        .skip_while(|a| a.as_str() == "--")
        .map(String::as_str)
        .collect();

    if args.is_empty() {
        return run_raw(&["git"], "git", ctx);
    }

    match args[0] {
        "status" => git_status(&args, ctx),
        "diff" => git_diff(&args, ctx),
        "log" => git_log(&args, ctx),
        "add" | "commit" | "push" | "pull" | "fetch" | "merge" | "rebase" | "stash" => {
            git_action(&args, ctx)
        }
        _ => run_raw(&build_cmd("git", &args), args[0], ctx),
    }
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

fn git_status(args: &[&str], ctx: &Ctx) -> Result<()> {
    let raw = capture(&build_cmd_str("git", args))?;

    // Ultra: one-line summary
    if ctx.ultra {
        let modified: Vec<&str> = raw
            .lines()
            .filter(|l| l.starts_with('\t') || l.starts_with("  "))
            .map(|l| l.trim())
            .collect();
        let output = if modified.is_empty() {
            "clean".to_string()
        } else {
            format!("{} changed: {}", modified.len(), modified.join(", "))
        };
        emit("git status", &raw, &output, ctx);
        return Ok(());
    }

    let pipeline = Pipeline::new()
        .push(StripAnsi)
        .push(FilterNoise {
            patterns: GIT_NOISE.to_vec(),
        })
        .push(CollapseBlank)
        .push(Truncate { max: 80, tail: 10 });

    let (out, raw_toks, _) = pipeline.run(&raw);
    emit("git status", &raw, &out, ctx);
    let _ = raw_toks;
    Ok(())
}

fn git_diff(args: &[&str], ctx: &Ctx) -> Result<()> {
    let raw = capture(&build_cmd_str("git", args))?;

    // Keep: +/- diff lines, @@ hunks, file headers — drop most context lines
    let pipeline = Pipeline::new()
        .push(StripAnsi)
        .push(FilterNoise {
            patterns: ["No newline at end of file"].to_vec(),
        })
        .push(Truncate { max: 300, tail: 20 });

    let (out, _, _) = pipeline.run(&raw);
    emit("git diff", &raw, &out, ctx);
    Ok(())
}

fn git_log(args: &[&str], ctx: &Ctx) -> Result<()> {
    // Force --oneline if not already set
    let mut full_args: Vec<String> = build_cmd_str("git", args);
    if !full_args
        .iter()
        .any(|a| a == "--oneline" || a == "--format")
    {
        full_args.push("--oneline".into());
    }
    if !full_args
        .iter()
        .any(|a| a.starts_with("-n") || a.starts_with("--max-count"))
    {
        full_args.push("-n".into());
        full_args.push("20".into());
    }
    let raw = capture(&full_args)?;
    emit("git log", &raw, &raw, ctx); // already compact after --oneline
    Ok(())
}

fn git_action(args: &[&str], ctx: &Ctx) -> Result<()> {
    use std::process::Command;
    let full: Vec<&str> = std::iter::once("git").chain(args.iter().copied()).collect();
    let output = Command::new(full[0]).args(&full[1..]).output()?;

    let raw = {
        let mut s = String::from_utf8_lossy(&output.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&output.stderr));
        s
    };

    let subcommand = args[0];
    let summary = match subcommand {
        "push" | "pull" | "fetch" => {
            // Extract remote/branch and file counts
            compact_push_pull(&raw, subcommand)
        }
        "commit" => {
            // "ok abc1234 message"
            let hash = raw
                .lines()
                .find_map(|l| {
                    // "[branch abc1234]" pattern
                    if l.contains('[') {
                        let mut parts = l.split_whitespace();
                        parts.next(); // skip branch
                        parts.next().map(|h| h.trim_end_matches(']').to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            if hash.is_empty() {
                "ok".into()
            } else {
                format!("ok {hash}")
            }
        }
        "add" => "ok".to_string(),
        _ => {
            let pipeline = Pipeline::new()
                .push(StripAnsi)
                .push(FilterNoise {
                    patterns: GIT_NOISE.to_vec(),
                })
                .push(CollapseBlank);
            let (out, _, _) = pipeline.run(&raw);
            out
        }
    };

    emit(&format!("git {subcommand}"), &raw, &summary, ctx);
    if !output.status.success() {
        std::process::exit(output.status.code().unwrap_or(1));
    }
    Ok(())
}

fn compact_push_pull(raw: &str, subcmd: &str) -> String {
    // Look for "branch -> branch" or "files changed" lines
    let branch = raw.lines().find_map(|l| {
        if l.contains("->") {
            Some(l.trim().to_string())
        } else {
            None
        }
    });
    let stats = raw
        .lines()
        .find(|l| l.contains("changed") || l.contains("up to date"));

    match (branch, stats) {
        (Some(b), Some(s)) => format!("ok {b} | {}", s.trim()),
        (Some(b), None) => format!("ok {b}"),
        (None, Some(s)) => format!("ok {}", s.trim()),
        (None, None) => format!("ok ({subcmd})"),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run_raw(args: &[&str], cmd_name: &str, ctx: &Ctx) -> Result<()> {
    let str_args: Vec<String> = args.iter().map(|&s| s.to_string()).collect();
    let raw = capture(&str_args)?;
    let pipeline = Pipeline::new()
        .push(StripAnsi)
        .push(FilterNoise {
            patterns: GIT_NOISE.to_vec(),
        })
        .push(Dedup)
        .push(CollapseBlank)
        .push(Truncate::default());
    let (out, _, _) = pipeline.run(&raw);
    emit(&format!("git {cmd_name}"), &raw, &out, ctx);
    Ok(())
}

fn emit(cmd: &str, raw: &str, filtered: &str, _ctx: &Ctx) {
    print!("{filtered}");
    if !filtered.ends_with('\n') {
        println!();
    }
    log::append(&log::Entry {
        cmd,
        raw_tokens: token_estimate(raw),
        filtered_tokens: token_estimate(filtered),
    });
}

fn build_cmd<'a>(prefix: &'a str, args: &[&'a str]) -> Vec<&'a str> {
    let mut v = vec![prefix];
    v.extend_from_slice(args);
    v
}

fn build_cmd_str(prefix: &str, args: &[&str]) -> Vec<String> {
    let mut v = vec![prefix.to_string()];
    v.extend(args.iter().map(|&s| s.to_string()));
    v
}
