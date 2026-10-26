//! The `nvs.*` settings the client configured, and what they mean when it
//! configured nothing.
//!
//! A setting reaches this server in `initialize`'s `initializationOptions`, and
//! again in each `workspace/didChangeConfiguration` the client sends after a
//! setting changed. Both are read here into one value [`crate::serve`] holds
//! beside its stores. There is no `workspace/configuration` round trip: the
//! client sends the section, and the server never asks for it.
//!
//! **A change is applied between two messages, never under a reader.**
//! [`Settings::change`] runs on the thread that answers every request, so a
//! request sees the settings before the change or after it. It says which
//! settings are different, because two of them decide what the server *built*.
//! A new `nvs.check.scope` rebuilds the symbol index over the new tree and
//! publishes every open document again, since the unused-member dimming
//! depends on it. A new `nvs.codeLens.enable` asks the client to fetch its
//! lenses again. `nvs.lsp.debounce` applies to the next edit. `nvs.stubs.dir`,
//! `coverage.dir` and the workspace root keep the values `initialize` gave
//! them: the client fills in the first two itself, and the last is not a
//! setting. Keeping the
//! server running keeps its index warm, which a restart would throw away.
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
//! each takes what `rule:ide/check-scope-defaults-to-the-workspace` and the
//! roster say the default is. The alternative is refusing `initialize` over a
//! typo in a `settings.json`, which leaves a developer with no server at all
//! and nothing on screen saying why. A change carries the whole section, so a
//! setting it leaves out is one the user removed, and it takes the default
//! too. A change whose `settings` is not an object names no section at all, as
//! a client that expects to be asked for its settings sends, and is ignored.

use std::path::PathBuf;
use std::time::Duration;

use lsp_types::{InitializeParams, ResourceOperationKind};
use serde_json::Value;

use crate::document::path_of;
use crate::index::CheckScope;

/// What one `initialize` configured.
///
/// The workspace root is in here with the settings because it arrives in the
/// same message and answers the same question they do — which files this
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
    /// `nvs.lsp.debounce`: how long an edit waits before the analysis it made
    /// stale runs.
    ///
    /// The wait is the point: a keystroke in the same buffer inside the window
    /// replaces the edit waiting there, so a line being typed is analysed once
    /// rather than once per character. Zero is no wait at all, which is what
    /// this server did before it read the setting.
    ///
    /// Milliseconds on the wire, and a value that is not a whole count of them
    /// — a negative number, a fraction — is one this cannot read and takes the
    /// default like any other.
    pub debounce: Duration,
    /// What the client said it does with a completion item beyond inserting
    /// its text. Not a setting, and here because it arrives in the same
    /// message and is read once the same way.
    pub client: Client,
    /// The workspace directory a [`CheckScope::Workspace`] pass walks, from
    /// the first folder the client named.
    ///
    /// `None` is a client that named none — a single file opened outside any
    /// project — and under `Workspace` that leaves the open documents, because
    /// guessing a root from an open file's parent would index whatever happened
    /// to be beside it.
    pub root: Option<PathBuf>,
    /// `nvs.stubs.dir`: where the `Core` stub tree is written
    /// (`crate::stubs`). `None` is a client that named none, which takes
    /// [`crate::stubs::Stubs::default_dir`] — the VS Code extension names a
    /// directory of its own storage, per server version, and a bare `nvs lsp`
    /// names nothing.
    pub stubs: Option<PathBuf>,
    /// `coverage.dir`: where the client keeps the JSON report of each
    /// program's last coverage run ([`crate::reports`]). Not a setting a user
    /// writes: the VS Code client fills it with a directory of its own
    /// storage, as it fills `stubs.dir`. `None` is a client that named none,
    /// and every test count is then read off the call graph.
    pub coverage: Option<PathBuf>,
    /// The debug index the query lens reads
    /// (`rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site`):
    /// `nvs_config::data::Folder::debug_index` of the data folder, and `None`
    /// where the process has none. Not a setting a client writes.
    pub debug_index: Option<PathBuf>,
}

/// What a client does with a completion item beyond inserting its text, and
/// whether it moves a file a `WorkspaceEdit` names.
///
/// Every field is `false` for a client that said nothing. An item for that
/// client is plain text and carries no command
/// (`rule:ide/an-accepted-type-writes-what-follows-it`), and a rename that
/// would move a file is refused for it.
///
/// The two commands are the editor's own and not this server's, so they are
/// never in `executeCommandProvider`. A client names the ones it runs in
/// `capabilities.experimental.commands`, a list of command ids, and a command
/// it did not name is never sent: a client that does not know an id answers it
/// with an error on every accepted item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Client {
    /// `completionItem.snippetSupport`: an item may place the cursor.
    pub snippets: bool,
    /// The client runs [`Client::SUGGEST`], which opens the completion list.
    pub suggest: bool,
    /// The client runs [`Client::PARAMETER_HINTS`], which opens signature help.
    pub parameter_hints: bool,
    /// The client runs [`Client::SHOW_LOCATIONS`], so a lens may be clicked.
    pub show_locations: bool,
    /// The client runs [`Client::RUN_TESTS`], so a "used by" lens may run
    /// the tests it counts.
    pub run_tests: bool,
    /// The client runs [`Client::OPEN`], so a query lens may open the
    /// request it describes in the debug viewer.
    pub open: bool,
    /// `workspace.codeLens.refreshSupport`: the server may ask the client to
    /// fetch its lenses again, which a changed setting does.
    pub refresh_lenses: bool,
    /// `workspace.workspaceEdit`: the client reads `documentChanges` and its
    /// `resourceOperations` include `rename`, so an edit may move a file.
    pub rename_files: bool,
}

