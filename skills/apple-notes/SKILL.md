---
name: apple-notes
description: Use this skill when the user wants to interact with Apple Notes on macOS - creating, reading, updating, deleting, or browsing notes, folders, and accounts. This skill provides direct access to the Apple Notes app through MCP tools backed by ScriptingBridge (no osascript, no child processes).
---

# Apple Notes Skill

This skill enables you to manage Apple Notes on macOS through natural language. Use it whenever the user mentions notes,
wants to save information to Notes, or needs to retrieve, update, or organize their notes.

## When to Use This Skill

Use this skill when the user:

- Wants to create a new note or save information
- Asks to find or look up a note by title
- Wants to read the contents of a note or all notes
- Needs to update or edit an existing note's title or body
- Wants to delete a note
- Asks to browse notes in a specific folder or account
- Wants to list folders, subfolders, or accounts
- Mentions Apple Notes, Notes app, or "my notes"

## Available Tools

### Note Operations

| Tool                   | Scope  | Purpose                                                   |
|------------------------|--------|-----------------------------------------------------------|
| `list_notes`           | read   | List every note title (fast — no body fetch)              |
| `get_note`             | read   | Read full content of one note by exact title              |
| `get_all_notes`        | read   | Read full content of every note (slow on large libraries) |
| `get_notes_in_folder`  | read   | Read all notes inside a specific folder                   |
| `get_notes_in_account` | read   | Read all notes inside a specific account                  |
| `create_note`          | write  | Create a new note in the default folder                   |
| `update_note`          | write  | Change a note's title and/or HTML body                    |
| `delete_note`          | delete | Permanently delete a note (cannot be undone)              |

### Folder & Account Operations

| Tool             | Scope | Purpose                                                     |
|------------------|-------|-------------------------------------------------------------|
| `list_folders`   | read  | List all folders and subfolders across every account        |
| `get_subfolders` | read  | List direct and nested subfolders of a specific folder      |
| `list_accounts`  | read  | List all configured accounts (iCloud, On My Mac, Exchange…) |

## Usage Patterns

### Finding a Note

There is no full-text search tool. Use `list_notes` to browse titles, then `get_note` with the exact title:

```
User: "Show me my meeting notes"
Action:
1. list_notes → scan titles for matches
2. get_note with the exact matching title
```

If the user is looking for notes on a topic across many folders:

```
User: "Find all notes about the budget"
Action: get_all_notes → filter results by inspecting titles and body content
Note: only use get_all_notes when targeted lookup fails; it is slow on large libraries.
```

### Reading Notes

```
User: "What's in my Shopping List note?"
Action: get_note with title="Shopping List"
```

```
User: "Show me everything in my Work folder"
Action:
1. list_folders → confirm exact folder name
2. get_notes_in_folder with folder="Work"
```

### Creating Notes

Content must be an HTML string. Wrap plain text in `<div>` tags when no special formatting is needed:

```
User: "Save this summary as a note called Project Plan"
Action: create_note with title="Project Plan" and content="<div>…</div>"
```

```
User: "Create a shopping list note with milk and eggs"
Action: create_note with title="Shopping List" and content="<div>milk</div><div>eggs</div>"
```

### Updating Notes

Omit `new_title` or `new_content` to leave that field unchanged:

```
User: "Add 'butter' to my Shopping List"
Action:
1. get_note with title="Shopping List" → read current body
2. update_note with title="Shopping List" and new_content=<current body with butter appended>
```

```
User: "Rename my 'Draft' note to 'Final Report'"
Action: update_note with title="Draft" and new_title="Final Report"
```

### Deleting Notes

> **Warning:** `delete_note` permanently deletes the note. It does **not** move it to Recently Deleted. Confirm with the
> user before deleting.

```
User: "Delete my old TODO note"
Action: delete_note with title="TODO"
```

### Browsing Accounts and Folders

```
User: "What accounts do I have in Notes?"
Action: list_accounts
```

```
User: "Show me all my folders"
Action: list_folders
```

```
User: "What subfolders does Work have?"
Action:
1. list_folders → confirm exact name of Work folder
2. get_subfolders with folder="Work"
```

## Important Guidelines

1. **Exact title matching**: `get_note`, `update_note`, and `delete_note` require the exact note title. If unsure, call
   `list_notes` first to find the correct title.

2. **HTML content**: Notes store their body as HTML. When reading, the `body` field contains HTML tags. When writing,
   pass an HTML string — plain text wrapped in `<div>` tags works fine.

3. **Scope availability**: Tools are only registered if the server was started with the matching scope (
   `--scopes read,write,delete`). If a write or delete tool is unavailable, inform the user that the server may be
   running in read-only mode.

4. **Default account**: `create_note` always creates in the default folder of the default account. There is no parameter
   to target a specific folder or account on creation.

5. **Performance**: `get_all_notes` fetches every note's HTML body — avoid it on large libraries. Prefer `get_note`,
   `get_notes_in_folder`, or `get_notes_in_account` when the scope is known.

6. **Password-protected notes**: These notes return an empty `body`. Check the `password_protected` field on a
   `NoteInfo` object and inform the user if it is `true`.

7. **macOS only**: This skill only works on macOS. The server communicates directly with Notes.app via ScriptingBridge —
   no osascript, no cloud API.

## Error Handling

- **`success: false` from update/delete**: No note matched the given title. Call `list_notes` to find the correct title
  and retry.
- **Empty results from all tools**: The Automation permission for Notes has probably not been granted. Direct the user
  to **System Settings → Privacy & Security → Automation** to enable it for the MCP client.
- **`body` is empty on a note**: The note is password-protected. Inform the user — it cannot be read or modified through
  this skill.

## Examples

### Save a conversation summary

```
User: "Save our discussion about the API design to my notes"
→ create_note title="API Design Discussion" content="<div>…summary…</div>"
```

### Daily review

```
User: "Read my TODO note"
→ get_note title="TODO"
```

### Bulk read by folder

```
User: "What are all the notes in my Archive folder?"
→ list_folders  (confirm name)
→ get_notes_in_folder folder="Archive"
```

### Update and rename in one call

```
User: "Rename my Draft note to Final Report and clear the body"
→ update_note title="Draft" new_title="Final Report" new_content="<div></div>"
```

### Browse all iCloud notes

```
User: "List all notes in my iCloud account"
→ list_accounts  (confirm name)
→ get_notes_in_account account="iCloud"
```
