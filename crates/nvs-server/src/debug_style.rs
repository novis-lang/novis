//! `rule:errors/debug-dump`'s `[debug] inline` block, as this server sends it:
//! the style policy that lets the block's `<style>` element apply.
//!
//! The runtime writes the block once per response, with a `<style nonce="N">`
//! element, and the request's completion carries `N` out as
//! `Completion::style_nonce`. A policy that limits styles would block that
//! element, and the block would show without its layout. So `serve` leaves a
//! [`DebugStyle`] on such a response, and after the secure header set is
//! filled, [`allow_style`] adds `'nonce-N'` to the directive that governs an
//! inline `<style>` element: `style-src-elem`, else `style-src`, else
//! `default-src`. [`crate::csp`] owns which policies are left alone.
//!
//! # What it spends
//!
//! One short string per response that has a block, and nothing on any other.

use hyper::HeaderMap;

/// The mark `serve` leaves on a response whose body has a `[debug] inline`
/// block, carrying the nonce of its `<style>` element. It tells the policy step
/// to call [`allow_style`].
#[derive(Clone, Debug)]
pub struct DebugStyle(pub Box<str>);

/// Adds `'nonce-<nonce>'` to every `Content-Security-Policy` in `headers` that
/// limits inline styles, as the module doc says.
pub fn allow_style(headers: &mut HeaderMap, nonce: &str) {
    let source = format!("'nonce-{nonce}'");
    crate::csp::add_sources(
        headers,
        &["style-src-elem", "style-src", "default-src"],
        &[&source],
    );
}

#[cfg(test)]
mod tests {
    use super::allow_style;

    fn amended(policy: &str) -> String {
        let mut headers = hyper::HeaderMap::new();
        headers.insert(
            hyper::header::CONTENT_SECURITY_POLICY,
            hyper::header::HeaderValue::from_str(policy).expect("a policy the wire can carry"),
        );
        allow_style(&mut headers, "N1");
        headers[hyper::header::CONTENT_SECURITY_POLICY]
            .to_str()
            .expect("an amended policy is text")
            .to_owned()
    }

    #[test]
    fn a_style_src_policy_gains_the_style_nonce() {
        assert_eq!(
            amended("style-src 'self'; frame-ancestors 'none'"),
            "style-src 'self' 'nonce-N1'; frame-ancestors 'none'"
        );
        assert_eq!(
            amended("style-src 'self'; style-src-elem 'self'"),
            "style-src 'self'; style-src-elem 'self' 'nonce-N1'"
        );
    }

    #[test]
    fn a_default_src_none_policy_becomes_the_style_nonce_alone() {
        assert_eq!(amended("default-src 'none'"), "default-src 'nonce-N1'");
    }

    #[test]
    fn a_policy_that_does_not_limit_styles_gets_no_nonce() {
        for unchanged in [
            "frame-ancestors 'none'",
            "img-src 'self'; script-src 'self'",
            "style-src 'unsafe-inline'",
            "default-src 'self' 'unsafe-inline'",
        ] {
            assert_eq!(
                amended(unchanged),
                unchanged,
                "a policy that allows the style changed"
            );
        }
    }

    #[test]
    fn unsafe_inline_beside_a_nonce_still_gains_the_style_nonce() {
        assert_eq!(
            amended("style-src 'nonce-abc' 'unsafe-inline'"),
            "style-src 'nonce-abc' 'unsafe-inline' 'nonce-N1'"
        );
    }
}
