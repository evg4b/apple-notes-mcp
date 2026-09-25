---
name: apple-notes
description: Use this skill when the user wants to interact with Apple Notes on macOS - creating, reading, searching, updating, deleting, moving, or browsing notes, folders, and accounts, through the apple-notes MCP server's tools.
---

# Apple Notes

Reach for this whenever the user talks about their notes: saving something, finding something
they wrote down, tidying up folders. It works against the real Notes.app on their Mac, so
everything you do here is immediately visible to them, and some deletes are permanent.

## When to use it

Whenever the user wants to:

- Save something into Notes, or create a note
- Find a note, whether they remember the title or only roughly what was in it
- Read a note, or everything in a folder
- Change a note: rename it, add to it, rewrite it
- Move a note somewhere else, or organise folders
- Delete a note or a folder
- See what accounts or folders exist

They may say "Apple Notes", "the Notes app", or just "my notes".

## The tools

### Notes

| Tool                   | Scope  | What it does                                          |
|------------------------|--------|-------------------------------------------------------|
| `list_notes`           | read   | Every title, no bodies. The cheapest call there is    |
| `get_note`             | read   | One note by exact title                               |
| `search_notes`         | read   | Find notes by a fragment of the title or body         |
| `get_all_notes`        | read   | Everything, bodies included. Expensive                |
| `get_notes_in_folder`  | read   | Everything in one folder                              |
| `get_notes_in_account` | read   | Everything in one account                             |
| `get_attachments`      | read   | Files attached to a note                              |
| `create_note`          | write  | New note, optionally in a named folder                |
| `update_note`          | write  | Replace the title, the body, or both                  |
| `append_to_note`       | write  | Add to the end, keeping what's there                  |
| `move_note`            | write  | Move a note to a different folder                     |
| `delete_note`          | delete | iCloud: to Recently Deleted. Elsewhere: permanent     |

### Folders and accounts

| Tool             | Scope  | What it does                                        |
|------------------|--------|-----------------------------------------------------|
| `list_folders`   | read   | Every folder and subfolder, with its account        |
| `get_subfolders` | read   | What's nested under one folder                      |
| `list_accounts`  | read   | iCloud, On My Mac, Exchange, whatever's configured  |
| `create_folder`  | write  | New top-level folder                                |
| `delete_folder`  | delete | Permanent, and takes every note in it with it       |

## How to use them well

### Finding something

`search_notes` is almost always the right first call. It matches on titles and bodies, ignores
case, and is far cheaper than pulling the library down and reading through it.

```
User: "Find my notes about the budget"
→ search_notes query="budget"
```

It returns 50 notes at most. If the response comes back with `truncated: true`, say so and
either narrow the query or raise `limit` rather than quietly showing a partial answer as if it
were the whole thing.

`list_notes` is useful when the user wants to browse rather than search, or when you need to
find an exact title to pass to another tool.

Only use `get_all_notes` when the user genuinely wants everything. It returns full bodies and is
the most expensive call in the set.

### Reading

```
User: "What's in my Shopping List note?"
→ get_note title="Shopping List"
```

Bodies come back as plain text by default, which is what you want for reading and summarising.

```
User: "Show me everything in my Work folder"
→ list_folders            (get the exact name)
→ get_notes_in_folder folder="Work"
```

### Creating

`content` is HTML. Wrap plain text in `<div>` tags, one per line:

```
User: "Make a shopping list with milk and eggs"
→ create_note title="Shopping List" content="<div>milk</div><div>eggs</div>"
```

```
User: "Save the project plan in my Work folder"
→ list_folders            (get the exact name)
→ create_note title="Project Plan" content="<div>…</div>" folder="Work"
```

### Adding to a note

Use `append_to_note`. Don't read the body and write it back with `update_note`; that's more
calls and it risks losing formatting.

```
User: "Add butter to my Shopping List"
→ append_to_note title="Shopping List" content="<div>butter</div>"
```

`update_note` is for replacing things, not adding to them:

```
User: "Rename my Draft note to Final Report"
→ update_note title="Draft" new_title="Final Report"
```

If you do need to rewrite a body and keep its formatting, read it as HTML first, edit that, and
write the HTML back:

```
→ get_note title="Notes" format="html"
→ update_note title="Notes" new_content="<the edited HTML>"
```

Passing plain text to `new_content` will flatten whatever formatting the note had.

### Organising

```
User: "Move the project plan into Archive"
→ list_folders            (get the exact name)
→ move_note title="Project Plan" folder="Archive"
```

```
User: "Make a folder called Receipts"
→ create_folder name="Receipts"
```

### Deleting

Confirm with the user first. `delete_note` sends an iCloud note to Recently Deleted, but in other
accounts it's permanent. `delete_folder` permanently removes every note inside the folder.

```
User: "Delete my old TODO note"
→ delete_note title="TODO"
```

## Things worth knowing

**Names must be exact.** Every tool that takes a title or a folder name matches it exactly.
Search or list first when you're not certain. If two notes share a title, tools act on the first
one found. Tell the user when `list_notes` shows duplicates.

**Every account has a "Notes" folder.** A bare folder name can be ambiguous. `create_note`
prefers the default account's folder, and `move_note` looks in the note's own account first. The
other folder tools take the first match, so check `list_folders` when there's more than one
account.

**Recently Deleted is out of reach.** Trashed notes don't show up in listings, searches or title
lookups, so nothing can accidentally edit or permanently delete them. To show the user what's
in the trash, use `get_notes_in_folder folder="Recently Deleted"`.

**Bodies are HTML underneath.** You read plain text by default and write HTML always. Ask for
`format: "html"` when the markup matters.

**Bulk reads are capped.** `get_all_notes`, `get_notes_in_folder`, `get_notes_in_account` and
`search_notes` return 50 notes unless you raise `limit`, and set `truncated` when there was more.

**Not every tool will be there.** Scopes are chosen when the server starts. If a write or delete
tool is missing, the server is running read-only, and you should tell the user that rather than
guessing at a workaround.

**Locked notes read as empty.** A password-protected note comes back with an empty body and
`password_protected: true`, and any write to it is refused. Tell the user to unlock it in Notes.

**Moves stay inside one account.** `move_note` refuses to move a note to a folder in another
account, because Notes would trash the original. If the user asks for that, explain the limit.
Don't work around it with create-then-delete unless they agree.

**Folders are created flat.** `create_folder` makes top-level folders only, though `create_note`
and `move_note` will happily use an existing nested folder.

## When something goes wrong

**`success: false`** — read the `error` field. Usually it means no note or folder matched the
name you gave. Search for the right name and try again.

**A read tool returns an error** — Notes couldn't be queried. Report it rather than treating it as
"no notes".

**Every tool returns nothing** — the Automation permission almost certainly hasn't been granted.
Point the user at **System Settings → Privacy & Security → Automation** and tell them to enable
Notes for their MCP client.

**A body is empty** — the note is password-protected, and the contents aren't available to any
scripting client.
