//! The `nvs.*` settings the client configured, and what they mean when it
//! configured nothing.
//!
//! A setting reaches this server exactly once, in `initialize`'s
//! `initializationOptions`, and is read here into one value [`crate::serve`]
//! holds beside its stores. There is no `workspace/configuration` round trip
//! and no `didChangeConfiguration` arm: either would make a setting something
//! that changes under a reader mid-session, and two of these decide what the
//! server *built* — the tree the symbol index was constructed over, and whether
//! a lens is offered at all — which is not a thing the next request can simply
//! be answered differently for. A client that changes one restarts the server,
//! which is what the roster's `nvs.restartServer` is for.
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
//! and nothing on screen saying why.

use std::path::PathBuf;
use std::time::Duration;

use lsp_types::InitializeParams;
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
    /// `nvs.completion.phpNames`: which of the PHP inventory's names a
    /// half-written one is offered beside.
    pub php_names: PhpNames,
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
}

/// What a client does with a completion item beyond inserting its text.
///
/// Every field is `false` for a client that said nothing, and an item for that
/// client is plain text and carries no command
/// (`rule:ide/an-accepted-type-writes-what-follows-it`).
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
}

impl Client {
    /// The command that opens the completion list at the cursor.
    pub const SUGGEST: &'static str = "editor.action.triggerSuggest";
    /// The command that opens signature help at the cursor.
    pub const PARAMETER_HINTS: &'static str = "editor.action.triggerParameterHints";

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
        }
    }
}

/// How much of the PHP inventory a completion offers.
///
/// `rule:php-migration/every-php-builtin-is-a-completion-candidate` makes every
/// built-in a candidate and
/// `rule:ide/three-of-four-item-shapes-insert-nothing` makes three of the four
/// item shapes insert nothing, so the whole layer is worth exactly what the
/// migration table's coverage is worth to the person reading it. That judgement
/// is theirs and not this server's, which is why it is three values rather than
/// a boolean: a developer converting a PHP codebase wants the undecided and
/// dropped names — they are the audit — and one writing new Novis wants only
/// the names that go somewhere, or none at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PhpNames {
    /// Every candidate the inventory lists, whatever its row says. The default,
    /// because a name that does not appear reads as a language that cannot do
    /// the job where one that says *undecided* reads as a language with a
    /// schedule.
    #[default]
    All,
    /// Only the items that insert — a destination the `Core` registry holds.
    Resolved,
    /// No PHP name at all.
    Off,
}

impl Default for Settings {
    /// The roster's own defaults, which are what a client that configured
    /// nothing gets.
    fn default() -> Self {
        Self {
            scope: CheckScope::Workspace,
            code_lens: true,
            php_names: PhpNames::All,
            // The same 150 the manifest declares, because a client that sends
            // the section it holds sends this number anyway and the two
            // disagreeing would be a default nobody can read off either side.
            debounce: Duration::from_millis(150),
            client: Client::default(),
            root: None,
            stubs: None,
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
            php_names: at(options, &["completion", "phpNames"])
                .and_then(Value::as_str)
                .and_then(php_names_named)
                .unwrap_or(defaults.php_names),
            debounce: at(options, &["lsp", "debounce"])
                .and_then(Value::as_u64)
                .map(Duration::from_millis)
                .unwrap_or(defaults.debounce),
            client: Client::from_initialize(params),
            root: root_of(params),
            stubs: at(options, &["stubs", "dir"])
                .and_then(Value::as_str)
                .filter(|dir| !dir.trim().is_empty())
                .map(PathBuf::from),
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
/// two `rule:ide/check-scope-defaults-to-the-workspace` names.
fn scope_named(setting: &str) -> Option<CheckScope> {
    match setting {
        "open" => Some(CheckScope::Open),
        "workspace" => Some(CheckScope::Workspace),
        _ => None,
    }
}

/// The value `setting` spells, or `None` for a spelling that is not one of the
/// three [`PhpNames`] names.
fn php_names_named(setting: &str) -> Option<PhpNames> {
    match setting {
        "all" => Some(PhpNames::All),
        "resolved" => Some(PhpNames::Resolved),
        "off" => Some(PhpNames::Off),
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
