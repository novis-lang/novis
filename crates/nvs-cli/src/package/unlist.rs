//! `nvs pkg unlist` — the package, or one version of it, hidden from search and listings, and
//! `--undo` to show it again (`rule:packaging/an-owner-may-unlist`). An unlisted version stays
//! fetchable, and a lockfile that names it keeps building.
//!
//! **The package is the nearest `package.toml`'s**, found as [`super::retract::package`] finds
//! it. A version is parsed by clap as a [`Version`] before anything connects, and no version
//! sends an empty one, which the registry reads as every version.
//!
//! **The request is `POST /api/packages/<vendor>/<name>/unlist`, or `/relist` for `--undo`,**
//! with the token `nvs pkg login` stored and the JSON body `version`. Only an owner signed in
//! with a passkey can unlist, so a token gets `202` and a message naming the page where an owner
//! confirms it. That message is printed and the command succeeds, since nothing went wrong. A
//! `200` lists the versions that changed. A refusal is the reply's `error`, printed as a fault
//! with a non-zero exit, as [`super::publish`] prints one.
//!
//! Cost: one `package.toml` read and one request. Nothing here runs on the request path.

use std::io::{self, Write as _};
use std::process::ExitCode;

use serde_json::{Map, Value};

use super::fetch::{self, Failed};
use super::login;
use super::manifest::Version;
use super::retract::{NOT_SENT, package, post, refused, text, unexpected};

/// `nvs pkg unlist`: `version` of the nearest package, or every version for `None`, unlisted, or
/// listed again when `undo`.
pub(crate) fn run(registry: Option<&str>, version: Option<Version>, undo: bool) -> ExitCode {
    fetch::exit(listing(registry, version, undo))
}

/// [`run`], returning what stopped it.
fn listing(registry: Option<&str>, version: Option<Version>, undo: bool) -> Result<(), Failed> {
    let origin = login::origin(registry)?;
    let slug = package()?;
    let token = login::stored(&origin)?;
    let action = if undo { "relist" } else { "unlist" };
    let mut body = Map::new();
    body.insert(
        "version".into(),
        Value::from(version.map(|v| v.to_string()).unwrap_or_default()),
    );
    let (status, reply) = post(&origin, &token, &slug, action, Value::Object(body))?;
    let mut out = io::stdout().lock();
    match status {
        202 => {
            let message =
                text(&reply, "message").ok_or_else(|| unexpected(&origin, &slug, action))?;
            // Standard output that cannot be written to has nobody to tell.
            let _ = writeln!(out, "{message}");
        }
        200..=299 => {
            let versions = reply
                .get("versions")
                .and_then(Value::as_array)
                .map(|all| all.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                .ok_or_else(|| unexpected(&origin, &slug, action))?;
            let done = if undo { "Listed again" } else { "Unlisted" };
            let _ = writeln!(out, "{done}: {slug} {}", versions.join(", "));
            if reply.get("sent").and_then(Value::as_bool) == Some(false) {
                let _ = writeln!(out, "{NOT_SENT}");
            }
        }
        status => {
            let what = match (undo, version) {
                (false, Some(v)) => format!("did not unlist {slug} {v}"),
                (false, None) => format!("did not unlist {slug}"),
                (true, Some(v)) => format!("did not list {slug} {v} again"),
                (true, None) => format!("did not list {slug} again"),
            };
            return Err(refused(&origin, status, &reply, &what).into());
        }
    }
    Ok(())
}
