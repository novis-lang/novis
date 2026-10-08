//! The server's schema as far as a typed reader needs it: the syntax of every attribute type, read from the subschema entry
//!
//! `rule:core-classes/ldap-value-types` takes an attribute's base type from
//! the schema. [`read`] asks the root DSE for `subschemaSubentry` and reads
//! that entry's `attributeTypes`, RFC 4512 § 4.1.2's descriptions, and
//! [`Schema::parse`] keeps each type's names and `SYNTAX` OID. Active
//! Directory and Samba publish the same descriptions at
//! `CN=Aggregate,CN=Schema,CN=Configuration,…`, with each `SYNTAX` derived
//! from the type's `attributeSyntax` and `oMSyntax`, so one reader serves
//! every server. A type with no `SYNTAX` takes its `SUP`'s.
//!
//! A server that names no subschema entry has an empty [`Schema`], and every
//! attribute reads as [`Syntax::Other`]. Who caches a schema, and for how
//! long, is the caller's decision.

use std::collections::HashMap;

use crate::conn::{Connection, Cursor};
use crate::error::Error;
use crate::proto::{Filter, Scope, SearchRequest};

/// The syntaxes a typed reader treats apart, by the OID that names each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    /// RFC 4517's Boolean, `TRUE` or `FALSE`.
    Boolean,
    /// RFC 4517's Integer.
    Integer,
    /// AD's `LargeInteger`, a 64-bit integer: a FILETIME, an interval, or a count.
    LargeInteger,
    /// RFC 4517's GeneralizedTime.
    GeneralizedTime,
    /// Any other syntax, or an attribute the schema does not declare.
    Other,
}

impl Syntax {
    /// The syntax `oid` names, with an RFC 4512 `{length}` suffix ignored.
    #[must_use]
    pub fn of_oid(oid: &str) -> Self {
        let oid = oid.split_once('{').map_or(oid, |(bare, _)| bare);
        match oid {
            "1.3.6.1.4.1.1466.115.121.1.7" => Self::Boolean,
            "1.3.6.1.4.1.1466.115.121.1.27" => Self::Integer,
            "1.2.840.113556.1.4.906" => Self::LargeInteger,
            "1.3.6.1.4.1.1466.115.121.1.24" => Self::GeneralizedTime,
            _ => Self::Other,
        }
    }
}

/// Every attribute type's syntax, by each of its names and its OID, without case.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schema {
    syntaxes: HashMap<String, Syntax>,
}

/// One attribute type description, before `SUP` is followed.
struct Described {
    keys: Vec<String>,
    syntax: Option<String>,
    sup: Option<String>,
}

impl Schema {
    /// The schema the `attributeTypes` values in `descriptions` declare. A
    /// value that does not read as a description is skipped.
    #[must_use]
    pub fn parse<'a>(descriptions: impl IntoIterator<Item = &'a [u8]>) -> Self {
        let described: Vec<Described> = descriptions
            .into_iter()
            .filter_map(|value| describe(&String::from_utf8_lossy(value)))
            .collect();
        let mut by_key = HashMap::new();
        for (at, one) in described.iter().enumerate() {
            for key in &one.keys {
                by_key.insert(key.clone(), at);
            }
        }
        let mut syntaxes = HashMap::new();
        for one in &described {
            // A `SUP` chain is followed a bounded number of steps, so a
            // schema whose types name each other in a loop still parses.
            let mut current = one;
            let mut syntax = Syntax::Other;
            for _ in 0..16 {
                if let Some(oid) = &current.syntax {
                    syntax = Syntax::of_oid(oid);
                    break;
                }
                let Some(sup) = current
                    .sup
                    .as_ref()
                    .and_then(|sup| by_key.get(&sup.to_ascii_lowercase()))
                else {
                    break;
                };
                current = &described[*sup];
            }
            for key in &one.keys {
                syntaxes.insert(key.clone(), syntax);
            }
        }
        Self { syntaxes }
    }

    /// The syntax of the attribute named `name` or numbered by it, matched
    /// without case, or [`Syntax::Other`] for one the schema does not declare.
    #[must_use]
    pub fn syntax(&self, name: &str) -> Syntax {
        self.syntaxes
            .get(&name.to_ascii_lowercase())
            .copied()
            .unwrap_or(Syntax::Other)
    }

    /// How many names and OIDs the schema declares a syntax for.
    #[must_use]
    pub fn len(&self) -> usize {
        self.syntaxes.len()
    }

    /// Whether the schema declares nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.syntaxes.is_empty()
    }
}

