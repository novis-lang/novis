//! `nvs pkg add`, `nvs pkg update` and `nvs pkg outdated`: the commands that choose a minimum
//! (`rule:packaging/resolution-takes-the-highest-minimum`).
//!
//! **A minimum a command chooses is the newest released version** ([`newest`]): one whose state
//! is released, or pending past the time it becomes selectable. A pending version is chosen only
//! when it is named exactly, as in `nvs pkg add acme/http@1.4.0`
//! (`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`). An unlisted version is
//! never chosen, and a minimum that names it still resolves (`rule:packaging/an-owner-may-unlist`).
//! A retracted or withheld version is never chosen, and naming one gives resolution's fault, which
//! names the fix or the advisory (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`,
//! `rule:packaging/the-operator-only-withholds`). The lock's allowance stays: a retracted version
//! the lock already has may stay selected, so an update of one package is not blocked by another.
//!
//! **`add` and `update` write the minimum into `package.toml`'s text** ([`with_minimum`]), so
//! every comment and every other line stays as written. Then they resolve, print what the change
//! brings ([`report::report`]), and write the file, the tree and the lock as `nvs pkg fetch` does
//! ([`fetch::land`]). A graph that does not resolve writes nothing. `add` without a version never
//! lowers a minimum, `update` only raises one and names `nvs pkg add` for a package not in
//! `[dependencies]`, and `outdated` resolves, prints each package with a
//! newer released version, and writes nothing.
//!
//! Cost: what `nvs pkg fetch` holds, and the file's text. Nothing here runs on the request path.

use std::fs;
use std::process::ExitCode;

use super::fetch::{self, App, Failed, Mode};
use super::index::{Fault, Index, Release, State, fault};
use super::manifest::{Manifest, Name, Source, Version};
use super::report;
use super::resolve::{self, line};

/// The newest version of `index` a command may choose on its own at the Unix time `now`.
pub(crate) fn newest(index: &Index, now: u64) -> Option<&Release> {
    index
        .releases
        .iter()
        .rev()
        .find(|release| match release.state {
            State::Released => true,
            State::Pending { selectable_at } => selectable_at <= now,
            State::Unlisted | State::Retracted { .. } | State::Withheld { .. } => false,
        })
}

/// The current Unix time, in seconds.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// `nvs pkg add <package>[@<version>]`, in the working directory.
pub(crate) fn add(package: &str) -> ExitCode {
    fetch::exit(adding(package))
}

/// [`add`], returning what stopped it.
fn adding(package: &str) -> Result<(), Failed> {
    let (written, exact) = match package.split_once('@') {
        Some((name, version)) => (name, Some(version.parse::<Version>().map_err(fault)?)),
        None => (package, None),
    };
    let name = Name::checked(&written.replace('/', "\\"))
        .map_err(|(message, help)| fault(format!("{message}: {help}")))?;
    let app = App::open()?;
    let existing = app
        .manifest
        .dependencies
        .iter()
        .find(|dependency| dependency.name.slug() == name.slug());
    let minimum = match existing.map(|dependency| &dependency.source) {
        Some(Source::Git { .. }) => return Err(git_dependency(&name).into()),
        Some(Source::Registry { minimum }) => Some(*minimum),
        None => None,
    };
    let mut registries = app.registries(&app.manifest);
    let index = listed(&mut registries, &name)?;
    let name = index.name.clone();
    let version = match exact {
        Some(version) => {
            if index.release(version).is_none() {
                return Err(fault(format!("`{name}` has no version {version}")).into());
            }
            version
        }
        None => {
            let Some(release) = newest(index, now()) else {
                return Err(fault(format!(
                    "`{name}` has no released version yet. A new version is released seven \
                     days after it is published. Add one before that with its exact version, \
                     such as `{}@<version>`.",
                    name.slug()
                ))
                .into());
            };
            minimum.map_or(release.version, |minimum| minimum.max(release.version))
        }
    };
    let text = read(&app)?;
    let text = with_minimum(&text, &name, version, existing.is_some())?;
    let manifest = app.reread(&text)?;
    change(&app, &manifest, &text, &[name])
}

/// `nvs pkg update [<package>...]`, in the working directory.
pub(crate) fn update(packages: &[String]) -> ExitCode {
    fetch::exit(updating(packages))
}

