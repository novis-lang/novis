//! The entries of the registry's transparency log
//! (`rule:packaging/the-registry-keeps-an-append-only-log`): one kind per event the registry acts
//! on, a closed list, each written as canonical JSON with a schema version.
//!
//! An entry is a JSON object with `"v": 1`, a `"kind"` and that kind's fields, every one a string
//! that is not empty, each `version` and `fixed_in` a package version, each `versions` a
//! [`Range`] of them, and each `digest` a package's BLAKE3
//! digest, 64 lowercase hex digits (`rule:packaging/a-package-is-its-digest`). Its canonical
//! form has the keys sorted by their bytes, no whitespace, and strings escaped as `serde_json`
//! escapes them: `"`, `\` and the control characters alone, which is RFC 8785's form for such an
//! object.
//! [`Entry::parse`] refuses an unknown kind, a missing or extra field and another schema version,
//! and [`Entry::canonical`] writes the form the log stores, so one event is the same bytes
//! whoever wrote it. The log has no time in it: the order of its records is the only order the
//! registry claims.

use std::str::FromStr as _;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::index;
use super::manifest::{Range, Version};

/// The schema version every entry carries as `"v"`.
pub(crate) const SCHEMA: u64 = 1;

/// Whether an account became an owner or stopped being one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Change {
    Added,
    Removed,
}

/// Whether a token was created or revoked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TokenChange {
    Created,
    Revoked,
}

/// One event the registry acts on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Entry {
    /// A version was published: its archive's digest, and the repository and commit it was
    /// built from.
    Publish {
        package: String,
        version: String,
        digest: String,
        repository: String,
        commit: String,
    },
    /// An account became an owner of a package, or stopped being one.
    Ownership {
        package: String,
        account: String,
        change: Change,
    },
    /// An account's publish token was created or revoked. `token` is the token's id, never its
    /// secret.
    Token {
        account: String,
        token: String,
        change: TokenChange,
    },
    /// The repository a package publishes from was bound to a new one.
    Repository { package: String, repository: String },
    /// A version was unlisted.
    Unlist { package: String, version: String },
    /// An unlist was reversed.
    Relist { package: String, version: String },
    /// A version was retracted, and the version that fixes it when one is named. An entry with
    /// no `fixed_in` leaves it out of its canonical form.
    Retract {
        package: String,
        version: String,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fixed_in: Option<String>,
    },
    /// A version was withheld.
    Withhold {
        package: String,
        version: String,
        reason: String,
    },
    /// An advisory was published against a range of a package's versions.
    Advisory {
        id: String,
        package: String,
        versions: String,
        summary: String,
    },
    /// An owner released a version before its seven days ended.
    ReleaseConfirmed { package: String, version: String },
    /// An owner confirmed a version's API as stable.
    StableApiConfirmed { package: String, version: String },
}

impl Entry {
    /// Reads one entry from JSON in any key order and spacing.
    pub(crate) fn parse(json: &[u8]) -> Result<Self, String> {
        let value: Value =
            serde_json::from_slice(json).map_err(|err| format!("not JSON: {err}"))?;
        let Value::Object(mut object) = value else {
            return Err("an entry is a JSON object".to_owned());
        };
        match object.remove("v") {
            Some(Value::Number(v)) if v.as_u64() == Some(SCHEMA) => {}
            Some(v) => return Err(format!("schema version {v} is not {SCHEMA}")),
            None => return Err("the entry has no schema version `v`".to_owned()),
        }
        for (key, value) in &object {
            match value {
                Value::String(text) if text.is_empty() => {
                    return Err(format!("`{key}` is empty"));
                }
                Value::String(text) if key == "version" || key == "fixed_in" => {
                    Version::from_str(text)?;
                }
                Value::String(text) if key == "versions" => {
                    Range::from_str(text)?;
                }
                Value::String(text) if key == "digest" && index::digest(text).is_none() => {
                    return Err(format!("`{text}` is not 64 lowercase hex digits"));
                }
                _ => {}
            }
        }
        serde_json::from_value(Value::Object(object)).map_err(|err| err.to_string())
    }

