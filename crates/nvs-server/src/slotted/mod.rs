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
//! cannot stand beside a source.
//!
//! # What it spends
//!
//! Nothing per request: the scripts are static text, and a slotted response
//! sends the polyfill once and the trigger's 23 bytes per fill.

use hyper::HeaderMap;
use hyper::header::{self, HeaderValue};
use nvs_runtime::stream::Scripts;

#[cfg(test)]
mod tests;

/// The polyfill, sent once ahead of a slotted response's first fill.
pub const POLYFILL: &str = include_str!("polyfill.js");

/// The trigger, sent after every fill.
pub const TRIGGER: &str = include_str!("trigger.js");

/// [`POLYFILL`]'s hash, as a policy source. A test recomputes it from the file.
pub const POLYFILL_HASH: &str = "'sha256-I0MrixJBC+aKunTzQh/2rN/uwSaydex/JBM1irlqrOw='";

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
    let policies: Vec<HeaderValue> = headers
        .get_all(header::CONTENT_SECURITY_POLICY)
        .iter()
        .cloned()
        .collect();
    if policies.is_empty() {
        return;
    }
    headers.remove(header::CONTENT_SECURITY_POLICY);
    for policy in policies {
        let amended = policy
            .to_str()
            .ok()
            .map(|text| {
                text.split(',')
                    .map(|one| amend(one).unwrap_or_else(|| one.to_owned()))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .and_then(|text| HeaderValue::from_str(&text).ok());
        headers.append(header::CONTENT_SECURITY_POLICY, amended.unwrap_or(policy));
    }
}

/// One policy with the two hashes added, and `None` where it needs none.
fn amend(policy: &str) -> Option<String> {
    let mut directives: Vec<String> = policy.split(';').map(str::to_owned).collect();
    let index = ["script-src-elem", "script-src", "default-src"]
        .iter()
        .find_map(|wanted| {
            directives.iter().position(|directive| {
                directive
                    .split_ascii_whitespace()
                    .next()
                    .is_some_and(|name| name.eq_ignore_ascii_case(wanted))
            })
        })?;
    let mut words = directives[index].split_ascii_whitespace();
    let name = words.next()?.to_owned();
    let sources: Vec<&str> = words.collect();
    let is = |source: &str, keyword: &str| source.eq_ignore_ascii_case(keyword);
    let keyed = sources.iter().any(|source| {
        let source = source.to_ascii_lowercase();
        source.starts_with("'nonce-") || source.starts_with("'sha") || source == "'strict-dynamic'"
    });
    if !keyed && sources.iter().any(|source| is(source, "'unsafe-inline'")) {
        return None;
    }
    let mut rewritten = format!(" {name}");
    for source in sources.iter().filter(|source| !is(source, "'none'")) {
        rewritten.push(' ');
        rewritten.push_str(source);
    }
    for hash in [POLYFILL_HASH, TRIGGER_HASH] {
        rewritten.push(' ');
        rewritten.push_str(hash);
    }
    if index == 0 {
        rewritten.remove(0);
    }
    directives[index] = rewritten;
    Some(directives.join(";"))
}
