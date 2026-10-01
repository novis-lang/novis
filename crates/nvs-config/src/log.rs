//! `rule:errors/engine-floor`'s `[log] target` and
//! `rule:errors/log-level` and `rule:errors/renderings`'s
//! `[log] level` and `[log] format`: what each names, and the boot-time refusal of everything else.
//!
//! **The grammar is here and not at the sink, because two readers need it.** [`Target::of`] is the
//! only place a written target is turned into a destination: [`validate`] asks it whether a tree
//! boots, and `nvs_runtime::Ctx::write_log_record` asks it where a record goes. A parser at the sink
//! with a checker beside it would be two spellings of one grammar, and the failure they drift into
//! is the worst-shaped one available — a tree that boots green and routes its records nowhere.
//!
//! The level's grammar is [`nvs_render::Level::of`] and is read by that same pair, one crate down:
//! `[log] level` names one of `rule:errors/log-level`'s five, and the roster's home is the enum a record
//! already carries. Only the *refusal* is here — [`levelled`] — because only this crate has the
//! tree and the origins to say which file the word was written in.
//!
//! The format's grammar is [`Format::of`], and it is here rather than one crate down for the
//! target's reason and not the level's: § 3 names a *rendering to select*, and `nvs-render` carries
//! one function per rendering with no enum over them. The HTML rendering the response sink picks is
//! deliberately not a value of this directive, which § 3 states outright, so the two rosters are not
//! the same roster and must not become one type.
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
use std::path::Path;

use nvs_diagnostics::{Diagnostic, code};
use nvs_render::Level;

use crate::resolve::{Origin, origin_note};
use crate::tree::{Config, Log};

/// One of `rule:errors/engine-floor`'s three destinations, as written.
///
/// Borrowed rather than owned: both callers have the written value in hand and neither keeps this
/// past the sink it builds, so a `String` here would be an allocation per boot per block for a
/// value that is about to be a path anyway.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target<'a> {
    /// `stderr` — the default the floor has with nothing configured.
    Stderr,
    /// `file:<path>` — a rotating file under `rule:http-server/the-floor-cannot-fill-the-disk`'s bound, carrying the path as written.
    File(&'a str),
    /// `syslog` — spelled by § 4 and not yet transported;
    /// `nvs_runtime::Ctx::write_log_record`'s own doc owns what that means for a record.
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

/// One of `rule:errors/renderings`'s two log-target renderings, as written.
///
/// Owned by nothing and borrowing nothing, unlike [`Target`] above: a rendering is a choice
/// between two functions rather than a value carried into a sink, so both readers keep the answer
/// and neither keeps the word.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Format {
    /// `json` — JSON Lines, `nvs_render::json::line`, and the default with nothing written.
    ///
    /// The default is here as well as in `rule:config/a-mode-is-five-defaults`'s mode table because a context configured by
    /// something other than a resolved tree has said nothing about the shape it wants, and one
    /// record per line is the shape a log pipeline can read without being told.
    #[default]
    Json,
    /// `text` — § 3's plaintext rendering, `nvs_render::plain::render`, uncoloured.
    ///
    /// Colour is a property of the terminal sink and not of this directive: § 3 makes plaintext
    /// coloured *iff* `Cli::colorDepth() != None`, and a file or a redirected `stderr` answers
    /// `None`, so a target's plaintext is the same bytes either way.
    Text,
}

impl Format {
    /// What `written` names, or `None` for a rendering § 3 does not put on this directive.
    ///
    /// `html` is the near miss worth naming: it is a real rendering of the same record, reached
    /// through the response sink, and it is refused here rather than accepted and ignored.
    #[must_use]
    pub fn of(written: &str) -> Option<Self> {
        match written {
            "json" => Some(Self::Json),
            "text" => Some(Self::Text),
            _ => None,
        }
    }
}

/// § 4's target, asked of every `[log]` block the merged tree holds.
///
/// The global block and each `[[app]]`'s own, because `rule:config/an-application-is-its-entry-file-path` lets an application carry its
/// own `[app.log]` and a target written there reaches the floor exactly as the global one does.
///
/// # Errors
///
/// One [`Diagnostic`] for the first value of the first block that names nothing — `E0613` for a
/// target, `E0614` for a level, `E0615` for a format — naming the value, the spellings that would
/// have worked, and the file the value was written in.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    if let Some(log) = config.log.as_ref() {
        block(log, "log", origins)?;
    }
    for (index, app) in config.app.iter().enumerate() {
        if let Some(log) = app.log.as_ref() {
            block(log, &format!("app.{index}.log"), origins)?;
        }
    }
    Ok(())
}

