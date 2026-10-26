//! `rule:testing/debug-mode-directive`'s `[debug] mode`: the probe set a program run from the
//! command line starts with.
//!
//! **The names are four**, [`NAMES`], one per probe a file may turn on. Any other name is refused
//! where it is written ([`validate`]), so the runtime reader never meets a fifth. Which bit each name
//! arms is `nvs_runtime`'s, because this crate does not link the runtime.
//!
//! **`nvs run` and `nvs test` arm what the file names, and a server refuses all but one.**
//! `nvs_runtime`'s `Ctx::set_config` arms the named bits on a context that is not server work. A
//! server whose tree names `branch`, `trace` or `profile` fails its boot and its reload
//! ([`refuse_serving`]): a `trace` probe files two owned strings per compiled call with no ceiling,
//! nothing reads them on a served request, and `branch` counts edges nothing on a server reads. A
//! server whose `[mode] default` is `development` may name `coverage` ([`counts_served`]): its
//! compiler builds a located table beside each unit, and each development request counts into a copy
//! of its own, which the debug viewer draws over the source. Whether a server counts is fixed at its
//! boot, so a reload that changes it is refused ([`refuse_switch`]).
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

/// Whether a server under `config` counts coverage: its `[debug] mode` names `coverage`, and its
/// `[mode] default` is `development`.
#[must_use]
pub fn counts_served(config: &Config) -> bool {
    named(config).iter().any(|name| name == "coverage") && development(config)
}

/// Whether `config`'s `[mode] default` is `development`.
fn development(config: &Config) -> bool {
    config
        .mode
        .as_ref()
        .and_then(|mode| mode.default.as_deref())
        == Some(crate::mode::DEVELOPMENT)
}

/// The same tree, asked by a server before it starts or reloads.
///
/// # Errors
///
/// `E0601` when `[debug] mode` names any probe but `coverage`, or names `coverage` and the
/// `[mode] default` is not `development`.
pub fn refuse_serving(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let written = named(config);
    if written.is_empty() || (written.iter().all(|name| name == "coverage") && development(config))
    {
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
        "a server may count `coverage` when its `[mode] default` is `development`, and runs no \
         other probe{}",
        origin_note(origins.get(KEY))
    ))
    .with_help("write `mode = []` in the configuration this server reads".to_string()))
}

/// A reload's tree, asked by a server that booted counting coverage when `counting` is set.
///
/// # Errors
///
/// `E0601` when the reload would start or stop counting coverage.
pub fn refuse_switch(
    counting: bool,
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    if counts_served(config) == counting {
        return Ok(());
    }
    Err(Diagnostic::error(
        code::E_BAD_DIRECTIVE,
        if counting {
            "this reload stops counting coverage, which a running server cannot do".to_string()
        } else {
            "this reload starts counting coverage, which a running server cannot do".to_string()
        },
    )
    .with_note(format!(
        "whether a server counts coverage is fixed when it starts{}",
        origin_note(origins.get(KEY))
    ))
    .with_help("restart the server to change `[debug] mode`".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(text: &str) -> Config {
        toml::from_str(text).expect("the fixture parses")
    }

    // covers: directive:debug.mode
    #[test]
    fn every_probe_name_is_accepted_and_any_other_is_refused() {
        let origins = BTreeMap::new();
        let four = tree("[debug]\nmode = [\"coverage\", \"branch\", \"trace\", \"profile\"]\n");
        assert!(validate(&four, &origins).is_ok());
        let wrong = tree("[debug]\nmode = [\"coverage\", \"tracing\"]\n");
        let refused = validate(&wrong, &origins).expect_err("`tracing` is not a probe");
        assert!(refused.message.contains("`tracing`"), "{}", refused.message);
    }

    // covers: directive:debug.mode
    #[test]
    fn a_server_refuses_any_probe_and_accepts_none() {
        let origins = BTreeMap::new();
        assert!(refuse_serving(&tree(""), &origins).is_ok());
        assert!(refuse_serving(&tree("[debug]\nmode = []\n"), &origins).is_ok());
        let refused = refuse_serving(&tree("[debug]\nmode = [\"coverage\"]\n"), &origins)
            .expect_err("a production server runs no probe");
        assert!(
            refused.message.contains("\"coverage\""),
            "{}",
            refused.message
        );
    }

    // covers: directive:debug.mode
    #[test]
    fn a_development_server_counts_coverage_and_runs_no_other_probe() {
        let origins = BTreeMap::new();
        let counting = tree("[mode]\ndefault = \"development\"\n[debug]\nmode = [\"coverage\"]\n");
        assert!(refuse_serving(&counting, &origins).is_ok());
        assert!(counts_served(&counting));
        let traced =
            tree("[mode]\ndefault = \"development\"\n[debug]\nmode = [\"coverage\", \"trace\"]\n");
        assert!(refuse_serving(&traced, &origins).is_err());
        assert!(refuse_switch(true, &counting, &origins).is_ok());
        assert!(refuse_switch(false, &counting, &origins).is_err());
        assert!(refuse_switch(true, &tree(""), &origins).is_err());
    }
}
