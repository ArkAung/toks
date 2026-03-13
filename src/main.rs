mod cmds;
mod hook;
mod log;
mod pipeline;

use anyhow::Result;
use clap::{Parser, Subcommand};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "toks",
    about = "Token Slim — agent-agnostic CLI output compressor",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,

    /// Extra token savings: ASCII icons, inline format
    #[arg(short = 'u', long, global = true)]
    ultra: bool,
}

#[derive(Subcommand)]
enum Cmd {
    // ── File ops ──────────────────────────────────────────────────────────
    /// Compact directory listing
    Ls { path: Option<String> },
    /// Smart file read (strips boilerplate, optional sig-only mode)
    Read {
        file: String,
        #[arg(short, long, value_enum, default_value = "normal")]
        level: cmds::files::ReadLevel,
    },
    /// Grouped grep / ripgrep
    Grep {
        pattern: String,
        path: Option<String>,
        #[arg(short = 'r', long)]
        recursive: bool,
    },

    // ── Git ───────────────────────────────────────────────────────────────
    /// Compact git sub-commands (status / diff / log / add / commit / push / pull)
    Git {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    // ── Test runners ──────────────────────────────────────────────────────
    /// Run a test command and show failures only
    Test {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Show stderr/errors only from any command
    Err {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    // ── Build / lint ──────────────────────────────────────────────────────
    /// cargo build / test / clippy — failures only
    Cargo {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// ESLint / Biome / ruff — grouped by rule
    Lint {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// TypeScript errors grouped by file
    Tsc {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// pytest — failures only
    Pytest {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// go test / build / vet — compact
    Go {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    // ── Containers ───────────────────────────────────────────────────────
    /// docker ps/images/logs — compact
    Docker {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    // ── Generic helpers ──────────────────────────────────────────────────
    /// Auto-detect JSON + print schema/structure
    Json { file: Option<String> },
    /// Deduplicate + summarise any log file / command output
    Log {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    // ── Analytics ────────────────────────────────────────────────────────
    /// Show token savings from append-only log
    Gain {
        #[arg(long)]
        history: bool,
        #[arg(long)]
        json: bool,
    },

    // ── Hook management ──────────────────────────────────────────────────
    /// Install/remove the POSIX bash hook (agent-agnostic)
    Init {
        #[arg(long)]
        uninstall: bool,
        /// Print hook snippet only, don't modify files
        #[arg(long)]
        dry_run: bool,
    },
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();
    let ctx = pipeline::Ctx { ultra: cli.ultra };

    match cli.command {
        Cmd::Ls { path } => cmds::files::ls(&path.unwrap_or_else(|| ".".into()), &ctx),
        Cmd::Read { file, level } => cmds::files::read(&file, level, &ctx),
        Cmd::Grep { pattern, path, recursive } => {
            cmds::files::grep(&pattern, &path.unwrap_or_else(|| ".".into()), recursive, &ctx)
        }

        Cmd::Git { args } => cmds::git::run(&args, &ctx),
        Cmd::Cargo { args } => cmds::cargo::run(&args, &ctx),
        Cmd::Test { args } => cmds::test_runner::run(&args, &ctx),
        Cmd::Err { args } => cmds::test_runner::err_only(&args, &ctx),
        Cmd::Lint { args } => cmds::lint::run(&args, &ctx),
        Cmd::Tsc { args } => cmds::lint::tsc(&args, &ctx),
        Cmd::Pytest { args } => cmds::pytest::run(&args, &ctx),
        Cmd::Go { args } => cmds::go_cmd::run(&args, &ctx),
        Cmd::Docker { args } => cmds::docker::run(&args, &ctx),
        Cmd::Json { file } => cmds::data::json_schema(&file, &ctx),
        Cmd::Log { args } => cmds::data::log_dedup(&args, &ctx),

        Cmd::Gain { history, json } => log::gain(history, json),
        Cmd::Init { uninstall, dry_run } => hook::init(uninstall, dry_run),
    }
}

#[cfg(test)]
mod pipeline_tests;