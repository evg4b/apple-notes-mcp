use super::AppleNotesMCP;
use super::models::{
    AccountNotesRequest, AccountsResponse, AttachmentsResponse, BulkNotesRequest, EmptyRequest,
    FolderNotesRequest, FolderRequest, FoldersResponse, GetNoteRequest, NoteResponse,
    NoteTitlesResponse, NotesResponse, SearchRequest, TitleRequest,
};
use anyhow::Result;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, tool};
use tracing::{info, warn};

/// A failed read must be an error: an empty payload reads as "nothing matched",
/// and the client acts on it, for example by creating a note that already
/// exists.
fn read<T>(tool: &'static str, result: Result<T>) -> Result<T, String> {
    result.map_err(|error| {
        warn!(tool, error = format!("{error:#}"), "read failed");
        format!("{error:#}")
    })
}

fn listed<T>(tool: &'static str, count: usize, response: T) -> Result<Json<T>, String> {
    info!(tool, count, "ok");
    Ok(Json(response))
}

fn notes_ok(tool: &'static str, response: NotesResponse) -> Result<Json<NotesResponse>, String> {
    info!(
        tool,
        count = response.notes.len(),
        truncated = response.truncated,
        "ok"
    );
    Ok(Json(response))
}

impl AppleNotesMCP {
    #[tool(
        description = "Titles of every note, no bodies. The cheapest way to see what \
                       exists; follow up with get_note."
    )]
    pub async fn list_notes(
        &self,
        _: Parameters<EmptyRequest>,
    ) -> Result<Json<NoteTitlesResponse>, String> {
        self.blocking(|app| {
            let titles = read("list_notes", app.list_notes())?;
            listed("list_notes", titles.len(), NoteTitlesResponse { titles })
        })
        .await?
    }

    #[tool(
        description = "One note by exact title, with its body. Returns null if nothing \
                       matches — search_notes first when unsure of the title."
    )]
    pub async fn get_note(
        &self,
        Parameters(req): Parameters<GetNoteRequest>,
    ) -> Result<Json<NoteResponse>, String> {
        self.blocking(move |app| {
            let note = read("get_note", app.get_note_by_title(&req.title))?;
            info!(tool = "get_note", found = note.is_some(), "ok");
            Ok(Json(NoteResponse::new(
                note,
                req.format.unwrap_or_default(),
            )))
        })
        .await?
    }

    #[tool(
        description = "Notes whose title, or body unless in_body is false, contains the \
                       query. Case-insensitive. The right way to find notes by content."
    )]
    pub async fn search_notes(
        &self,
        Parameters(req): Parameters<SearchRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        self.blocking(move |app| {
            let page = read(
                "search_notes",
                app.search_notes(&req.query, req.in_body(), req.body.limit()),
            )?;
            notes_ok("search_notes", NotesResponse::new(page, req.body.format()))
        })
        .await?
    }

    #[tool(
        description = "Every note with its body, account by account. Expensive — use \
                       search_notes to find content and get_note for one known title."
    )]
    pub async fn get_all_notes(
        &self,
        Parameters(req): Parameters<BulkNotesRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        self.blocking(move |app| {
            let page = read("get_all_notes", app.get_all_notes(req.body.limit()))?;
            notes_ok("get_all_notes", NotesResponse::new(page, req.body.format()))
        })
        .await?
    }

    #[tool(
        description = "Notes in one folder, nested or not, by exact name. Subfolders' \
                          notes are not included. See list_folders."
    )]
    pub async fn get_notes_in_folder(
        &self,
        Parameters(req): Parameters<FolderNotesRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        self.blocking(move |app| {
            let page = read(
                "get_notes_in_folder",
                app.get_notes_in_folder(&req.folder, req.body.limit()),
            )?;
            notes_ok(
                "get_notes_in_folder",
                NotesResponse::new(page, req.body.format()),
            )
        })
        .await?
    }

    #[tool(description = "Notes in one account, by exact account name. See list_accounts.")]
    pub async fn get_notes_in_account(
        &self,
        Parameters(req): Parameters<AccountNotesRequest>,
    ) -> Result<Json<NotesResponse>, String> {
        self.blocking(move |app| {
            let page = read(
                "get_notes_in_account",
                app.get_notes_in_account(&req.account, req.body.limit()),
            )?;
            notes_ok(
                "get_notes_in_account",
                NotesResponse::new(page, req.body.format()),
            )
        })
        .await?
    }

    #[tool(description = "Files attached to a note, by exact title. Empty if it has none.")]
    pub async fn get_attachments(
        &self,
        Parameters(req): Parameters<TitleRequest>,
    ) -> Result<Json<AttachmentsResponse>, String> {
        self.blocking(move |app| {
            let attachments = read("get_attachments", app.get_note_attachments(&req.title))?;
            listed(
                "get_attachments",
                attachments.len(),
                AttachmentsResponse { attachments },
            )
        })
        .await?
    }

    #[tool(
        description = "Every folder and subfolder, with its account and parent. Call \
                       before any tool that takes a folder name."
    )]
    pub async fn list_folders(
        &self,
        _: Parameters<EmptyRequest>,
    ) -> Result<Json<FoldersResponse>, String> {
        self.blocking(|app| {
            let folders = read("list_folders", app.list_folders())?;
            listed("list_folders", folders.len(), FoldersResponse { folders })
        })
        .await?
    }

    #[tool(description = "Subfolders of one folder, nested ones included. Empty if none.")]
    pub async fn get_subfolders(
        &self,
        Parameters(req): Parameters<FolderRequest>,
    ) -> Result<Json<FoldersResponse>, String> {
        self.blocking(move |app| {
            let folders = read("get_subfolders", app.get_subfolders(&req.folder))?;
            listed("get_subfolders", folders.len(), FoldersResponse { folders })
        })
        .await?
    }

    #[tool(description = "Configured accounts: iCloud, On My Mac, Exchange, and so on.")]
    pub async fn list_accounts(
        &self,
        _: Parameters<EmptyRequest>,
    ) -> Result<Json<AccountsResponse>, String> {
        self.blocking(|app| {
            let accounts = read("list_accounts", app.list_accounts())?;
            listed(
                "list_accounts",
                accounts.len(),
                AccountsResponse { accounts },
            )
        })
        .await?
    }
}
