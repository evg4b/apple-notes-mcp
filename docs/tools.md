# Tools Reference

All tools communicate over the MCP stdio transport. Parameters are JSON objects; responses are
JSON objects as described below.

| Tool                                            | Scope    |
|-------------------------------------------------|----------|
| [`list_notes`](#list_notes)                     | `read`   |
| [`get_note`](#get_note)                         | `read`   |
| [`search_notes`](#search_notes)                 | `read`   |
| [`get_all_notes`](#get_all_notes)               | `read`   |
| [`get_notes_in_folder`](#get_notes_in_folder)   | `read`   |
| [`get_notes_in_account`](#get_notes_in_account) | `read`   |
| [`get_attachments`](#get_attachments)           | `read`   |
| [`list_folders`](#list_folders)                 | `read`   |
| [`get_subfolders`](#get_subfolders)             | `read`   |
| [`list_accounts`](#list_accounts)               | `read`   |
| [`create_note`](#create_note)                   | `write`  |
| [`update_note`](#update_note)                   | `write`  |
| [`append_to_note`](#append_to_note)             | `write`  |
| [`move_note`](#move_note)                       | `write`  |
| [`create_folder`](#create_folder)               | `write`  |
| [`delete_note`](#delete_note)                   | `delete` |
| [`delete_folder`](#delete_folder)               | `delete` |

---

## Notes

### `list_notes`

Returns the titles of all notes without fetching body content. Use this for a fast overview
before calling `get_note` on individual items.

**Parameters:** none

**Returns:**

```json
{ "titles": ["Shopping list", "Meeting notes"] }
```

---

### `get_note`

Returns full metadata and HTML body for a single note looked up by exact title.

**Parameters:**

| Name    | Type   | Description             |
|---------|--------|-------------------------|
| `title` | string | Exact title of the note |

**Returns:** `{ "note": ` [NoteInfo](#noteinfo) ` }` — `note` is `null` when no match is found.

---

### `search_notes`

Returns notes whose title — and, by default, body — contains `query`, compared
case-insensitively. This is the cheapest way to find notes by content: titles are fetched in one
batch per folder, and the remaining fields only for folders that actually contain a match.

**Parameters:**

| Name      | Type     | Description                                        |
|-----------|----------|----------------------------------------------------|
| `query`   | string   | Case-insensitive substring to look for             |
| `in_body` | boolean? | Also search bodies, not just titles. Default `true` |
| `limit`   | integer? | Maximum number of notes to return. Default `50`    |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …] }`

---

### `get_all_notes`

Returns full metadata and HTML body for every note across all accounts and folders.

**Parameters:** none

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …] }`

> Fetches the HTML body of every note. On large libraries this is slow — prefer `search_notes`
> for content lookups and `get_note` for a known title.

---

### `get_notes_in_folder`

Returns all notes inside a folder, matched by exact folder name.

**Parameters:**

| Name     | Type   | Description       |
|----------|--------|-------------------|
| `folder` | string | Exact folder name |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …] }`

> Call `list_folders` first if the folder name is unknown.

---

### `get_notes_in_account`

Returns all notes belonging to a specific account, matched by exact account name.

**Parameters:**

| Name      | Type   | Description                                    |
|-----------|--------|------------------------------------------------|
| `account` | string | Account name, e.g. `"iCloud"` or `"On My Mac"` |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …] }`

> Call `list_accounts` first if the account name is unknown.

---

### `get_attachments`

Returns the files attached to a note, matched by exact title. Empty when the note has no
attachments or does not exist.

**Parameters:**

| Name    | Type   | Description             |
|---------|--------|-------------------------|
| `title` | string | Exact title of the note |

**Returns:** `{ "attachments": [` [AttachmentInfo](#attachmentinfo)`, …] }`

---

## Folders & Accounts

### `list_folders`

Returns all folders and subfolders across every account, each with its account and parent name.

**Parameters:** none

**Returns:** `{ "folders": [` [FolderInfo](#folderinfo)`, …] }`

---

### `get_subfolders`

Returns all direct and nested subfolders of a specific folder, matched by exact folder name.
Returns an empty list when the folder has no children or does not exist.

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

## Write Operations

> Write tools require the `write` scope (`--scopes read,write`).

### `create_note`

Creates a new note. Without `folder` it lands in the Notes default folder.

**Parameters:**

| Name      | Type    | Description                                       |
|-----------|---------|---------------------------------------------------|
| `title`   | string  | Title for the new note                            |
| `content` | string  | HTML body, e.g. `"<b>Hello</b> world"`            |
| `folder`  | string? | Destination folder name. Default: default folder  |

**Returns:** [WriteResponse](#writeresponse)

```json
{
  "success": true,
  "note": {
    "id": "x-coredata://…",
    "title": "My note",
    "body": "<div>…HTML…</div>",
    "creation_date": "2024-01-15 09:30:00 +0000",
    "modification_date": "2024-01-15 09:30:00 +0000"
  }
}
```

---

### `update_note`

Replaces the title and/or HTML body of an existing note, matched by exact title. Omit
`new_title` or `new_content` to leave that field unchanged. To add to a body without replacing
it, use [`append_to_note`](#append_to_note).

**Parameters:**

| Name          | Type    | Description                            |
|---------------|---------|----------------------------------------|
| `title`       | string  | Current exact title of the note        |
| `new_title`   | string? | New title (omit to keep unchanged)     |
| `new_content` | string? | New HTML body (omit to keep unchanged) |

**Returns:** [WriteResponse](#writeresponse)

---

### `append_to_note`

Appends HTML to the end of a note's body, leaving existing content intact.

**Parameters:**

| Name      | Type   | Description                           |
|-----------|--------|---------------------------------------|
| `title`   | string | Exact title of the note               |
| `content` | string | HTML appended to the end of the body  |

**Returns:** [WriteResponse](#writeresponse) — `note.body` is the full body after the append.

---

### `move_note`

Moves a note into another folder. The note keeps its id, dates and attachments.

**Parameters:**

| Name     | Type   | Description                    |
|----------|--------|--------------------------------|
| `title`  | string | Exact title of the note        |
| `folder` | string | Exact destination folder name  |

**Returns:** [WriteResponse](#writeresponse)

---

### `create_folder`

Creates a top-level folder. Nested folders are not supported.

**Parameters:**

| Name      | Type    | Description                                         |
|-----------|---------|-----------------------------------------------------|
| `name`    | string  | Name of the new folder                              |
| `account` | string? | Account to create it in. Default: the first account |

**Returns:** [FolderWriteResponse](#folderwriteresponse)

---

## Delete Operations

> Delete tools require the `delete` scope (`--scopes read,delete`).

### `delete_note`

Permanently deletes a note by exact title. Cannot be undone — the note does **not** go to
Recently Deleted.

**Parameters:**

| Name    | Type   | Description                       |
|---------|--------|-----------------------------------|
| `title` | string | Exact title of the note to delete |

**Returns:** [WriteResponse](#writeresponse) — `note` is always absent.

---

### `delete_folder`

Permanently deletes a top-level folder **and every note inside it**, matched by exact name.
Cannot be undone.

**Parameters:**

| Name   | Type   | Description                         |
|--------|--------|-------------------------------------|
| `name` | string | Exact name of the folder to delete  |

**Returns:** [FolderWriteResponse](#folderwriteresponse) — `folder` is always absent.

---

## Data Types

### NoteInfo

Full metadata and HTML body of a single note.

```json
{
  "id":                  "x-coredata://…",
  "title":               "Shopping list",
  "body":                "<div>…HTML…</div>",
  "creation_date":       "2024-01-15 09:30:00 +0000",
  "modification_date":   "2024-03-02 14:05:12 +0000",
  "folder":              "Personal",
  "account":             "iCloud",
  "shared":              false,
  "password_protected":  false
}
```

> Password-protected notes return an empty `body`. Check the `password_protected` field.

---

### PartialNoteInfo

Partial metadata returned by write operations, which avoid a full re-fetch of the note. All
fields except `id` are optional and vary by operation.

```json
{
  "id":                "x-coredata://…",
  "title":             "My note",
  "body":              "<div>…HTML…</div>",
  "creation_date":     "2024-01-15 09:30:00 +0000",
  "modification_date": "2024-01-15 09:30:00 +0000"
}
```

---

### FolderInfo

A Notes folder (may be nested inside another folder or directly under an account).

```json
{
  "id":      "x-coredata://…",
  "name":    "Work",
  "account": "iCloud",
  "parent":  "iCloud"
}
```

`parent` is the immediate container: the account name for top-level folders, or the parent
folder name for nested ones.

---

### AccountInfo

An account configured in Apple Notes (e.g. iCloud, On My Mac, Exchange).

```json
{ "id": "x-coredata://…", "name": "iCloud" }
```

---

### AttachmentInfo

A file attached to a note.

```json
{
  "id":                "x-coredata://…",
  "name":              "diagram.png",
  "creation_date":     "2024-01-15 09:30:00 +0000",
  "modification_date": "2024-01-15 09:30:00 +0000",
  "url":               "file:///…",
  "note_title":        "Design review"
}
```

`url` is empty for inline attachments.

---

### WriteResponse

Returned by every note write and by `delete_note`.

```json
{ "success": true, "note": { "id": "x-coredata://…" } }
```

| Field     | Description                                                            |
|-----------|------------------------------------------------------------------------|
| `success` | `true` when the operation applied                                      |
| `note`    | [PartialNoteInfo](#partialnoteinfo); absent on failure and on delete   |
| `error`   | Why it failed — no such note, or the reason Notes refused the command  |

---

### FolderWriteResponse

Returned by `create_folder` and `delete_folder`.

```json
{ "success": true, "folder": { "id": "x-coredata://…", "name": "Work" } }
```

| Field     | Description                                                      |
|-----------|------------------------------------------------------------------|
| `success` | `true` when the operation applied                                |
| `folder`  | [FolderInfo](#folderinfo); absent on failure and on delete       |
| `error`   | Why it failed. Absent on success                                 |
