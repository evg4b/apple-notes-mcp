# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Important constraints

- **No osascript** — all Apple Notes interaction must go through ScriptingBridge (objc2 + `SBApplication`), never via spawning `osascript` or any other external process.
- macOS only. Build targets are `aarch64-apple-darwin` and `x86_64-apple-darwin` exclusively.

## Commands

```sh
cargo build                          # debug build
cargo build --release                # release build
cargo fmt                            # format
cargo clippy --all-targets -- -D warnings   # lint (must pass clean)
cargo test                           # unit tests (no Notes.app required)
cargo test -- --ignored --nocapture  # integration/debug tests (requires live Notes.app + Automation permission)
cargo test <test_name>               # run single test by name
make build                           # cross-compile debug for both x86_64 and aarch64
make build-release                   # cross-compile release for both targets
make fmt                             # fmt + clippy combined
make inspector-debug                 # build + launch MCP inspector (npx)
```

## Architecture

This is a macOS MCP (Model Context Protocol) server that exposes Apple Notes to AI assistants over stdio (newline-delimited JSON-RPC). There is no HTTP server and no background daemon.

### Startup flow (`main.rs`)
1. Parse CLI args (`--scopes read/write/delete`, optional log path) via `clap`.
2. Initialize file-based tracing logger (default: `~/Library/Logs/apple-notes-mcp.log`).
3. `NotesApp::connect()` — acquires an `SBApplication` proxy to `com.apple.Notes`.
4. `AppleNotesMCP::new(notes_app, scopes).serve(stdio()).await` — MCP server runs on stdin/stdout.

### Layer breakdown

| Module | Role |
|--------|------|
| `src/mcp.rs` | MCP tool definitions via `rmcp` `#[tool]` / `#[tool_router]` macros; scope-gated |
| `src/models.rs` | Request/response types (`serde` + `schemars` JSON Schema) |
| `src/notes/api.rs` | `NotesApp` — all public business logic; unit tests live here |
| `src/notes/bridge.rs` | Low-level Objective-C bridge: `SBApplication`/`SBObject` extern classes, batch-fetch helpers |
| `src/notes/helpers.rs` | KVC helpers (`kvc_string`, `kvc_bool`, `sb_count`, etc.); fully unit-tested without Notes.app |
| `src/notes/types.rs` | Plain data types: `NoteInfo`, `FolderInfo`, `AccountInfo` |
| `src/notes/debug_tests.rs` | Manual `#[ignore]` tests requiring live Notes.app |

### Data flow
```
AI client  →(stdio JSON-RPC)→  AppleNotesMCP (rmcp)  →  NotesApp (api.rs)
  →  bridge.rs + helpers.rs  →(Apple Events via ScriptingBridge)→  Notes.app
```

### Performance pattern
Batch KVC (`valueForKey:` on SBObject collections) is used in `bridge.rs` to reduce Apple Events round-trips from O(N) per-note to O(1) per-folder for bulk reads. New bulk-fetch code should follow this pattern.

### Scopes
`--scopes` restricts which MCP tools are registered. Read-only tools are always available; write/delete tools require their respective scope. Scope checking is enforced in `mcp.rs` at server construction, not at call time.
