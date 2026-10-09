mod delete;
mod read;
mod write;

use anyhow::{Result, anyhow, bail};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2_foundation::NSString;
use std::sync::{Arc, Mutex, PoisonError};
use tracing::{info, trace, warn};

use super::bridge::{
    NoteLocation, SBApplication, app_accounts, locate_note_in_folders, obj_folders,
};
use super::delegate::EventErrorDelegate;
use super::helpers::{keys, kvc_get, kvc_index_of, kvc_string_vec, sb_at, sb_count, take_at};

pub struct NotesApp {
    sb_app: Retained<SBApplication>,
    events: Retained<EventErrorDelegate>,
    /// Held for the whole of every operation; see [`NotesApp::run`].
    session: Mutex<()>,
}

// SAFETY: every use of `sb_app` and `events` after construction happens inside
// `run`, which holds `session`, so neither is ever used from two threads at
// once. Neither is tied to the thread that created it.
unsafe impl Send for NotesApp {}
unsafe impl Sync for NotesApp {}

const APPLE_NOTES_BUNDLE_ID: &str = "com.apple.Notes";

impl NotesApp {
    pub fn connect() -> Result<Self> {
        let bundle_id = NSString::from_str(APPLE_NOTES_BUNDLE_ID);
        trace!(
            bundle_id = APPLE_NOTES_BUNDLE_ID,
            "connecting via ScriptingBridge"
        );

        let sb_app = unsafe { SBApplication::applicationWithBundleIdentifier(&bundle_id) }
            .ok_or_else(|| anyhow!("Cannot connect to Apple Notes via ScriptingBridge"))?;
        let events = EventErrorDelegate::new();
        unsafe { sb_app.setDelegate(Some(ProtocolObject::from_ref(&*events))) };
        let app = Self {
            sb_app,
            events,
            session: Mutex::new(()),
        };

        // A failed probe is not fatal: every tool call reports the same error
        // to the client, which is where the user will actually see it.
        match app.list_accounts() {
            Ok(accounts) if accounts.is_empty() => warn!(
                "Notes returned 0 accounts — Automation permission is probably missing. \
                 Go to System Settings → Privacy & Security → Automation and allow \
                 this binary to control Notes.app, then restart."
            ),
            Ok(accounts) => info!(accounts = accounts.len(), "Notes.app connected"),
            Err(error) => warn!(error = format!("{error:#}"), "Notes.app probe failed"),
        }

        Ok(app)
    }

    /// Run one operation against Notes.
    ///
    /// Operations are serialized, which is what makes sharing `NotesApp`
    /// across threads sound. Each one drains its own autorelease pool: the
    /// calling threads have none, so the batch-fetched arrays of note bodies
    /// would otherwise pile up until the thread exits.
    ///
    /// A failed Apple Event makes its call return nil, which reads as "nothing
    /// there"; it is reported here instead, so an error is never mistaken for
    /// an empty result.
    fn run<T>(&self, op: impl FnOnce() -> Result<T>) -> Result<T> {
        let _session = self.session.lock().unwrap_or_else(PoisonError::into_inner);
        autoreleasepool(|_| {
            self.events.take_error();
            let result = op();
            match (self.events.take_error(), result) {
                (None, result) => result,
                (Some(event), Ok(_)) => Err(anyhow!(event)),
                (Some(event), Err(error)) => Err(anyhow!(event).context(error)),
            }
        })
    }

    /// Walks folders instead of the application's flat `notes`, which includes
    /// trashed notes: a write could land on a deleted copy, and deleting that
    /// copy is permanent.
    unsafe fn find_note(&self, title: &str) -> Option<FoundNote> {
        unsafe {
            let target = NSString::from_str(title);
            self.accounts().find_map(|(account, account_name)| {
                locate_note_in_folders(&obj_folders(&account), &target).map(|location| FoundNote {
                    location,
                    account: account_name,
                })
            })
        }
    }

    /// Locate a folder by exact name, top-level or nested, in any account.
    ///
    /// The search goes breadth-first across all accounts, so a top-level folder
    /// wins over a nested one of the same name. `prefer` puts one account's
    /// folders ahead of the rest at every depth. Each level costs one batched
    /// name fetch per folder array, plus one count per folder to descend.
    unsafe fn find_folder(&self, folder_name: &str, prefer: Option<&str>) -> Option<FoundFolder> {
        unsafe {
            let target = NSString::from_str(folder_name);
            let mut level: Vec<(Retained<AnyObject>, Arc<str>)> = self
                .accounts()
                .map(|(account, account_name)| (obj_folders(&account), account_name))
                .collect();
            if let Some(preferred) = prefer {
                level.sort_by_key(|(_, account)| &**account != preferred);
            }
            while !level.is_empty() {
                for (arr, account) in &level {
                    if let Some(index) = kvc_index_of(arr, keys::name(), &target) {
                        return Some(FoundFolder {
                            parent: arr.clone(),
                            index,
                            account: Arc::clone(account),
                        });
                    }
                }
                let mut next = Vec::new();
                for (arr, account) in &level {
                    for i in 0..sb_count(arr) {
                        next.push((obj_folders(&sb_at(arr, i)), Arc::clone(account)));
                    }
                }
                level = next;
            }
            None
        }
    }

    /// The account Notes creates new notes in, falling back to the first one.
    unsafe fn default_account(&self) -> Result<Retained<AnyObject>> {
        unsafe {
            if let Some(account) = kvc_get(self.sb_app.as_ref(), keys::default_account()) {
                return Ok(account);
            }
            let accounts_arr = app_accounts(&self.sb_app);
            if sb_count(&accounts_arr) == 0 {
                bail!("Notes reported no accounts — check Automation permission");
            }
            Ok(sb_at(&accounts_arr, 0))
        }
    }

    unsafe fn find_account(&self, name: &str) -> Option<Retained<AnyObject>> {
        unsafe {
            let accounts_arr = app_accounts(&self.sb_app);
            let index = kvc_index_of(&accounts_arr, keys::name(), &NSString::from_str(name))?;
            Some(sb_at(&accounts_arr, index))
        }
    }

    /// Every account with its name. The names are batch-fetched in one Apple
    /// Event; the accounts themselves are resolved lazily, so a caller that
    /// stops early skips the rest.
    unsafe fn accounts(&self) -> impl Iterator<Item = (Retained<AnyObject>, Arc<str>)> {
        let accounts_arr = unsafe { app_accounts(&self.sb_app) };
        let mut names = unsafe { kvc_string_vec(&accounts_arr, keys::name()) };
        (0..names.len()).map(move |i| {
            let account = unsafe { sb_at(&accounts_arr, i) };
            (account, take_at(&mut names, i).into())
        })
    }
}

struct FoundNote {
    location: NoteLocation,
    account: Arc<str>,
}

/// Kept as a position in the parent's element array so it can be removed as
/// well as read.
struct FoundFolder {
    parent: Retained<AnyObject>,
    index: usize,
    account: Arc<str>,
}

impl FoundFolder {
    unsafe fn folder(&self) -> Retained<AnyObject> {
        unsafe { sb_at(&self.parent, self.index) }
    }
}
