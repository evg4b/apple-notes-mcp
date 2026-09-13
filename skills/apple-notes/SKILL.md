---
name: apple-notes
description: Use this skill when the user wants to interact with Apple Notes on macOS - creating, reading, searching, updating, deleting, moving, or browsing notes, folders, and accounts. This skill provides direct access to the Apple Notes app through MCP tools backed by ScriptingBridge (no osascript, no child processes).
---

# Apple Notes Skill

Manage Apple Notes on macOS through natural language. Use it whenever the user mentions notes,
wants to save information to Notes, or needs to retrieve, update, or organize their notes.

## When to Use This Skill

Use this skill when the user:

- Wants to create a new note or save information
- Asks to find or look up a note, by title or by what is written in it
- Wants to read the contents of a note or all notes
- Needs to update, append to, or rename an existing note
- Wants to delete a note, or move one into a different folder
- Asks to browse notes in a specific folder or account
- Wants to list, create, or delete folders, or list accounts
- Mentions Apple Notes, Notes app, or "my notes"

## Available Tools

### Note Operations

| Tool                   | Scope  | Purpose                                                     |
|------------------------|--------|-------------------------------------------------------------|
| `list_notes`           | read   | List every note title (fast — no body fetch)                |
| `get_note`             | read   | Read full content of one note by exact title                |
| `search_notes`         | read   | Find notes by a case-insensitive fragment of title or body  |
| `get_all_notes`        | read   | Read full content of every note (slow on large libraries)   |
| `get_notes_in_folder`  | read   | Read all notes inside a specific folder                     |
| `get_notes_in_account` | read   | Read all notes inside a specific account                    |
| `get_attachments`      | read   | List the files attached to a note                           |
| `create_note`          | write  | Create a note, optionally in a named folder                 |
| `update_note`          | write  | Replace a note's title and/or HTML body                     |
| `append_to_note`       | write  | Add HTML to the end of a note, keeping what is there        |
| `move_note`            | write  | Move a note into another folder                             |
| `delete_note`          | delete | Permanently delete a note (cannot be undone)                |

### Folder & Account Operations

| Tool             | Scope  | Purpose                                                       |
|------------------|--------|---------------------------------------------------------------|
| `list_folders`   | read   | List all folders and subfolders across every account          |
| `get_subfolders` | read   | List direct and nested subfolders of a specific folder        |
| `list_accounts`  | read   | List all configured accounts (iCloud, On My Mac, Exchange…)   |
| `create_folder`  | write  | Create a top-level folder in an account                       |
| `delete_folder`  | delete | Permanently delete a folder **and every note in it**          |

## Usage Patterns

### Finding a Note

`search_notes` is the right first call whenever the exact title is unknown:

```
User: "Find all notes about the budget"
→ search_notes query="budget"
```

It searches titles and bodies by default and returns at most 50 notes. Narrow it with
`in_body=false` to match titles only, or raise/lower `limit`.

Only reach for `get_all_notes` when the user genuinely wants the whole library — it fetches
every note's HTML body and is slow.

### Reading Notes

```
User: "What's in my Shopping List note?"
→ get_note title="Shopping List"
```

```
User: "Show me everything in my Work folder"
→ list_folders  (confirm exact folder name)
→ get_notes_in_folder folder="Work"
```

### Creating Notes

Content must be an HTML string. Wrap plain text in `<div>` tags when no special formatting is
needed:

```
User: "Create a shopping list note with milk and eggs"
→ create_note title="Shopping List" content="<div>milk</div><div>eggs</div>"
```

```
User: "Save the project plan in my Work folder"
→ list_folders  (confirm exact folder name)
→ create_note title="Project Plan" content="<div>…</div>" folder="Work"
```

### Adding to a Note

Use `append_to_note` rather than reading the body and writing it back:

```
User: "Add 'butter' to my Shopping List"
→ append_to_note title="Shopping List" content="<div>butter</div>"
```

Use `update_note` when the user wants to *replace* content or rename the note:

```
User: "Rename my 'Draft' note to 'Final Report'"
→ update_note title="Draft" new_title="Final Report"
```

### Organising Notes

```
User: "Move my Project Plan note into the Archive folder"
→ list_folders  (confirm exact folder name)
→ move_note title="Project Plan" folder="Archive"
```

```
User: "Make a folder called Receipts"
→ create_folder name="Receipts"
```

### Deleting

> **Warning:** deletes are permanent — nothing moves to Recently Deleted. `delete_folder` also
> destroys every note inside the folder. Confirm with the user before either call.

```
User: "Delete my old TODO note"
→ delete_note title="TODO"
```

## Important Guidelines

1. **Exact title matching**: `get_note`, `update_note`, `append_to_note`, `move_note` and
   `delete_note` require the exact note title. If unsure, call `search_notes` or `list_notes`
   first.

2. **HTML content**: Notes store their body as HTML. When reading, `body` contains HTML tags.
   When writing, pass an HTML string — plain text wrapped in `<div>` tags works fine.

3. **Scope availability**: tools are only registered when the server was started with the
   matching scope (`--scopes read,write,delete`). If a write or delete tool is unavailable, tell
   the user the server may be running in read-only mode.

4. **Folders are flat on creation**: `create_folder` and `delete_folder` operate on top-level
   folders only. `create_note` and `move_note` accept any folder name that `list_folders`
   reports.

5. **Performance**: prefer `search_notes` for content lookups and `get_note` for a known title.
   `get_all_notes` fetches every note's HTML body — avoid it on large libraries.

6. **Password-protected notes**: these return an empty `body`. Check `password_protected` on a
   `NoteInfo` and tell the user if it is `true`.

7. **macOS only**: the server talks to Notes.app via ScriptingBridge — no osascript, no cloud
   API.

## Error Handling

- **`success: false` with an `error` field**: the `error` string says what went wrong — usually
  no note or folder matched the given name. Call `search_notes` or `list_folders` to find the
  right name and retry.
- **Empty results from every tool**: the Automation permission for Notes has probably not been
  granted. Direct the user to **System Settings → Privacy & Security → Automation** to enable
  Notes for the MCP client.
- **`body` is empty on a note**: the note is password-protected. It cannot be read or modified
  through this skill.
