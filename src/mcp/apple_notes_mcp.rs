use super::scope::ScopeSet;
use crate::notes::NotesApp;
use rmcp::handler::server::tool::ToolRouter;
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct AppleNotesMCP {
    pub(super) app: Arc<NotesApp>,
    /// Built once at construction: `list_tools` and `call_tool` run on every
    /// request and must not rebuild the route map.
    pub(super) router: Arc<ToolRouter<Self>>,
}

impl AppleNotesMCP {
    pub fn new(app: NotesApp, scopes: ScopeSet) -> Self {
        let router = Self::build_router(scopes);
        debug!(?scopes, tools = router.map.len(), "tool router built");
        Self {
            app: Arc::new(app),
            router: Arc::new(router),
        }
    }

    pub(super) fn build_router(scopes: ScopeSet) -> ToolRouter<Self> {
        let mut router = ToolRouter::new();

        if scopes.contains(ScopeSet::READ) {
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
                .with_route((Self::search_notes_tool_attr(), Self::search_notes))
                .with_route((Self::get_attachments_tool_attr(), Self::get_attachments))
                .with_route((Self::list_folders_tool_attr(), Self::list_folders))
                .with_route((Self::get_subfolders_tool_attr(), Self::get_subfolders))
                .with_route((Self::list_accounts_tool_attr(), Self::list_accounts))
        }
        if scopes.contains(ScopeSet::WRITE) {
            router = router
                .with_route((Self::create_note_tool_attr(), Self::create_note))
                .with_route((Self::update_note_tool_attr(), Self::update_note))
                .with_route((Self::append_to_note_tool_attr(), Self::append_to_note))
                .with_route((Self::move_note_tool_attr(), Self::move_note))
                .with_route((Self::create_folder_tool_attr(), Self::create_folder))
        }
        if scopes.contains(ScopeSet::DELETE) {
            router = router
                .with_route((Self::delete_note_tool_attr(), Self::delete_note))
                .with_route((Self::delete_folder_tool_attr(), Self::delete_folder))
        }

        router
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::Scope;
    use rmcp::serde_json::Value;

    fn tool_names(scopes: ScopeSet) -> Vec<String> {
        AppleNotesMCP::build_router(scopes)
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect()
    }

    #[test]
    fn no_scopes_registers_no_tools() {
        assert!(tool_names(ScopeSet::default()).is_empty());
    }

    #[test]
    fn read_scope_registers_only_read_tools() {
        let names = tool_names(ScopeSet::READ);
        assert!(names.contains(&"list_notes".to_string()));
        assert!(names.contains(&"get_note".to_string()));
        assert!(!names.contains(&"create_note".to_string()));
        assert!(!names.contains(&"delete_note".to_string()));
    }

    #[test]
    fn write_scope_does_not_leak_read_or_delete_tools() {
        let names = tool_names(ScopeSet::WRITE);
        assert_eq!(
            names,
            vec![
                "append_to_note",
                "create_folder",
                "create_note",
                "move_note",
                "update_note",
            ]
        );
    }

    #[test]
    fn delete_scope_registers_only_delete_tools() {
        assert_eq!(
            tool_names(ScopeSet::DELETE),
            vec!["delete_folder", "delete_note"]
        );
    }

    #[test]
    fn full_access_registers_every_tool() {
        let names = tool_names(ScopeSet::from_iter([
            Scope::Read,
            Scope::Write,
            Scope::Delete,
        ]));
        assert_eq!(names.len(), 17);
        for tool in [
            "list_notes",
            "search_notes",
            "get_attachments",
            "create_note",
            "append_to_note",
            "move_note",
            "create_folder",
            "delete_note",
            "delete_folder",
        ] {
            assert!(names.contains(&tool.to_string()), "missing {tool}");
        }
    }

    #[test]
    fn every_tool_has_a_description() {
        let scopes = ScopeSet::from_iter([Scope::Read, Scope::Write, Scope::Delete]);
        for tool in AppleNotesMCP::build_router(scopes).list_all() {
            let description = tool.description.as_deref().unwrap_or_default();
            assert!(!description.is_empty(), "{} has no description", tool.name);
        }
    }

    #[test]
    fn tool_names_are_unique() {
        let scopes = ScopeSet::from_iter([Scope::Read, Scope::Write, Scope::Delete]);
        let mut names = tool_names(scopes);
        let total = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), total, "duplicate tool names registered");
    }

    fn input_schema(scopes: ScopeSet, tool: &str) -> Value {
        let router = AppleNotesMCP::build_router(scopes);
        let schema = router
            .get(tool)
            .unwrap_or_else(|| panic!("{tool} is not registered"))
            .input_schema
            .as_ref()
            .clone();
        Value::Object(schema)
    }

    #[test]
    fn search_notes_requires_only_a_query() {
        let schema = input_schema(ScopeSet::READ, "search_notes");
        let properties = schema["properties"].as_object().unwrap();
        for field in ["query", "in_body", "limit"] {
            assert!(properties.contains_key(field), "missing {field}");
        }
        let required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(required, ["query"]);
    }

    #[test]
    fn create_note_folder_is_not_required() {
        let schema = input_schema(ScopeSet::WRITE, "create_note");
        let required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(required.contains(&"title"));
        assert!(required.contains(&"content"));
        assert!(!required.contains(&"folder"), "folder should be optional");
    }

    #[test]
    fn destructive_tools_warn_in_their_description() {
        let scopes = ScopeSet::from_iter([Scope::Delete]);
        for tool in AppleNotesMCP::build_router(scopes).list_all() {
            let description = tool.description.as_deref().unwrap_or_default();
            assert!(
                description.contains("Cannot be undone"),
                "{} does not warn that it is destructive",
                tool.name
            );
        }
    }
    /// Every client pays for the whole tool list on every session, and many
    /// keep it in context for every turn afterwards. It is the one payload
    /// whose size is entirely our choice, so hold it to a budget: a verbose new
    /// tool description should have to be argued for, not slip in unnoticed.
    #[test]
    fn the_tool_list_stays_within_its_budget() {
        const BUDGET_BYTES: usize = 23_000;

        let scopes = ScopeSet::from_iter([Scope::Read, Scope::Write, Scope::Delete]);
        let tools = AppleNotesMCP::build_router(scopes).list_all();
        let bytes = rmcp::serde_json::to_string(&tools).unwrap().len();
        assert!(
            bytes <= BUDGET_BYTES,
            "tools/list is {bytes} bytes, over the {BUDGET_BYTES} budget — \
             trim a description or a schema rather than raising the ceiling"
        );
    }

    #[test]
    fn no_single_tool_description_runs_long() {
        const MAX_CHARS: usize = 200;

        let scopes = ScopeSet::from_iter([Scope::Read, Scope::Write, Scope::Delete]);
        for tool in AppleNotesMCP::build_router(scopes).list_all() {
            let description = tool.description.as_deref().unwrap_or_default();
            assert!(
                description.len() <= MAX_CHARS,
                "{} has a {}-char description; keep it under {MAX_CHARS}",
                tool.name,
                description.len()
            );
        }
    }
}
