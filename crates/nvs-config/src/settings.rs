//! `rule:config/settings-and-secrets-are-a-programs-own-values`: the four open blocks —
//! `[settings]` and `[secrets]`, and `[app.settings]` and `[app.secrets]` under an `[[app]]` block.
//!
//! Every other block is closed, and `deny_unknown_fields` refuses a key it does not know. These four
//! hold keys the operator names, so the typed tree carries each as a `toml::Table`, and what the
//! closed blocks get from `serde` this module checks by hand, per file and before the merge, so a
//! refusal names the line it was written on:
//!
//! - every key is a plain name, `[A-Za-z_][A-Za-z0-9_]*`, at every depth. A dot inside a quoted
//!   key would make `"api.base"` and `[settings.api] base` two settings read by one name.
//! - no value is a TOML date or time, which has no Novis value without a time-zone choice.
//! - a secret is a string, and a `[secrets]` block has no sub-table.
//!
//! Exactly one of `name` and `name_file` is [`mod@crate::secret`]'s check, over the merged tree,
//! because a later file may write the half an earlier one did not.
//!
//! **Nothing here is a directive**, so `Core\Config::get`, `all`, `set` and `restore` never see
//! these keys ([`is_program_value`]). A secret would otherwise come back from `get` as a plain
//! string, and from `all` in a page that prints every setting.
//!
//! Cost: one extra parse of a file that writes one of the four blocks, at boot and at each reload.
//! A file that writes none of them pays one look at four fields. Nothing here runs per request.

use std::ops::Range;

use nvs_diagnostics::{Diagnostic, SourceId, code};
use toml::de::{DeTable, DeValue};

use crate::file::span_of;
use crate::tree::Config;

/// The top-level block a setting is written in, and its `[[app]]` copy's name under the block.
pub const SETTINGS: &str = "settings";

/// The top-level block a secret is written in, and its `[[app]]` copy's name under the block.
pub const SECRETS: &str = "secrets";

/// Whether a dotted key is one of the four blocks or under one, as `[[app]]`'s copies are once the
/// fold has put them over the global blocks. `Core\Config`'s directive readers skip every such key.
#[must_use]
pub fn is_program_value(key: &str) -> bool {
    [SETTINGS, SECRETS].iter().any(|block| {
        key.strip_prefix(block)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
    })
}

/// Whether `name` is a plain name a program can pass to `Core\Config::setting` or `secret`.
#[must_use]
pub fn is_plain_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|next| next.is_ascii_alphanumeric() || next == '_')
}

/// Checks the four blocks of one file, which `text` holds and `file` is already typed from.
///
/// # Errors
///
/// `E0601` for a key that is not a plain name, a date or time in either block, or a secret that is
/// not a string, each pointing at the line it was written on.
pub fn check_file(id: SourceId, text: &str, file: &Config) -> Result<(), Diagnostic> {
    let written = file.settings.is_some()
        || file.secrets.is_some()
        || file
            .app
            .iter()
            .any(|app| app.settings.is_some() || app.secrets.is_some());
    if !written {
        return Ok(());
    }
    // The typed parse above accepted the same bytes, so this succeeds. On the impossible path there
    // is no span to point at, and the typed tree has already said everything it can.
    let Ok(root) = DeTable::parse(text) else {
        return Ok(());
    };
    check_blocks(id, root.get_ref(), "")?;
    for (key, value) in root.get_ref() {
        if key.get_ref() != "app" {
            continue;
        }
        if let DeValue::Array(entries) = value.get_ref() {
            for entry in entries.iter() {
                if let DeValue::Table(block) = entry.get_ref() {
                    check_blocks(id, block, "app.")?;
                }
            }
        }
    }
    Ok(())
}

/// The `settings` and `secrets` keys of one table, the root or an `[[app]]` block.
fn check_blocks(id: SourceId, table: &DeTable<'_>, prefix: &str) -> Result<(), Diagnostic> {
    for (key, value) in table {
        let block = format!("{prefix}{}", key.get_ref());
        match (key.get_ref().as_ref(), value.get_ref()) {
            (SETTINGS, DeValue::Table(inner)) => check_setting_table(id, inner, &block)?,
            (SECRETS, DeValue::Table(inner)) => check_secrets(id, inner, &block)?,
            _ => {}
        }
    }
    Ok(())
}

/// Every key of a settings table and of each table under it, and every value.
fn check_setting_table(id: SourceId, table: &DeTable<'_>, block: &str) -> Result<(), Diagnostic> {
    for (key, value) in table {
        check_name(id, key.get_ref(), key.span(), block)?;
        check_setting(id, value.get_ref(), value.span(), block)?;
    }
    Ok(())
}

/// One setting's value, and every value inside it.
fn check_setting(
    id: SourceId,
    value: &DeValue<'_>,
    span: Range<usize>,
    block: &str,
) -> Result<(), Diagnostic> {
    match value {
        DeValue::Datetime(when) => Err(refusal(
            id,
            span,
            "a date or a time cannot be a setting",
            block,
        )
        .with_note(
            "a date or a time needs a time zone, and a setting does not have one".to_string(),
        )
        .with_help(format!(
            "write it as a string: `\"{when}\"`. Your program can then read it with the time zone \
             it needs"
        ))),
        DeValue::Array(entries) => {
            for entry in entries.iter() {
                check_setting(id, entry.get_ref(), entry.span(), block)?;
            }
            Ok(())
        }
        DeValue::Table(inner) => check_setting_table(id, inner, block),
        _ => Ok(()),
    }
}

/// Every key of a secrets table: a plain name with a string value.
fn check_secrets(id: SourceId, table: &DeTable<'_>, block: &str) -> Result<(), Diagnostic> {
    for (key, value) in table {
        check_name(id, key.get_ref(), key.span(), block)?;
        if !matches!(value.get_ref(), DeValue::String(_)) {
            return Err(
                refusal(id, value.span(), "a secret must be a string", block).with_help(format!(
                    "write the value in quotes, or name a file with `{}_file = \"…\"`",
                    key.get_ref().strip_suffix("_file").unwrap_or(key.get_ref())
                )),
            );
        }
    }
    Ok(())
}

/// One key: a plain name, or `E0601`.
fn check_name(id: SourceId, name: &str, span: Range<usize>, block: &str) -> Result<(), Diagnostic> {
    if is_plain_name(name) {
        return Ok(());
    }
    Err(refusal(
        id,
        span,
        &format!("`{name}` is not a valid name for a setting or a secret"),
        block,
    )
    .with_note(
        "a name starts with a letter or `_`, and has only letters, digits and `_`".to_string(),
    )
    .with_help(
        "to group settings, use a table: `[settings.api]` and then `base = \"…\"`. Your program \
         reads it as `api.base`"
            .to_string(),
    ))
}

/// `E0601` at `span`, naming the block.
fn refusal(id: SourceId, span: Range<usize>, message: &str, block: &str) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(code::E_BAD_DIRECTIVE, message.to_string());
    if let Some(span) = span_of(id, Some(span)) {
        diagnostic = diagnostic.with_primary(span, "here");
    }
    diagnostic.with_note(format!("in `[{block}]`"))
}
