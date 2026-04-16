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
exposes Apple Notes to AI assistants via the stdio transport. It uses macOS
[ScriptingBridge](https://developer.apple.com/documentation/scriptingbridge) to talk directly to
Notes.app — no cloud API, no extra processes, no background daemon.

## Requirements

| Requirement           | Details                                                                 |
|-----------------------|-------------------------------------------------------------------------|
| macOS                 | 13 Ventura or later recommended                                         |
| Apple Notes           | Must be open or at least configured (iCloud or On My Mac account)       |
| Automation permission | The binary must be allowed to control Notes.app (prompted on first run) |
| Rust toolchain        | Only needed to build from source                                        |

## Installation

### Build from source

```sh
git clone https://github.com/evg4b/apple-notes-mcp
cd apple-notes-mcp
cargo build --release
# Binary is at: target/release/apple-notes-mcp
```

Copy or symlink it somewhere on your `$PATH`:

```sh
cp target/release/apple-notes-mcp /usr/local/bin/
```

## MCP Client Setup

### Claude Desktop

Add the following to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "apple-notes": {
      "command": "/usr/local/bin/apple-notes-mcp"
    }
  }
}
```

Restart Claude Desktop. On the first tool call macOS will show an Automation permission dialog —
click **OK**. If you miss it, grant permission manually in
**System Settings → Privacy & Security → Automation**.

### Other MCP clients

The server speaks the stdio transport (newline-delimited JSON on stdin/stdout), which is
supported by any compliant MCP client. Point `command` at the binary path.

## Automation Permission

Apple requires explicit user consent before any app can control Notes via AppleScript /
ScriptingBridge. On the first run `apple-notes-mcp` probes Notes.app immediately at startup so
macOS shows the permission dialog before entering the stdio loop.

If the dialog was dismissed or permission was denied:

1. Open **System Settings → Privacy & Security → Automation**.
2. Find the entry for the MCP client (e.g. Claude) and enable **Notes**.
3. Restart the MCP server.

When permission is missing the server still starts, but all tools return empty results and the
log file contains:

```
WARN Notes returned 0 accounts — Automation permission is probably missing. ...
```

## Tools Reference

### Notes

#### `list_notes`

Returns the titles of all notes without fetching body content. Use this for a fast overview
before calling `get_note` on individual items.

**Parameters:** none

**Returns:**

```json
{
  "titles": [
    "Shopping list",
    "Meeting notes",
    "…"
  ]
}
```

---

#### `get_all_notes`

Returns full metadata and content for every note across all accounts and folders.

**Parameters:** none

**Returns:** array of [NoteInfo](#noteinfo) objects wrapped in `{ "notes": […] }`

> This call fetches HTML body and plaintext for every note. On large libraries it can be slow.
> Prefer `list_notes` + `get_note` when you only need a few notes.

---

#### `get_note`

Returns full metadata and content for a single note looked up by title.

**Parameters:**

| Name    | Type   | Description             |
|---------|--------|-------------------------|
| `title` | string | Exact title of the note |

**Returns:** `{ "note": `[NoteInfo](#noteinfo)` }` — `note` is `null` when no match is found.

---

#### `get_notes_in_folder`

Returns all notes inside a folder, matched by folder name.

**Parameters:**

| Name     | Type   | Description       |
|----------|--------|-------------------|
| `folder` | string | Exact folder name |

**Returns:** array of [NoteInfo](#noteinfo) objects wrapped in `{ "notes": […] }`

---

#### `get_notes_in_account`

Returns all notes belonging to a specific account.

**Parameters:**

| Name      | Type   | Description                                    |
|-----------|--------|------------------------------------------------|
| `account` | string | Account name, e.g. `"iCloud"` or `"On My Mac"` |

**Returns:** array of [NoteInfo](#noteinfo) objects wrapped in `{ "notes": […] }`

---

### Folders & Accounts

#### `list_folders`

Returns all folders across all accounts, including nested subfolders, with their account and
parent-folder context.

**Parameters:** none

**Returns:** array of [FolderInfo](#folderinfo) objects wrapped in `{ "folders": […] }`

---

#### `get_subfolders`

Returns all direct and nested subfolders of a specific folder.

**Parameters:**

| Name     | Type   | Description       |
|----------|--------|-------------------|
| `folder` | string | Exact folder name |

**Returns:** array of [FolderInfo](#folderinfo) objects wrapped in `{ "folders": […] }`

---

#### `list_accounts`

Returns all accounts configured in Apple Notes (iCloud, On My Mac, Exchange, etc.).

**Parameters:** none

**Returns:**

```json
{
  "accounts": [
    {
      "id": "x-coredata://…",
      "name": "iCloud",
      "upgraded": true
    },
    {
      "id": "x-coredata://…",
      "name": "On My Mac",
      "upgraded": true
    }
  ]
}
```

---

### Attachments

#### `get_note_attachments`

Returns all attachments embedded in a specific note.

**Parameters:**

| Name    | Type   | Description             |
|---------|--------|-------------------------|
| `title` | string | Exact title of the note |

**Returns:** array of [AttachmentInfo](#attachmentinfo) objects wrapped in `{ "attachments": […] }`

---

#### `get_all_attachments`

Returns every attachment from every note across all accounts.

**Parameters:** none

**Returns:** array of [AttachmentInfo](#attachmentinfo) objects wrapped in `{ "attachments": […] }`

---

### Write Operations

#### `create_note`

Creates a new note in the default Notes folder with the given title and HTML body.

**Parameters:**

| Name      | Type   | Description                            |
|-----------|--------|----------------------------------------|
| `title`   | string | Title for the new note                 |
| `content` | string | HTML body, e.g. `"<b>Hello</b> world"` |

**Returns:** `{ "success": true }`

---

#### `update_note`

Updates the title and/or body of an existing note looked up by its current title.
Omit `new_title` or `new_content` to leave that field unchanged.

**Parameters:**

| Name          | Type    | Description                            |
|---------------|---------|----------------------------------------|
| `title`       | string  | Current title of the note to update    |
| `new_title`   | string? | New title (omit to keep unchanged)     |
| `new_content` | string? | New HTML body (omit to keep unchanged) |

**Returns:** `{ "success": true }` — `false` when no note with `title` was found.

---

#### `delete_note`

Permanently deletes a note by title.

**Parameters:**

| Name    | Type   | Description                       |
|---------|--------|-----------------------------------|
| `title` | string | Exact title of the note to delete |

**Returns:** `{ "success": true }` — `false` when no note with `title` was found.

---

## Data Types

### NoteInfo

```json
{
  "id": "x-coredata://…",
  "title": "Shopping list",
  "body": "<div>…HTML…</div>",
  "plaintext": "Milk\nEggs\n…",
  "creation_date": "2024-01-15 09:30:00 +0000",
  "modification_date": "2024-03-02 14:05:12 +0000",
  "folder": "Personal",
  "account": "iCloud",
  "shared": false,
  "password_protected": false
}
```

### FolderInfo

```json
{
  "id": "x-coredata://…",
  "name": "Work",
  "shared": false,
  "account": "iCloud",
  "parent": "iCloud"
}
```

`parent` is the immediate container: the account name for top-level folders, or the parent
folder name for nested ones.

### AttachmentInfo

```json
{
  "id": "x-coredata://…",
  "name": "diagram.png",
  "content_identifier": "public.png",
  "creation_date": "2024-01-15 09:30:00 +0000",
  "modification_date": "2024-01-15 09:30:00 +0000",
  "url": "file:///…/diagram.png",
  "shared": false,
  "note_title": "Architecture notes"
}
```

---

## Logging

All logs go to a file (stdout is reserved for MCP stdio transport):

|              |                                                       |
|--------------|-------------------------------------------------------|
| Default path | `~/Library/Logs/apple-notes-mcp.log`                  |
| Override     | Set `APPLE_NOTES_MCP_LOG=/path/to/file.log`           |
| Verbosity    | Set `RUST_LOG=debug` (or `trace`) for detailed output |

### RUST_LOG examples

```sh
# Default — info and above
RUST_LOG=info

# Everything
RUST_LOG=debug

# Quiet rmcp, verbose server
RUST_LOG=apple_notes_mcp=trace,rmcp=warn
```

---

## Troubleshooting

**Empty results / `success: false` on every call**
The Automation permission is not granted. See [Automation Permission](#automation-permission).

**`Cannot connect to Apple Notes via ScriptingBridge`**
Notes.app is not installed or the bundle ID `com.apple.Notes` is not registered.
Open Notes at least once to let macOS register it.

**Notes are found but body is empty**
Password-protected notes return empty body and plaintext — check the `password_protected` field.

**Server not appearing in Claude Desktop**
Verify the path in `claude_desktop_config.json` is absolute and the binary is executable
(`chmod +x /usr/local/bin/apple-notes-mcp`).
