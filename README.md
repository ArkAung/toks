# toks

**Token Slim — agent‑agnostic CLI output compressor**

`toks` (pronounced “tocks”) is a small Rust CLI that compresses or filters the output of other commands before it reaches an LLM‑based agent.  
By stripping boilerplate, deduplicating lines, truncating noisy output and providing ultra‑compact modes, it helps keep LLM prompts within token limits while preserving the essential information.

---

## Features

- **Smart subcommands** for common developer tasks:
  - `ls` – compact directory listing
  - `read` – smart file reading (strips boilerplate, optional signature‑only mode)
  - `grep` – grouped grep / ripgrep output
  - `git` – compact git status/diff/log/add/commit/push/pull etc.
  - `cargo` – cargo build/test/clippy – failures only
  - `test`, `err` – run a command and show only failures / stderr
  - `lint`, `tsc`, `pytest`, `go`, `docker` – analogous filters
  - `json` – auto‑detect JSON and print schema/structure
  - `log` – deduplicate + summarise any log file or command output
  - `gain` – show token‑savings statistics from an append‑only log
  - `init` – install/remove a POSIX bash/zsh hook that transparently rewrites common commands (`git`, `cargo`, `ls`, `grep`, `rg`, `pytest`, `docker`) to go through `toks`

- **Ultra mode** (`-u/--ultra`) – extra token savings using ASCII icons and inline formatting.

- **Hook‑based transparent compression** – after `toks init`, everyday shell commands are automatically compressed without changing your workflow.

- **Token‑savings logging** – each run appends a JSONL entry to `~/.toks.log`; `toks gain` reports total tokens saved.

---

## Installation

### From source (requires Rust)

```sh
git clone https://github.com/arkaung/toks.git
cd toks
cargo install --path .
```

This installs the `toks` binary into `~/.cargo/bin` (or `%USERPROFILE%\.cargo\bin` on Windows).

### Pre‑built binaries

Check the [Releases](../../releases) page for downloadable binaries for Linux, macOS, and Windows.

---

## Quick start

```sh
# See help
toks --help

# Compact directory listing
toks ls

# Smart file read (default normal mode)
toks read src/main.rs

# Show only test failures
toks test -- cargo test

# Install the transparent hook (bash/zsh)
toks init

# After restarting your shell, try:
git status   # -> runs through toks and shows compressed output
```

### Hook usage

After `toks init` (or `toks init --dry-run` to just see the snippet), the following aliases are created:

```sh
alias git='_toks_hook git'
alias cargo='_toks_hook cargo'
alias ls='_toks_hook ls'
alias cat='_toks_hook read'   # also head/tail
alias grep='_toks_hook grep'
alias rg='_toks_hook grep'
alias pytest='_toks_hook pytest'
alias docker='_toks_hook docker'
```

The hook intercepts the command, runs it through `toks` with sensible defaults, and prints the compressed result.  
Original command output is still logged for token‑savings statistics.

To remove the hook:

```sh
toks init --uninstall
```

---

## Examples

### `toks ls`

```
$ toks ls
src/  Cargo.toml  target/
```

### `toks read` (signature‑only)

```
$ toks read src/main.rs --level sig
fn main() -> Result<()> { ... }
```

### `toks git`

```
$ toks git status
On branch main
Your branch is up to date with 'origin/main'.

Changes not staged for commit:
  modified:   src/main.rs
```

### `toks gain`

```
$ toks gain
toks token savings
─────────────────────────────────
  Commands processed : 157
  Raw tokens         : 842,300
  After filtering    : 127,850
  Tokens saved       : 714,450 (85%)
  Log file           : /home/user/.toks.log
```

---

## Configuration

- The token‑savings log lives at `~/.toks.log` (or `%USERPROFILE%\.toks.log` on Windows).  
  You can change the location by setting the environment variable `TOKS_LOG_PATH`.

- The hook snippet can be customized by editing the output of `toks init --dry-run` and sourcing it manually.

---

## Development

```sh
# Build (debug)
cargo build

# Build (release, small binary)
cargo build --release

# Run tests
cargo test

# Format
cargo fmt

# Lint
cargo clippy --all-targets --all-features -- -D warnings
```

---

## License

MIT © 2026 arkaung

See [LICENSE](LICENSE) for details.