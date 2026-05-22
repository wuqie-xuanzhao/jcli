# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**j-cli** (`j`) is a fast CLI productivity tool written in Rust. It provides alias management, daily reports, todo management, AI chat with tool-calling, browser automation, a skill/hook system, and an interactive REPL. Binary name is `j`.

- Rust 2024 edition, minimum rustc 1.93.1
- Published to crates.io as `j-cli`
- All UI text is in Chinese (中文)

## Common Commands

```bash
# Build
cargo build                        # Debug build (workspace)
cargo build -p j-cli               # Build j-cli only
cargo build --release              # Release build
cargo build --features browser_cdp # Build with Chrome DevTools Protocol support

# Test
cargo test                         # Run all tests
cargo test --all-features          # Run tests including CDP features
cargo test -p j-tui                # Run tests for specific crate
cargo test <test_name>             # Run a single test

# Code quality
cargo fmt                          # Format code
cargo clippy -- -D warnings        # Lint (treats warnings as errors)
bash crates/j-cli/scripts/check_lint.sh  # Full lint (11 checks: fmt, clippy, file size, function size, params, unwrap, mod.rs, super::super::, docs, println, unsafe SAFETY)

# Pre-commit (format + lint + test)
make pre-commit

# Install locally
make install                       # Builds release and copies to /usr/local/bin/j
```

## Workspace Architecture

```
jcli/
├── Cargo.toml              # Workspace root (resolver = "2")
├── crates/
│   ├── j-agent/            # Agent loop, LLM client, tools, context management
│   ├── j-cli/              # CLI binary + all commands + Chat TUI
│   ├── j-tui/              # Terminal UI components, editor core, Markdown renderer
│   └── j-md/               # Markdown IR types and parser
└── apps/
    └── desktop/src-tauri/  # Tauri desktop app (j-gui)
```

### Dependency Graph

```
j-cli → j-agent → (async-openai, serde)
j-cli → j-tui → j-md
j-cli → ratatui, crossterm
desktop → j-agent
```

### Crate Boundaries

**j-agent** — Agent loop, LLM communication, tool definitions and execution:
- `agent/` — Agent loop (streaming, tool call processing)
- `llm/` — OpenAI-compatible API client
- `tools/` — 20+ tool implementations (shell, file, browser, task, web, etc.)
- `context/` — Context window management
- `permission/` — `.jcli` permission system
- `teammate/` — Multi-agent teammate support
- `storage/` — Session persistence
- `infra/` — Hooks, skills, config infrastructure

**j-tui** — Shared TUI components (depends on j-md):
- `components/` — Reusable ratatui widgets (file picker, theme gallery, etc.)
- `editor_core/` — Terminal text editor (vim-like keybindings, syntax highlighting)
- `markdown/` — Markdown-to-ratatui rendering (parser from j-md, highlight via syntect)
- `util/text.rs` — Unicode-aware text wrapping and display width

**j-md** — Markdown intermediate representation and parsing:
- `ir.rs` — IR types (Block, Inline, TableData, etc.)
- `parser.rs` — pulldown-cmark → IR conversion
- `util.rs` — Shared parser utilities

**j-cli** — CLI binary, all commands, Chat TUI application:
- Entry point → interactive REPL or clap subcommand dispatch
- `command/chat/` — Full Chat TUI (rendering, input, handlers, tool confirm dialogs)
- `command/` — Other commands (alias, report, todo, notebook, etc.)
- `interactive/` — REPL mode (rustyline, tab completion)
- `tui/` — Bridge to j-tui (theme conversion, editor launcher)
- `theme/` — Color themes (types, parsing, impls)

## Entry Flow

`main.rs` → If no args, enters interactive REPL (`interactive::run_interactive`). Otherwise, attempts clap parsing (`cli::Cli`). If clap fails (unrecognized subcommand), falls back to alias-open logic (`command::open::handle_open`).

## Chat Module (`command/chat/`) — Largest Subsystem

The AI chat is a full TUI application with tool-calling, streaming, and context management:

- **app/** — ChatApp state machine (update, actions, stream poll, tool executor, session)
- **handler/** — TUI event loop, chat logic, config UI, tool confirm, message browse
- **render/** — Message rendering cache (incremental build, tool call/result rendering, bubbles)
- **ui/** — UI drawing (chat view, config view, archive list, input, title bar, popups)
- **input/** — Input handling, autocomplete, file index
- **tools/** — Tool classification, name constants
- **remote/** — WebSocket remote control (pair mode)
- **oneshot/** — Non-interactive chat mode

Agent loop and tool execution live in `j-agent`. The Chat TUI in `j-cli` consumes `j-agent`'s streaming API and renders results.

## Key Patterns

- **CommandHandler trait + `command_handlers!` macro**: Declarative command registration in `handler.rs`. Each command is a struct implementing `CommandHandler::execute()`.
- **`dispatch()` function**: Maps `SubCmd` enum variants to handler structs in `command/mod.rs`.
- **Module pattern**: `name.rs + name/` (strictly no `mod.rs`). Lint enforces this.
- **Import paths**: Use `crate::` absolute paths for cross-module references. `super::` only for same-parent siblings. Lint rejects `super::super::`.
- **Logging macros**: `info!`, `error!`, `usage!`, `debug_log!(config, ...)` defined in `util/log.rs`. `debug_log!` only prints in verbose mode.
- **Agent file logging**: `write_info_log()` and `write_error_log()` in `util/log.rs` write to `~/.jdata/agent/logs/`.
- **Config**: `YamlConfig` wraps `~/.jdata/config.yaml`. Agent config is separate JSON at `~/.jdata/agent/data/agent_config.json`.
- **Data directory**: All user data lives under `~/.jdata/` (customizable via `J_DATA_PATH` env var).
- **Theme**: j-tui uses `EditorTheme`; j-cli converts via `impl From<&Theme> for EditorTheme`.
- **Optional feature**: `browser_cdp` feature flag enables Chrome DevTools Protocol support via `chromiumoxide` crate.
- **File size**: Lint warns at 600 lines, fails at 1000 lines per file. Functions warn at 80 lines.