//! `rule:http-server/static-serving-is-one-policy`'s conditional and range
//! decision: given the three request headers it reads, a representation's tag
//! and its length, which bytes the response carries and with which status.
//!
//! One pure function over header text, because it has two callers that must
//! never disagree: `nvs_server::statics::send` for a file, and
//! `Core\Response::stream` for a stream that states its length. That rule says
//! a second policy is a second security model, and two copies of this function
//! would be exactly that. It lives here because both `nvs-server` and
//! `nvs-stdlib` reach this crate and neither reaches the other.
//!
//! What it decides, in order:
//!
//! - **`If-None-Match`** against the tag, compared weakly
//!   (RFC 9110 § 13.1.2): a list is honoured, `*` matches, and a `W/` prefix is stripped.
//!   A match is a `304`.
//! - **`If-Range`**, read only when `Range` is present, compared strongly
//!   (RFC 9110 § 13.1.5): anything but the current tag makes the range ignored, so a
//!   resumed download of a changed representation is never two of them
//!   spliced together. A `W/` tag, a date and a value that does not parse all
//!   miss, because this server sends no `Last-Modified`.
//! - **One `Range`**, `bytes=` and a single spec. Anything else, including a
//!   range the length cannot satisfy, is a `416`, never a silent whole body:
//!   `rule:errors/ambiguous-input-refused`'s direction, since a client resuming
//!   a download would write the wrong bytes on the strength of it. A bound is
//!   ASCII digits and nothing else, so `+1` and a space are a `416` too, and so
//!   is a second `Range` line: a cache that read the last line would answer
//!   another window for the same request.
//!
//! A representation with no tag matches neither validator, so a resumed
//! download gets the whole body while a bare `Range` still seeks.
//!
//! It allocates nothing and holds nothing, so it spends nothing per request
//! (`rule:programs/memory-priority`).

/// The request headers the decision reads, as their raw values.
///
/// Bytes rather than text, because a request header value is bytes
/// ([`crate::Inbound::headers`]) and a value that is not header text is a
/// case the decision answers rather than one its callers each filter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Asked<'a> {
    /// The first `If-None-Match` field.
    pub if_none_match: Option<&'a [u8]>,
    /// The first `If-Range` field.
    pub if_range: Option<&'a [u8]>,
    /// The first `Range` field.
    pub range: Option<&'a [u8]>,
    /// Whether a second `Range` field arrived. The decision answers it as it
    /// answers a list of ranges.
    pub range_repeated: bool,
}

impl<'a> Asked<'a> {
    /// The three headers out of a request's field lines, matching names
    /// case-insensitively and keeping the first line of each name, which is
    /// what `http::HeaderMap::get` gives the server's own caller.
    pub fn from_fields(fields: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Self {
        let mut asked = Self::default();
        for (name, value) in fields {
            let slot = if name.eq_ignore_ascii_case("if-none-match") {
                &mut asked.if_none_match
            } else if name.eq_ignore_ascii_case("if-range") {
                &mut asked.if_range
            } else if name.eq_ignore_ascii_case("range") {
                asked.range_repeated |= asked.range.is_some();
                &mut asked.range
            } else {
                continue;
            };
            slot.get_or_insert(value);
        }
        asked
    }
}

/// What the response is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// `304`: `If-None-Match` named the current tag, and no body is sent.
    NotModified,
    /// `416`: a `Range` this server cannot honour exactly, and no body is sent.
    /// The response states `Content-Range: bytes */<length>`.
    Unsatisfiable,
    /// These bytes are sent: the whole body with a `200`, or one range with a
    /// `206`.
    Send(Window),
}

/// Which bytes of a representation the response carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    /// The first byte, counted from the start of the representation.
    pub at: u64,
    /// How many bytes, which is the response's `Content-Length`.
    pub len: u64,
    /// Whether this is a `206` over a `Range` rather than the whole body.
    pub partial: bool,
}

impl Window {
    /// The `Content-Range` value a `206` states, for a representation of
    /// `total` bytes.
    #[must_use]
    pub fn content_range(self, total: u64) -> String {
        // A partial window is never empty, so this neither wraps nor reads as
        // a range when it is not one.
        let last = (self.at + self.len).saturating_sub(1);
        format!("bytes {}-{last}/{total}", self.at)
    }
}

/// The `Content-Range` value a `416` states, for a representation of `total`
/// bytes.
#[must_use]
pub fn unsatisfied_range(total: u64) -> String {
    format!("bytes */{total}")
}

/// The head a stream of `total` bytes tagged `etag` is sent with, and the
/// window the program writes — [`decide`] for `Core\Response::stream`.
#[derive(Debug)]
pub struct Ranged {
    /// The status the decision sets, and `None` for a `200`, which a status
    /// the program declared replaces.
    pub status: Option<u16>,
    /// `Accept-Ranges`, the `ETag` where there is one, and `Content-Range`
    /// on a `206` and a `416`. The program's own headers go after these.
    pub headers: Vec<crate::DeclaredHeader>,
    /// The bytes the program writes, which are none on a `304` and a `416`.
    pub window: crate::stream::Window,
}