    /// The entry's canonical JSON, the bytes the log stores.
    pub(crate) fn canonical(&self) -> Vec<u8> {
        let Value::Object(mut object) =
            serde_json::to_value(self).expect("an entry is a map of strings")
        else {
            unreachable!("an internally tagged enum serializes as an object")
        };
        object.insert("v".to_owned(), Value::from(SCHEMA));
        let mut out = Vec::new();
        write_canonical(&Value::Object(object), &mut out);
        out
    }
}

/// Writes `value` with every object's keys sorted and no whitespace.
fn write_canonical(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Object(object) => {
            out.push(b'{');
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            for (n, key) in keys.into_iter().enumerate() {
                if n > 0 {
                    out.push(b',');
                }
                write_canonical(&Value::String(key.clone()), out);
                out.push(b':');
                write_canonical(&object[key], out);
            }
            out.push(b'}');
        }
        Value::Array(items) => {
            out.push(b'[');
            for (n, item) in items.iter().enumerate() {
                if n > 0 {
                    out.push(b',');
                }
                write_canonical(item, out);
            }
            out.push(b']');
        }
        scalar => out.extend(serde_json::to_vec(scalar).expect("a scalar serializes")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_reads_in_any_order_and_writes_one_canonical_form() {
        let json = br#" { "version": "1.4.0", "kind": "publish", "v": 1,
            "package": "acme/http", "digest": "00000000000000000000000000000000000000000000000000000000000000ff", "commit": "abc",
            "repository": "https://example.com/acme/http" } "#;
        let entry = Entry::parse(json).unwrap();
        let canonical = entry.canonical();
        assert_eq!(
            String::from_utf8(canonical.clone()).unwrap(),
            r#"{"commit":"abc","digest":"00000000000000000000000000000000000000000000000000000000000000ff","kind":"publish","package":"acme/http","repository":"https://example.com/acme/http","v":1,"version":"1.4.0"}"#
        );
        assert_eq!(Entry::parse(&canonical).unwrap(), entry);
    }

    #[test]
    fn an_entry_outside_the_list_or_its_schema_is_refused() {
        for json in [
            r#"{"v":1,"kind":"delete","package":"acme/http"}"#,
            r#"{"v":2,"kind":"unlist","package":"acme/http","version":"1.0.0"}"#,
            r#"{"kind":"unlist","package":"acme/http","version":"1.0.0"}"#,
            r#"{"v":1,"kind":"unlist","package":"acme/http","version":"1.0.0","why":"x"}"#,
            r#"{"v":1,"kind":"unlist","package":"acme/http"}"#,
            r#"{"v":1,"kind":"unlist","package":"","version":"1.0.0"}"#,
            r#"{"v":1,"kind":"unlist","package":"acme/http","version":"1.0"}"#,
            r#"{"v":1,"kind":"token","account":"a","token":"t","change":"lost"}"#,
            r#"{"v":1,"kind":"retract","package":"a/b","version":"1.0.0","reason":"r","fixed_in":"1.1"}"#,
            r#"{"v":1,"kind":"publish","package":"a/b","version":"1.0.0","digest":"sha256:00","repository":"r","commit":"c"}"#,
            r#"[1]"#,
        ] {
            assert!(Entry::parse(json.as_bytes()).is_err(), "{json}");
        }
    }

    #[test]
    fn an_advisory_range_is_closed_or_open_and_never_empty() {
        let advisory = |versions: &str| {
            format!(
                r#"{{"v":1,"kind":"advisory","id":"a/b-1","package":"a/b","versions":"{versions}","summary":"s"}}"#
            )
        };
        for good in ["1.2.0..1.2.4", "1.2.0..", "0.0.0..0.0.1"] {
            assert!(Entry::parse(advisory(good).as_bytes()).is_ok(), "{good}");
        }
        for bad in [
            "1.2.0",
            "..1.2.4",
            "1.2.4..1.2.0",
            "1.2.0..1.2.0",
            "1.2..1.3.0",
            "1.2.0...1.3.0",
            "1.2.0..1.3.0..",
            "1.2.0..1.3.0,2.0.0..",
            ">=1.2.0",
        ] {
            assert!(Entry::parse(advisory(bad).as_bytes()).is_err(), "{bad}");
        }
        for text in ["1.2.0..1.2.4", "1.2.0.."] {
            assert_eq!(text.parse::<Range>().unwrap().to_string(), text);
        }
    }
}
