# Tools reference

Everything here goes over the MCP stdio transport. You send a JSON object of parameters and get
a JSON object back.

A few things hold across all of them:

- **Names are matched exactly.** There's no fuzzy matching on note or folder titles. When you're
  not sure of a name, list or search for it first.
- **Reads never fail loudly.** If Notes refuses a request (usually because the Automation
  permission hasn't been granted), a read comes back empty rather than erroring. Writes do tell
  you what went wrong, in an `error` field.
- **Bodies are plain text unless you ask otherwise.** Pass `format: "html"` when you need the
  real markup. There's more on why in [Keeping responses small](#keeping-responses-small).

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

Every note title, and nothing else. This is the cheapest call in the server, because it skips
bodies entirely. It's the right way to see what's there before pulling any single note.

**Parameters:** none

**Returns:**

```json
{ "titles": ["Shopping list", "Meeting notes"] }
```

---

### `get_note`

One note, by its exact title.

**Parameters:**

| Name     | Type    | Description                                 |
|----------|---------|---------------------------------------------|
| `title`  | string  | Exact title of the note                     |
| `format` | string? | `"text"` (default) or `"html"`              |

**Returns:** `{ "note": ` [NoteInfo](#noteinfo) ` }`, with `note` set to `null` if nothing
matches.

If you're about to rewrite the note with `update_note`, read it back with `format: "html"`
first. Writing plain text into a note replaces its formatting.

---

### `search_notes`

Finds notes containing `query`, ignoring case. Searches titles and bodies by default.

This is what you want for "find my notes about X". It reads one batch of titles per folder and
only pulls the full record for folders that actually match, so it's dramatically cheaper than
fetching everything and filtering afterwards.

**Parameters:**

| Name      | Type     | Description                                                  |
|-----------|----------|--------------------------------------------------------------|
| `query`   | string   | Substring to look for, matched case-insensitively            |
| `in_body` | boolean? | Search bodies too, not only titles. Default `true`           |
| `limit`   | integer? | Most notes to return. Default `50`                           |
| `format`  | string?  | `"text"` (default) or `"html"`                               |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …], "truncated": bool }`

Setting `in_body: false` makes the search noticeably faster on a large library, since bodies
never have to be fetched.

---

### `get_all_notes`

Every note in every account, bodies included.

**Parameters:**

| Name     | Type     | Description                          |
|----------|----------|--------------------------------------|
| `limit`  | integer? | Most notes to return. Default `50`   |
| `format` | string?  | `"text"` (default) or `"html"`       |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …], "truncated": bool }`

> This is the most expensive call here, and usually not the one you want. If you're looking for
> something, `search_notes` will find it for a fraction of the cost. If you know the title,
> `get_note` is one call.

The default `limit` exists because an unbounded version of this tool can return a whole library
in a single response. When the limit cuts the result short, `truncated` comes back `true`.

---

### `get_notes_in_folder`

Everything in one folder, by exact folder name. Call `list_folders` first if you're guessing at
the name.

**Parameters:**

