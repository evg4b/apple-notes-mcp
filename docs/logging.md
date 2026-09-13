# Logging and troubleshooting

## Where the log goes

Everything the server has to say goes to a file. It can't go to stdout, because stdout is the
MCP transport and anything printed there would corrupt the protocol. The file is plain text with
colour codes turned off, so it stays readable in an editor.

|              |                                                                    |
|--------------|--------------------------------------------------------------------|
| Default path | `~/Library/Logs/apple-notes-mcp/apple-notes-mcp.log`               |
| Override     | `--log-file /path/to/file.log`                                     |
| Level        | `--log-level <error\|warn\|info\|debug\|trace>` (default: `error`)  |

By default you'll only see errors, which means a healthy server writes almost nothing. Turn the
level up when you're trying to work out what a client is actually asking for:

```sh
# Send the log somewhere you'll find it
apple-notes-mcp --log-file /tmp/notes-debug.log

# Show what each tool call is doing
apple-notes-mcp --log-level debug

# Both at once
apple-notes-mcp --log-level debug --log-file /tmp/notes-debug.log
```

`debug` logs one line per tool call with the result count. `trace` adds the ScriptingBridge
connection details, which is mostly useful when the server won't start at all.

## When something's wrong

### Everything comes back empty

This is almost always the Automation permission. macOS asks for it once, and if that dialog gets
dismissed, Notes silently returns nothing from then on rather than reporting an error.

1. Open **System Settings → Privacy & Security → Automation**.
2. Find your MCP client in the list, for instance Claude.
3. Turn on **Notes** underneath it.
4. Restart the server.

The log will have said as much, if you had the level turned up:

```
WARN Notes returned 0 accounts — Automation permission is probably missing. ...
```

### "Cannot connect to Apple Notes via ScriptingBridge"

macOS doesn't know about `com.apple.Notes`, which usually means Notes has never been opened on
this machine. Launch Notes once so the bundle ID gets registered, then start the server again.

### A note is found but its body is empty

The note is password-protected. Notes won't hand over the contents of a locked note to any
scripting client, so there's nothing the server can do about it. You can tell these apart from
genuinely empty notes by the `password_protected` field on
[NoteInfo](tools.md#noteinfo), which is only present when it's true.

### A response says `truncated`

That's the result limit doing its job, not an error. Bulk reads stop at 50 notes unless you
raise `limit`, so you don't accidentally pull an entire library into a model's context. Either
raise the limit or, better, narrow what you're asking for with `search_notes`.

### The server doesn't show up in Claude Desktop

- Make sure the path in `claude_desktop_config.json` is absolute. A relative one won't resolve.
- Check the binary is executable: `chmod +x /usr/local/bin/apple-notes-mcp`.
- Restart Claude Desktop. It only reads that config at startup.
