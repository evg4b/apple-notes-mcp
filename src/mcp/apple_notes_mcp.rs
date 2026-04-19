use crate::notes::NotesApp;
use clap::ValueEnum;
use rmcp::handler::server::tool::ToolRouter;
use std::sync::Arc;
use tracing::trace;

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum Scope {
    Read,
    Write,
    Delete,
}

#[derive(Clone)]
pub struct AppleNotesMCP {
    pub(super) app: Arc<NotesApp>,
    pub(super) scopes: Vec<Scope>,
}

impl AppleNotesMCP {
    pub fn new(app: NotesApp, scopes: Vec<Scope>) -> Self {
        Self {
            app: Arc::new(app),
            scopes,
        }
    }

    pub(super) fn tool_router(&self) -> ToolRouter<Self> {
        let mut router = ToolRouter::<Self>::new();
        trace!("adding tools for scopes: {:?}", self.scopes);

        if self.scopes.contains(&Scope::Read) {
            trace!("adding read scope");
            router = router
                .with_route((Self::list_notes_tool_attr(), Self::list_notes))
                .with_route((Self::get_all_notes_tool_attr(), Self::get_all_notes))
                .with_route((Self::get_note_tool_attr(), Self::get_note))
                .with_route((
                    Self::get_notes_in_folder_tool_attr(),
                    Self::get_notes_in_folder,
                ))
                .with_route((
                    Self::get_notes_in_account_tool_attr(),
                    Self::get_notes_in_account,
                ))
                .with_route((Self::list_folders_tool_attr(), Self::list_folders))
                .with_route((Self::get_subfolders_tool_attr(), Self::get_subfolders))
                .with_route((Self::list_accounts_tool_attr(), Self::list_accounts))
        }
        if self.scopes.contains(&Scope::Write) {
            trace!("adding write scope");
            router = router
                .with_route((Self::create_note_tool_attr(), Self::create_note))
                .with_route((Self::update_note_tool_attr(), Self::update_note))
        }
        if self.scopes.contains(&Scope::Delete) {
            trace!("adding delete scope");
            router = router.with_route((Self::delete_note_tool_attr(), Self::delete_note))
        }

        router
    }
}