/// [`decide`] over a stream, as the head and window [`Ranged`] describes.
#[must_use]
pub fn ranged(asked: Asked<'_>, etag: Option<&str>, total: u64) -> Ranged {
    use crate::DeclaredHeader;
    use crate::stream::Window;

    let mut headers = vec![DeclaredHeader::set("Accept-Ranges", "bytes")];
    if let Some(etag) = etag {
        headers.push(DeclaredHeader::set("ETag", etag));
    }
    let (status, window) = match decide(asked, etag, total) {
        Decision::NotModified => (Some(304), Window::new(0, 0)),
        Decision::Unsatisfiable => {
            headers.push(DeclaredHeader::set(
                "Content-Range",
                &unsatisfied_range(total),
            ));
            (Some(416), Window::new(0, 0))
        }
        Decision::Send(window) => {
            let status = window.partial.then(|| {
                headers.push(DeclaredHeader::set(
                    "Content-Range",
                    &window.content_range(total),
                ));
                206
            });
            (status, Window::new(window.at, window.len))
        }
    };
    Ranged {
        status,
        headers,
        window,
    }
}

/// The decision for a representation of `total` bytes whose strong tag is
/// `tag`, quoted, and `None` where it has none.
#[must_use]
pub fn decide(asked: Asked<'_>, tag: Option<&str>, total: u64) -> Decision {
    if tag.is_some_and(|tag| unchanged(asked.if_none_match, tag)) {
        return Decision::NotModified;
    }
    let range = asked.range.filter(|_| still_the_same(asked.if_range, tag));
    if range.is_some() && asked.range_repeated {
        return Decision::Unsatisfiable;
    }
    asked_for(range, total).map_or(Decision::Unsatisfiable, Decision::Send)
}

/// A header value as text, where it is the visible ASCII and tabs a field
/// value may carry, and `None` where it is anything else.
fn text(value: &[u8]) -> Option<&str> {
    value
        .iter()
        .all(|&byte| byte == b'\t' || (b' '..=b'~').contains(&byte))
        .then(|| std::str::from_utf8(value).ok())
        .flatten()
}

/// Whether `If-None-Match` names `tag`.
fn unchanged(asked: Option<&[u8]>, tag: &str) -> bool {
    let Some(asked) = asked.and_then(text) else {
        return false;
    };
    asked.split(',').map(str::trim).any(|candidate| {
        candidate == "*" || candidate.strip_prefix("W/").unwrap_or(candidate) == tag
    })
}

/// Whether a `Range` may be honoured: there is no `If-Range`, or it names the
/// current tag exactly.
fn still_the_same(asked: Option<&[u8]>, tag: Option<&str>) -> bool {
    let Some(asked) = asked else {
        return true;
    };
    tag.is_some_and(|tag| text(asked).is_some_and(|asked| asked.trim() == tag))
}

/// The range policy: no header is the whole body, one `bytes=` spec is that
/// spec, and anything else is `None`.
///
/// The refusals are one arm each on purpose — a comma, a unit that is not
/// `bytes`, a bound that is not ASCII digits alone (a sign or a space included),
/// an inverted pair, a suffix of zero and a start past the end all answer
/// `None`. An `end` past the last byte is the one repair the RFC requires, and
/// it is a clamp of a range that was satisfiable, not a different answer.
fn asked_for(range: Option<&[u8]>, total: u64) -> Option<Window> {
    let Some(range) = range else {
        return Some(Window {
            at: 0,
            len: total,
            partial: false,
        });
    };
    let spec = text(range)?.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (first, last) = spec.split_once('-')?;
    let (at, end) = match (first.is_empty(), last.is_empty()) {
        (true, true) => return None,
        // `bytes=-N`: the last N bytes, and never more than there are.
        (true, false) => {
            let suffix = position(last)?;
            if suffix == 0 {
                return None;
            }
            (total.saturating_sub(suffix), total.checked_sub(1)?)
        }
        // `bytes=N-`: from N to the end.
        (false, true) => (position(first)?, total.checked_sub(1)?),
        (false, false) => {
            let at = position(first)?;
            let end = position(last)?;
            if end < at {
                return None;
            }
            (at, end.min(total.checked_sub(1)?))
        }
    };
    if at > end {
        return None;
    }
    Some(Window {
        at,
        len: end - at + 1,
        partial: true,
    })
}

