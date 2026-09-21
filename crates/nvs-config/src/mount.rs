//! `rule:http-server/a-path-is-never-derived-from-a-url` and `rule:http-server/a-mount-table-expands-at-boot`'s mount table: `[[server.mount]]` blocks read into the literal set of entry
//! files a server may execute, with every glob already expanded against the disk.
//!
//! § 2 is the rule the rest of the ADR is built to keep — a request **selects** an entry point from
//! a set enumerated before it arrived, and never **constructs** one. That is what this module is:
//! the one place a `*` in a `scan` meets a directory listing, and it runs at boot and at
//! `rule:config/the-config-is-an-immutable-snapshot`'s reload, never on a
//! request path. The same glob evaluated per request would be `cgi.fix_pathinfo` with a different
//! spelling, which is § 2's own sentence for why the expansion is here rather than in the router.
//!
//! **Two halves, because only one of them needs a disk.** [`check`] is everything a block can be
//! wrong about on its own — naming both `entry` and `scan` or neither, matching on neither `prefix`
//! nor `host`, referring to a `{2}` its glob has no second `*` for, writing a brace that is not a
//! capture reference — and it runs inside
//! [`crate::server::validate`], so `nvs config check` refuses a malformed mount on a machine that
//! holds none of the files. [`expand`] is the other half: it walks the tree under `[server] root`,
//! and it is the server's own boot step rather than the resolver's, because a `nvs run` of a CLI
//! program has no business failing over a `/www` that is not mounted on this host.
//!
//! **A capture that reaches a candidate is refused rather than skipped.** § 3 bounds a capture to
//! `[A-Za-z0-9._-]+`, forbids a leading dot and refuses a reserved Windows device name
//! (`rule:errors/path-component-refusals` owns that
//! list, and § 5's own reasoning is why it fires on every platform). Only a directory that actually
//! holds the scanned entry ever reaches the test, so `.git` beside a module costs nothing — and a
//! module directory named `CON` that *does* hold one is a boot refusal rather than a mount silently
//! missing from the table, which is `rule:errors/ambiguous-input-refused`'s headline applied to the thing § 2 enumerates.
//!
//! **Where the origin check is.** `rule:routing/an-origin-is-per-mount-and-checked-at-boot` makes a
//! mount whose unit contains a literal `Core\Router::urlAbsolute` call and resolves no `origin` a boot
//! error. That question needs the compiled unit, so it is asked beside whatever compiles a mount's
//! entry — `nvs serve`'s boot loop, over the rows this module resolved — and not here. What is here is
//! the half it reads: [`Mounted::origin`], already substituted.
//!
//! Cost: one directory listing per `*` per scanned segment, at boot and at reload, and one
//! [`Mounted`] per resolved mount held per configuration generation. Nothing here runs per request.
//!

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Files, Origin, origin_note};
use crate::tree::{Config, Mount};

/// One resolved mount — a literal entry file and the key a request selects it by.
///
/// Every field is already substituted: `{1}` is gone by the time one of these exists, because § 2's
/// enumeration is only worth anything if what it enumerates is what a request matches against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mounted {
    /// The path prefix a request's path must begin with, always beginning with `/` and never
    /// ending with one unless it *is* `/`. § 4 step 2 strips exactly this.
    pub prefix: String,
    /// The host this mount answers on, in ASCII lower case, or `None` for a host-less mount. § 4
    /// step 1 tries the host mounts before the host-less ones.
    pub host: Option<String>,
    /// The entry file, canonical and inside `[server] root`.
    pub entry: PathBuf,
    /// The directory the entry sits in — § 4's "the mount root", which steps 3 and 4 resolve a
    /// remainder against. It is the entry's own directory rather than the module's, because a PHP
    /// deployment's document root is the `public/` the entry lives in and § 4's static policy is
    /// "an existing file under the mount root": pointing it a level higher would serve the module's
    /// source tree.
    pub root: PathBuf,
    /// What `Core\Router::urlAbsolute` prepends for a request arriving here (§ 3).
    pub origin: Option<String>,
    /// § 3's glob captures, in order — `{1}` is `captures[0]`. `Core\Request::mount()` returns
    /// these `tainted`, which is how a multi-tenant application learns which tenant it is serving
    /// without the route table naming a host.
    pub captures: Vec<String>,
}

