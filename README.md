<div align="center">
   <img src=".github/feature_image.png" width="80%" alt="Apple Notes MCP Server" />
   <h1>Apple Notes MCP Server</h1>
   <p>Read and write Apple Notes from any MCP-compatible AI client.</p>
   <p>
      ⚡ Single binary &nbsp;|&nbsp;
      🚫 No runtime dependencies &nbsp;|&nbsp;
      🌐 No HTTP server
   </p>
</div>

## Overview

`apple-notes-mcp` is a [Model Context Protocol](https://modelcontextprotocol.io) server that
exposes Apple Notes to AI assistants via the stdio transport. It talks directly to Notes.app
through [ScriptingBridge](https://developer.apple.com/documentation/scriptingbridge) — no cloud
API, no osascript, no spawned child processes, no background daemon.

> [!WARNING]
> This tool uses the ScriptingBridge API, which Apple does not officially support.
> It's a low-level interface that may change in future macOS releases.
> **Use at your own risk**.

> [!IMPORTANT]
> ScriptingBridge performs best on small to medium libraries. Reading every note in a
> very large library may be slow. Avoid using Apple Notes as a long-term memory store for models.

## Installation

### Pre-built binaries

Download the binary for your architecture from the
[latest release](https://github.com/evg4b/apple-notes-mcp/releases/latest), verify the
checksum with the bundled `SHA256SUMS.txt`, then install:

```sh
# Apple Silicon
curl -Lo apple-notes-mcp https://github.com/evg4b/apple-notes-mcp/releases/latest/download/apple-notes-mcp-aarch64-apple-darwin
chmod +x apple-notes-mcp
sudo mv apple-notes-mcp /usr/local/bin/

# Intel
curl -Lo apple-notes-mcp https://github.com/evg4b/apple-notes-mcp/releases/latest/download/apple-notes-mcp-x86_64-apple-darwin
chmod +x apple-notes-mcp
sudo mv apple-notes-mcp /usr/local/bin/
```

### Build from source

```sh
git clone https://github.com/evg4b/apple-notes-mcp
cd apple-notes-mcp
cargo build --release
cp target/release/apple-notes-mcp /usr/local/bin/
```

## Setup

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "apple-notes": {
      "command": "/usr/local/bin/apple-notes-mcp",
      "args": ["--scopes", "read,write,delete"]
    }
  }
}
```

Restart Claude Desktop. On the first tool call macOS will show an **Automation permission**
dialog — click OK. If you miss it, go to
**System Settings → Privacy & Security → Automation** and enable Notes for your client.

### Other MCP clients

The server uses the stdio transport (newline-delimited JSON on stdin/stdout).
Point `command` at the binary path and pass `--scopes` as needed.

## CLI reference

```
apple-notes-mcp [OPTIONS]

Options:
      --scopes <SCOPES>        Comma-separated list of scopes to enable.
                               Valid values: read, write, delete.
                               [default: read]
      --log-file <LOG_FILE>    Path to the log file.
                               [default: ~/Library/Logs/apple-notes-mcp/apple-notes-mcp.log]
      --log-level <LOG_LEVEL>  Log verbosity level.
                               Valid values: error, warn, info, debug, trace.
                               [default: error]
  -h, --help                   Print help
  -V, --version                Print version
```

### Scopes

| Scope    | Tools enabled                                                                 |
|----------|-------------------------------------------------------------------------------|
| `read`   | `list_notes`, `get_note`, `get_all_notes`, `get_notes_in_folder`, `get_notes_in_account`, `list_folders`, `get_subfolders`, `list_accounts` |
| `write`  | `create_note`, `update_note`                                                  |
| `delete` | `delete_note`                                                                 |

`read` is always enabled by default. Combine scopes as needed:

```sh
# Read-only (default)
apple-notes-mcp

# Read + write
apple-notes-mcp --scopes read,write

# Full access
apple-notes-mcp --scopes read,write,delete
```

## Documentation

- [Tools reference](docs/tools.md) — all 11 tools with parameters, return shapes, and data-type schemas
- [Logging & troubleshooting](docs/logging.md) — log file location, log levels, common errors