/// One bound of a range spec: ASCII digits and nothing else. `u64::from_str`
/// also takes a leading `+`, which RFC 9110's `DIGIT` does not.
fn position(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{Asked, Decision, Window, decide};

    const TAG: &str = "\"a-1\"";

    fn asked<'a>(fields: &[(&'a str, &'a str)]) -> Asked<'a> {
        Asked::from_fields(fields.iter().map(|(name, value)| (*name, value.as_bytes())))
    }

    fn part(at: u64, len: u64) -> Decision {
        Decision::Send(Window {
            at,
            len,
            partial: true,
        })
    }

    const WHOLE: Decision = Decision::Send(Window {
        at: 0,
        len: 10,
        partial: false,
    });

    /// The names are matched in any case, and the first line of a name is the
    /// one read.
    #[test]
    fn the_fields_are_read_by_name_in_any_case() {
        let read = asked(&[
            ("RANGE", "bytes=0-1"),
            ("range", "bytes=5-6"),
            ("If-Range", TAG),
        ]);
        assert_eq!(read.range, Some("bytes=0-1".as_bytes()));
        assert_eq!(read.if_range, Some(TAG.as_bytes()));
        assert_eq!(read.if_none_match, None);
        assert!(read.range_repeated);
    }

    /// No header is the whole body, one range is that range clamped to the
    /// end, and every other range is a `416`.
    #[test]
    fn one_range_is_honoured_and_every_other_is_unsatisfiable() {
        assert_eq!(decide(asked(&[]), Some(TAG), 10), WHOLE);
        assert_eq!(
            decide(asked(&[("Range", "bytes=2-4")]), Some(TAG), 10),
            part(2, 3)
        );
        assert_eq!(
            decide(asked(&[("Range", "bytes=8-99")]), Some(TAG), 10),
            part(8, 2)
        );
        assert_eq!(
            decide(asked(&[("Range", "bytes=-3")]), Some(TAG), 10),
            part(7, 3)
        );
        assert_eq!(
            decide(asked(&[("Range", "bytes=4-")]), Some(TAG), 10),
            part(4, 6)
        );
        for refused in [
            "bytes=0-1,4-5",
            "items=0-1",
            "bytes=5-2",
            "bytes=-0",
            "bytes=10-",
            "bytes=x-",
            "bytes=+1-2",
            "bytes=1-+2",
            "bytes=-+3",
            "bytes= 0-1",
            "bytes=0 -1",
            "bytes=0- 1",
        ] {
            assert_eq!(
                decide(asked(&[("Range", refused)]), Some(TAG), 10),
                Decision::Unsatisfiable,
                "{refused}"
            );
        }
        assert_eq!(
            decide(asked(&[("Range", "bytes=0-")]), Some(TAG), 0),
            Decision::Unsatisfiable
        );
    }

    /// Two `Range` lines are a `416`, as a list of ranges is, and an `If-Range`
    /// that misses still answers the whole body.
    #[test]
    fn two_range_lines_are_unsatisfiable() {
        let twice = [("Range", "bytes=0-1"), ("Range", "bytes=5-6")];
        assert_eq!(
            decide(asked(&twice), Some(TAG), 10),
            Decision::Unsatisfiable
        );
        let stale = [
            ("Range", "bytes=0-1"),
            ("Range", "bytes=5-6"),
            ("If-Range", "\"old\""),
        ];
        assert_eq!(decide(asked(&stale), Some(TAG), 10), WHOLE);
        assert!(!asked(&[("Range", "bytes=0-1")]).range_repeated);
    }

    /// `If-None-Match` compares weakly and wins over a range; `If-Range`
    /// compares strongly, and a miss is the whole body.
    #[test]
    fn the_validators_compare_against_the_tag() {
        for matching in [TAG, "W/\"a-1\"", "\"z\", \"a-1\"", "*"] {
            assert_eq!(
                decide(
                    asked(&[("If-None-Match", matching), ("Range", "bytes=0-1")]),
                    Some(TAG),
                    10
                ),
                Decision::NotModified,
                "{matching}"
            );
        }
        assert_eq!(
            decide(
                asked(&[("If-Range", TAG), ("Range", "bytes=0-1")]),
                Some(TAG),
                10
            ),
            part(0, 2)
        );
        for missing in ["\"old\"", "W/\"a-1\"", "Tue, 15 Nov 1994 08:12:31 GMT", ""] {
            assert_eq!(
                decide(
                    asked(&[("If-Range", missing), ("Range", "bytes=0-1")]),
                    Some(TAG),
                    10
                ),
                WHOLE,
                "{missing}"
            );
        }
        // A range that would be a `416` is ignored too when `If-Range` misses.
        assert_eq!(
            decide(
                asked(&[("If-Range", "\"old\""), ("Range", "bytes=99-")]),
                Some(TAG),
                10
            ),
            WHOLE
        );
    }

    /// Without a tag neither validator matches, and a bare range still seeks.
    #[test]
    fn a_representation_without_a_tag_matches_no_validator() {
        assert_eq!(decide(asked(&[("If-None-Match", "*")]), None, 10), WHOLE);
        assert_eq!(
            decide(
                asked(&[("If-Range", TAG), ("Range", "bytes=0-1")]),
                None,
                10
            ),
            WHOLE
        );
        assert_eq!(
            decide(asked(&[("Range", "bytes=0-1")]), None, 10),
            part(0, 2)
        );
    }
}