/// § 3's implicit mount, for a tree that writes no `[[server.mount]]` at all.
const IMPLICIT_ENTRY: &str = "public/index.nvs";

/// Why a brace that is not a capture reference is refused rather than kept as text. § 4 step 1
/// matches a prefix against the path as it was sent, so a brace that survived expansion would be a
/// mount no request reaches.
const UNREACHABLE: &str = "`rule:http-server/a-mount-table-expands-at-boot` reads a brace as `{n}` \
     or `{n:lower}` and as nothing else: a host name and a URL path never carry one unescaped, so \
     a mount that kept it would be one no request reaches";

/// Everything a `[[server.mount]]` block can be wrong about without asking the disk.
///
/// Called from [`crate::server::validate`], so a tree is refused by `nvs config check` and by every
/// boot alike. [`expand`] runs the same rules before it lists anything, so a caller that only has
/// the disk half still gets all of them.
///
/// # Errors
///
/// `E0621` for a block naming both `entry` and `scan` or neither, for one matching on neither
/// `prefix` nor `host`, for a prefix that does not begin with `/`, for a `{n}` naming a capture the
/// block's glob cannot produce, and for a brace that is not a capture reference at all.
pub fn check(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(server) = config.server.as_ref() else {
        return Ok(());
    };
    for (index, block) in server.mount.iter().enumerate() {
        shape(index, block, origins)?;
    }
    Ok(())
}

/// One block's shape, and the arithmetic between its `{n}` references and its `*`s.
fn shape(
    index: usize,
    block: &Mount,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let key = |field: &str| origins.get(&format!("server.mount.{index}.{field}"));
    match (block.scan.as_deref(), block.entry.as_deref()) {
        (Some(_), Some(_)) => {
            return Err(refuse(
                format!("{} names both `scan` and `entry`", ordinal(index)),
                "`rule:http-server/a-mount-table-expands-at-boot` gives a mount one source: `scan` is a glob expanded against the disk \
                 at boot, `entry` is the one literal file that overrides it",
                "keep `scan` to cover a fleet of modules, or drop it and keep `entry` for the one \
                 irregular module this block is for",
                key("scan"),
            ));
        }
        (None, None) => {
            return Err(refuse(
                format!("{} names neither `scan` nor `entry`", ordinal(index)),
                "`rule:http-server/a-path-is-never-derived-from-a-url` enumerates every path the server may execute before it accepts \
                 anything, and a block naming no file contributes none",
                "add `entry` with the one file this mount serves, or `scan` with the glob that \
                 finds it under `[server] root`",
                key("prefix"),
            ));
        }
        _ => {}
    }
    if block.prefix.is_none() && block.host.is_none() {
        return Err(refuse(
            format!("{} matches on neither `prefix` nor `host`", ordinal(index)),
            "`rule:http-server/a-request-resolves-in-five-steps` step 1 selects a mount by host and by prefix, so a block naming neither \
             is one no request can ever reach",
            "add `prefix = \"/\"` to serve this mount at the root, or `host` for the name it \
             answers on",
            key("prefix"),
        ));
    }
    if let Some(prefix) = block.prefix.as_deref()
        && !prefix.starts_with('/')
    {
        return Err(refuse(
            format!(
                "{}'s prefix `{prefix}` does not begin with `/`",
                ordinal(index)
            ),
            "§ 4 step 2 strips the prefix off a request path, which always begins with `/`, so a \
             prefix that does not is one nothing matches",
            "write the prefix as it appears in a URL, as `/admin`",
            key("prefix"),
        ));
    }
    let captures = block.scan.as_deref().map_or(0, |glob| {
        glob.split(is_separator).filter(|s| *s == "*").count()
    });
    for (field, template) in [
        ("prefix", block.prefix.as_deref()),
        ("host", block.host.as_deref()),
        ("origin", block.origin.as_deref()),
    ] {
        let Some(template) = template else { continue };
        let (what, why, help) = match pieces(template, captures) {
            Ok(_) => continue,
            Err(Malformed::OutOfRange(reference)) => (
                format!(
                    "{}'s `{field}` refers to `{{{reference}}}`, and its glob has {captures} \
                     capture(s)",
                    ordinal(index)
                ),
                "§ 3 numbers the captures by the `*`s in `scan`, from 1, so a reference past the \
                 last one names a segment no expansion can produce",
                "renumber the reference, or add the `*` that captures the segment it means",
            ),
            Err(Malformed::Unknown(written)) => (
                format!(
                    "{}'s `{field}` writes `{{{written}}}`, which is not a capture reference",
                    ordinal(index)
                ),
                UNREACHABLE,
                "write `{1}` for the first captured segment as the disk spells it, or \
                 `{1:lower}` for the same segment in lower case",
            ),
            Err(Malformed::Unpaired(brace)) => (
                format!(
                    "{}'s `{field}` has a `{brace}` that pairs with nothing",
                    ordinal(index)
                ),
                UNREACHABLE,
                "close the reference, as `{1}`, or take the brace out",
            ),
        };
        return Err(refuse(what, why, help, key(field)));
    }
    Ok(())
}