/// Makes every `[log] handler` absolute against the file that wrote it —
/// `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`, as
/// [`crate::db::canonicalize`] does for a `[db]` path.
///
/// The global block and each `[[app]]`'s own, for [`validate`]'s reason. The joined path replaces
/// the written one in the typed tree and in the table, because `Snapshot::retype` reads the tree
/// back out of the table and the escalation ladder reads `log.handler` from that. It is arithmetic
/// on a string and cannot fail: whether the script exists and is granted is asked when it runs.
pub fn anchor(
    config: &mut Config,
    table: &mut toml::value::Table,
    origins: &BTreeMap<String, Origin>,
) {
    if let Some(log) = config.log.as_mut() {
        anchor_block(log, table.get_mut("log"), "log", origins);
    }
    for (index, app) in config.app.iter_mut().enumerate() {
        let Some(log) = app.log.as_mut() else {
            continue;
        };
        let written = table
            .get_mut("app")
            .and_then(toml::Value::as_array_mut)
            .and_then(|blocks| blocks.get_mut(index))
            .and_then(toml::Value::as_table_mut)
            .and_then(|block| block.get_mut("log"));
        anchor_block(log, written, &format!("app.{index}.log"), origins);
    }
}

/// One block's `handler`, joined to the folder of the file that wrote it, under the prefix it was
/// merged as.
fn anchor_block(
    log: &mut Log,
    table: Option<&mut toml::Value>,
    prefix: &str,
    origins: &BTreeMap<String, Origin>,
) {
    let Some(written) = log
        .handler
        .as_deref()
        .filter(|written| !written.is_empty() && !Path::new(written).has_root())
    else {
        return;
    };
    let base = crate::db::written_in(origins, &format!("{prefix}.handler"));
    let handler = crate::resolve::absolute(base, Path::new(written))
        .to_string_lossy()
        .into_owned();
    if let Some(block) = table.and_then(toml::Value::as_table_mut) {
        block.insert("handler".to_owned(), toml::Value::String(handler.clone()));
    }
    log.handler = Some(handler);
}

/// Both spelled values of one `[log]` block, under the prefix it was merged as.
fn block(log: &Log, prefix: &str, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    if let Some(written) = log.target.as_deref() {
        spelled(written, &format!("{prefix}.target"), origins)?;
    }
    if let Some(written) = log.level.as_deref() {
        levelled(written, &format!("{prefix}.level"), origins)?;
    }
    if let Some(written) = log.format.as_deref() {
        formatted(written, &format!("{prefix}.format"), origins)?;
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
        "`rule:errors/engine-floor`'s floor writes to `stderr`, to `file:<path>` or to `syslog`{}",
        origin_note(origins.get(key))
    ))
    .with_help(help.to_string()))
}

/// [`validate`]'s refusal for one written level, under the key it was merged as.
///
/// The grammar itself is [`nvs_render::Level::of`], for the reason this module's own doc gives
/// about the target: one place turns a written value into the thing it names, and both readers ask
/// it. What is different here is what an unspelled value costs — a level nobody can resolve leaves
/// the floor at `Debug`, so the deployment collects *more* than it asked for rather than nothing,
/// and the mistake is invisible in the records themselves. `E0614`'s own doc is the home of that.
fn levelled(
    written: &str,
    key: &str,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    if Level::of(written).is_some() {
        return Ok(());
    }
    Err(Diagnostic::error(
        code::E_UNSPELLED_LOG_LEVEL,
        format!("`[log] level = \"{written}\"` names no level"),
    )
    .with_note(format!(
        "`rule:errors/log-level`'s roster is `Debug`, `Info`, `Warn`, `Error` and `Critical`{}",
        origin_note(origins.get(key))
    ))
    .with_help(
        "write the case as § 2 spells it — `level = \"Info\"` — or as a record renders it, \
         `level = \"info\"`"
            .to_owned(),
    ))
}