| Name     | Type     | Description                        |
|----------|----------|------------------------------------|
| `folder` | string   | Exact folder name                  |
| `limit`  | integer? | Most notes to return. Default `50` |
| `format` | string?  | `"text"` (default) or `"html"`     |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …], "truncated": bool }`

---

### `get_notes_in_account`

Everything belonging to one account. `list_accounts` will tell you what they're called.

**Parameters:**

| Name      | Type     | Description                                    |
|-----------|----------|------------------------------------------------|
| `account` | string   | Account name, e.g. `"iCloud"` or `"On My Mac"` |
| `limit`   | integer? | Most notes to return. Default `50`             |
| `format`  | string?  | `"text"` (default) or `"html"`                 |

**Returns:** `{ "notes": [` [NoteInfo](#noteinfo)`, …], "truncated": bool }`

---

### `get_attachments`

The files attached to a note. Comes back empty both when the note has no attachments and when
there's no such note.

**Parameters:**

| Name    | Type   | Description             |
|---------|--------|-------------------------|
| `title` | string | Exact title of the note |

**Returns:** `{ "attachments": [` [AttachmentInfo](#attachmentinfo)`, …] }`

---

## Folders & Accounts

### `list_folders`

Every folder and subfolder across all accounts, each one telling you which account it belongs to
and what it sits inside. Worth calling before anything that takes a folder name.

**Parameters:** none

**Returns:** `{ "folders": [` [FolderInfo](#folderinfo)`, …] }`

---

### `get_subfolders`

The subfolders of one folder, nested ones included. Empty if the folder has no children, and
also empty if there's no such folder.

**Parameters:**

| Name     | Type   | Description       |
|----------|--------|-------------------|
| `folder` | string | Exact folder name |

**Returns:** `{ "folders": [` [FolderInfo](#folderinfo)`, …] }`

---

### `list_accounts`

Whatever accounts Notes is configured with: iCloud, On My Mac, Exchange, and so on.

**Parameters:** none

**Returns:** `{ "accounts": [` [AccountInfo](#accountinfo)`, …] }`

---

## Writing

These need the `write` scope (`--scopes read,write`).

They all report failure the same way: `success` comes back `false` and `error` says what went
wrong, so you can tell "there's no note called that" apart from "Notes refused the command".

### `create_note`

Makes a new note. Without `folder` it goes wherever Notes puts new notes by default.

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

Replaces a note's title, its body, or both. Leave a field out and it stays as it was.

Be careful with `new_content`: it replaces the whole body, and since it's HTML, writing plain
text into it flattens the note's formatting. Read the current body with
`get_note` and `format: "html"` first if you mean to preserve it. To add a line rather than
rewrite, [`append_to_note`](#append_to_note) is simpler and safer.

**Parameters:**

| Name          | Type    | Description                            |
|---------------|---------|----------------------------------------|
| `title`       | string  | Current exact title of the note        |
| `new_title`   | string? | New title (omit to keep unchanged)     |
| `new_content` | string? | New HTML body (omit to keep unchanged) |

**Returns:** [WriteResponse](#writeresponse)

---

### `append_to_note`

Adds HTML to the end of a note and leaves everything already there alone. This is the right tool
for "add milk to my shopping list".

**Parameters:**

| Name      | Type   | Description                           |
|-----------|--------|---------------------------------------|
| `title`   | string | Exact title of the note               |
| `content` | string | HTML appended to the end of the body  |

**Returns:** [WriteResponse](#writeresponse) — `note.body` is the full body after the append.

---

### `move_note`

Moves a note to a different folder. It keeps its id, its dates and its attachments, so this is a
real move rather than a copy-and-delete.

**Parameters:**

| Name     | Type   | Description                    |
|----------|--------|--------------------------------|
| `title`  | string | Exact title of the note        |
| `folder` | string | Exact destination folder name  |

**Returns:** [WriteResponse](#writeresponse)

---

### `create_folder`

Makes a new top-level folder. You can't create nested ones through this server, though
`create_note` and `move_note` are happy to use a nested folder that already exists.

**Parameters:**

| Name      | Type    | Description                                         |
|-----------|---------|-----------------------------------------------------|
| `name`    | string  | Name of the new folder                              |
| `account` | string? | Account to create it in. Default: the first account |

**Returns:** [FolderWriteResponse](#folderwriteresponse)

---

## Deleting

These need the `delete` scope (`--scopes read,delete`).

Both are permanent. Nothing here goes to Recently Deleted, and there's no undo. It's worth
confirming with whoever asked before calling either one.

### `delete_note`

Deletes a note by exact title, for good.

**Parameters:**

| Name    | Type   | Description                       |
|---------|--------|-----------------------------------|
| `title` | string | Exact title of the note to delete |

**Returns:** [WriteResponse](#writeresponse) — `note` is always absent.

---

### `delete_folder`

Deletes a folder by exact name, **along with every note inside it**. This is the most
destructive call in the server.

**Parameters:**

| Name   | Type   | Description                         |
|--------|--------|-------------------------------------|
| `name` | string | Exact name of the folder to delete  |

**Returns:** [FolderWriteResponse](#folderwriteresponse) — `folder` is always absent.

---

## Keeping responses small

Whatever a tool returns ends up in the model's context, so size matters more here than in an
ordinary API. Two defaults do most of the work.

Bodies are plain text unless you ask for HTML. Notes wraps every line in its own `<div>` and
usually attaches inline styles, so for a typical note the markup outweighs the writing. Stripping
it more than halves the body. `format: "html"` gives you the original whenever you need it.

Bulk reads stop at 50 notes by default and tell you when they stopped, via `truncated`. Raise
`limit` deliberately if you want more.

Beyond those, responses leave out what they can: `shared` and `password_protected` only appear
when true, `truncated` only when it's true, and the optional fields of
[PartialNoteInfo](#partialnoteinfo) only when the operation actually knows them.

## Data types

### NoteInfo

A note and its body.

```json
{
  "id":                "x-coredata://…",
  "title":             "Shopping list",
  "body":              "milk\neggs\nbutter",
  "creation_date":     "2024-01-15 09:30:00 +0000",
  "modification_date": "2024-03-02 14:05:12 +0000",
  "folder":            "Personal",
  "account":           "iCloud"
}
```

`body` is plain text, or HTML when the request asked for `format: "html"`.

`shared` and `password_protected` are booleans that only appear when they're true. A
password-protected note always has an empty `body`, which is the one case where empty content
doesn't mean an empty note.

---

### PartialNoteInfo

What a write already knew about the note it touched. Writes return this instead of re-reading
the note, which would cost another round of Apple Events for information the caller mostly
supplied. Everything but `id` is optional and depends on the operation.

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

A folder, which might sit inside another folder or directly under an account.

```json
{
  "id":      "x-coredata://…",
  "name":    "Work",
  "account": "iCloud",
  "parent":  "iCloud"
}
```

`parent` is whatever the folder sits in directly: an account name for top-level folders, another
folder's name for nested ones.

---

### AccountInfo

One of the accounts Notes is set up with.

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

`url` is empty for attachments stored inline rather than as files on disk.

---

### WriteResponse

What every note write and `delete_note` give back.

```json
{ "success": true, "note": { "id": "x-coredata://…" } }
```

| Field     | Description                                                                    |
|-----------|--------------------------------------------------------------------------------|
| `success` | `true` when the operation went through                                         |
| `note`    | [PartialNoteInfo](#partialnoteinfo). Absent on failure, and always on delete   |
| `error`   | Why it didn't work: no note by that name, or whatever reason Notes gave        |

---

### FolderWriteResponse

What `create_folder` and `delete_folder` give back.

```json
{ "success": true, "folder": { "id": "x-coredata://…", "name": "Work" } }
```

| Field     | Description                                                                |
|-----------|----------------------------------------------------------------------------|
| `success` | `true` when the operation went through                                     |
| `folder`  | [FolderInfo](#folderinfo). Absent on failure, and always on delete         |
| `error`   | Why it didn't work. Absent on success                                      |
