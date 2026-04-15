use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router, Json};

#[derive(Clone)]
pub struct AppleNotesMCP;

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct ListRequest {}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct ListResponse {
    notes: Vec<String>,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct GetNoteRequest {
    title: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct GetNoteResponse {
    title: String,
    content: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct CreateNoteRequest {
    title: String,
    content: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct CreateNoteResponse {
    title: String,
}

#[derive(Clone, serde::Deserialize, schemars::JsonSchema)]
struct DeleteNoteRequest {
    title: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct DeleteNoteResponse {}

#[tool_router(server_handler)]
impl AppleNotesMCP {
    #[tool(description = "List of notes")]
    pub fn list(&self, param: Parameters<ListRequest>) -> Json<ListResponse> {
        // Implement the logic to list notes here
        Json(ListResponse {
            notes: vec!["Note 1".to_string(), "Note 2".to_string()],
        })
    }

    #[tool(description = "Get a specific note")]
    pub fn get_note(&self, params: Parameters<GetNoteRequest>) -> Json<GetNoteResponse> {
        Json(GetNoteResponse {
            title: params.0.title,
            content: "This is the content of the note".to_string(),
        })
    }

    #[tool(description = "Create a new note")]
    pub fn create_note(&self, params: Parameters<CreateNoteRequest>) -> Json<CreateNoteResponse> {
        Json(CreateNoteResponse {
            title: params.0.title,
        })
    }

    #[tool(description = "Delete a note")]
    pub fn delete_note(&self, params: Parameters<DeleteNoteRequest>) -> Json<DeleteNoteResponse> {
        Json(DeleteNoteResponse {})
    }
}