/// [`validate`]'s refusal for one written format, under the key it was merged as.
///
/// The last of this block's checks, and the one whose unspelled value costs the least at the sink
/// and the most downstream: the records are the right records, written in the rendering the
/// deployment asked not to have. `E0615`'s own doc is the home of that, and of why `html` — a real
/// rendering of the same record, and not one of this directive's two — is refused here rather than
/// quietly answered with JSON.
fn formatted(
    written: &str,
    key: &str,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    if Format::of(written).is_some() {
        return Ok(());
    }
    Err(Diagnostic::error(
        code::E_UNSPELLED_LOG_FORMAT,
        format!("`[log] format = \"{written}\"` names no rendering a log target emits"),
    )
    .with_note(format!(
        "`rule:errors/renderings` gives this directive two values, `json` and `text`{}",
        origin_note(origins.get(key))
    ))
    .with_help(
        "write `format = \"json\"` for JSON Lines or `format = \"text\"` for the plaintext \
         rendering; the HTML one is what an HTTP response renders a record as, and is not a \
         destination's"
            .to_owned(),
    ))
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
            log: Some(Log {
                target: Some(target.to_string()),
                ..Log::default()
            }),
            ..Config::default()
        }
    }

    /// A tree whose global `[log]` block names `level` and nothing else.
    fn levelled_tree(level: &str) -> Config {
        Config {
            log: Some(Log {
                level: Some(level.to_string()),
                ..Log::default()
            }),
            ..Config::default()
        }
    }

    /// The level directive's grammar, asked through the same check the target's is: `rule:errors/log-level`'s five in both of the spellings the documentation uses, and one refusal for everything
    /// else — including the near-miss a PSR-3 habit produces, which is the whole reason this is
    /// checked at all.
    ///
    /// Asserted over the *whole roster* rather than on one case, so a level that stopped
    /// resolving — the one failure that silently widens what a deployment collects — cannot pass
    /// here while the case the test happened to name still does.
    #[test]
    fn every_level_resolves_in_both_of_its_spellings_and_nothing_else_does() {
        for level in Level::ALL {
            for written in [level.case_name(), level.name()] {
                validate(&levelled_tree(written), &BTreeMap::new()).unwrap_or_else(|_| {
                    panic!("`{written}` is how `rule:errors/log-level` is read back")
                });
            }
        }
        for written in ["warning", "notice", "DEBUG", "trace", ""] {
            let Err(refused) = validate(&levelled_tree(written), &BTreeMap::new()) else {
                panic!("`{written}` is not one of `rule:errors/log-level`'s five");
            };
            assert_eq!(refused.code, Some(code::E_UNSPELLED_LOG_LEVEL));
            assert!(
                refused.message.contains(written),
                "the value is what the operator has to find: {}",
                refused.message
            );
        }
    }

    /// A tree whose global `[log]` block names `format` and nothing else.
    fn formatted_tree(format: &str) -> Config {
        Config {
            log: Some(Log {
                format: Some(format.to_string()),
                ..Log::default()
            }),
            ..Config::default()
        }
    }

    /// `rule:errors/renderings`'s two values, and the refusal of everything else — including `html`, which is a
    /// rendering of the same record and is not one of this directive's two.
    ///
    /// The bound is named on both sides in one test because the roster is two long: a check that
    /// only accepted would pass on a directive that accepts anything, and one that only refused
    /// would pass on a directive that accepts nothing and never boots.
    #[test]
    fn the_two_renderings_a_target_emits_resolve_and_the_third_one_does_not() {
        for written in ["json", "text"] {
            validate(&formatted_tree(written), &BTreeMap::new())
                .unwrap_or_else(|_| panic!("`{written}` is one of `rule:errors/renderings`'s two"));
        }
        for written in ["html", "JSON", "plain", "jsonl", ""] {
            let Err(refused) = validate(&formatted_tree(written), &BTreeMap::new()) else {
                panic!("`{written}` is not a rendering a log target emits");
            };
            assert_eq!(refused.code, Some(code::E_UNSPELLED_LOG_FORMAT));
            assert!(
                refused.message.contains(written),
                "the value is what the operator has to find: {}",
                refused.message
            );
        }
    }

    /// An `[app.log]` block's target is checked too — `rule:config/an-application-is-its-entry-file-path` gives an application its own
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
