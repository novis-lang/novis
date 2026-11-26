//! The call site an `http`, `cache`, `session` or `file` event names
//! (`rule:observability/a-query-span-names-its-call-site`).
//!
//! Every member on [`crate::registry::RECORDED_SITE_MEMBERS`] takes its call site's id as its last
//! argument, the way a `Core\Db` statement member does: the compiler passes it in every mode, so
//! a program's code is the same whether a request is recorded or not. [`watched`] runs a member's
//! body, and only when [`Ctx::records_spans`] is true does it read the clock and the id, and file
//! one event with the member's name, its target and the site's row. [`watched_with`] adds the
//! call's outcome, such as a cache read's hit or a file read's length, read off its result only
//! when the event is filed. `Core\Mail::send` and `Core\Queue::push` file their `mail` and
//! `queue` events through [`watched_with`] too, with a null site word, so the event names no site.
//!
//! **What it spends:** with recording off, one flag test per call. With it on, two clock reads,
//! one label and one target string per call, freed with the spool line.

use std::time::Instant;

use nvs_runtime::spool::Outcome;
use nvs_runtime::{Ctx, Fault, TraceKind, Value};

/// The site id in a site word: its low 32 bits, or zero, which names no site, for a word that is
/// not an `int`.
#[must_use]
pub(crate) fn site_id(word: Value) -> u32 {
    word.as_int()
        .and_then(|word| u32::try_from(word & crate::registry::QUERY_SITE_ID_MASK).ok())
        .unwrap_or(0)
}

/// Runs `body`, and files one `kind` event for it when the request is recorded.
///
/// `member` is the event's label, such as `Core\Cache\Store::get`, and a failed call's label
/// ends in ` failed`. `target` is what the editor's lens counts repeats of: the key or the file
/// path, and `None` for a member whose key is a `secret`'s. `site` is the member's last argument.
/// The event is filed on every way out of `body`, a thrown error included.
///
/// # Errors
///
/// What `body` returns.
pub(crate) fn watched<T>(
    ctx: &mut Ctx,
    site: Value,
    kind: TraceKind,
    member: &str,
    target: Option<&str>,
    body: impl FnOnce(&mut Ctx) -> Result<T, Fault>,
) -> Result<T, Fault> {
    watched_with(ctx, site, kind, member, target, body, |_| {
        Outcome::default()
    })
}

/// [`watched`], and the event carries the outcome `describe` reads off the call's result.
///
/// `describe` runs only when the event is filed, so a request nobody records never calls it.
/// It is given `None` for a call that failed.
///
/// # Errors
///
/// What `body` returns.
pub(crate) fn watched_with<T>(
    ctx: &mut Ctx,
    site: Value,
    kind: TraceKind,
    member: &str,
    target: Option<&str>,
    body: impl FnOnce(&mut Ctx) -> Result<T, Fault>,
    describe: impl FnOnce(Option<&T>) -> Outcome,
) -> Result<T, Fault> {
    if !ctx.records_spans() {
        return body(ctx);
    }
    let began = Instant::now();
    let result = body(ctx);
    let took = began.elapsed();
    if ctx.records_spans() {
        let label = if result.is_ok() {
            member.to_owned()
        } else {
            format!("{member} failed")
        };
        let outcome = describe(result.as_ref().ok());
        ctx.record_site_event(
            kind,
            &label,
            took,
            target.map(str::to_owned),
            site_id(site),
            outcome,
        );
    }
    result
}

/// The length of a `string` or `bytes` value, as a `file` event's `bytes`, and `None` for any
/// other value.
#[must_use]
pub(crate) fn length_of(value: &Value) -> Option<u64> {
    value
        .as_bytes()
        .or_else(|| value.as_text().map(str::as_bytes))
        .and_then(|bytes| u64::try_from(bytes.len()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A site word's low 32 bits are its id, and a word that is not an `int` names no site.
    #[test]
    fn a_site_word_gives_its_low_bits() {
        assert_eq!(
            site_id(Value::int(7 | crate::registry::QUERY_SITE_PLAIN_BINDS)),
            7
        );
        assert_eq!(site_id(Value::null()), 0);
    }
}
