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

Returns full metadata and HTML body for every note across all accounts and folders.

**Parameters:** none

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …] }`

> Fetches the HTML body for every note. On large libraries this can be slow.
> Prefer `list_notes` + `get_note` when you only need specific notes.

---

### `get_note`

Returns full metadata and HTML body for a single note looked up by exact title.

**Parameters:**

| Name    | Type   | Description             |
|---------|--------|-------------------------|
| `title` | string | Exact title of the note |

**Returns:** `{ "note": ` [NoteInfo](#noteinfo) ` }` — `note` is `null` when no match is found.

---

### `get_notes_in_folder`

Returns all notes inside a folder, matched by exact folder name.

**Parameters:**

| Name     | Type   | Description            |
|----------|--------|------------------------|
| `folder` | string | Exact folder name      |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …] }`

> Call `list_folders` first if the folder name is unknown.

---

### `get_notes_in_account`

Returns all notes belonging to a specific account, matched by exact account name.

**Parameters:**

| Name      | Type   | Description                                     |
|-----------|--------|-------------------------------------------------|
| `account` | string | Account name, e.g. `"iCloud"` or `"On My Mac"` |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …] }`

> Call `list_accounts` first if the account name is unknown.

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

| Name     | Type   | Description            |
|----------|--------|------------------------|
| `folder` | string | Exact folder name      |

**Returns:** `{ "folders": [` [FolderInfo](#folderinfo)`, …] }`

---

### `list_accounts`

Returns all accounts configured in Apple Notes (iCloud, On My Mac, Exchange, etc.).

**Parameters:** none

**Returns:** `{ "accounts": [` [AccountInfo](#accountinfo)`, …] }`

---

## Write Operations

> Write tools require the `write` scope (`--scopes write` or `--scopes read,write,delete`).

### `create_note`

Creates a new note in the default Notes folder.

**Parameters:**

| Name      | Type   | Description                            |
|-----------|--------|----------------------------------------|
| `title`   | string | Title for the new note                 |
| `content` | string | HTML body, e.g. `"<b>Hello</b> world"` |

**Returns:**

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

`note` contains partial metadata of the created note. See [PartialNoteInfo](#partialnoteinfo).

---

### `update_note`

Updates the title and/or HTML body of an existing note by exact title. Omit `new_title` or
`new_content` to leave that field unchanged. Returns `success: false` when no note with that
title is found.

**Parameters:**

| Name          | Type    | Description                            |
|---------------|---------|----------------------------------------|
| `title`       | string  | Current exact title of the note        |
| `new_title`   | string? | New title (omit to keep unchanged)     |
| `new_content` | string? | New HTML body (omit to keep unchanged) |

**Returns:**

```json
{ "success": true, "note": { … } }
```

`note` is a [PartialNoteInfo](#partialnoteinfo). `success` is `false` when no note with `title`
was found; in that case `note` is omitted.

---

## Delete Operations

> Delete tools require the `delete` scope (`--scopes delete` or `--scopes read,write,delete`).

### `delete_note`

Permanently deletes a note by exact title. Cannot be undone. Returns `success: false` when no
note with that title is found.

**Parameters:**

| Name    | Type   | Description                       |
|---------|--------|-----------------------------------|
| `title` | string | Exact title of the note to delete |

**Returns:**

```json
{ "success": true, "note": { … } }
```

`note` is a [PartialNoteInfo](#partialnoteinfo) of the deleted note. `success` is `false` when
no matching note was found; in that case `note` is omitted.

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

Partial metadata returned by write and delete operations to avoid expensive full-note fetches.

```json
{
  "id":                "x-coredata://…",
  "title":             "My note",
  "body":              "<div>…HTML…</div>",
  "creation_date":     "2024-01-15 09:30:00 +0000",
  "modification_date": "2024-01-15 09:30:00 +0000"
}
```

All fields except `id` are optional and may be absent depending on the operation.

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
