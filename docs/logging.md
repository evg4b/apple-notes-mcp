# Logging & Troubleshooting

## Log file

All output goes to a log file — stdout is reserved for the MCP stdio transport.
The file output is plain text, with ANSI terminal color codes disabled.

|              |                                                            |
|--------------|------------------------------------------------------------|
| Default path | `~/Library/Logs/apple-notes-mcp/apple-notes-mcp.log`      |
| Override     | `--log-file /path/to/file.log`                             |
| Level        | `--log-level <error\|warn\|info\|debug\|trace>` (default: `error`) |

### Examples

```sh
# Use a custom log file
apple-notes-mcp --log-file /tmp/notes-debug.log

# Enable debug output
apple-notes-mcp --log-level debug

# Both together
apple-notes-mcp --log-level debug --log-file /tmp/notes-debug.log
```

---

## Troubleshooting

### Empty results or `success: false` on every call

The Automation permission has not been granted.

1. Open **System Settings → Privacy & Security → Automation**.
2. Find the MCP client (e.g. Claude) and enable **Notes**.
3. Restart the server.

The log file will contain:

```
WARN Notes returned 0 accounts — Automation permission is probably missing. ...
```

---

### `Cannot connect to Apple Notes via ScriptingBridge`

Notes.app is not installed or has never been opened. Open Notes at least once so macOS
registers the `com.apple.Notes` bundle ID, then restart the server.

---

### Notes are found but body is empty

Password-protected notes return an empty `body`. Check the `password_protected` field on the
[NoteInfo](tools.md#noteinfo) object.

---

### Server not appearing in Claude Desktop

- Verify the path in `claude_desktop_config.json` is absolute.
- Ensure the binary is executable: `chmod +x /usr/local/bin/apple-notes-mcp`.
- Restart Claude Desktop after any config change.