impl Client {
    /// The command that opens the completion list at the cursor.
    pub const SUGGEST: &'static str = "editor.action.triggerSuggest";
    /// The command that opens signature help at the cursor.
    pub const PARAMETER_HINTS: &'static str = "editor.action.triggerParameterHints";
    /// The command that lists places in the editor's reference view. A lens
    /// carries it with the places it counts ([`crate::serve`]'s `code_lens`).
    /// The VS Code client registers it and passes the places to its own
    /// `editor.action.showReferences`, which takes editor objects rather than
    /// the JSON a lens can carry.
    pub const SHOW_LOCATIONS: &'static str = "nvs.showLocations";
    /// The command that runs tests in the editor's own test view. A "used by"
    /// lens carries it with the tests it counts, each as `Class::method`.
    pub const RUN_TESTS: &'static str = "nvs.runTests";
    /// The editor's own command that opens a URI, which a query lens carries
    /// with the viewer's page for the request it describes. VS Code opens an
    /// `http` URI in the browser.
    pub const OPEN: &'static str = "vscode.open";

    /// What `params` declared.
    fn from_initialize(params: &InitializeParams) -> Self {
        let capabilities = &params.capabilities;
        let names = |command: &str| {
            at(capabilities.experimental.as_ref(), &["commands"])
                .and_then(Value::as_array)
                .is_some_and(|commands| commands.iter().any(|id| id.as_str() == Some(command)))
        };
        Self {
            snippets: capabilities
                .text_document
                .as_ref()
                .and_then(|document| document.completion.as_ref())
                .and_then(|completion| completion.completion_item.as_ref())
                .and_then(|item| item.snippet_support)
                .unwrap_or(false),
            suggest: names(Self::SUGGEST),
            parameter_hints: names(Self::PARAMETER_HINTS),
            show_locations: names(Self::SHOW_LOCATIONS),
            run_tests: names(Self::RUN_TESTS),
            open: names(Self::OPEN),
            refresh_lenses: capabilities
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.code_lens.as_ref())
                .and_then(|lens| lens.refresh_support)
                .unwrap_or(false),
            rename_files: capabilities
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.workspace_edit.as_ref())
                .is_some_and(|edit| {
                    edit.document_changes == Some(true)
                        && edit
                            .resource_operations
                            .as_ref()
                            .is_some_and(|kinds| kinds.contains(&ResourceOperationKind::Rename))
                }),
        }
    }
}

impl Default for Settings {
    /// The roster's own defaults, which are what a client that configured
    /// nothing gets.
    fn default() -> Self {
        Self {
            scope: CheckScope::Workspace,
            code_lens: true,
            // The same 150 the manifest declares, because a client that sends
            // the section it holds sends this number anyway and the two
            // disagreeing would be a default nobody can read off either side.
            debounce: Duration::from_millis(150),
            client: Client::default(),
            root: None,
            stubs: None,
            coverage: None,
            debug_index: None,
        }
    }
}

impl Settings {
    /// What `params` configured, with every unset or unreadable value at its
    /// default.
    #[must_use]
    pub fn from_initialize(params: &InitializeParams) -> Self {
        let options = params.initialization_options.as_ref();
        let mut settings = Self {
            client: Client::from_initialize(params),
            root: root_of(params),
            stubs: at(options, &["stubs", "dir"])
                .and_then(Value::as_str)
                .filter(|dir| !dir.trim().is_empty())
                .map(PathBuf::from),
            coverage: at(options, &["coverage", "dir"])
                .and_then(Value::as_str)
                .filter(|dir| !dir.trim().is_empty())
                .map(PathBuf::from),
            debug_index: nvs_config::data::current().map(|folder| folder.debug_index()),
            ..Self::default()
        };
        settings.read(options);
        settings
    }

    /// Applies one `workspace/didChangeConfiguration`'s `settings`, and says
    /// which of the settings that decide what the server built are different.
    ///
    /// `section` is the whole `nvs` section, in the shape `initialize` sends.
    /// Anything that is not an object changes nothing.
    pub fn change(&mut self, section: &Value) -> Changed {
        if !section.is_object() {
            return Changed::default();
        }
        let before = (self.scope, self.code_lens);
        self.read(Some(section));
        Changed {
            scope: self.scope != before.0,
            code_lens: self.code_lens != before.1,
        }
    }

    /// Reads the three settings a change may carry out of `options`, each at
    /// its default where `options` does not say.
    fn read(&mut self, options: Option<&Value>) {
        let defaults = Self::default();
        self.scope = at(options, &["check", "scope"])
            .and_then(Value::as_str)
            .and_then(scope_named)
            .unwrap_or(defaults.scope);
        self.code_lens = at(options, &["codeLens", "enable"])
            .and_then(Value::as_bool)
            .unwrap_or(defaults.code_lens);
        self.debounce = at(options, &["lsp", "debounce"])
            .and_then(Value::as_u64)
            .map(Duration::from_millis)
            .unwrap_or(defaults.debounce);
    }
}

/// Which settings one change made different, of the two that decide what the
/// server built rather than how it answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Changed {
    /// `nvs.check.scope`: the index is rebuilt over the new tree.
    pub scope: bool,
    /// `nvs.codeLens.enable`: the client fetches its lenses again.
    pub code_lens: bool,
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
/// two `rule:ide/check-scope-defaults-to-the-workspace` names.
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
