//! `nvs pkg retract` — a known-bad version of the package marked retracted with a reason and,
//! when there is one, the version that fixes it
//! (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`). Nothing is deleted: the
//! registry keeps the version fetchable, and resolution stops selecting it.
//!
//! **The package is the nearest `package.toml`'s** ([`package`]), the file `nvs pkg publish`
//! reads, and its name is sent as the slug, `Acme\Http` as `acme/http`. Both versions are parsed
//! by clap as [`Version`]s, so a malformed one stops the command before anything connects, and an
//! empty reason does too.
//!
//! **The request is `POST /api/packages/<vendor>/<name>/retract`** with the token `nvs pkg login`
//! stored ([`login::stored`]) and the JSON body `version`, `reason` and `fixedIn`, empty for none.
//! The registry decides when it takes effect: with a token alone it shows seven days later, and
//! the reply's `message` says so. That message is printed as the registry wrote it. A refusal is
//! the reply's `error`, printed as a fault with a non-zero exit, as [`super::publish`] prints one.
//!
//! [`post`] and [`package`] are shared with [`super::unlist`], which sends to the same routes.
//!
//! Cost: one `package.toml` read and one request of at most a reply's limit
//! ([`login::send`]). Nothing here runs on the request path.

use std::io::{self, Write as _};
use std::process::ExitCode;

use nvs_diagnostics::{Renderer, SourceMap};
use serde_json::{Map, Value};

use super::fetch::{self, Failed};
use super::index::{Fault, fault};
use super::login;
use super::manifest::{self, Version};

/// `nvs pkg retract`: `version` of the nearest package retracted with `reason`, naming
/// `fixed_in` when given.
pub(crate) fn run(
    registry: Option<&str>,
    version: Version,
    reason: &str,
    fixed_in: Option<Version>,
) -> ExitCode {
    fetch::exit(retracting(registry, version, reason, fixed_in))
}

/// [`run`], returning what stopped it.
fn retracting(
    registry: Option<&str>,
    version: Version,
    reason: &str,
    fixed_in: Option<Version>,
) -> Result<(), Failed> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(fault("The reason is empty. Say why the version is retracted.").into());
    }
    let origin = login::origin(registry)?;
    let slug = package()?;
    let token = login::stored(&origin)?;
    let mut body = Map::new();
    body.insert("version".into(), Value::from(version.to_string()));
    body.insert("reason".into(), Value::from(reason));
    body.insert(
        "fixedIn".into(),
        Value::from(fixed_in.map(|v| v.to_string()).unwrap_or_default()),
    );
    let (status, reply) = post(&origin, &token, &slug, "retract", Value::Object(body))?;
    if !(200..=299).contains(&status) {
        return Err(refused(
            &origin,
            status,
            &reply,
            &format!("did not retract {slug} {version}"),
        )
        .into());
    }
    let message = text(&reply, "message").ok_or_else(|| unexpected(&origin, &slug, "retract"))?;
    let mut out = io::stdout().lock();
    // Standard output that cannot be written to has nobody to tell.
    let _ = writeln!(out, "{message}");
    if reply.get("sent").and_then(Value::as_bool) == Some(false) {
        let _ = writeln!(out, "{NOT_SENT}");
    }
    Ok(())
}

/// What a command prints when the registry saved a change but could not yet send the index files
/// it changes.
pub(super) const NOT_SENT: &str =
    "The registry saved the change. It updates the package index on its next run.";

/// The slug of the package whose `package.toml` is the nearest one to the current folder.
///
/// # Errors
///
/// No `package.toml`, one that does not read, which is printed as a diagnostic, and one with no
/// `[package] name`.
pub(super) fn package() -> Result<String, Failed> {
    let cwd = std::env::current_dir()
        .map_err(|err| fault(format!("cannot read the current folder: {err}")))?;
    let Some(path) = manifest::find(&cwd) else {
        return Err(fault(format!(
            "There is no `package.toml` in `{}` or a folder above it.",
            cwd.display()
        ))
        .into());
    };
    let mut sources = SourceMap::new();
    let read = match manifest::read(&path, &mut sources) {
        Ok(read) => read,
        Err(diagnostic) => {
            // Standard error that cannot be written to has nobody to tell.
            let _ = Renderer::new().render(&diagnostic, &sources, &mut io::stderr().lock());
            return Err(Failed::Reported(ExitCode::FAILURE));
        }
    };
    let Some(name) = read.name else {
        return Err(fault(format!(
            "`{}` has no `[package] name`, so its folder is an application and not a package.",
            path.display()
        ))
        .into());
    };
    Ok(name.slug())
}

/// `body` sent as JSON to `<origin>/api/packages/<slug>/<action>` with `token`: the status and
/// the reply's JSON, `Null` for a reply that is not JSON.
///
/// # Errors
///
/// A registry that cannot be reached, and a reply that cannot be read.
pub(super) fn post(
    origin: &str,
    token: &str,
    slug: &str,
    action: &str,
    body: Value,
) -> Result<(i64, Value), Fault> {
    let body = serde_json::to_vec(&body).expect("a tree of strings serialises");
    let path = format!("/api/packages/{slug}/{action}");
    let (status, reply) = login::send(origin, token, &path, Some(body), login::WITHIN)?;
    Ok((
        status,
        serde_json::from_slice(&reply).unwrap_or(Value::Null),
    ))
}

/// The string at `key` in `reply`.
pub(super) fn text(reply: &Value, key: &str) -> Option<String> {
    reply.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// The fault for a non-2xx `status`: `The registry <origin> <what>:` and the reply's `error`, or
/// the status when it has none.
pub(super) fn refused(origin: &str, status: i64, reply: &Value, what: &str) -> Fault {
    let why = text(reply, "error").unwrap_or_else(|| format!("it returned status {status}"));
    let hint = if status == 401 {
        " Run `nvs pkg login` with a token for this package."
    } else {
        ""
    };
    fault(format!("The registry `{origin}` {what}: {why}{hint}"))
}

/// The fault for a 2xx reply that is not what a registry returns.
pub(super) fn unexpected(origin: &str, slug: &str, action: &str) -> Fault {
    fault(format!(
        "The reply of `{origin}/api/packages/{slug}/{action}` is not what a registry returns."
    ))
}
