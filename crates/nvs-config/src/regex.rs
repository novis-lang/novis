//! `rule:core-classes/regex-compile-time-tiering`'s `[regex] backtracking`: whether a pattern that
//! needs the backtracking engine compiles, the three values it takes, and the refusal of every
//! other.
//!
//! **The default is `allow` in every mode, and the mode table does not name it.** It is how every
//! program ran before the block existed, and `[limits] max_regex_steps` already bounds a
//! backtracking match (`rule:core-classes/regex-two-tiers`), so a stricter default would only break
//! working programs.
//!
//! **`System`-class and reloadable** (`crate::directive`'s `regex.backtracking` row): it decides
//! what compiles, so no request may set it, and its value is an input of
//! [`crate::cache::env_hash`], so a reload that changes it rekeys every unit and the next request
//! to resolve each path recompiles it under the new value.
//!
//! **Global only.** `[[app]]` takes no `[regex]` block ([`crate::tree::App`]), because no rule
//! describes a policy per application.
//!
//! Cost: one byte of hash input per environment, one match over a short string per tree at boot
//! and at reload, and one per compile that reads the value.

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, code};

use crate::request::Request;
use crate::resolve::{Origin, origin_note};
use crate::tree::Config;

/// The dotted key, as `Core\Config::get` names it.
pub const KEY: &str = "regex.backtracking";

/// What a pattern that needs the backtracking engine does when it is compiled.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Backtracking {
    /// `allow` — the pattern compiles and runs under the step budget. The default with nothing
    /// written, in every mode.
    #[default]
    Allow,
    /// `warn` — a pattern literal compiles with a warning.
    Warn,
    /// `deny` — a pattern literal does not compile.
    Deny,
}

impl Backtracking {
    /// What `written` names, or `None` for a word the directive does not take.
    #[must_use]
    pub fn of(written: &str) -> Option<Self> {
        match written {
            "allow" => Some(Self::Allow),
            "warn" => Some(Self::Warn),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }

    /// The value `config` writes, or [`Allow`] with nothing written. A value [`validate`] refuses
    /// never reaches here from a resolved tree, so a word it does not name reads as [`Allow`] too.
    ///
    /// [`Allow`]: Self::Allow
    #[must_use]
    pub fn of_config(config: &Config) -> Self {
        config
            .regex
            .as_ref()
            .and_then(|regex| regex.backtracking.as_deref())
            .and_then(Self::of)
            .unwrap_or_default()
    }

    /// The value in force for `request`. A request cannot set a `System` key, so this is what the
    /// file wrote, then [`Allow`](Self::Allow).
    #[must_use]
    pub fn in_force(request: &Request) -> Self {
        request.backtracking()
    }

    /// The value as `nvs.toml` writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
        }
    }
}

/// [`Regex`](crate::tree::Regex)' one key, asked of the merged tree.
///
/// # Errors
///
/// `E0601`, the code a directive with an invalid value gets, naming the value, the three it takes,
/// and the file the value was written in.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(written) = config
        .regex
        .as_ref()
        .and_then(|regex| regex.backtracking.as_deref())
    else {
        return Ok(());
    };
    if Backtracking::of(written).is_some() {
        return Ok(());
    }
    Err(Diagnostic::error(
        code::E_BAD_DIRECTIVE,
        format!("`[regex] backtracking = \"{written}\"` is not a value that key can hold"),
    )
    .with_note(format!(
        "`backtracking` decides whether a pattern that needs the backtracking regex engine compiles{}",
        origin_note(origins.get(KEY))
    ))
    .with_help(
        "write `backtracking = \"allow\"`, the default, `backtracking = \"warn\"` or \
         `backtracking = \"deny\"`"
            .to_string(),
    ))
}
