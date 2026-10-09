use super::NotesApp;
use crate::notes::bridge::is_trash;
use anyhow::{Result, bail};
use objc2::msg_send;
use tracing::{debug, instrument};

impl NotesApp {
    /// Refuses Recently Deleted: removing it would erase every trashed note.
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

    /// Accounts with Recently Deleted move the note there. Trashed notes are
    /// never matched, so none is erased for good.
    #[instrument(skip(self))]
    pub fn delete_note(&self, title: &str) -> Result<bool> {
        self.run(|| unsafe {
            let Some(found) = self.find_note(title) else {
                debug!("note not found");
                return Ok(false);
            };
            // SBObject has no static `delete`; removing the element from its
            // collection is the ScriptingBridge way.
            let location = found.location;
            let _: () = msg_send![&*location.notes, removeObjectAtIndex: location.index];
            debug!("note deleted");
            Ok(true)
        })
    }
}
