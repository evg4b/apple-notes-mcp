//! Tests that drive a real Notes.app. They are `#[ignore]`d so `cargo test`
//! stays hermetic; run them with:
//!
//!   cargo test -- --ignored --nocapture
//!
//! They require Notes.app to be installed and the test binary to hold the
//! Automation permission for it. The write tests create and then remove notes
//! and folders prefixed with `__apple_notes_mcp_test`.

use super::api::NotesApp;

const TEST_NOTE: &str = "__apple_notes_mcp_test_note__";
const RENAMED_NOTE: &str = "__apple_notes_mcp_test_note_renamed__";
const TEST_FOLDER: &str = "__apple_notes_mcp_test_folder__";
const LIMIT: usize = 1000;
const MISSING: &str = "__apple_notes_mcp_test_missing_xyzzy__";

fn app() -> NotesApp {
    NotesApp::connect().expect("failed to connect to Notes.app")
}

/// Remove anything a previous aborted run may have left behind.
fn clean(app: &NotesApp) {
    let _ = app.delete_note(TEST_NOTE);
    let _ = app.delete_note(RENAMED_NOTE);
    let _ = app.delete_folder(TEST_FOLDER);
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn accounts_have_ids_and_names() {
    let accounts = app().list_accounts().unwrap();
    assert!(!accounts.is_empty(), "expected at least one account");
    for account in &accounts {
        assert!(!account.id.is_empty(), "empty account id: {account:?}");
        assert!(!account.name.is_empty(), "empty account name: {account:?}");
    }
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn folders_carry_account_and_parent() {
    let folders = app().list_folders().unwrap();
    assert!(!folders.is_empty(), "expected at least one folder");
    for folder in &folders {
        assert!(!folder.name.is_empty(), "empty folder name: {folder:?}");
        assert!(!folder.account.is_empty(), "empty account: {folder:?}");
        assert!(!folder.parent.is_empty(), "empty parent: {folder:?}");
    }
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn subfolders_are_nested_under_their_parent() {
    let app = app();
    let folders = app.list_folders().unwrap();
    let top = folders.first().expect("expected at least one folder");
    for sub in app.get_subfolders(&top.name).unwrap() {
        assert_eq!(sub.account, top.account);
    }
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn subfolders_of_missing_folder_are_empty() {
    assert!(app().get_subfolders(MISSING).unwrap().is_empty());
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn note_titles_are_listed() {
    assert!(!app().list_notes().unwrap().is_empty());
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn all_notes_are_fully_populated() {
    let notes = app().get_all_notes(LIMIT).unwrap().notes;
    let first = notes.first().expect("expected at least one note");
    assert!(!first.id.is_empty(), "empty id: {first:?}");
    assert!(!first.creation_date.is_empty(), "empty created: {first:?}");
    assert!(!first.folder.is_empty(), "empty folder: {first:?}");
    assert!(!first.account.is_empty(), "empty account: {first:?}");
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn every_listed_title_is_retrievable() {
    let app = app();
    let titles = app.list_notes().unwrap();
    let title = titles.first().expect("expected at least one note");
    let note = app.get_note_by_title(title).unwrap().expect("not found");
    assert_eq!(&note.title, title);
    assert!(!note.folder.is_empty(), "empty folder: {note:?}");
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn missing_note_is_none() {
    assert!(app().get_note_by_title(MISSING).unwrap().is_none());
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn notes_in_folder_report_that_folder() {
    let app = app();
    let folders = app.list_folders().unwrap();
    let folder = folders.first().expect("expected at least one folder");
    for note in app.get_notes_in_folder(&folder.name, LIMIT).unwrap().notes {
        assert_eq!(note.folder, folder.name);
    }
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn notes_in_account_report_that_account() {
    let app = app();
    let accounts = app.list_accounts().unwrap();
    let account = accounts.first().expect("expected at least one account");
    for note in app
        .get_notes_in_account(&account.name, LIMIT)
        .unwrap()
        .notes
    {
        assert_eq!(note.account, account.name);
    }
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn notes_in_missing_folder_or_account_are_empty() {
    let app = app();
    assert!(
        app.get_notes_in_folder(MISSING, LIMIT)
            .unwrap()
            .notes
            .is_empty()
    );
    assert!(
        app.get_notes_in_account(MISSING, LIMIT)
            .unwrap()
            .notes
            .is_empty()
    );
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn search_finds_a_note_by_a_fragment_of_its_title() {
    let app = app();
    clean(&app);
    app.create_note(TEST_NOTE, "<div>haystack</div>", None)
        .unwrap();

    let hits = app
        .search_notes("apple_notes_mcp_test_note", false, 10)
        .unwrap()
        .notes;
    assert!(
        hits.iter().any(|n| n.title == TEST_NOTE),
        "search missed the note it should have found"
    );

    clean(&app);
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn search_is_case_insensitive_and_can_match_bodies() {
    let app = app();
    clean(&app);
    app.create_note(TEST_NOTE, "<div>Distinctive Haystack Token</div>", None)
        .unwrap();

    let body_hits = app
        .search_notes("distinctive haystack token", true, 10)
        .unwrap()
        .notes;
    assert!(body_hits.iter().any(|n| n.title == TEST_NOTE));

    let title_only = app
        .search_notes("distinctive haystack token", false, 10)
        .unwrap()
        .notes;
    assert!(!title_only.iter().any(|n| n.title == TEST_NOTE));

    clean(&app);
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn search_respects_the_limit() {
    let app = app();
    assert!(app.search_notes("e", true, 3).unwrap().notes.len() <= 3);
    assert!(app.search_notes("e", true, 0).unwrap().notes.is_empty());
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn a_bulk_read_stops_at_the_limit_and_says_so() {
    let app = app();
    let all = app.get_all_notes(LIMIT).unwrap();
    assert!(!all.truncated, "library is larger than the test limit");

    let page = app.get_all_notes(1).unwrap();
    if all.notes.len() > 1 {
        assert_eq!(page.notes.len(), 1);
        assert!(page.truncated, "a cut-off page should say it was cut off");
    }
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn an_exact_fit_is_not_reported_as_truncated() {
    let app = app();
    let total = app.get_all_notes(LIMIT).unwrap().notes.len();
    let page = app.get_all_notes(total).unwrap();
    assert_eq!(page.notes.len(), total);
    assert!(!page.truncated, "an exact fit is not truncated");
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn stored_bodies_are_html() {
    let app = app();
    clean(&app);
    app.create_note(TEST_NOTE, "<div><b>bold</b> text</div>", None)
        .unwrap();

    let note = app.get_note_by_title(TEST_NOTE).unwrap().unwrap();
    assert!(
        note.body.contains('<'),
        "the notes layer should return raw HTML: {:?}",
        note.body
    );

    clean(&app);
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn search_for_a_missing_term_is_empty() {
    assert!(
        app()
            .search_notes(MISSING, true, 10)
            .unwrap()
            .notes
            .is_empty()
    );
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn attachments_of_a_missing_note_are_empty() {
    assert!(app().get_note_attachments(MISSING).unwrap().is_empty());
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn attachments_name_their_note() {
    let app = app();
    let titles = app.list_notes().unwrap();
    for title in titles.iter().take(20) {
        for attachment in app.get_note_attachments(title).unwrap() {
            assert_eq!(&attachment.note_title, title);
            assert!(!attachment.id.is_empty(), "empty id: {attachment:?}");
        }
    }
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn create_then_delete_round_trips() {
    let app = app();
    clean(&app);

    let created = app.create_note(TEST_NOTE, "<div>body</div>", None).unwrap();
    assert!(!created.id.is_empty());

    let fetched = app.get_note_by_title(TEST_NOTE).unwrap();
    assert!(fetched.is_some(), "created note was not readable back");

    assert!(app.delete_note(TEST_NOTE).unwrap());
    assert!(app.get_note_by_title(TEST_NOTE).unwrap().is_none());
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn update_renames_and_replaces_the_body() {
    let app = app();
    clean(&app);
    app.create_note(TEST_NOTE, "<div>original</div>", None)
        .unwrap();

    let updated = app
        .update_note(TEST_NOTE, Some(RENAMED_NOTE), Some("<div>updated</div>"))
        .unwrap();
    assert!(updated.is_some(), "update reported no such note");

    let note = app.get_note_by_title(RENAMED_NOTE).unwrap().unwrap();
    assert!(note.body.contains("updated"), "body not replaced: {note:?}");

    clean(&app);
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn append_keeps_the_existing_body() {
    let app = app();
    clean(&app);
    app.create_note(TEST_NOTE, "<div>first</div>", None)
        .unwrap();

    app.append_to_note(TEST_NOTE, "<div>second</div>").unwrap();

    let note = app.get_note_by_title(TEST_NOTE).unwrap().unwrap();
    assert!(note.body.contains("first"), "body lost: {note:?}");
    assert!(note.body.contains("second"), "body not appended: {note:?}");

    clean(&app);
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn folder_create_and_delete_round_trips() {
    let app = app();
    clean(&app);

    let folder = app.create_folder(TEST_FOLDER, None).unwrap();
    assert_eq!(folder.name, TEST_FOLDER);
    assert!(!folder.account.is_empty(), "empty account: {folder:?}");
    assert!(
        app.list_folders()
            .unwrap()
            .iter()
            .any(|f| f.name == TEST_FOLDER),
        "created folder is not listed"
    );

    assert!(app.delete_folder(TEST_FOLDER).unwrap());
    assert!(!app.delete_folder(TEST_FOLDER).unwrap());
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn note_can_be_created_in_and_moved_between_folders() {
    let app = app();
    clean(&app);
    app.create_folder(TEST_FOLDER, None).unwrap();

    app.create_note(TEST_NOTE, "<div>body</div>", Some(TEST_FOLDER))
        .unwrap();
    let note = app.get_note_by_title(TEST_NOTE).unwrap().unwrap();
    assert_eq!(note.folder, TEST_FOLDER);

    let default_folder = app
        .list_folders()
        .unwrap()
        .into_iter()
        .find(|f| f.name != TEST_FOLDER)
        .expect("expected another folder to move into");
    app.move_note(TEST_NOTE, &default_folder.name).unwrap();
    let moved = app.get_note_by_title(TEST_NOTE).unwrap().unwrap();
    assert_eq!(moved.folder, default_folder.name);

    clean(&app);
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn writes_to_a_missing_note_report_not_found() {
    let app = app();
    assert!(app.update_note(MISSING, Some("x"), None).unwrap().is_none());
    assert!(app.append_to_note(MISSING, "x").unwrap().is_none());
    assert!(!app.delete_note(MISSING).unwrap());
    assert!(!app.delete_folder(MISSING).unwrap());
}

#[test]
#[ignore = "requires Notes.app with Automation permission"]
fn creating_into_a_missing_folder_is_an_error() {
    let app = app();
    assert!(
        app.create_note(TEST_NOTE, "<div>x</div>", Some(MISSING))
            .is_err()
    );
    assert!(app.create_folder(TEST_FOLDER, Some(MISSING)).is_err());
    clean(&app);
}
