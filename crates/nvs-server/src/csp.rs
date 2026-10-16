//! Adding sources to a `Content-Security-Policy` so that an inline element this
//! server writes itself is allowed to apply.
//!
//! Two callers use it: [`crate::slotted::allow_scripts`] adds the hashes of a
//! slotted page's two scripts, and [`crate::debug_style::allow_style`] adds the
//! nonce of the `[debug] inline` block's `<style>` element.
//!
//! [`add_sources`] amends the first directive in a list it is given that the
//! policy has, such as `script-src-elem`, then `script-src`, then
//! `default-src`. A policy with none of them limits no such element and is left
//! alone. So is one whose directive allows every inline element already:
//! `'unsafe-inline'` with no nonce, hash or `'strict-dynamic'` beside it, which
//! an added nonce or hash would switch off. A `'none'` in that directive is
//! dropped, since it cannot stand beside a source.

use hyper::HeaderMap;
use hyper::header::{self, HeaderValue};

/// Adds `sources` to every `Content-Security-Policy` in `headers`, in the first
/// of `directives` each policy has, as the module doc says.
///
/// A header line may carry several policies separated by commas, and each one
/// is amended on its own. A line that is not text, or that would not be a valid
/// header value once amended, is kept as it was.
pub fn add_sources(headers: &mut HeaderMap, directives: &[&str], sources: &[&str]) {
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
                    .map(|one| amend(one, directives, sources).unwrap_or_else(|| one.to_owned()))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .and_then(|text| HeaderValue::from_str(&text).ok());
        headers.append(header::CONTENT_SECURITY_POLICY, amended.unwrap_or(policy));
    }
}

/// One policy with `sources` added, and `None` where it needs none.
fn amend(policy: &str, wanted: &[&str], added: &[&str]) -> Option<String> {
    let mut directives: Vec<String> = policy.split(';').map(str::to_owned).collect();
    let index = wanted.iter().find_map(|wanted| {
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
    for source in added {
        rewritten.push(' ');
        rewritten.push_str(source);
    }
    if index == 0 {
        rewritten.remove(0);
    }
    directives[index] = rewritten;
    Some(directives.join(";"))
}
