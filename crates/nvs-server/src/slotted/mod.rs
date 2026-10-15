//! `rule:core-classes/html-later`'s slotted page, as this server sends it: the
//! two inline scripts beside the fills, and the script policy that lets them
//! run.
//!
//! `nvs_runtime::later` writes the page — the shell, each `<template for>`
//! fill as its slot finishes, the held-back end — through the response-stream
//! cell this server offers every request. The text of the two scripts lives
//! here rather than there, because this is the crate that writes the
//! `Content-Security-Policy` that must name them, and a script and its hash in
//! two crates would drift. The cell carries [`SCRIPTS`] into the runtime.
//!
//! # The two scripts
//!
//! `polyfill.js` defines `_nvs()`, which applies every `<template for>` still
//! in the document to the range between its `<?start name>` and the next
//! `<?end>`, then removes the template. A browser that applies the template
//! itself leaves nothing for it to find, so the polyfill acts only where the
//! browser did not, and a second call changes nothing. It is sent once per
//! response, ahead of the first fill, and a response with no fill never
//! carries it. `trigger.js` is the call, sent after every fill: one identical
//! script, so one hash covers every fill.
//!
//! # The script policy
//!
//! A policy that limits scripts would block both, and the page would show its
//! placeholders forever. So on a slotted response, after the secure header
//! set is filled, [`allow_scripts`] adds the two hashes to the directive that
//! governs an inline `<script>` element — `script-src-elem`, else
//! `script-src`, else `default-src`. A policy with none of the three limits no
//! script and is left alone, as is one that allows every inline script
//! already (`'unsafe-inline'` with no nonce or hash beside it, which a hash
//! would switch off). A `'none'` in that directive is dropped, since it
//! cannot stand beside a source. [`crate::csp`] does the amending.
//!
//! # What it spends
//!
//! Nothing per request: the scripts are static text, and a slotted response
//! sends the polyfill once and the trigger's 23 bytes per fill.

use hyper::HeaderMap;
use nvs_runtime::stream::Scripts;

#[cfg(test)]
mod tests;

/// The polyfill, sent once ahead of a slotted response's first fill.
pub const POLYFILL: &str = include_str!("polyfill.js");

/// The trigger, sent after every fill.
pub const TRIGGER: &str = include_str!("trigger.js");

/// [`POLYFILL`]'s hash, as a policy source. A test recomputes it from the file.
pub const POLYFILL_HASH: &str = "'sha256-Cv56uYStB0yzKrxuhDebXTZTtjoiUMDKcPelMO0jPSQ='";

/// [`TRIGGER`]'s hash, as a policy source. A test recomputes it from the file.
pub const TRIGGER_HASH: &str = "'sha256-3Nf+JVg1PpD5oeimsznw4iqTL5f3sp5qoBVgFxvr/g0='";

/// What the response-stream cell carries into the runtime.
pub const SCRIPTS: Scripts = Scripts {
    polyfill: POLYFILL,
    trigger: TRIGGER,
};

/// The mark `serve::streamed` leaves on a slotted response, which is what
/// tells the policy step to call [`allow_scripts`].
#[derive(Clone, Copy, Debug)]
pub struct Slotted;

/// Adds the two hashes to every `Content-Security-Policy` in `headers` that
/// limits inline scripts, as the module doc says.
pub fn allow_scripts(headers: &mut HeaderMap) {
    crate::csp::add_sources(
        headers,
        &["script-src-elem", "script-src", "default-src"],
        &[POLYFILL_HASH, TRIGGER_HASH],
    );
}
