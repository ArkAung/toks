/// The context passed to every command handler.
/// Kept flat and Copy-friendly — no heap allocation on the hot path.
#[derive(Clone, Copy, Default)]
pub struct Ctx {
    /// -u/--ultra: maximum compression (icons, inline)
    pub ultra: bool,
}

// ---------------------------------------------------------------------------
// Transformer trait
// ---------------------------------------------------------------------------

/// A single stage in the output pipeline.
///
/// Each transformer receives an iterator of lines and emits an iterator of lines.
/// This models the Unix philosophy: composable, zero-copy where possible.
///
/// Implementing a new command is exactly:
///   1. impl Transformer
///   2. push it onto a Pipeline
///
/// There is no enum to extend, no match arm to add in a central dispatcher.
pub trait Transformer {
    fn transform<'a>(
        &self,
        lines: Box<dyn Iterator<Item = String> + 'a>,
    ) -> Box<dyn Iterator<Item = String> + 'a>;
}

// ---------------------------------------------------------------------------
// Built-in transformers
// ---------------------------------------------------------------------------

/// Strip ANSI escape codes and carriage returns.
pub struct StripAnsi;

impl Transformer for StripAnsi {
    fn transform<'a>(
        &self,
        lines: Box<dyn Iterator<Item = String> + 'a>,
    ) -> Box<dyn Iterator<Item = String> + 'a> {
        Box::new(lines.map(|l| strip_ansi_str(&l)))
    }
}

/// Remove lines that match any of the given substrings (progress bars, banners, etc.)
pub struct FilterNoise {
    pub patterns: Vec<&'static str>,
}

impl Transformer for FilterNoise {
    fn transform<'a>(
        &self,
        lines: Box<dyn Iterator<Item = String> + 'a>,
    ) -> Box<dyn Iterator<Item = String> + 'a> {
        let patterns = self.patterns.clone();
        Box::new(lines.filter(move |l| {
            let trimmed = l.trim();
            !patterns.iter().any(|p| trimmed.contains(p))
        }))
    }
}

/// Keep only lines that match at least one of the given substrings.
pub struct KeepOnly {
    pub patterns: Vec<&'static str>,
}

impl Transformer for KeepOnly {
    fn transform<'a>(
        &self,
        lines: Box<dyn Iterator<Item = String> + 'a>,
    ) -> Box<dyn Iterator<Item = String> + 'a> {
        let patterns = self.patterns.clone();
        Box::new(lines.filter(move |l| {
            let lower = l.to_lowercase();
            patterns.iter().any(|p| lower.contains(p))
        }))
    }
}

/// Collapse consecutive identical or near-identical lines into "line (×N)".
pub struct Dedup;

impl Transformer for Dedup {
    fn transform<'a>(
        &self,
        lines: Box<dyn Iterator<Item = String> + 'a>,
    ) -> Box<dyn Iterator<Item = String> + 'a> {
        Box::new(DedupIter {
            inner: lines,
            last: None,
            count: 0,
        })
    }
}

struct DedupIter<'a> {
    inner: Box<dyn Iterator<Item = String> + 'a>,
    last: Option<String>,
    count: usize,
}

impl<'a> Iterator for DedupIter<'a> {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        loop {
            match self.inner.next() {
                None => {
                    return self.last.take().map(|l| {
                        if self.count > 1 {
                            format!("{l} (×{})", self.count)
                        } else {
                            l
                        }
                    });
                }
                Some(line) => {
                    if Some(&line) == self.last.as_ref() {
                        self.count += 1;
                    } else {
                        let prev = self.last.replace(line);
                        let n = self.count;
                        self.count = 1;
                        if let Some(p) = prev {
                            return Some(if n > 1 { format!("{p} (×{n})") } else { p });
                        }
                    }
                }
            }
        }
    }
}

/// Keep at most `max` lines; if exceeded, show first half + "... N more ..." + last few.
pub struct Truncate {
    pub max: usize,
    pub tail: usize,
}

