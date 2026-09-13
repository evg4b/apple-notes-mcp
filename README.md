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

## What this is

`apple-notes-mcp` lets an AI assistant read and write the notes you already have in Notes.app.
It's a [Model Context Protocol](https://modelcontextprotocol.io) server that speaks over stdio,
and it talks to Notes through
[ScriptingBridge](https://developer.apple.com/documentation/scriptingbridge) in its own process.
Nothing goes to the cloud, nothing shells out to `osascript`, and there's no daemon sitting in
the background.

> [!WARNING]
> ScriptingBridge isn't an API Apple officially supports. It works well today, but it's
> low-level and Apple could change it in any macOS release. **Use at your own risk.**

> [!IMPORTANT]
> ScriptingBridge is happiest with small and medium libraries. Reading thousands of notes will
> be slow no matter how well the calls are batched, so it's worth keeping Apple Notes as a place
> your notes live rather than as a memory store for a model.

## Installing

With Homebrew:

```shell
brew install evg4b/tap/apple-notes-mcp
```

With Stew:

```shell
stew install evg4b/apple-notes-mcp
```

Or grab a binary from the [latest release](https://github.com/evg4b/apple-notes-mcp/releases/latest).
Check it against the bundled `SHA256SUMS.txt` before you install it:

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

Building it yourself needs nothing but a Rust toolchain:

```sh
git clone https://github.com/evg4b/apple-notes-mcp
cd apple-notes-mcp
cargo build --release
cp target/release/apple-notes-mcp /usr/local/bin/
```

## Setting it up

### Claude Desktop

Add this to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "apple-notes": {
      "command": "/usr/local/bin/apple-notes-mcp",
      "args": [
        "--scopes",
        "read,write,delete"
      ]
    }
  }
}
```

Restart Claude Desktop. The first time it calls a tool, macOS will ask whether to allow
automation of Notes. Say yes. If you miss the dialog, you can grant it later under
**System Settings → Privacy & Security → Automation**.

### Anything else

Any client that speaks the stdio transport will work. Point its `command` at the binary and pass
whichever `--scopes` you want.

## Choosing what it can do

Scopes decide which tools exist at all. A tool outside your chosen scopes is never registered,
so the model can't see it, let alone call it.

| Scope    | What it unlocks                                                                                                                                                               |
|----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `read`   | `list_notes`, `get_note`, `search_notes`, `get_all_notes`, `get_notes_in_folder`, `get_notes_in_account`, `get_attachments`, `list_folders`, `get_subfolders`, `list_accounts` |
| `write`  | `create_note`, `update_note`, `append_to_note`, `move_note`, `create_folder`                                                                                                   |
| `delete` | `delete_note`, `delete_folder`                                                                                                                                                |

You get `read` if you don't ask for anything:

```sh
# Read-only, the default
apple-notes-mcp

# Let it write, but not delete
apple-notes-mcp --scopes read,write

# Everything
apple-notes-mcp --scopes read,write,delete
```

Deletes are worth thinking about before you enable them. `delete_note` doesn't move anything to
Recently Deleted, and `delete_folder` takes every note in the folder with it. Neither can be
undone.

## Keeping responses small

Reading notes is the expensive part of any MCP server, because whatever comes back lands in the
model's context and you pay for it. Two defaults keep that under control:

**Bodies come back as plain text.** Notes stores each line as its own `<div>`, usually with
inline styles attached, so the markup is often bigger than the writing. Stripping it roughly
halves what a note costs. If you need the real HTML, ask for it with `format: "html"` — you'll
want that before rewriting a note, so its formatting survives.

**Bulk reads stop at 50 notes.** `get_all_notes` used to hand back every body in your library in
one go, which is an easy way to fill a context window by accident. Now anything that returns
many notes takes a `limit`, and the response sets `truncated` when there was more to see.

For finding things, reach for `search_notes` rather than pulling everything down and filtering.
It only fetches the full record for folders that actually contain a match.

## Command line

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

## More reading

- [Tools reference](docs/tools.md) — what each of the 17 tools takes and gives back
- [Logging & troubleshooting](docs/logging.md) — where the log lives, and what to do when
  something isn't working
