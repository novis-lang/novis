//! The viewer's settings: the editor its links open, its theme, and the saved filters
//! (`rule:testing/the-viewer-searches-every-record-and-saves-a-filter`,
//! `rule:testing/a-site-in-the-viewer-opens-in-the-editor`).
//!
//! **One file beside the index, outside it.** `GET /api/settings` reads
//! [`nvs_config::data::Folder::viewer_settings`] and `PUT /api/settings` replaces it whole, so
//! discarding the index keeps every saved filter. A file that is not there is the defaults. A file
//! that does not parse is an error and is never overwritten by a read, so a hand edit that went
//! wrong loses nothing. A write goes to a temporary file in the same folder and is renamed over the
//! old one, so a reader sees the old file or the new one and never half of one.
//!
//! **The editor is a preset or a template, and the viewer builds the link.** `editor` is one of
//! `vscode` (the default), `vscode-insiders`, `vscodium`, `cursor`, `zed`, `idea`, `phpstorm` or
//! `custom`, and `custom` takes `editor_template`, a URL with `{file}`, `{line}` and `{column}`.
//! `path_mappings` rewrite a recorded path's prefix first. The viewer's `editor-link.ts` is where
//! a link is built; this module keeps only a template whose scheme cannot run script in the page,
//! so a link is never `javascript:` or `data:`.
//!
//! **A saved filter is a name, a view and that view's query.** The view is `requests`, `stream`,
//! `log`, `search` or `exceptions`, and the query is the part of the view's link after `?`, so the
//! viewer opens a filter the way it opens its link. Its own link is `/saved/<name>`, with the name
//! escaped as a path segment.
//!
//! **What it spends:** one small file in the data folder's private `debug/` directory, read on each
//! `GET` and written on each `PUT`. Its size is bounded by [`FILTERS_MAX`] filters of at most
//! [`QUERY_MAX`] bytes each.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use super::api::error;

/// The most saved filters.
const FILTERS_MAX: usize = 200;

/// The longest name of a saved filter, in characters.
const NAME_MAX: usize = 80;

/// The longest query of a saved filter, in bytes.
const QUERY_MAX: usize = 2_000;

/// The longest editor template, in bytes.
const TEMPLATE_MAX: usize = 200;

/// The most path mappings.
const MAPPINGS_MAX: usize = 20;

/// The longest prefix of a path mapping, on either side, in bytes.
const PREFIX_MAX: usize = 500;

/// The schemes a template may not have: each runs or embeds content in the viewer's own page.
const SCHEMES_REFUSED: [&str; 4] = ["javascript", "data", "vbscript", "blob"];

/// The largest `PUT /api/settings` body the listener reads.
pub(crate) const BODY_MAX: usize = 1 << 20;

/// The settings file's contents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Settings {
    /// The editor a `file:line` opens in.
    #[serde(default)]
    editor: Editor,
    /// The link of a `custom` editor, with `{file}`, `{line}` and `{column}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    editor_template: Option<String>,
    /// The prefixes rewritten before a link is built, the first that matches.
    #[serde(default)]
    path_mappings: Vec<Mapping>,
    #[serde(default)]
    theme: Theme,
    #[serde(default)]
    filters: Vec<Saved>,
}

/// The editors the viewer has a link for, and `custom` for a template.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Editor {
    #[default]
    Vscode,
    VscodeInsiders,
    Vscodium,
    Cursor,
    Zed,
    Idea,
    Phpstorm,
    Custom,
}

/// One path mapping: a recorded path that starts with `from` starts with `to` in the link.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mapping {
    from: String,
    to: String,
}

/// The viewer's colours: the browser's preference, or one theme fixed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

/// One saved filter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    name: String,
    view: View,
    query: String,
}

/// The view a saved filter opens.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum View {
    Requests,
    Stream,
    Log,
    Search,
    Exceptions,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            editor: Editor::Vscode,
            editor_template: None,
            path_mappings: Vec::new(),
            theme: Theme::System,
            filters: Vec::new(),
        }
    }
}