impl Default for Truncate {
    fn default() -> Self {
        Self { max: 200, tail: 10 }
    }
}

impl Transformer for Truncate {
    fn transform<'a>(
        &self,
        lines: Box<dyn Iterator<Item = String> + 'a>,
    ) -> Box<dyn Iterator<Item = String> + 'a> {
        let max = self.max;
        let tail = self.tail;
        let all: Vec<String> = lines.collect();
        if all.len() <= max {
            return Box::new(all.into_iter());
        }
        let head = max - tail;
        let skipped = all.len() - max;
        let mut out = all[..head].to_vec();
        out.push(format!("... {skipped} lines omitted ..."));
        out.extend_from_slice(&all[all.len() - tail..]);
        Box::new(out.into_iter())
    }
}

/// Remove leading/trailing blank lines and collapse runs of blanks to one.
pub struct CollapseBlank;

impl Transformer for CollapseBlank {
    fn transform<'a>(
        &self,
        lines: Box<dyn Iterator<Item = String> + 'a>,
    ) -> Box<dyn Iterator<Item = String> + 'a> {
        Box::new(CollapseBlankIter {
            inner: lines,
            last_blank: true,
        })
    }
}

struct CollapseBlankIter<'a> {
    inner: Box<dyn Iterator<Item = String> + 'a>,
    last_blank: bool,
}

impl<'a> Iterator for CollapseBlankIter<'a> {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        loop {
            let line = self.inner.next()?;
            if line.trim().is_empty() {
                if self.last_blank {
                    continue;
                }
                self.last_blank = true;
            } else {
                self.last_blank = false;
            }
            return Some(line);
        }
    }
}

// ---------------------------------------------------------------------------
// Pipeline builder
// ---------------------------------------------------------------------------

/// Compose transformers left-to-right and run them against a line iterator.
/// Returns the final collected output and the raw token estimate.
pub struct Pipeline(Vec<Box<dyn Transformer>>);

impl Pipeline {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(mut self, t: impl Transformer + 'static) -> Self {
        self.0.push(Box::new(t));
        self
    }

    /// Run the pipeline. Returns (filtered_output, raw_lines, filtered_lines).
    pub fn run(&self, raw: &str) -> (String, usize, usize) {
        let raw_lines: Vec<String> = raw.lines().map(String::from).collect();
        let raw_count = raw_lines.len();
        let mut iter: Box<dyn Iterator<Item = String>> =
            Box::new(raw_count_vec(raw_lines).into_iter());
        for t in &self.0 {
            iter = t.transform(iter);
        }
        let out: Vec<String> = iter.collect();
        let filtered_count = out.len();
        (out.join("\n"), raw_count, filtered_count)
    }
}

fn raw_count_vec(v: Vec<String>) -> Vec<String> {
    v
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strip ANSI escape codes from a string without any external crate.
/// Covers CSI sequences (\x1b[...m) and OSC sequences (\x1b]...\x07).
pub fn strip_ansi_str(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            i += 1;
            if i >= bytes.len() {
                break;
            }
            match bytes[i] {
                b'[' => {
                    i += 1;
                    while i < bytes.len() && !bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    i += 1;
                }
                b']' => {
                    i += 1;
                    while i < bytes.len() && bytes[i] != 0x07 {
                        i += 1;
                    }
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        } else {
            if bytes[i] != b'\r' {
                out.push(bytes[i]);
            }
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Execute a shell command, capture stdout+stderr, return combined output.
pub fn capture(args: &[String]) -> anyhow::Result<String> {
    use std::process::Command;
    if args.is_empty() {
        anyhow::bail!("no command given");
    }
    let output = Command::new(&args[0]).args(&args[1..]).output()?;
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&stderr);
    }
    Ok(combined)
}

/// Estimate token count: roughly 1 token per 4 chars (OpenAI heuristic).
pub fn token_estimate(s: &str) -> usize {
    s.len().div_ceil(4)
}