/// [`update`], returning what stopped it.
fn updating(packages: &[String]) -> Result<(), Failed> {
    let app = App::open()?;
    let mut targets = Vec::new();
    if packages.is_empty() {
        targets.extend(app.manifest.dependencies.iter().filter_map(|dependency| {
            match dependency.source {
                Source::Registry { minimum } => Some((dependency.name.clone(), minimum)),
                Source::Git { .. } => None,
            }
        }));
    }
    for package in packages {
        let slug = package.replace('\\', "/").to_lowercase();
        let Some(dependency) = app
            .manifest
            .dependencies
            .iter()
            .find(|dependency| dependency.name.slug() == slug)
        else {
            return Err(fault(format!(
                "`{package}` is not in [dependencies]. Add it with `nvs pkg add {slug}`."
            ))
            .into());
        };
        match dependency.source {
            Source::Registry { minimum } => targets.push((dependency.name.clone(), minimum)),
            Source::Git { .. } => return Err(git_dependency(&dependency.name).into()),
        }
    }
    let mut text = read(&app)?;
    let mut raised = Vec::new();
    let mut registries = app.registries(&app.manifest);
    let now = now();
    for (name, minimum) in targets {
        let index = listed(&mut registries, &name)?;
        let Some(release) = newest(index, now).filter(|release| release.version > minimum) else {
            continue;
        };
        text = with_minimum(&text, &name, release.version, true)?;
        println!(
            "`{name}`: the minimum is raised from {minimum} to {}.",
            release.version
        );
        raised.push(name);
    }
    if raised.is_empty() {
        println!("Every minimum is already the newest released version.");
        return Ok(());
    }
    let manifest = app.reread(&text)?;
    change(&app, &manifest, &text, &raised)
}

/// `nvs pkg outdated`, in the working directory.
pub(crate) fn outdated() -> ExitCode {
    fetch::exit(listing())
}

/// [`outdated`], returning what stopped it.
fn listing() -> Result<(), Failed> {
    let app = App::open()?;
    let mut registries = app.registries(&app.manifest);
    let graph = resolve::resolve(&app.manifest, &mut registries, app.old.as_ref())?;
    let now = now();
    let mut found = false;
    for selected in &graph.packages {
        if registries.cloned(&selected.name).is_some() {
            continue;
        }
        let Some(index) = registries.index(&selected.name)? else {
            continue;
        };
        let current = selected.release.version;
        let Some(release) = newest(index, now).filter(|release| release.version > current) else {
            continue;
        };
        let major = if line(release.version) == line(current) {
            ""
        } else {
            " (a new major version)"
        };
        println!(
            "`{}` {current} -> {}{major}",
            selected.name, release.version
        );
        found = true;
    }
    if !found {
        println!("Every package is at its newest released version.");
    }
    Ok(())
}

/// The fault for `nvs pkg add` or `update` of a git dependency.
fn git_dependency(name: &Name) -> Fault {
    fault(format!(
        "`{name}` is a git dependency. Change its `rev` in {} to move it.",
        nvs_config::grants::FILE
    ))
}

/// The index of `name`, or a fault when its registry does not list it.
fn listed<'r>(
    registries: &'r mut resolve::Registries<'_>,
    name: &Name,
) -> Result<&'r Index, Fault> {
    let url = registries.registry(name)?.url.clone();
    registries
        .index(name)?
        .ok_or_else(|| fault(format!("the registry `{url}` has no package `{name}`")))
}

/// The text of the application's `package.toml`.
fn read(app: &App) -> Result<String, Fault> {
    fs::read_to_string(&app.path)
        .map_err(|err| fault(format!("cannot read `{}`: {err}", app.path.display())))
}

/// `manifest`, resolved and checked against the lock, with what it brings printed, then written
/// as `text` with the tree and the lock. `targets` are the packages the command named.
fn change(app: &App, manifest: &Manifest, text: &str, targets: &[Name]) -> Result<(), Failed> {
    let mut registries = app.registries(manifest);
    let graph = resolve::resolve(manifest, &mut registries, app.old.as_ref())?;
    if let Some(old) = &app.old {
        old.check(&graph, false)?;
    }
    let files = fetch::files_before(&graph, &mut registries, &app.dir.join(fetch::VENDOR))?;
    print!(
        "{}",
        report::report(&graph, app.old.as_ref(), targets, &files)
    );
    fetch::replace_file(&app.path, text)?;
    fetch::land(app, manifest, Mode::Fetch, false, &graph, &mut registries)
}