impl Settings {
    /// Why these settings cannot be kept, or `None` when they can.
    fn problem(&self) -> Option<String> {
        match (&self.editor, &self.editor_template) {
            (Editor::Custom, None) => {
                return Some("A `custom` editor needs `editor_template`.".to_owned());
            }
            (Editor::Custom, Some(template)) => {
                if let Some(problem) = template_problem(template) {
                    return Some(problem);
                }
            }
            (_, Some(_)) => {
                return Some(
                    "`editor_template` is read only when `editor` is `custom`.".to_owned(),
                );
            }
            (_, None) => {}
        }
        if self.path_mappings.len() > MAPPINGS_MAX {
            return Some(format!(
                "There can be at most {MAPPINGS_MAX} path mappings."
            ));
        }
        for mapping in &self.path_mappings {
            if mapping.from.is_empty()
                || mapping.from.len() > PREFIX_MAX
                || mapping.to.len() > PREFIX_MAX
                || mapping
                    .from
                    .chars()
                    .chain(mapping.to.chars())
                    .any(char::is_control)
            {
                return Some(format!(
                    "A path mapping's `from` must be 1 to {PREFIX_MAX} bytes and its `to` at most \
                     {PREFIX_MAX}, with no control characters."
                ));
            }
        }
        if self.filters.len() > FILTERS_MAX {
            return Some(format!("There can be at most {FILTERS_MAX} saved filters."));
        }
        for (n, filter) in self.filters.iter().enumerate() {
            let length = filter.name.chars().count();
            if !(1..=NAME_MAX).contains(&length) || filter.name.chars().any(char::is_control) {
                return Some(format!(
                    "A filter's name must be 1 to {NAME_MAX} characters, with no control characters."
                ));
            }
            if filter.name.trim() != filter.name {
                return Some(format!(
                    "The filter name `{}` starts or ends with a space.",
                    filter.name
                ));
            }
            if filter.query.len() > QUERY_MAX {
                return Some(format!(
                    "The filter `{}` has a query longer than {QUERY_MAX} bytes.",
                    filter.name
                ));
            }
            if self.filters[..n]
                .iter()
                .any(|other| other.name == filter.name)
            {
                return Some(format!("Two filters are named `{}`.", filter.name));
            }
        }
        None
    }
}

/// Why `template` cannot be an editor's link, or `None` when it can.
fn template_problem(template: &str) -> Option<String> {
    if template.len() > TEMPLATE_MAX || template.chars().any(char::is_control) {
        return Some(format!(
            "`editor_template` must be at most {TEMPLATE_MAX} bytes, with no control characters."
        ));
    }
    if !template.contains("{file}") {
        return Some("`editor_template` must contain `{file}`.".to_owned());
    }
    let scheme = template.split_once(':').map(|(scheme, _)| scheme);
    let valid = scheme.is_some_and(|scheme| {
        let mut chars = scheme.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    });
    match scheme {
        Some(scheme) if valid => {
            let scheme = scheme.to_ascii_lowercase();
            SCHEMES_REFUSED
                .contains(&scheme.as_str())
                .then(|| format!("`editor_template` cannot use the `{scheme}:` scheme."))
        }
        _ => Some(
            "`editor_template` must start with a URL scheme, such as `myeditor://`.".to_owned(),
        ),
    }
}

/// The settings file, and the lock that keeps two writes from interleaving.
pub(crate) struct Store {
    path: PathBuf,
    writing: Mutex<()>,
}

