//! `rule:core-classes/image-pixel-cap`'s `[image] max_pixels`: the most pixels the image component
//! decodes in one call, read off the header before a buffer is allocated.
//!
//! **The block reads the cap and nothing else.** No key in it grants anything (ADR 0246 § 9), and a
//! second key is a new decision rather than a row added here.
//!
//! **`System`-class and reloadable** (`crate::directive`'s `image.max_pixels` row). A program
//! lowers the cap for one call through that call's own argument, so a request-set value would be a
//! second way to say the same thing, and the only direction it could move the cap that the argument
//! cannot is up. A changed cap reaches the next request, because the caller reads it out of the
//! snapshot its request cloned.
//!
//! **A count of pixels written in the size grammar** (`crate::value`): a whole number, or one with a
//! `K`, `M`, `G` or `T` suffix in binary multiples, so `"24M"` is 25,165,824 pixels — the same
//! number `"24M"` is in every other key of the file. Neither `false` nor zero is a cap, and both are
//! refused where they are written: a decoder with no pixel cap is the decompression bomb the key
//! exists to stop, and a cap of nothing decodes no image at all.
//!
//! Cost: one parse per tree at boot and at reload, and one per call that reads it.

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, code};

use crate::request::Request;
use crate::resolve::{Origin, origin_note};
use crate::tree::{Config, Setting};
use crate::value::{Quantity, Unit, as_written};

/// The dotted key, as `Core\Config::get` names it.
pub const KEY: &str = "image.max_pixels";

/// The cap with nothing written: `"24M"`, read as every other `M` in the file is read.
pub const DEFAULT: u64 = 24 << 20;

/// The cap `written` spells, or `None` for a value that is not one — a malformed size, `false`,
/// or zero.
fn cap_of(written: &Setting) -> Option<u64> {
    match Quantity::parse(KEY, Unit::Bytes, written) {
        Ok(Quantity::Bytes(pixels)) if pixels > 0 => Some(pixels),
        _ => None,
    }
}

/// The cap a file wrote, or [`DEFAULT`] with nothing written. A value [`validate`] would refuse
/// never reaches a reader, so it is read as the default rather than as no cap.
#[must_use]
pub fn max_pixels(config: &Config) -> u64 {
    config
        .image
        .as_ref()
        .and_then(|image| image.max_pixels.as_ref())
        .and_then(cap_of)
        .unwrap_or(DEFAULT)
}

/// The cap in force for `request`, read out of the snapshot it started under.
#[must_use]
pub fn in_force(request: &Request) -> u64 {
    request
        .get(KEY)
        .and_then(|written| cap_of(&Setting::Text(written)))
        .unwrap_or(DEFAULT)
}

/// [`Image`](crate::tree::Image)'s one key, asked of the merged tree.
///
/// # Errors
///
/// `E0601`, the code a directive with an invalid value gets: a value the size grammar does not
/// read, through [`Quantity::parse`]'s own refusal, and `false` or zero, which parse and are not a
/// cap.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(written) = config
        .image
        .as_ref()
        .and_then(|image| image.max_pixels.as_ref())
    else {
        return Ok(());
    };
    Quantity::parse(KEY, Unit::Bytes, written)
        .map_err(|invalid| invalid.diagnostic(origins.get(KEY)))?;
    if cap_of(written).is_some() {
        return Ok(());
    }
    Err(Diagnostic::error(
        code::E_BAD_DIRECTIVE,
        format!(
            "`[image] max_pixels = {}` is not a pixel cap",
            as_written(written)
        ),
    )
    .with_note(format!(
        "`max_pixels` is the most pixels one image may declare before it is decoded, and an image \
         with no cap, or a cap of nothing, is not allowed{}",
        origin_note(origins.get(KEY))
    ))
    .with_help("write `max_pixels = \"24M\"`, the default, or a smaller size".to_string()))
}