/// One piece of a `prefix`, a `host` or an `origin`, as it is written.
#[derive(Debug, PartialEq, Eq)]
enum Piece<'a> {
    /// Text that means itself.
    Text(&'a str),
    /// `{n}` or `{n:lower}`: the nth capture counting from 1, as the disk spells it or ASCII
    /// lower-cased.
    Capture { nth: usize, lower: bool },
}

/// What a template can be wrong about.
#[derive(Debug, PartialEq, Eq)]
enum Malformed<'a> {
    /// A `{n}` the glob has no `*` for. `{0}` is one: § 3 numbers from 1, and a `{0}` is an operator
    /// who counted from zero rather than one who meant something this could resolve.
    OutOfRange(usize),
    /// Braces around something that is neither `n` nor `n:lower`.
    Unknown(&'a str),
    /// A `{` nothing closes, or a `}` nothing opened.
    Unpaired(char),
}

/// `template` read into its pieces, against a glob with `captures` `*`s.
///
/// The transform set is closed at `lower`. A module directory is named for the namespace it holds,
/// whose case `autoload` compares exactly, while a URL is conventionally lower case, and `lower` is
/// what lets one glob serve both. Nothing is refused for being mixed case without it: `{n}` stays
/// the capture verbatim.
fn pieces(template: &str, captures: usize) -> Result<Vec<Piece<'_>>, Malformed<'_>> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find(['{', '}']) {
        if rest[open..].starts_with('}') {
            return Err(Malformed::Unpaired('}'));
        }
        if open > 0 {
            out.push(Piece::Text(&rest[..open]));
        }
        rest = &rest[open + 1..];
        let close = rest
            .find(['{', '}'])
            .filter(|at| rest[*at..].starts_with('}'))
            .ok_or(Malformed::Unpaired('{'))?;
        let written = &rest[..close];
        let (number, lower) = match written.split_once(':') {
            Some((number, "lower")) => (number, true),
            Some(_) => return Err(Malformed::Unknown(written)),
            None => (written, false),
        };
        if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(Malformed::Unknown(written));
        }
        // A run of digits too long for a `usize` is past every glob's last capture as well.
        let nth = number.parse::<usize>().unwrap_or(usize::MAX);
        if nth == 0 || nth > captures {
            return Err(Malformed::OutOfRange(nth));
        }
        out.push(Piece::Capture { nth, lower });
        rest = &rest[close + 1..];
    }
    if !rest.is_empty() {
        out.push(Piece::Text(rest));
    }
    Ok(out)
}

