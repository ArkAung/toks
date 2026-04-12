/// Installs a POSIX-compatible bash/zsh hook that transparently rewrites
/// common commands to their `toks` equivalents.
///
/// Unlike rtk, we do NOT patch Claude Code's settings.json.
/// We write to ~/.bashrc / ~/.zshrc / ~/.profile — works with any LLM agent
/// (Claude Code, Cursor, Aider, Gemini CLI, Codex, Cline, etc.)
use anyhow::Result;

// ---------------------------------------------------------------------------
// The hook snippet
// ---------------------------------------------------------------------------

/// Returns the shell function that intercepts commands.
/// Designed to be idempotent — safe to source multiple times.
fn hook_snippet(toks_bin: &str) -> String {
    format!(
        r#"
# >>> toks token compressor hook — do not edit <<<
_toks_hook() {{
  local cmd="$1"; shift
  case "$cmd" in
    git)      command {toks} git -- "$@" ;;
    cargo)    command {toks} cargo -- "$@" ;;
    ls)       command {toks} ls "$@" ;;
    cat|head|tail) command {toks} read "$@" ;;
    grep|rg)  command {toks} grep -- "$@" ;;
    pytest)   command {toks} pytest "$@" ;;
    docker)   command {toks} docker -- "$@" ;;
    *)        command "$cmd" "$@" ;;
  esac
}}

# Alias the commands we want to intercept
for _toks_cmd in git cargo ls grep rg pytest docker; do
  # shellcheck disable=SC2139
  alias "$_toks_cmd"="_toks_hook ${{_toks_cmd}}"
done
unset _toks_cmd
# <<< toks token compressor hook >>>
"#,
        toks = toks_bin
    )
}

const HOOK_START: &str = "# >>> toks token compressor hook — do not edit <<<";
const HOOK_END: &str = "# <<< toks token compressor hook >>>";

// ---------------------------------------------------------------------------
// Shell rc file discovery
// ---------------------------------------------------------------------------

fn rc_files() -> Vec<std::path::PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let h = std::path::Path::new(&home);
    // Prefer zshrc if it exists, then bashrc; always include profile as fallback
    let mut files = Vec::new();
    for name in &[".zshrc", ".bashrc", ".bash_profile", ".profile"] {
        let p = h.join(name);
        if p.exists() {
            files.push(p);
        }
    }
    if files.is_empty() {
        files.push(h.join(".profile"));
    }
    files
}

fn tks_binary_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "toks".into())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn init(uninstall: bool, dry_run: bool) -> Result<()> {
    let snippet = hook_snippet(&tks_binary_path());

    if dry_run {
        println!("# Paste into your shell rc file:\n{snippet}");
        return Ok(());
    }

    let files = rc_files();

    if uninstall {
        for path in &files {
            remove_hook(path)?;
        }
        println!("toks hook removed from {} file(s).", files.len());
        return Ok(());
    }

    // Install into the first rc file found (usually .zshrc or .bashrc)
    let target = &files[0];

    // Idempotency: if already installed, skip
    if target.exists() {
        let content = std::fs::read_to_string(target)?;
        if content.contains(HOOK_START) {
            println!(
                "Hook already installed in {}. Nothing to do.",
                target.display()
            );
            println!("Restart your shell or run:  source {}", target.display());
            return Ok(());
        }
    }

    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target)?;

    use std::io::Write;
    writeln!(f, "\n{snippet}")?;

    println!("toks hook installed in {}.", target.display());
    println!("Works with any LLM agent (Claude Code, Cursor, Aider, Gemini CLI, …)");
    println!("\nRestart your shell or run:");
    println!("  source {}", target.display());
    println!("\nCommands now compressed:");
    println!("  git, cargo, ls, grep, rg, pytest, docker");
    println!("\nTest it:");
    println!("  git status   # -> toks git status");

    Ok(())
}

fn remove_hook(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    if !content.contains(HOOK_START) {
        return Ok(());
    }

    // Remove lines between HOOK_START and HOOK_END inclusive
    let mut out = Vec::new();
    let mut in_hook = false;
    for line in content.lines() {
        if line.trim() == HOOK_START {
            in_hook = true;
        }
        if !in_hook {
            out.push(line);
        }
        if line.trim() == HOOK_END {
            in_hook = false;
        }
    }
    std::fs::write(path, out.join("\n") + "\n")?;
    println!("Removed hook from {}", path.display());
    Ok(())
}
