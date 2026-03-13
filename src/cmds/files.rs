use crate::log;
use crate::pipeline::{token_estimate, Ctx};
use anyhow::Result;
use clap::ValueEnum;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// ls — compact directory tree
// ---------------------------------------------------------------------------

pub fn ls(path: &str, ctx: &Ctx) -> Result<()> {
    let p = Path::new(path);
    if !p.exists() {
        anyhow::bail!("{path} does not exist");
    }

    let mut lines: Vec<String> = Vec::new();
    let root = p.display().to_string();
    lines.push(format!("{root}/"));

    collect_tree(p, "", 0, ctx.ultra.then_some(1).unwrap_or(3), &mut lines)?;

    let out = lines.join("\n");
    emit("ls", &out, &out); // already compact
    Ok(())
}

fn collect_tree(
    dir: &Path,
    prefix: &str,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<String>,
) -> Result<()> {
    if depth >= max_depth { return Ok(()); }

    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            // Skip hidden and common noise dirs
            !s.starts_with('.') && s != "node_modules" && s != "target" && s != "__pycache__"
        })
        .collect();

    entries.sort_by_key(|e| e.file_name());
    let count = entries.len();

    // If a directory has many files at leaf level, summarise
    if depth == max_depth - 1 {
        let files: Vec<_> = entries.iter().filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false)).collect();
        let dirs: Vec<_> = entries.iter().filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)).collect();
        if files.len() > 6 {
            for d in &dirs {
                out.push(format!("{}├── {}/", prefix, d.file_name().to_string_lossy()));
            }
            out.push(format!("{}└── ({} files)", prefix, files.len()));
            return Ok(());
        }
    }

    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let ft = entry.file_type().unwrap();

        if ft.is_dir() {
            out.push(format!("{prefix}{connector}{name_str}/"));
            let new_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });
            collect_tree(&entry.path(), &new_prefix, depth + 1, max_depth, out)?;
        } else {
            out.push(format!("{prefix}{connector}{name_str}"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// read — smart file read
// ---------------------------------------------------------------------------

#[derive(Clone, ValueEnum)]
pub enum ReadLevel {
    /// Show full file with minor cleanup
    Normal,
    /// Show function/class signatures only (no bodies)
    Aggressive,
}

pub fn read(file: &str, level: ReadLevel, _ctx: &Ctx) -> Result<()> {
    let raw = fs::read_to_string(file)?;
    let out = match level {
        ReadLevel::Normal => {
            // Strip trailing whitespace, collapse blank runs
            let mut prev_blank = false;
            let mut lines: Vec<&str> = Vec::new();
            for line in raw.lines() {
                let blank = line.trim().is_empty();
                if blank && prev_blank { continue; }
                prev_blank = blank;
                lines.push(line);
            }
            lines.join("\n")
        }
        ReadLevel::Aggressive => {
            // Heuristic: keep lines that look like signatures (fn, def, class, struct, impl, interface, type)
            let mut out = Vec::new();
            for line in raw.lines() {
                let t = line.trim_start();
                if is_signature_line(t) {
                    out.push(line);
                }
            }
            out.join("\n")
        }
    };
    emit("read", &raw, &out);
    println!("{out}");
    Ok(())
}

fn is_signature_line(t: &str) -> bool {
    let keywords = ["fn ", "pub fn", "async fn", "def ", "class ", "struct ", "impl ",
                    "interface ", "type ", "enum ", "export function", "export class",
                    "export const", "export default", "func ", "#[", "//", "/*", "\"\"\""];
    keywords.iter().any(|k| t.starts_with(k))
}

// ---------------------------------------------------------------------------
// grep — grouped results
// ---------------------------------------------------------------------------

pub fn grep(pattern: &str, path: &str, recursive: bool, _ctx: &Ctx) -> Result<()> {
    use std::process::Command;

    // Prefer ripgrep, fall back to grep
    let (bin, args) = if which_exists("rg") {
        ("rg", vec!["--no-heading", "--line-number"])
    } else {
        let mut a = vec!["-n"];
        if recursive { a.push("-r"); }
        ("grep", a)
    };

    let output = Command::new(bin)
        .args(&args)
        .arg(pattern)
        .arg(path)
        .output()?;

    let raw = String::from_utf8_lossy(&output.stdout).into_owned();
    let _ = args;

    // Group by file
    let mut by_file: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for line in raw.lines() {
        if let Some((file, rest)) = line.split_once(':') {
            by_file.entry(file.to_string()).or_default().push(rest.to_string());
        }
    }

    let mut out_lines: Vec<String> = Vec::new();
    for (file, hits) in &by_file {
        out_lines.push(format!("{file} ({} matches):", hits.len()));
        for h in hits.iter().take(5) {
            out_lines.push(format!("  {h}"));
        }
        if hits.len() > 5 {
            out_lines.push(format!("  … {} more", hits.len() - 5));
        }
    }

    let out = out_lines.join("\n");
    emit("grep", &raw, &out);
    println!("{out}");
    Ok(())
}

fn which_exists(bin: &str) -> bool {
    std::process::Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn emit(cmd: &str, raw: &str, filtered: &str) {
    log::append(&log::Entry {
        cmd,
        raw_tokens: token_estimate(raw),
        filtered_tokens: token_estimate(filtered),
    });
    println!("{filtered}");
}