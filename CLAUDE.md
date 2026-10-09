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

This is a macOS MCP (Model Context Protocol) server that exposes Apple Notes to AI assistants over stdio (newline-delimited JSON-RPC). There is no HTTP server, no background daemon, no osascript, and no spawned child processes — all Notes.app interaction goes through ScriptingBridge in-process.

### Startup flow (`main.rs`)
1. Parse CLI args (`--scopes read/write/delete`, optional `--log-file`, optional `--log-level`) via `clap`.
2. Initialize file-based tracing logger (default: `~/Library/Logs/apple-notes-mcp/apple-notes-mcp.log`).
3. `NotesApp::connect()` — acquires an `SBApplication` proxy to `com.apple.Notes`.
4. `AppleNotesMCP::new(notes_app, scopes)` builds the tool router once, then `.serve(stdio()).await` runs the MCP server on stdin/stdout.

### Layer breakdown

| Module                          | Role                                                                                          |
|---------------------------------|-----------------------------------------------------------------------------------------------|
| `src/cli.rs`                    | CLI arg parsing (`clap` derive) — `Args` struct with `scopes`, `log_file`, `log_level`       |
| `src/log.rs`                    | Tracing/logging initialisation; default path and level                                        |
| `src/mcp/apple_notes_mcp.rs`   | `AppleNotesMCP` struct; `build_router` wires scopes → tool sets once, at construction time    |
| `src/mcp/scope.rs`              | `Scope` enum and the `ScopeSet` bitset used for scope checks                                  |
| `src/mcp/read_scope.rs`         | 10 read tools (`list_notes`, `get_note`, `search_notes`, …) via `#[tool]` macros             |
| `src/mcp/write_scope.rs`        | 5 write tools (`create_note`, `append_to_note`, `move_note`, …) via `#[tool]` macros         |
| `src/mcp/delete_scope.rs`       | 2 delete tools (`delete_note`, `delete_folder`) via `#[tool]` macros                          |
| `src/mcp/models/`               | Tool request (`requests.rs`) and response (`responses.rs`) types, `serde` + `schemars`        |
| `src/notes/api/`                | `NotesApp`: shared lookups in `mod.rs`, operations split into `read.rs`, `write.rs`, `delete.rs` |
| `src/notes/bridge.rs`           | Batch-fetch helpers over ScriptingBridge; `walk_folders` is the one folder walker (skips Recently Deleted) |
| `src/notes/helpers.rs`          | KVC helpers (`kvc_string`, `kvc_index_of`, `sb_count`, …); fully unit-tested without Notes.app |
| `src/notes/html.rs`             | `to_plain_text`: HTML note body → plain text (the default body format)                        |
| `src/notes/types.rs`            | Plain data types: `NoteInfo`, `FolderInfo`, `AccountInfo`, `PartialNoteInfo`, `NotePage`      |
| `src/notes/integration_tests.rs` | `#[ignore]` tests requiring a live Notes.app                                                  |

### Data flow
```
AI client  →(stdio JSON-RPC)→  AppleNotesMCP (rmcp)  →  NotesApp (notes/api/)
  →  bridge.rs + helpers.rs  →(Apple Events via ScriptingBridge)→  Notes.app
```

### Performance pattern
Apple Events are the bottleneck, so `bridge.rs` batches them: `valueForKey:` on an SBObject
*collection* fetches one property for every element in a single round-trip, turning O(N) per-note
into O(1) per-folder. New bulk-fetch code should follow this pattern.

Two supporting rules:
- Look objects up with `kvc_index_of`, which compares the batch-fetched `NSString`s in place
  instead of materialising a `Vec<String>` of every name.
- Move batch-fetched columns into the result structs with `take_at`; never clone them, note
  bodies are the largest strings in the payload.

### Scopes
`--scopes` restricts which MCP tools are registered. Scope checking happens once, in
`AppleNotesMCP::build_router`, not per call: a tool outside the granted scopes is never
registered, so it cannot be listed or invoked.