/// The tokens of one description: `(`, `)`, a quoted string without its
/// quotes, or a bare word.
fn tokens(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    loop {
        rest = rest.trim_start();
        let Some(first) = rest.chars().next() else {
            return out;
        };
        let taken = match first {
            '(' | ')' => {
                out.push(&rest[..1]);
                1
            }
            '\'' => {
                let end = rest[1..].find('\'').map_or(rest.len(), |at| at + 1);
                out.push(&rest[1..end]);
                (end + 1).min(rest.len())
            }
            _ => {
                let end = rest
                    .find(|c: char| c.is_whitespace() || c == '(' || c == ')')
                    .unwrap_or(rest.len());
                out.push(&rest[..end]);
                end
            }
        };
        rest = &rest[taken..];
    }
}

/// The OID, names, `SYNTAX` and `SUP` of one attribute type description.
fn describe(text: &str) -> Option<Described> {
    let tokens = tokens(text);
    if tokens.first() != Some(&"(") {
        return None;
    }
    let mut keys = vec![tokens.get(1)?.to_ascii_lowercase()];
    let mut syntax = None;
    let mut sup = None;
    let mut at = 2;
    while at < tokens.len() {
        match tokens[at] {
            "NAME" => {
                at += 1;
                if tokens.get(at) == Some(&"(") {
                    at += 1;
                    while let Some(&name) = tokens.get(at).filter(|&&token| token != ")") {
                        keys.push(name.to_ascii_lowercase());
                        at += 1;
                    }
                } else if let Some(name) = tokens.get(at) {
                    keys.push(name.to_ascii_lowercase());
                }
            }
            "SYNTAX" => {
                at += 1;
                syntax = tokens.get(at).map(|&oid| oid.to_owned());
            }
            "SUP" => {
                at += 1;
                sup = tokens.get(at).map(|&name| name.to_owned());
            }
            _ => {}
        }
        at += 1;
    }
    Some(Described { keys, syntax, sup })
}

/// Reads the schema from the subschema entry the root DSE names, over `connection`.
///
/// # Errors
///
/// Every failure the two searches report.
pub fn read(connection: &mut Connection) -> Result<Schema, Error> {
    let every = Filter::Present("objectClass".to_owned());
    let Some(subentry) = base_values(connection, "", &every, "subschemaSubentry")?
        .into_iter()
        .next()
    else {
        return Ok(Schema::default());
    };
    let subentry = String::from_utf8_lossy(&subentry).into_owned();
    let descriptions = base_values(connection, &subentry, &every, "attributeTypes")?;
    Ok(Schema::parse(descriptions.iter().map(Vec::as_slice)))
}

/// The values of `attribute` on the entry at `dn`, or none when it has no such entry.
fn base_values(
    connection: &mut Connection,
    dn: &str,
    filter: &Filter,
    attribute: &str,
) -> Result<Vec<Vec<u8>>, Error> {
    let mut cursor = Cursor::new(&SearchRequest {
        base: dn,
        scope: Scope::Base,
        filter,
        attributes: &[attribute],
        page_size: 1,
        size_limit: 0,
        time_limit: 0,
    });
    let mut values = Vec::new();
    // The loop runs to the end so the connection is settled when it returns.
    while let Some(next) = cursor.next(connection) {
        if let Some(found) = next?.get(attribute) {
            values = found.to_vec();
        }
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_description_gives_its_names_and_syntax_and_a_sup_lends_its_own() {
        let schema = Schema::parse([
            b"( 1.2.840.113556.1.4.96 NAME 'pwdLastSet' SYNTAX '1.2.840.113556.1.4.906' SINGLE-VALUE )"
                .as_slice(),
            b"( 2.5.18.1 NAME ( 'createTimestamp' 'created' ) SYNTAX 1.3.6.1.4.1.1466.115.121.1.24{64} )",
            b"( 1.2.840.113556.1.2.48 NAME 'isDeleted' SYNTAX '1.3.6.1.4.1.1466.115.121.1.7' )",
            b"( 1.3 NAME 'isGone' SUP isDeleted )",
            b"( 2.5.4.3 NAME 'cn' SYNTAX 1.3.6.1.4.1.1466.115.121.1.15{64} )",
            b"( 1.1 NAME 'loopA' SUP loopB )",
            b"( 1.2 NAME 'loopB' SUP loopA )",
            b"not a description",
        ]);
        assert_eq!(schema.syntax("PWDLASTSET"), Syntax::LargeInteger);
        assert_eq!(schema.syntax("1.2.840.113556.1.4.96"), Syntax::LargeInteger);
        assert_eq!(schema.syntax("created"), Syntax::GeneralizedTime);
        assert_eq!(schema.syntax("createTimestamp"), Syntax::GeneralizedTime);
        assert_eq!(
            schema.syntax("isGone"),
            Syntax::Boolean,
            "taken from its `SUP`"
        );
        assert_eq!(schema.syntax("cn"), Syntax::Other);
        assert_eq!(schema.syntax("loopA"), Syntax::Other);
        assert_eq!(schema.syntax("undeclared"), Syntax::Other);
    }
}
