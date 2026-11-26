//! `nvs/debugViewer`: the debug viewer's two links, whether a service runs and whether requests are
//! recorded, for the editor's *Open Debug Viewer* button
//! (`rule:ide/the-debug-viewer-opens-from-one-button`).
//!
//! It is the eighth request of Novis's own (`rule:ide/the-request-set-is-closed`), and its answer
//! is what `nvs debug link --json` prints: `url`, `embed`, `running` and `recording`. Both read
//! the same settings and the same token through [`nvs_config::viewer`], so the link the editor
//! frames is the one the next `nvs debug` admits. The server already reads the program's
//! configuration tree for the query lens, so the editor learns the link without a process of its own.
//!
//! **The tree is the document's**, or the workspace root's when the request names no document, and
//! `[debug] listen`, `token_file` and `store` are read from it through [`nvs_config::Request`], as
//! `nvs debug` reads them. A missing token file is created in the data folder's private `debug/`,
//! which [`nvs_config::data::Folder::prepare_spool`] checks first, exactly as `nvs debug link` does.
//!
//! **What it spends:** one read of the configuration tree and the token, and one lock probe, per
//! request. Nothing is kept.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::document::{Tree, program_tree};

/// The method this request is asked under.
///
/// Namespaced for [`crate::redactions::METHOD`]'s reason: it is not LSP's.
pub const METHOD: &str = "nvs/debugViewer";

/// What the request carries: the document the button was pressed in, when there is one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Params {
    /// The document, as a path. `None` reads the workspace root's tree.
    pub path: Option<PathBuf>,
}

impl Params {
    /// Read one request's params off the wire. Absent params and an absent or unreadable `uri`
    /// each name no document.
    ///
    /// # Errors
    ///
    /// The `serde_json` error for params that are neither absent nor an object.
    pub fn from_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        if value.is_null() {
            return Ok(Self::default());
        }
        let mut fields: serde_json::Map<String, serde_json::Value> = serde_json::from_value(value)?;
        let path = fields
            .remove("uri")
            .and_then(|uri| uri.as_str()?.parse::<lsp_types::Uri>().ok())
            .and_then(|uri| crate::document::path_of(&uri));
        Ok(Self { path })
    }
}

/// The answer, as `nvs debug link --json` prints it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Answer {
    /// The link a browser opens, with `?token=`.
    pub url: String,
    /// The link an editor frames, `/embed/<token>/` under the service's origin.
    pub embed: String,
    /// Whether an `nvs debug` holds the data folder's service lock.
    pub running: bool,
    /// Whether `[debug] store` is on, so requests are recorded.
    pub recording: bool,
}

/// The settings the answer is made from, read from one configuration tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    /// `[debug] listen`, as written.
    pub listen: String,
    /// `[debug] token_file`.
    pub token_file: Option<PathBuf>,
    /// `[debug] store`, after the run mode's defaults.
    pub store: bool,
}

/// The settings of the tree `params` names, with `root` for a request that names no document.
#[must_use]
pub fn read(params: &Params, root: Option<&Path>) -> Read {
    let path = match (&params.path, root) {
        (Some(path), _) => path.clone(),
        // `program_tree` reads the tree of a document's folder, so the root's tree is the tree of
        // a document in it.
        (None, Some(root)) => root.join("nvs.toml"),
        (None, None) => PathBuf::from("nvs.toml"),
    };
    let data = nvs_config::data::config_file();
    match program_tree(&path, data.as_deref()) {
        Tree::Read { snapshot, .. } => {
            let request = nvs_config::Request::new(snapshot);
            Read {
                listen: request
                    .get("debug.listen")
                    .unwrap_or_else(|| nvs_config::defaults::DEBUG_LISTEN.to_owned()),
                token_file: request.get("debug.token_file").map(PathBuf::from),
                store: request.debug_store(),
            }
        }
        _ => Read {
            listen: nvs_config::defaults::DEBUG_LISTEN.to_owned(),
            token_file: None,
            store: nvs_config::defaults::DEBUG_STORE,
        },
    }
}

/// The answer for `read` in the data folder `folder`, or the message the editor shows.
///
/// # Errors
///
/// When `[debug] listen` is not an IP address and a port or has the port `0`, when the data folder
/// cannot be used, and when the token cannot be read or created.
pub fn answer(read: &Read, folder: &nvs_config::data::Folder) -> Result<Answer, String> {
    let Ok(addr) = read.listen.parse::<SocketAddr>() else {
        return Err(format!(
            "`[debug] listen` is `{}`, and it must be an IP address and a port, such as \
             `127.0.0.1:8137`",
            read.listen
        ));
    };
    if addr.port() == 0 {
        return Err("`[debug] listen` has the port `0`, so `nvs debug` picks a port when it \
                    starts. Open the link that `nvs debug` printed."
            .to_owned());
    }
    folder
        .prepare_spool()
        .map_err(|unusable| format!("The debug viewer cannot use the data folder: {}", unusable.reason()))?;
    let [_, _, own_token] = folder.viewer_secrets();
    let token = nvs_config::viewer::token(read.token_file.as_deref(), &own_token)?;
    let lock = folder.service_lock();
    let running = nvs_config::viewer::probe(&lock)
        .map_err(|err| format!("cannot read `{}`: {err}", lock.display()))?;
    let (url, embed) = nvs_config::viewer::links(addr, &token);
    Ok(Answer {
        url,
        embed,
        running,
        recording: read.store,
    })
}

/// The answer in this process's data folder.
///
/// # Errors
///
/// [`answer`]'s, and when there is no data folder.
pub fn answer_here(params: &Params, root: Option<&Path>) -> Result<Answer, String> {
    let folder = nvs_config::data::current()
        .ok_or_else(|| "The debug viewer needs a data folder, and Novis cannot use one.".to_owned())?;
    answer(&read(params, root), folder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_answer_is_the_link_the_service_admits_and_no_service_runs_in_a_new_folder() {
        let dir = nvs_repo::scratch("lsp-debug-viewer");
        let folder = nvs_config::data::Folder::new(dir.path().join(".nvsdata"));
        let read = Read {
            listen: "127.0.0.1:8137".to_owned(),
            token_file: None,
            store: true,
        };
        let first = answer(&read, &folder).expect("a new folder gives an answer");
        let token = std::fs::read_to_string(folder.viewer_secrets()[2].clone()).unwrap();
        assert_eq!(first.url, format!("http://127.0.0.1:8137/?token={}", token.trim()));
        assert_eq!(first.embed, format!("http://127.0.0.1:8137/embed/{}/", token.trim()));
        assert!(!first.running);
        assert!(first.recording);
        assert_eq!(answer(&read, &folder), Ok(first), "the token is kept");
        let zero = Read {
            listen: "127.0.0.1:0".to_owned(),
            ..read
        };
        assert!(answer(&zero, &folder).unwrap_err().contains("port `0`"));
    }

    #[test]
    fn params_name_a_document_by_its_uri_or_name_none() {
        assert_eq!(Params::from_value(serde_json::Value::Null).unwrap(), Params::default());
        assert_eq!(Params::from_value(serde_json::json!({})).unwrap(), Params::default());
        assert!(Params::from_value(serde_json::json!(3)).is_err());
    }
}