impl Store {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            writing: Mutex::new(()),
        }
    }

    /// `GET /api/settings`: the status and the JSON body.
    pub(crate) fn get(&self) -> (StatusCode, Vec<u8>) {
        match self.load() {
            Ok(settings) => match serde_json::to_vec(&settings) {
                Ok(body) => (StatusCode::OK, body),
                Err(err) => error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
            },
            Err(message) => error(StatusCode::SERVICE_UNAVAILABLE, &message),
        }
    }

    /// `PUT /api/settings` with `body`, the whole new settings: the status and the JSON body,
    /// which is the settings as kept.
    pub(crate) fn put(&self, body: &[u8]) -> (StatusCode, Vec<u8>) {
        let settings: Settings = match serde_json::from_slice(body) {
            Ok(settings) => settings,
            Err(err) => {
                return error(
                    StatusCode::BAD_REQUEST,
                    &format!("The settings are not valid: {err}"),
                );
            }
        };
        if let Some(problem) = settings.problem() {
            return error(StatusCode::BAD_REQUEST, &problem);
        }
        let _writing = self.writing.lock().unwrap_or_else(PoisonError::into_inner);
        match self.store(&settings) {
            Ok(()) => self.get(),
            Err(err) => error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!(
                    "The settings cannot be written to `{}`: {err}",
                    self.path.display()
                ),
            ),
        }
    }

    /// The file's settings, the defaults when there is no file, or why it cannot be read.
    fn load(&self) -> Result<Settings, String> {
        let text = match std::fs::read(&self.path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Settings::default());
            }
            Err(err) => return Err(format!("`{}` cannot be read: {err}", self.path.display())),
        };
        serde_json::from_slice(&text).map_err(|err| {
            format!(
                "`{}` is not valid: {err}. Correct it or delete it.",
                self.path.display()
            )
        })
    }

    /// Writes `settings` beside the file and renames it over the file.
    fn store(&self, settings: &Settings) -> std::io::Result<()> {
        let mut text = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
        text.push(b'\n');
        let mut staged = self.path.clone().into_os_string();
        staged.push(".new");
        let staged = PathBuf::from(staged);
        std::fs::write(&staged, &text)?;
        std::fs::rename(&staged, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_need_a_unique_plain_name_for_each_filter() {
        let parse = |text: &str| serde_json::from_str::<Settings>(text);
        let settings = parse(
            r#"{"filters":[{"name":"Slow orders","view":"search","query":"q=route%3A%2Forders"}]}"#,
        )
        .expect("valid settings");
        assert_eq!(settings.editor, Editor::Vscode);
        assert_eq!(settings.problem(), None);
        let twice = parse(
            r#"{"filters":[{"name":"a","view":"log","query":""},{"name":"a","view":"log","query":""}]}"#,
        )
        .expect("well formed");
        assert!(
            twice
                .problem()
                .is_some_and(|problem| problem.contains("Two filters"))
        );
        let spaced =
            parse(r#"{"filters":[{"name":" a","view":"log","query":""}]}"#).expect("well formed");
        assert!(spaced.problem().is_some());
        assert!(parse(r#"{"filters":[],"colour":"red"}"#).is_err());
        assert!(parse(r#"{"filters":[{"name":"a","view":"page","query":""}]}"#).is_err());
    }

    #[test]
    fn an_editor_is_a_preset_or_a_template_that_cannot_run_script() {
        let problem = |text: &str| {
            serde_json::from_str::<Settings>(text)
                .expect("well formed")
                .problem()
        };
        assert_eq!(problem(r#"{"editor":"zed"}"#), None);
        assert_eq!(
            problem(
                r#"{"editor":"custom","editor_template":"subl://open?url={file}&line={line}"}"#
            ),
            None
        );
        assert!(problem(r#"{"editor":"custom"}"#).is_some());
        assert!(problem(r#"{"editor":"zed","editor_template":"zed://{file}"}"#).is_some());
        assert!(problem(r#"{"editor":"custom","editor_template":"subl://open"}"#).is_some());
        assert!(
            problem(r#"{"editor":"custom","editor_template":"JavaScript:alert('{file}')"}"#)
                .is_some()
        );
        assert!(problem(r#"{"editor":"custom","editor_template":"/open?{file}"}"#).is_some());
        assert_eq!(
            problem(r#"{"path_mappings":[{"from":"/app/","to":"C:/work/shop/"}]}"#),
            None
        );
        assert!(problem(r#"{"path_mappings":[{"from":"","to":"/x"}]}"#).is_some());
        assert!(serde_json::from_str::<Settings>(r#"{"editor":"notepad"}"#).is_err());
    }
}
