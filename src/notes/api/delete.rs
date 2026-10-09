use super::NotesApp;
use crate::notes::bridge::is_trash;
use anyhow::{Result, bail};
use objc2::msg_send;
use tracing::{debug, instrument};

impl NotesApp {
    /// Refuses Recently Deleted itself: removing it would erase every trashed
    /// note for good.
    #[instrument(skip(self))]
    pub fn delete_folder(&self, name: &str) -> Result<bool> {
        self.run(|| unsafe {
            let Some(found) = self.find_folder(name, None) else {
                debug!("folder not found");
                return Ok(false);
            };
            if is_trash(name, found.top_level) {
                bail!("{name:?} holds deleted notes and cannot be deleted; empty it in Notes");
            }
            let _: () = msg_send![&*found.parent, removeObjectAtIndex: found.index];
            debug!("folder deleted");
            Ok(true)
        })
    }

    /// Notes moves the note to Recently Deleted in accounts that have one.
    /// Notes already there are never matched, so this cannot erase one for
    /// good.
    #[instrument(skip(self))]
    pub fn delete_note(&self, title: &str) -> Result<bool> {
        self.run(|| unsafe {
            let Some(found) = self.find_note(title) else {
                debug!("note not found");
                return Ok(false);
            };
            // SBObject resolves `delete` dynamically; removing the element from
            // its parent collection is the supported ScriptingBridge spelling.
            let location = found.location;
            let _: () = msg_send![&*location.notes, removeObjectAtIndex: location.index];
            debug!("note deleted");
            Ok(true)
        })
    }
}