/// `text` with the `[dependencies]` line of `name` set to `version`. With `existing`, the line is
/// already there and its version string is replaced, so its comment stays. Otherwise the line is
/// added at the end of the table.
///
/// # Errors
///
/// `existing`, and no line of the table names `name` with a version string.
pub(crate) fn with_minimum(
    text: &str,
    name: &Name,
    version: Version,
    existing: bool,
) -> Result<String, Fault> {
    if !existing {
        let line = format!("'{name}' = \"{version}\"");
        return Ok(fetch::with_lines(text, "dependencies", &[line]));
    }
    let mut lines: Vec<&str> = text.lines().collect();
    let replaced = fetch::table_span(&lines, "dependencies").and_then(|(at, end)| {
        (at + 1..end).find_map(|i| {
            let key = key(lines[i])?;
            let key = key.strip_prefix('\\').unwrap_or(&key);
            if !key.eq_ignore_ascii_case(name.namespace()) {
                return None;
            }
            Some((i, versioned(lines[i], version)?))
        })
    });
    let Some((at, line)) = replaced else {
        return Err(fault(format!(
            "the version of `{name}` in [dependencies] is not a string on its own line. \
             Change it to \"{version}\" by hand."
        )));
    };
    lines[at] = &line;
    Ok(fetch::joined(text, &lines))
}

/// The key of the TOML line `line` when it is a quoted key followed by `=`.
fn key(line: &str) -> Option<String> {
    let line = line.trim_start();
    let quote = line.chars().next().filter(|c| *c == '\'' || *c == '"')?;
    let rest = &line[1..];
    let mut key = String::new();
    let mut chars = rest.char_indices();
    while let Some((at, c)) = chars.next() {
        if c == quote {
            return rest[at + 1..].trim_start().starts_with('=').then_some(key);
        }
        if c == '\\' && quote == '"' {
            key.push(chars.next()?.1);
        } else {
            key.push(c);
        }
    }
    None
}

/// `line`, a `key = "version"` line, with its version string replaced by `version`.
fn versioned(line: &str, version: Version) -> Option<String> {
    let key_end = line.find('=')?;
    // The key is quoted and has no `=` of its own when [`key`] read it, except one inside
    // quotes, which a namespace cannot have.
    let value = &line[key_end + 1..];
    let start = key_end + 1 + (value.len() - value.trim_start().len());
    let quote = line[start..]
        .chars()
        .next()
        .filter(|c| *c == '\'' || *c == '"')?;
    let close = start + 1 + line[start + 1..].find(quote)?;
    Some(format!("{}{version}{}", &line[..=start], &line[close..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acme() -> Name {
        Name::checked("Acme\\Http").expect("the name is valid")
    }

    #[test]
    fn a_minimum_is_replaced_in_place_and_its_comment_stays() {
        let text = "[dependencies]\n'Acme\\Http' = \"1.0.0\"  # the client\n\n[grants]\n";
        let version = "1.2.0".parse().expect("the version is valid");
        let got = with_minimum(text, &acme(), version, true).expect("the line is found");
        assert_eq!(
            got,
            "[dependencies]\n'Acme\\Http' = \"1.2.0\"  # the client\n\n[grants]\n"
        );
        let escaped = "[dependencies]\n\"acme\\\\http\" = '1.0.0'\n";
        let got = with_minimum(escaped, &acme(), version, true).expect("the line is found");
        assert_eq!(got, "[dependencies]\n\"acme\\\\http\" = '1.2.0'\n");
    }

    #[test]
    fn a_new_minimum_goes_at_the_end_of_the_dependencies_table() {
        let text = "[dependencies]\n'Acme\\Log' = \"1.0.0\"\n\n[registries]\n";
        let version = "1.0.0".parse().expect("the version is valid");
        let got = with_minimum(text, &acme(), version, false).expect("the line is added");
        assert_eq!(
            got,
            "[dependencies]\n'Acme\\Log' = \"1.0.0\"\n'Acme\\Http' = \"1.0.0\"\n\n[registries]\n"
        );
    }
}
