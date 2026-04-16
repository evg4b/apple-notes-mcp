# Tools Reference

All tools communicate over the MCP stdio transport. Parameters are JSON objects; responses are
JSON objects as described below.

---

## Notes

### `list_notes`

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

### `get_all_notes`

Returns full metadata and content for every note across all accounts and folders.

**Parameters:** none

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …] }`

> Fetches HTML body and plaintext for every note. On large libraries this can be slow.
> Prefer `list_notes` + `get_note` when you only need specific notes.

---

### `get_note`

Returns full metadata and content for a single note looked up by title.

**Parameters:**

| Name    | Type   | Description             |
|---------|--------|-------------------------|
| `title` | string | Exact title of the note |

**Returns:** `{ "note": ` [NoteInfo](#noteinfo) ` }` — `note` is `null` when no match is found.

---

### `get_notes_in_folder`

Returns all notes inside a folder, matched by name.

**Parameters:**

| Name     | Type   | Description       |
|----------|--------|-------------------|
| `folder` | string | Exact folder name |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …] }`

---

### `get_notes_in_account`

Returns all notes belonging to a specific account.

**Parameters:**

| Name      | Type   | Description                                    |
|-----------|--------|------------------------------------------------|
| `account` | string | Account name, e.g. `"iCloud"` or `"On My Mac"` |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …] }`

---

## Folders & Accounts

### `list_folders`

Returns all folders across all accounts, including nested subfolders, with their account and
parent-folder context.

**Parameters:** none

**Returns:** `{ "folders": [` [FolderInfo](#folderinfo)`, …] }`

---

### `get_subfolders`

Returns all direct and nested subfolders of a specific folder.

**Parameters:**

| Name     | Type   | Description       |
|----------|--------|-------------------|
| `folder` | string | Exact folder name |

**Returns:** `{ "folders": [` [FolderInfo](#folderinfo)`, …] }`

---

### `list_accounts`

Returns all accounts configured in Apple Notes (iCloud, On My Mac, Exchange, etc.).

**Parameters:** none

**Returns:** `{ "accounts": [` [AccountInfo](#accountinfo)`, …] }`

---

## Attachments

### `get_note_attachments`

Returns all attachments embedded in a specific note.

**Parameters:**

| Name    | Type   | Description             |
|---------|--------|-------------------------|
| `title` | string | Exact title of the note |

**Returns:** `{ "attachments": [` [AttachmentInfo](#attachmentinfo)`, …] }`

---

### `get_all_attachments`

Returns every attachment from every note across all accounts.

**Parameters:** none

**Returns:** `{ "attachments": [` [AttachmentInfo](#attachmentinfo)`, …] }`

---

## Write Operations

### `create_note`

Creates a new note in the default Notes folder.

**Parameters:**

| Name      | Type   | Description                            |
|-----------|--------|----------------------------------------|
| `title`   | string | Title for the new note                 |
| `content` | string | HTML body, e.g. `"<b>Hello</b> world"` |

**Returns:** `{ "success": true }`

---

### `update_note`

Updates the title and/or body of an existing note. Omit `new_title` or `new_content` to leave
that field unchanged.

**Parameters:**

| Name          | Type    | Description                            |
|---------------|---------|----------------------------------------|
| `title`       | string  | Current title of the note              |
| `new_title`   | string? | New title (omit to keep unchanged)     |
| `new_content` | string? | New HTML body (omit to keep unchanged) |

**Returns:** `{ "success": true }` — `false` when no note with `title` was found.

---

### `delete_note`

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

### AccountInfo

```json
{
  "id": "x-coredata://…",
  "name": "iCloud",
  "upgraded": true
}
```

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
