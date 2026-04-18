//! Manual debug tests for local development.
//!
//! These tests interact with the real Notes.app and are NOT run by default.
//! Run them explicitly:
//!
//!   cargo test -p apple-notes-mcp debug_ -- --ignored --nocapture
//!
//! Individual test:
//!
//!   cargo test -p apple-notes-mcp debug_create_and_delete -- --ignored --nocapture

use super::api::NotesApp;

fn app() -> NotesApp {
    NotesApp::connect().expect("failed to connect to Notes.app")
}

// ── Accounts ─────────────────────────────────────────────────────────────────

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_list_accounts() {
    let accounts = app().list_accounts().unwrap();
    println!("accounts ({}):", accounts.len());
    for a in &accounts {
        println!("  id={:?}  name={:?}", a.id, a.name);
    }
    assert!(!accounts.is_empty());
}

// ── Folders ───────────────────────────────────────────────────────────────────

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_list_folders() {
    let folders = app().list_folders().unwrap();
    println!("folders ({}):", folders.len());
    for f in &folders {
        println!(
            "  id={:?}  name={:?}  account={:?}  parent={:?}",
            f.id, f.name, f.account, f.parent
        );
    }
    assert!(!folders.is_empty());
}

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_get_subfolders() {
    let app = app();
    let folders = app.list_folders().unwrap();
    if let Some(top) = folders.first() {
        let subs = app.get_subfolders(&top.name).unwrap();
        println!("subfolders of {:?} ({}):", top.name, subs.len());
        for f in &subs {
            println!("  id={:?}  name={:?}  parent={:?}", f.id, f.name, f.parent);
        }
    }
}

// ── Notes list ────────────────────────────────────────────────────────────────

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_list_note_titles() {
    let titles = app().list_notes().unwrap();
    println!("note titles ({}):", titles.len());
    for t in titles.iter().take(20) {
        println!("  {:?}", t);
    }
    if titles.len() > 20 {
        println!("  … and {} more", titles.len() - 20);
    }
    assert!(!titles.is_empty());
}

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_get_all_notes() {
    let notes = app().get_all_notes().unwrap();
    println!("all notes ({}):", notes.len());
    for n in notes.iter().take(10) {
        println!(
            "  id={:?}  title={:?}  folder={:?}  account={:?}  shared={}  protected={}",
            n.id, n.title, n.folder, n.account, n.shared, n.password_protected
        );
    }
    if notes.len() > 10 {
        println!("  … and {} more", notes.len() - 10);
    }
    assert!(!notes.is_empty());
}

// ── Notes by folder / account ─────────────────────────────────────────────────

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_notes_in_folder() {
    let app = app();
    let folders = app.list_folders().unwrap();
    if let Some(f) = folders.first() {
        let notes = app.get_notes_in_folder(&f.name).unwrap();
        println!("notes in folder {:?} ({}):", f.name, notes.len());
        for n in &notes {
            println!("  {:?}", n.title);
        }
    }
}

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_notes_in_account() {
    let app = app();
    let accounts = app.list_accounts().unwrap();
    if let Some(acc) = accounts.first() {
        let notes = app.get_notes_in_account(&acc.name).unwrap();
        println!("notes in account {:?} ({}):", acc.name, notes.len());
        for n in notes.iter().take(10) {
            println!("  {:?}", n.title);
        }
    }
}

// ── Single note lookup ────────────────────────────────────────────────────────

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_get_note_by_title() {
    let app = app();
    let titles = app.list_notes().unwrap();
    if let Some(title) = titles.first() {
        let note = app.get_note_by_title(title).unwrap();
        println!("note by title {:?}:", title);
        println!("{:#?}", note);
        assert!(note.is_some());
    }
}

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_get_missing_note() {
    let result = app()
        .get_note_by_title("__debug_nonexistent_note_xyzzy__")
        .unwrap();
    println!("result for missing note: {:?}", result);
    assert!(result.is_none());
}

// ── Create / update / delete cycle ───────────────────────────────────────────

const DEBUG_NOTE_TITLE: &str = "__debug_test_note__";

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_create_and_delete() {
    let app = app();

    // clean up leftovers from a previous aborted run
    let _ = app.delete_note(DEBUG_NOTE_TITLE);

    app.create_note(DEBUG_NOTE_TITLE, "<div>debug body</div>")
        .unwrap();
    println!("created {:?}", DEBUG_NOTE_TITLE);

    let note = app.get_note_by_title(DEBUG_NOTE_TITLE).unwrap();
    println!("fetched: {:#?}", note);
    assert!(note.is_some());

    let deleted = app.delete_note(DEBUG_NOTE_TITLE).unwrap();
    println!("deleted: {}", deleted);
    assert!(deleted);
}

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_create_update_delete() {
    let app = app();

    let _ = app.delete_note(DEBUG_NOTE_TITLE);
    let _ = app.delete_note("__debug_test_note_renamed__");

    app.create_note(DEBUG_NOTE_TITLE, "<div>original</div>")
        .unwrap();

    let ok = app
        .update_note(
            DEBUG_NOTE_TITLE,
            Some("__debug_test_note_renamed__"),
            Some("<div>updated body</div>"),
        )
        .unwrap();
    println!("update returned: {:?}", ok);
    assert!(ok.is_some());

    let note = app
        .get_note_by_title("__debug_test_note_renamed__")
        .unwrap();
    println!("after rename: {:#?}", note);
    assert!(note.is_some());

    app.delete_note("__debug_test_note_renamed__").unwrap();
}

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_delete_nonexistent() {
    let deleted = app()
        .delete_note("__debug_nonexistent_note_xyzzy__")
        .unwrap();
    println!("delete nonexistent -> {}", deleted);
    assert!(!deleted);
}

// ── Note body / HTML inspection ───────────────────────────────────────────────

#[test]
#[ignore = "debug: requires Notes.app with Automation permission"]
fn debug_note_body_html() {
    let app = app();
    let titles = app.list_notes().unwrap();
    if let Some(title) = titles.first() {
        let note = app.get_note_by_title(title).unwrap().unwrap();
        println!("--- HTML body of {:?} ---", note.title);
        println!("{}", note.body);
        println!("--- end ---");
    }
}
