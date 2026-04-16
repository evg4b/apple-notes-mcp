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
through [ScriptingBridge](https://developer.apple.com/documentation/scriptingbridge) - no cloud
API, no extra processes, no background daemon.

## Installation

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
      "command": "/usr/local/bin/apple-notes-mcp"
    }
  }
}
```

Restart Claude Desktop. On the first tool call macOS will show an **Automation permission**
dialog — click OK. If you miss it, go to
**System Settings → Privacy & Security → Automation** and enable Notes for your client.

### Other MCP clients

The server uses the stdio transport (newline-delimited JSON on stdin/stdout).
Point `command` at the binary path.

## Documentation

- [Tools reference](docs/tools.md) — all 13 tools with parameters, return shapes, and data-type schemas
- [Logging & troubleshooting](docs/logging.md) — log file location, `RUST_LOG`, common errors
