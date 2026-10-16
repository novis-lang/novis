//! `rule:errors/a-use-of-deprecated-code-may-log-or-throw`'s `[errors] deprecated`: what a use of
//! deprecated code does while it runs, the three values it takes, and the refusal of every other.
//!
//! **The default is `ignore` in every mode, and the mode table does not name it.** Deprecated code
//! still works in production, and only a developer turns the check on, so
//! `rule:config/a-mode-is-five-defaults` keeps its five rows and this key is not a sixth.
//!
//! **`Runtime`-class and reloadable** (`crate::directive`'s `errors.deprecated` row): a test or a
//! single request sets it with `Core\Config::set`, and the set dies with that request
//! (`rule:config/a-runtime-set-is-request-local`). [`crate::request::Request::set`] refuses a value
//! [`Deprecated::of`] does not name, as [`validate`] refuses one written in a file, so the reader
//! never meets a word it has to guess about.
//!
//! Cost: one match over a short string per tree at boot and at reload, and one per request that
//! reads the value.

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, code};

use crate::request::Request;
use crate::resolve::{Origin, origin_note};
use crate::tree::Config;

/// The dotted key, as `Core\Config::get` and `set` name it.
pub const KEY: &str = "errors.deprecated";

/// What a use of deprecated code does while it runs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Deprecated {
    /// `ignore` — the use runs unchanged. The default with nothing written, in every mode.
    #[default]
    Ignore,
    /// `log` — one `warning` record per use site per request.
    Log,
    /// `throw` — the use throws `Core\DeprecatedError` with `W1003`'s text.
    Throw,
}

impl Deprecated {
    /// What `written` names, or `None` for a word the directive does not take.
    #[must_use]
    pub fn of(written: &str) -> Option<Self> {
        match written {
            "ignore" => Some(Self::Ignore),
            "log" => Some(Self::Log),
            "throw" => Some(Self::Throw),
            _ => None,
        }
    }

    /// The value in force for `request`: what it set, then what the file wrote, then [`Ignore`].
    ///
    /// [`Ignore`]: Self::Ignore
    #[must_use]
    pub fn in_force(request: &Request) -> Self {
        request
            .get(KEY)
            .and_then(|written| Self::of(&written))
            .unwrap_or_default()
    }

    /// The value as `nvs.toml` writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::Log => "log",
            Self::Throw => "throw",
        }
    }
}

/// [`Errors`](crate::tree::Errors)' one key, asked of the merged tree.
///
/// # Errors
///
/// `E0601`, the code a directive with an invalid value gets, naming the value, the three it takes,
/// and the file the value was written in.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(written) = config
        .errors
        .as_ref()
        .and_then(|errors| errors.deprecated.as_deref())
    else {
        return Ok(());
    };
    if Deprecated::of(written).is_some() {
        return Ok(());
    }
    Err(Diagnostic::error(
        code::E_BAD_DIRECTIVE,
        format!("`[errors] deprecated = \"{written}\"` is not a value that key can hold"),
    )
    .with_note(format!(
        "`deprecated` decides what a use of deprecated code does while it runs{}",
        origin_note(origins.get(KEY))
    ))
    .with_help(
        "write `deprecated = \"ignore\"`, the default, `deprecated = \"log\"` or \
         `deprecated = \"throw\"`"
            .to_string(),
    ))
}
