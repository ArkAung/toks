/// Append-only JSONL token savings log.
///
/// Format per line:
///   {"ts":1710000000,"cmd":"git status","raw":2100,"filtered":310}
///
/// No SQLite. No serde_json. We hand-write the tiny JSONL we need.
/// Reading is a simple line scan.
use anyhow::Result;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Path
// ---------------------------------------------------------------------------

fn log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".toks.log")
}

// ---------------------------------------------------------------------------
// Write
// ---------------------------------------------------------------------------

pub struct Entry<'a> {
    pub cmd: &'a str,
    pub raw_tokens: usize,
    pub filtered_tokens: usize,
}

pub fn append(e: &Entry) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Hand-written JSONL — no serde dependency.
    let line = format!(
        "{{\"ts\":{},\"cmd\":\"{}\",\"raw\":{},\"filtered\":{}}}\n",
        ts,
        e.cmd.replace('"', "'"),
        e.raw_tokens,
        e.filtered_tokens,
    );

    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let _ = f.write_all(line.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Read / gain command
// ---------------------------------------------------------------------------

struct Record {
    ts: u64,
    cmd: String,
    raw: usize,
    filtered: usize,
}

fn parse_line(line: &str) -> Option<Record> {
    // Tiny hand-rolled JSONL parser for our own format.
    let ts = extract_u64(line, "\"ts\":")?;
    let cmd = extract_str(line, "\"cmd\":\"")?;
    let raw = extract_u64(line, "\"raw\":")?;
    let filtered = extract_u64(line, "\"filtered\":")?;
    Some(Record {
        ts,
        cmd,
        raw: raw as usize,
        filtered: filtered as usize,
    })
}

fn extract_u64(s: &str, key: &str) -> Option<u64> {
    let start = s.find(key)? + key.len();
    let end = s[start..]
        .find(|c: char| !c.is_ascii_digit())
        .map(|i| start + i)
        .unwrap_or(s.len());
    s[start..end].parse().ok()
}

fn extract_str(s: &str, key: &str) -> Option<String> {
    let start = s.find(key)? + key.len();
    let end = s[start..].find('"').map(|i| start + i)?;
    Some(s[start..end].to_string())
}

pub fn gain(history: bool, as_json: bool) -> Result<()> {
    let path = log_path();
    if !path.exists() {
        println!(
            "No toks log found at {}. Run some commands first.",
            path.display()
        );
        return Ok(());
    }

    let file = std::fs::File::open(&path)?;
    let reader = std::io::BufReader::new(file);
    let mut records: Vec<Record> = reader
        .lines()
        .map_while(Result::ok)
        .filter_map(|l| parse_line(&l))
        .collect();

    if records.is_empty() {
        println!("Log is empty.");
        return Ok(());
    }

    // Sort by timestamp
    records.sort_by_key(|r| r.ts);

    let total_raw: usize = records.iter().map(|r| r.raw).sum();
    let total_filtered: usize = records.iter().map(|r| r.filtered).sum();
    let total_saved = total_raw.saturating_sub(total_filtered);
    let pct = if total_raw > 0 {
        total_saved * 100 / total_raw
    } else {
        0
    };

    if as_json {
        println!(
            "{{\"commands\":{},\"raw_tokens\":{},\"filtered_tokens\":{},\"saved_tokens\":{},\"efficiency_pct\":{}}}",
            records.len(), total_raw, total_filtered, total_saved, pct
        );
        return Ok(());
    }

    println!("toks token savings");
    println!("─────────────────────────────────");
    println!("  Commands processed : {}", records.len());
    println!("  Raw tokens         : {}", fmt_num(total_raw));
    println!("  After filtering    : {}", fmt_num(total_filtered));
    println!("  Tokens saved       : {} ({pct}%)", fmt_num(total_saved));
    println!("  Log file           : {}", path.display());

    if history {
        println!("\nRecent commands (last 20):");
        println!(
            "{:<20} {:>8} {:>8} {:>6}",
            "command", "raw", "filtered", "saved%"
        );
        println!("{}", "─".repeat(50));
        let start = records.len().saturating_sub(20);
        for r in &records[start..] {
            let saved = r.raw.saturating_sub(r.filtered);
            let pct = if r.raw > 0 { saved * 100 / r.raw } else { 0 };
            let cmd_short = if r.cmd.len() > 18 {
                &r.cmd[..18]
            } else {
                &r.cmd
            };
            println!(
                "{:<20} {:>8} {:>8} {:>5}%",
                cmd_short, r.raw, r.filtered, pct
            );
        }
    }

    Ok(())
}

fn fmt_num(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
