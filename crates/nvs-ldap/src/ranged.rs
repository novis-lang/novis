//! Ranged attribute retrieval: a multi-valued attribute too long for one answer, fetched to its end and returned whole under its plain name
//!
//! Active Directory sends at most `MaxValRange` values of one attribute in an
//! entry (1500 by default). Past that it sends `member;range=0-1499` instead of
//! `member`, and the client asks for `member;range=1500-*` with a base search
//! on the same entry, and so on until an answer's range ends in `*`
//! (`rule:core-classes/ldap-value-types`, ADR 0278 § 8). [`complete`] runs
//! that loop for every ranged attribute of an entry before [`Cursor::next`]
//! returns it, so no caller sees a range option or a partial list.
//!
//! Each step is one base search, run on the connection the entry came from
//! while the search that found it waits between pages. The merged list is the
//! whole attribute, held in the entry for as long as the entry is.

use crate::conn::{Connection, Cursor};
use crate::error::{Error, Kind};
use crate::proto::{Entry, Filter, Scope, SearchRequest};

/// An attribute description with a `range=<first>-<last>` option, split into
/// the description without that option, the first index and the last one,
/// which is `None` for `*`: the range runs to the end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    /// The description with the range option removed, `member` for `member;range=0-1499`.
    pub plain: String,
    /// The index of the first value the answer carries.
    pub first: u64,
    /// The index of the last value it carries, or `None` when it carries the rest.
    pub last: Option<u64>,
}

/// The range `name` carries, or `None` for a name with no `range=` option or
/// one that does not read as `<first>-<last>` or `<first>-*`.
#[must_use]
pub fn parse(name: &str) -> Option<Range> {
    let mut parts = name.split(';');
    let mut plain = parts.next()?.to_owned();
    let mut found = None;
    for option in parts {
        let bounds = option
            .get(..6)
            .filter(|head| head.eq_ignore_ascii_case("range="))
            .map(|_| &option[6..]);
        match bounds {
            Some(bounds) if found.is_none() => {
                let (first, last) = bounds.split_once('-')?;
                let first = first.parse().ok()?;
                let last = match last {
                    "*" => None,
                    digits => Some(digits.parse().ok()?),
                };
                found = Some((first, last));
            }
            _ => {
                plain.push(';');
                plain.push_str(option);
            }
        }
    }
    let (first, last) = found?;
    Some(Range { plain, first, last })
}

/// `entry` with every ranged attribute fetched to its end over `connection`
/// and named by its plain description. A plain attribute of the same name
/// the server sent beside the ranged one is folded into it.
///
/// # Errors
///
/// Every failure the follow-up searches report, and [`Kind::Protocol`] for an
/// answer whose range does not start where the last one ended.
pub fn complete(connection: &mut Connection, mut entry: Entry) -> Result<Entry, Error> {
    let mut at = 0;
    while at < entry.attributes.len() {
        let Some(range) = parse(&entry.attributes[at].name) else {
            at += 1;
            continue;
        };
        let mut values = std::mem::take(&mut entry.attributes[at].values);
        let mut last = range.last;
        while let Some(ended) = last {
            let next = ended + 1;
            match fetch(connection, &entry.dn, &range.plain, next)? {
                Some(slice) => {
                    values.extend(slice.values);
                    last = slice.last;
                }
                None => break,
            }
        }
        entry.attributes[at].name.clone_from(&range.plain);
        entry.attributes[at].values = values;
        if let Some(twin) = (0..entry.attributes.len()).find(|&other| {
            other != at
                && entry.attributes[other]
                    .name
                    .eq_ignore_ascii_case(&range.plain)
        }) {
            let mut before = entry.attributes.remove(twin).values;
            if twin < at {
                at -= 1;
            }
            before.append(&mut entry.attributes[at].values);
            entry.attributes[at].values = before;
        }
        at += 1;
    }
    Ok(entry)
}

/// One answer of a follow-up: its values, and the last index it carries, or
/// `None` when it carries the rest.
struct Slice {
    values: Vec<Vec<u8>>,
    last: Option<u64>,
}

/// The values of `plain` on the entry at `dn` from index `from` on, or
/// `None` when the entry no longer carries a range of it.
fn fetch(
    connection: &mut Connection,
    dn: &str,
    plain: &str,
    from: u64,
) -> Result<Option<Slice>, Error> {
    let every = Filter::Present("objectClass".to_owned());
    let asked = format!("{plain};range={from}-*");
    let mut cursor = Cursor::single(&SearchRequest {
        base: dn,
        scope: Scope::Base,
        filter: &every,
        attributes: &[asked.as_str()],
        page_size: 1,
        size_limit: 0,
        time_limit: 0,
        sort: None,
        window: None,
        show_deleted: false,
    });
    let mut found = None;
    // A base search has one entry at most, and the loop runs to the end so
    // the connection is settled when it returns.
    while let Some(next) = cursor.next(connection) {
        let answer = next?;
        for attribute in answer.attributes {
            let Some(range) = parse(&attribute.name) else {
                continue;
            };
            if !range.plain.eq_ignore_ascii_case(plain) {
                continue;
            }
            if range.first != from || range.last.is_some_and(|last| last < from) {
                return Err(Error::new(
                    Kind::Protocol,
                    format!(
                        "the server answered `{asked}` with `{}`, which does not continue \
                         the range",
                        attribute.name
                    ),
                ));
            }
            found = Some(Slice {
                values: attribute.values,
                last: range.last,
            });
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_option_splits_from_its_description() {
        assert_eq!(
            parse("member;range=0-1499"),
            Some(Range {
                plain: "member".to_owned(),
                first: 0,
                last: Some(1499)
            })
        );
        assert_eq!(
            parse("member;Range=1500-*"),
            Some(Range {
                plain: "member".to_owned(),
                first: 1500,
                last: None
            })
        );
        assert_eq!(
            parse("member;binary;range=3-4").map(|range| range.plain),
            Some("member;binary".to_owned())
        );
        for name in [
            "member",
            "member;range=",
            "member;range=1",
            "member;range=a-*",
        ] {
            assert_eq!(parse(name), None, "{name}");
        }
    }
}
