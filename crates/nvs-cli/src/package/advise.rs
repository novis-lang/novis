//! `nvs pkg advise` — an advisory against a range of a package's versions, with a summary
//! (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`). It changes no version's state:
//! `nvs pkg audit` reports each locked version inside the range, and resolution still selects it.
//!
//! **The package is named on the command line**, `Acme\Http` or `acme/http`, and sent as the
//! slug, because an advisory is often sent from outside the package's folder. The range is parsed
//! by clap as a [`Range`], the one parser the log and the registry's form share, so a malformed
//! one stops the command before anything connects, and an empty summary does too.
//!
//! **The request is `POST /api/packages/<vendor>/<name>/advisory`** with the token
//! `nvs pkg login` stored and the JSON body `versions` and `summary`, through
//! [`retract::post`]. The registry decides when it takes effect: with a token alone it is in the
//! feed seven days later, and the reply's `message` says so and is printed as written. A refusal
//! is the reply's `error`, printed as a fault with a non-zero exit.
//!
//! Cost: one request of at most a reply's limit. Nothing here runs on the request path.

use std::io::{self, Write};
use std::process::ExitCode;

use serde_json::{Map, Value};

use super::fetch::{self, Failed};
use super::index::{Fault, fault};
use super::login;
use super::manifest::Range;
use super::retract::{self, NOT_SENT, refused, text, unexpected};

/// `nvs pkg advise`: an advisory against `versions` of `package`, with `summary`.
pub(crate) fn run(
    registry: Option<&str>,
    package: &str,
    versions: Range,
    summary: &str,
) -> ExitCode {
    fetch::exit(advising(registry, package, versions, summary))
}

/// [`run`], returning what stopped it.
fn advising(
    registry: Option<&str>,
    package: &str,
    versions: Range,
    summary: &str,
) -> Result<(), Failed> {
    let summary = summary.trim();
    if summary.is_empty() {
        return Err(fault("The summary is empty. Say what is wrong with these versions.").into());
    }
    let slug = slug(package)?;
    let origin = login::origin(registry)?;
    let token = login::stored(&origin)?;
    let mut body = Map::new();
    body.insert("versions".into(), Value::from(versions.to_string()));
    body.insert("summary".into(), Value::from(summary));
    let (status, reply) = retract::post(&origin, &token, &slug, "advisory", Value::Object(body))?;
    if !(200..=299).contains(&status) {
        return Err(refused(
            &origin,
            status,
            &reply,
            &format!("did not take the advisory for {slug} {versions}"),
        )
        .into());
    }
    let message = text(&reply, "message").ok_or_else(|| unexpected(&origin, &slug, "advisory"))?;
    let mut out = io::stdout().lock();
    // Standard output that cannot be written to has nobody to tell.
    let _ = writeln!(out, "{message}");
    if reply.get("sent").and_then(Value::as_bool) == Some(false) {
        let _ = writeln!(out, "{NOT_SENT}");
    }
    Ok(())
}

/// The slug of `package`, given as `Acme\Http` or `acme/http`: `acme/http`.
///
/// # Errors
///
/// A name that is not a vendor and a package, each of letters, digits, `_` and `-`.
fn slug(package: &str) -> Result<String, Fault> {
    let slug = package.trim().replace('\\', "/").to_lowercase();
    let part = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    };
    match slug.split_once('/') {
        Some((vendor, name)) if part(vendor) && part(name) => Ok(slug),
        _ => Err(fault(format!(
            "`{package}` is not a package name. Write it as `Acme\\Http` or `acme/http`."
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn a_package_name_is_sent_as_its_slug() {
        assert_eq!(slug("Acme\\Http").unwrap(), "acme/http");
        assert_eq!(slug("acme/http").unwrap(), "acme/http");
        assert!(slug("acme").is_err());
        assert!(slug("acme/http/client").is_err());
        assert!(slug("acme/../x").is_err());
    }
}
