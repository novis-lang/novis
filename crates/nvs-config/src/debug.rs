//! `rule:testing/debug-mode-directive`'s `[debug] mode`: the probe set a program run from the
//! command line starts with.
//!
//! **The names are four**, [`NAMES`], one per probe a file may turn on. Any other name is refused
//! where it is written ([`validate`]), so the runtime reader never meets a fifth. Which bit each name
//! arms is `nvs_runtime`'s, because this crate does not link the runtime.
//!
//! **`nvs run` and `nvs test` arm what the file names, and a server refuses it.** `nvs_runtime`'s
//! `Ctx::set_config` arms the named bits on a context that is not server work. A server whose tree
//! names any bit fails its boot and its reload ([`refuse_serving`]): a `trace` probe files two owned
//! strings per compiled call with no ceiling, nothing reads them on a served request, and the
//! `coverage` and `branch` probes count into a table a served request does not have. Coverage of a
//! live server is its own feature, and not designed.
//!
//! **A request cannot set the key.** `RuntimeTighten` in `crate::directive`, but a list is not a
//! quantity `Core\Config::set` takes (`crate::request`), so no request-side call changes it.
//!
//! Cost: one scan of the written list per tree at boot and at reload, and one per context the
//! configuration is handed to. Nothing per request on a server, whose tree names nothing.

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::Config;

/// The dotted key, as `Core\Config::get` names it.
pub const KEY: &str = "debug.mode";

/// Every name `[debug] mode` accepts, one per probe.
pub const NAMES: [&str; 4] = ["coverage", "branch", "trace", "profile"];

/// The names `config` writes, in the order written, and none when nothing is written.
#[must_use]
pub fn named(config: &Config) -> &[String] {
    config
        .debug
        .as_ref()
        .and_then(|debug| debug.mode.as_deref())
        .unwrap_or_default()
}

/// The merged tree's `[debug] mode`, asked whatever command runs it.
///
/// # Errors
///
/// `E0601`, the code a directive with an invalid value gets, for a name outside [`NAMES`].
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(unknown) = named(config)
        .iter()
        .find(|name| !NAMES.contains(&name.as_str()))
    else {
        return Ok(());
    };
    Err(Diagnostic::error(
        code::E_BAD_DIRECTIVE,
        format!("`[debug] mode` names `{unknown}`, which is not a probe"),
    )
    .with_note(format!(
        "the probes are `coverage`, `branch`, `trace` and `profile`{}",
        origin_note(origins.get(KEY))
    ))
    .with_help("remove the name, or write `mode = []` to turn every probe off".to_string()))
}

/// The same tree, asked by a server before it starts or reloads.
///
/// # Errors
///
/// `E0601` when `[debug] mode` names any probe.
pub fn refuse_serving(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let written = named(config);
    if written.is_empty() {
        return Ok(());
    }
    Err(Diagnostic::error(
        code::E_BAD_DIRECTIVE,
        format!(
            "`[debug] mode = [{}]` cannot be used by a server",
            written
                .iter()
                .map(|name| format!("\"{name}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )
    .with_note(format!(
        "the probes run only under `nvs run` and `nvs test`{}",
        origin_note(origins.get(KEY))
    ))
    .with_help("write `mode = []` in the configuration this server reads".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(text: &str) -> Config {
        toml::from_str(text).expect("the fixture parses")
    }

    #[test]
    fn every_probe_name_is_accepted_and_any_other_is_refused() {
        let origins = BTreeMap::new();
        let four = tree("[debug]\nmode = [\"coverage\", \"branch\", \"trace\", \"profile\"]\n");
        assert!(validate(&four, &origins).is_ok());
        let wrong = tree("[debug]\nmode = [\"coverage\", \"tracing\"]\n");
        let refused = validate(&wrong, &origins).expect_err("`tracing` is not a probe");
        assert!(refused.message.contains("`tracing`"), "{}", refused.message);
    }

    #[test]
    fn a_server_refuses_any_probe_and_accepts_none() {
        let origins = BTreeMap::new();
        assert!(refuse_serving(&tree(""), &origins).is_ok());
        assert!(refuse_serving(&tree("[debug]\nmode = []\n"), &origins).is_ok());
        let refused = refuse_serving(&tree("[debug]\nmode = [\"coverage\"]\n"), &origins)
            .expect_err("a server runs no probe");
        assert!(
            refused.message.contains("\"coverage\""),
            "{}",
            refused.message
        );
    }
}
