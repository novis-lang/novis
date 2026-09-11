//! The `nvs.*` settings the client configured, and what they mean when it
//! configured nothing.
//!
//! A setting reaches this server exactly once, in `initialize`'s
//! `initializationOptions`, and is read here into one value [`crate::serve`]
//! holds beside its stores. There is no `workspace/configuration` round trip
//! and no `didChangeConfiguration` arm: either would make a setting something
//! that changes under a reader mid-session, and both settings here decide what
//! the server *built* — the tree the symbol index was constructed over, and
//! whether a lens is offered at all. A client that changes one restarts the
//! server, which is what the roster's `nvs.restartServer` is for.
//!
//! **The shape on the wire is the `nvs` section as the client already holds
//! it.** VS Code's `workspace.getConfiguration("nvs")` is a nested object, so
//! `nvs.check.scope` arrives as `{"check": {"scope": "workspace"}}` and the
//! client sends what it read rather than flattening it back into dotted keys.
//! The names are frozen by
//! `rule:ide/contributions-are-frozen-and-only-ever-added`: this module may
//! learn a name and may never rename one it already reads, because the spelling
//! it looks for is the spelling in somebody's `settings.json`.
//!
//! **A value this cannot read leaves the setting at its default.** A misspelled
//! scope, a string where a boolean belongs, a client that sent nothing at all —
//! each takes what `rule:ide/check-scope-defaults-to-open-documents` and the
//! roster say the default is. The alternative is refusing `initialize` over a
//! typo in a `settings.json`, which leaves a developer with no server at all
//! and nothing on screen saying why.

use std::path::PathBuf;

use lsp_types::InitializeParams;
use serde_json::Value;

use crate::document::path_of;
use crate::index::CheckScope;

/// What one `initialize` configured.
///
/// The workspace root is in here with the two settings because it arrives in
/// the same message and answers the same question they do — which files this
/// server is about — and because `CheckScope::Workspace` without a root is the
/// open documents, so the pair is only meaningful read together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// `nvs.check.scope`: which files the symbol index is built over, and
    /// which ones a workspace pass would report diagnostics for.
    pub scope: CheckScope,
    /// `nvs.codeLens.enable`: whether a lens is offered above a declaration.
    ///
    /// Default `true`, and the reason it is a setting at all is that a lens is
    /// a request per visible declaration — a large file is where it costs most
    /// and is wanted least.
    pub code_lens: bool,
    /// The workspace directory a [`CheckScope::Workspace`] pass walks, from
    /// the first folder the client named.
    ///
    /// `None` is a client that named none — a single file opened outside any
    /// project — and under `Workspace` that leaves the open documents, because
    /// guessing a root from an open file's parent would index whatever happened
    /// to be beside it.
    pub root: Option<PathBuf>,
}

impl Default for Settings {
    /// The roster's own defaults, which are what a client that configured
    /// nothing gets.
    fn default() -> Self {
        Self {
            scope: CheckScope::Open,
            code_lens: true,
            root: None,
        }
    }
}

impl Settings {
    /// What `params` configured, with every unset or unreadable value at its
    /// default.
    #[must_use]
    pub fn from_initialize(params: &InitializeParams) -> Self {
        let options = params.initialization_options.as_ref();
        let defaults = Self::default();
        Self {
            scope: at(options, &["check", "scope"])
                .and_then(Value::as_str)
                .and_then(scope_named)
                .unwrap_or(defaults.scope),
            code_lens: at(options, &["codeLens", "enable"])
                .and_then(Value::as_bool)
                .unwrap_or(defaults.code_lens),
            root: root_of(params),
        }
    }
}

/// The value `path` names under the `nvs` section, if the client sent one.
///
/// Every step is fallible in the same way — a key that is absent and a key
/// whose value is not an object are both "the client did not configure this" —
/// so the walk returns `None` rather than distinguishing them.
fn at<'a>(options: Option<&'a Value>, path: &[&str]) -> Option<&'a Value> {
    let mut value = options?;
    for key in path {
        value = value.get(key)?;
    }
    Some(value)
}

/// The scope `setting` spells, or `None` for a spelling that is not one of the
/// two `rule:ide/check-scope-defaults-to-open-documents` names.
fn scope_named(setting: &str) -> Option<CheckScope> {
    match setting {
        "open" => Some(CheckScope::Open),
        "workspace" => Some(CheckScope::Workspace),
        _ => None,
    }
}

/// The workspace directory, from the first folder the client named.
///
/// The first and not a union of them: `SymbolIndex::build` walks one root, and
/// a multi-root workspace is a decision about what a reference list spans
/// rather than a loop written here.
fn root_of(params: &InitializeParams) -> Option<PathBuf> {
    if let Some(folder) = params
        .workspace_folders
        .as_ref()
        .and_then(|folders| folders.first())
        && let Some(path) = path_of(&folder.uri)
    {
        return Some(path);
    }
    // A client too old to send `workspaceFolders` at all still names its one
    // directory here, and reading it is the difference between that client
    // getting a workspace pass and silently getting the open documents.
    #[allow(deprecated)]
    params.root_uri.as_ref().and_then(path_of)
}
