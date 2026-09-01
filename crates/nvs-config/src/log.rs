//! [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 4's `[log] target`: the grammar
//! of the three destinations it names, and the boot-time refusal of everything else.
//!
//! **The grammar is here and not at the sink, because two readers need it.** [`Target::of`] is the
//! only place a written target is turned into a destination: [`validate`] asks it whether a tree
//! boots, and `nvs_runtime::Ctx::write_log_record` asks it where a record goes. A parser at the sink
//! with a checker beside it would be two spellings of one grammar, and the failure they drift into
//! is the worst-shaped one available — a tree that boots green and routes its records nowhere.
//!
//! **Refused where it is written, never where it is used.** The one moment the engine cannot afford
//! to raise a diagnostic about its configuration is the moment it is already reporting a failure:
//! § 4's floor is the last rung, so a target checked on the first record would displace the record
//! it exists to write, and only in the deployments that had already gone wrong twice. Boot is where
//! an operator is reading output and can fix the file.
//!
//! Cost: one match over a short string per `[log]` block in the merged tree, at boot and at reload,
//! and nothing at all per record — the runtime resolves its target once per context.

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::Config;

/// One of ADR 0020 § 4's three destinations, as written.
///
/// Borrowed rather than owned: both callers have the written value in hand and neither keeps this
/// past the sink it builds, so a `String` here would be an allocation per boot per block for a
/// value that is about to be a path anyway.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target<'a> {
    /// `stderr` — the default the floor has with nothing configured.
    Stderr,
    /// `file:<path>` — a rotating file under ADR 0106 § 10's bound, carrying the path as written.
    File(&'a str),
    /// `syslog` — spelled by § 4 and not yet transported;
    /// `nvs_runtime::Ctx::write_log_record`'s own doc owns what that means for a record today.
    Syslog,
}

impl<'a> Target<'a> {
    /// What `written` names, or `None` for a target § 4 does not spell.
    ///
    /// A `file:` with nothing behind it is `None` rather than a file called nothing: an empty path
    /// is the shape a half-finished edit leaves, and the value it would otherwise open is whatever
    /// the working directory happens to be.
    #[must_use]
    pub fn of(written: &'a str) -> Option<Self> {
        match written {
            "stderr" => Some(Self::Stderr),
            "syslog" => Some(Self::Syslog),
            _ => match written.strip_prefix("file:") {
                Some(path) if !path.is_empty() => Some(Self::File(path)),
                _ => None,
            },
        }
    }
}

/// § 4's target, asked of every `[log]` block the merged tree holds.
///
/// The global block and each `[[app]]`'s own, because ADR 0104 § 1 lets an application carry its
/// own `[app.log]` and a target written there reaches the floor exactly as the global one does.
///
/// # Errors
///
/// One [`Diagnostic`], `E0613`, for the first target that is none of the three — naming the value,
/// the three spellings, and the file the value was written in.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    if let Some(written) = config.log.as_ref().and_then(|log| log.target.as_deref()) {
        spelled(written, "log.target", origins)?;
    }
    for (index, app) in config.app.iter().enumerate() {
        if let Some(written) = app.log.as_ref().and_then(|log| log.target.as_deref()) {
            spelled(written, &format!("app.{index}.log.target"), origins)?;
        }
    }
    Ok(())
}

/// [`validate`]'s refusal for one written value, under the key it was merged as.
fn spelled(written: &str, key: &str, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    if Target::of(written).is_some() {
        return Ok(());
    }
    let help = if written == "file:" {
        "a file target names the file: `target = \"file:/var/log/nvs.log\"`"
    } else {
        "write `target = \"stderr\"`, `target = \"file:<path>\"` or `target = \"syslog\"`"
    };
    Err(Diagnostic::error(
        code::E_UNSPELLED_LOG_TARGET,
        format!("`[log] target = \"{written}\"` names no destination"),
    )
    .with_note(format!(
        "ADR 0020 § 4's floor writes to `stderr`, to `file:<path>` or to `syslog`{}",
        origin_note(origins.get(key))
    ))
    .with_help(help.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// § 4's roster, as the grammar both readers share: the three spellings resolve and everything
    /// else is one refusal rather than a destination nobody opens.
    ///
    /// The two near-misses are the ones an edit produces — a `file:` whose path was deleted and a
    /// word from another logging library — and each is checked as a *`None`* rather than as a
    /// message, because what makes them safe is that no sink is built from either.
    #[test]
    fn the_grammar_is_the_three_spellings_and_nothing_beside_them() {
        assert_eq!(Target::of("stderr"), Some(Target::Stderr));
        assert_eq!(Target::of("syslog"), Some(Target::Syslog));
        assert_eq!(
            Target::of("file:/var/log/nvs.log"),
            Some(Target::File("/var/log/nvs.log"))
        );
        assert_eq!(Target::of("file:"), None, "a file target names a file");
        assert_eq!(Target::of("stdout"), None, "the floor is not `echo`");
        assert_eq!(Target::of("STDERR"), None, "a directive is not case-folded");
        assert_eq!(Target::of(""), None);
    }

    /// The refusal lands at the tree and names the value, so an operator reads it where the file is
    /// rather than at the first failure the deployment reports.
    #[test]
    fn an_unspelled_target_refuses_the_tree() {
        let refused =
            validate(&global("stdout"), &BTreeMap::new()).expect_err("`stdout` is not a target");
        assert_eq!(refused.code, Some(code::E_UNSPELLED_LOG_TARGET));
        assert!(
            refused.message.contains("stdout"),
            "the value is what the operator has to find: {}",
            refused.message
        );
        validate(&global("file:/var/log/nvs.log"), &BTreeMap::new())
            .expect("a spelled target boots");
    }

    /// A tree whose global `[log]` block names `target` and nothing else.
    fn global(target: &str) -> Config {
        Config {
            log: Some(crate::tree::Log {
                target: Some(target.to_string()),
                ..crate::tree::Log::default()
            }),
            ..Config::default()
        }
    }

    /// An `[app.log]` block's target is checked too — ADR 0104 § 1 gives an application its own
    /// block, and a value that reaches the floor from there is not a value the global check saw.
    #[test]
    fn an_applications_own_block_is_checked_as_well() {
        let config = Config {
            app: vec![crate::tree::App {
                entry: Some("app.nvs".to_string()),
                log: Some(crate::tree::Log {
                    target: Some("journald".to_string()),
                    ..crate::tree::Log::default()
                }),
                ..crate::tree::App::default()
            }],
            ..Config::default()
        };
        assert_eq!(
            validate(&config, &BTreeMap::new())
                .expect_err("a per-application target is a target")
                .code,
            Some(code::E_UNSPELLED_LOG_TARGET)
        );
    }
}