/// § 3's globs expanded against the disk: the literal table this configuration lets a server run.
///
/// The order is the order § 4 step 1 wants nothing in particular from — it sorts by host and then
/// by prefix so that two boots of the same tree over the same disk produce the same table, which is
/// what makes `nvs info --config`'s printing of it worth reading.
///
/// # Errors
///
/// `E0621` for everything [`check`] refuses, plus what only a disk can answer: a `[server] root`
/// that is not there, an entry that resolves outside it, a capture § 3's charset forbids, an
/// explicit `entry` naming a file that does not exist, and two mounts answering at one key.
pub fn expand(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<Vec<Mounted>, Diagnostic> {
    let root = root_of(config, origins, files)?;
    let blocks = config
        .server
        .as_ref()
        .map_or(&[][..], |server| server.mount.as_slice());
    // § 3: with nothing written there is exactly one mount, and it is the shape every single-module
    // deployment has anyway. Spelled as a block rather than as a special case so that it is refused
    // by the same sentences a written one would be.
    let implicit = Mount {
        prefix: Some("/".to_string()),
        entry: Some(IMPLICIT_ENTRY.to_string()),
        ..Mount::default()
    };
    let written: Vec<(usize, &Mount)> = if blocks.is_empty() {
        vec![(0, &implicit)]
    } else {
        blocks.iter().enumerate().collect()
    };

    // Each holder carries whether it was written literally, because that — and not the order the
    // blocks were read in — is what § 3's override is decided by.
    let mut table: Vec<(bool, Mounted)> = Vec::new();
    for (index, block) in written {
        shape(index, block, origins)?;
        let literal = block.entry.is_some();
        let resolved = if let Some(glob) = block.scan.as_deref() {
            scanned(index, block, glob, &root, origins, files)?
        } else {
            let entry = block.entry.as_deref().unwrap_or_default();
            vec![mounted(
                index,
                block,
                &root,
                &crate::resolve::absolute(&root, Path::new(entry)),
                Vec::new(),
                origins,
                files,
            )?]
        };
        for one in resolved {
            let at = table
                .iter()
                .position(|(_, held)| held.prefix == one.prefix && held.host == one.host);
            match at {
                None => table.push((literal, one)),
                // § 3: an explicit mount overrides a scanned one at the same key, so one irregular
                // module costs one block and not a second glob. Both orders are the same override,
                // because a scan reaching a key an `entry` already claimed is what § 3 describes
                // read from the other end.
                Some(at) if literal && !table[at].0 => table[at] = (true, one),
                Some(at) if !literal && table[at].0 => {}
                Some(_) => return Err(duplicate(index, &one.prefix, one.host.as_deref(), origins)),
            }
        }
    }
    let mut table: Vec<Mounted> = table.into_iter().map(|(_, one)| one).collect();
    table.sort_by(|left, right| (&left.host, &left.prefix).cmp(&(&right.host, &right.prefix)));
    Ok(table)
}

/// `[server] root`, canonical — the one directory every mount path must resolve inside.
///
/// An unwritten `root` is the directory the configuration was written in, which is
/// `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`'s rule for every
/// relative path in the tree rather than a default chosen here.
fn root_of(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<PathBuf, Diagnostic> {
    let beside = origins
        .get("server.root")
        .and_then(|origin| origin.path.parent())
        .unwrap_or(Path::new("."));
    let written = config
        .server
        .as_ref()
        .and_then(|server| server.root.as_deref())
        .unwrap_or(".");
    let root = crate::resolve::absolute(beside, Path::new(written));
    files.canonical(&root).map_err(|why| {
        refuse(
            format!(
                "`server.root` names `{}`, which is not there",
                root.display()
            ),
            format!(
                "`rule:http-server/a-path-is-never-derived-from-a-url` enumerates every executable path before the server accepts anything, \
                 and the enumeration starts here: {why}"
            ),
            "point `server.root` at the directory the mounted modules live under",
            origins.get("server.root"),
        )
    })
}

/// One `scan` glob, walked segment by segment from `root`.
///
/// A `*` lists a directory and a literal segment joins; a candidate that does not exist drops out
/// as the walk passes through it, so nothing is ever asked about a path the disk does not hold. The
/// capture rules are applied to the survivors and not to the listings, because a name only has to
/// spell what it resolves to once it is a path the server could execute.
fn scanned(
    index: usize,
    block: &Mount,
    glob: &str,
    root: &Path,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<Vec<Mounted>, Diagnostic> {
    let mut frontier = vec![(root.to_path_buf(), Vec::new())];
    for segment in glob
        .split(is_separator)
        .filter(|segment| !segment.is_empty())
    {
        let mut next = Vec::new();
        for (dir, captures) in &frontier {
            if segment == "*" {
                let mut names: Vec<String> = files
                    .list(dir)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .collect();
                names.sort();
                for name in names {
                    let mut captured = captures.clone();
                    captured.push(name.clone());
                    next.push((dir.join(&name), captured));
                }
            } else {
                let path = dir.join(segment);
                if files.exists(&path) {
                    next.push((path, captures.clone()));
                }
            }
        }
        frontier = next;
    }
    let mut resolved = Vec::new();
    for (path, captures) in frontier {
        if !files.exists(&path) {
            continue;
        }
        for capture in &captures {
            if !spells_itself(capture) {
                return Err(refuse(
                    format!("`{capture}` is not a name a mount capture may take"),
                    format!(
                        "`rule:http-server/a-mount-table-expands-at-boot` bounds a capture to `[A-Za-z0-9._-]`, forbids a leading dot \
                         and refuses a reserved device name, and `{}` matched it under \
                         `server.root`",
                        path.display()
                    ),
                    "rename the directory, or narrow the glob so it does not reach this one",
                    origins.get(&format!("server.mount.{index}.scan")),
                ));
            }
        }
        resolved.push(mounted(
            index, block, root, &path, captures, origins, files,
        )?);
    }
    Ok(resolved)
}

/// One candidate path, checked against `root` and read into a [`Mounted`].
fn mounted(
    index: usize,
    block: &Mount,
    root: &Path,
    path: &Path,
    captures: Vec<String>,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<Mounted, Diagnostic> {
    let entry = files.canonical(path).map_err(|why| {
        refuse(
            format!(
                "{}'s entry `{}` is not there",
                ordinal(index),
                path.display()
            ),
            format!(
                "`rule:http-server/a-path-is-never-derived-from-a-url` knows every path this server can execute before it binds a socket, \
                 so a mount naming a file that is not on disk is a server that would answer a \
                 request with nothing: {why}"
            ),
            "correct the path, or drop the block until the module it names is deployed",
            origins.get(&format!("server.mount.{index}.entry")),
        )
    })?;
    // § 3: every resolved path is checked to resolve inside `[server] root` — once, here, and never
    // per request. Both sides are canonical, so a `..` and a symlink out of the tree are the same
    // answer as a path written outside it.
    if !entry.starts_with(root) {
        return Err(refuse(
            format!(
                "{}'s entry resolves to `{}`, outside `server.root`",
                ordinal(index),
                entry.display()
            ),
            format!(
                "`rule:http-server/a-mount-table-expands-at-boot` bounds every mount to `{}`, which is what keeps the executable set \
                 something an operator can read off the tree",
                root.display()
            ),
            "move the module under `server.root`, or widen `server.root` to the directory that \
             holds them all",
            origins.get(&format!("server.mount.{index}.entry")),
        ));
    }
    let root_of_mount = entry.parent().unwrap_or(root).to_path_buf();
    Ok(Mounted {
        prefix: prefix_of(block.prefix.as_deref().unwrap_or("/"), &captures),
        // `rule:http-server/host-matching-is-on-the-host-part-only` folds ASCII case when a request
        // is matched, so the table holds the folded spelling: two blocks whose hosts differ only
        // in case are then one key to the duplicate check and to § 3's override, as they are to
        // a request.
        host: block
            .host
            .as_deref()
            .map(|host| substitute(host, &captures).to_ascii_lowercase()),
        entry,
        root: root_of_mount,
        origin: block
            .origin
            .as_deref()
            .map(|origin| substitute(origin, &captures)),
        captures,
    })
}

/// A written prefix with its captures in it, normalized to the one spelling § 4 step 2 strips.
///
/// A trailing separator is dropped so that `/admin/` and `/admin` are one mount rather than two
/// that never both match; `/` itself keeps its slash, being the whole of what it names.
fn prefix_of(written: &str, captures: &[String]) -> String {
    let substituted = substitute(written, captures);
    let trimmed = substituted.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

/// `{n}` replaced by the nth capture, counting from 1, and `{n:lower}` by the same capture ASCII
/// lower-cased.
///
/// [`shape`] has already refused every template [`pieces`] refuses, and [`expand`] runs it before
/// anything reaches here, so the unread template is returned only to keep this total.
fn substitute(template: &str, captures: &[String]) -> String {
    let Ok(pieces) = pieces(template, captures.len()) else {
        return template.to_string();
    };
    let mut out = String::with_capacity(template.len());
    for piece in pieces {
        match piece {
            Piece::Text(text) => out.push_str(text),
            Piece::Capture { nth, lower } => {
                let capture = captures.get(nth - 1).map_or("", String::as_str);
                if lower {
                    out.push_str(&capture.to_ascii_lowercase());
                } else {
                    out.push_str(capture);
                }
            }
        }
    }
    out
}

/// § 3's rule for a captured segment, which is `rule:errors/path-component-refusals`'s on every platform.
fn spells_itself(capture: &str) -> bool {
    !capture.is_empty()
        && !capture.starts_with('.')
        && capture
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
        && !is_reserved_device(capture)
}

/// `rule:errors/path-component-refusals`'s list, with or without an extension and on every platform.
fn is_reserved_device(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    let bytes = stem.as_bytes();
    bytes.len() == 4
        && matches!(&stem[..3], "COM" | "LPT")
        && bytes[3].is_ascii_digit()
        && bytes[3] != b'0'
}

/// Two mounts answering the same request, which § 3 makes a boot error rather than a race.
fn duplicate(
    index: usize,
    prefix: &str,
    host: Option<&str>,
    origins: &BTreeMap<String, Origin>,
) -> Diagnostic {
    let key = host.map_or_else(
        || format!("`{prefix}`"),
        |host| format!("`{prefix}` on `{host}`"),
    );
    refuse(
        format!(
            "{} answers at {key}, and so does an earlier one",
            ordinal(index)
        ),
        "`rule:http-server/a-mount-table-expands-at-boot` lets one explicit `entry` override one scanned mount at a key and stops \
         there: two blocks claiming a key is a request whose answer depends on which was read first",
        "give one of them its own `prefix` or `host`, or drop it if the other already covers the \
         module",
        origins.get(&format!("server.mount.{index}.prefix")),
    )
}

/// One `E0621`, in the shape every refusal in this crate takes.
fn refuse(
    message: impl Into<String>,
    why: impl Into<String>,
    help: impl Into<String>,
    written_in: Option<&Origin>,
) -> Diagnostic {
    Diagnostic::error(code::E_BAD_MOUNT, message.into())
        .with_note(format!("{}{}", why.into(), origin_note(written_in)))
        .with_help(help.into())
}

/// `the first [[server.mount]]`, and so on — a block has no name to be refused by.
fn ordinal(index: usize) -> String {
    let word = match index {
        0 => "first",
        1 => "second",
        2 => "third",
        3 => "fourth",
        4 => "fifth",
        _ => return format!("`[[server.mount]]` number {}", index + 1),
    };
    format!("the {word} `[[server.mount]]`")
}

/// A glob's separator, on either host: § 3 writes one with `/` and Windows spells the same tree
/// with `\`, and a mount table that read only one of them would be two configurations.
fn is_separator(ch: char) -> bool {
    ch == '/' || ch == '\\'
}
