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
//! **A saved filter is a name, a view and that view's query.** The view is `requests`, `stream`,
//! `log` or `search`, and the query is the part of the view's link after `?`, so the viewer opens a
//! filter the way it opens its link. Its own link is `/saved/<name>`, with the name escaped as a
//! path segment.
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

/// The longest editor setting, in bytes.
const EDITOR_MAX: usize = 200;

/// The largest `PUT /api/settings` body the listener reads.
pub(crate) const BODY_MAX: usize = 1 << 20;

/// The settings file's contents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Settings {
    /// The editor a `file:line` opens in. `vscode` is the default.
    #[serde(default = "default_editor")]
    editor: String,
    #[serde(default)]
    theme: Theme,
    #[serde(default)]
    filters: Vec<Saved>,
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
}

fn default_editor() -> String {
    "vscode".to_owned()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            editor: default_editor(),
            theme: Theme::System,
            filters: Vec::new(),
        }
    }
}

impl Settings {
    /// Why these settings cannot be kept, or `None` when they can.
    fn problem(&self) -> Option<String> {
        if self.editor.is_empty() || self.editor.len() > EDITOR_MAX {
            return Some(format!("`editor` must be 1 to {EDITOR_MAX} bytes long."));
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
        assert_eq!(settings.editor, "vscode");
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
}
