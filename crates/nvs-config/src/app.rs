//! `rule:config/an-application-is-its-entry-file-path`, `rule:config/every-matching-app-block-applies-least-specific-first` and `rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling`: an application is its entry file path.
//!
//! A `[[app]]` block is keyed on a directory (`root`) or on one file (`entry`), never both and
//! never neither, an entry file belongs to every block whose key covers it, those blocks fold into
//! one effective block least-specific first, and what any of them asks for is bounded by the global
//! `[limits.hard]`.
//!
//! **[`canonicalize`] and [`bound`] run at resolve time.** [`matching`] and [`layer`] need an
//! entry file, and `nvs run <file>`'s entry is not known when the tree is read, so they are
//! called by whatever builds the per-app snapshot rather than living in
//! [`resolve`](crate::resolve) the way `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s secrets do. What they need from that tree is the merged `toml::Table` and its origins, which is
//! why [`Resolved`] keeps both.
//!
//! **[`layer`] answers what the effective block *is*; it is not how the snapshot is built.**
//! [`Snapshot::build`](crate::snapshot::Snapshot::build) folds each matching block over the global
//! tree in [`matching`]'s order instead, by the same [`merge_table`](crate::resolve) — same values,
//! and every key still named by the file the block that wrote it lives in, which an effective block
//! folded from three files cannot do with one origin for all of its keys. This function is `rule:config/check-and-dump-audit-the-tree-offline`'s
//! per-app `nvs config dump`.
//!
//! **The comparison is canonicalize-then-prefix, and the canonicalization is
//! [`trust::canonical`](crate::trust::canonical)'s.** That is the whole security content of `rule:config/app-keys-are-canonicalized-before-matching`:
//! without it `/srv/www/shop/../other/x.nvs` matches `root = "/srv/www/shop"` and a symlink planted
//! inside an application's tree inherits that application's capabilities. Both sides are
//! canonicalized — the block's key by [`canonicalize`] once at boot, the entry file by [`matching`]
//! per run — and the prefix test is `Path::starts_with`, which compares whole components, so
//! `/srv/www/shopfront` is not under `/srv/www/shop`.
//!
//! **A block's key is canonicalized in place**, replacing what the operator wrote, exactly as `rule:config/a-secret-is-a-file-whose-content-is-the-value`
//! writes a secret file's content into `password`. Two reasons: a comparison against a path that has
//! not been through [`trust::canonical`](crate::trust::canonical) is the bug this module exists to
//! prevent, and `rule:config/check-and-dump-audit-the-tree-offline`'s `nvs config dump` then prints the path the match will actually use rather than
//! a relative fragment the reader has to resolve by hand.
//!
//! **A key that cannot be canonicalized refuses the boot** (`E0605`) rather than never matching.
//! The reason to refuse rather than never match is the priority ordering: a
//! `[[app]]` block that silently matches nothing hands every application under it the *global*
//! configuration, so a typo in a `root` that narrows limits or grants a capability is a security
//! change that reports nothing. A missing directory is a loud refusal instead.
//!
//! Cost: one canonicalization per block at boot or reload, and one per `nvs run` for the entry file
//! — a `stat` walk each, on a path already about to be opened. Nothing here runs per request.
//!

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Files, Origin, Override, Resolved, origin_note};
use crate::tree::{App, Config, LimitSet, Limits, Setting};
use crate::value::{Quantity, unit_of};

/// `rule:config/an-application-is-its-entry-file-path`'s keys, resolved: each `[[app]]` block's `root` or `entry` made absolute against the file
/// that wrote it (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`) and canonicalized, written back into the block.
///
/// Runs once over the flattened tree, beside `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s secrets and for the same reason: `[[app]]`
/// blocks accumulate across the tree (`rule:config/a-value-array-replaces-and-a-table-appends`), so the roster is a question only the merge
/// has answered.
///
/// # Errors
///
/// One [`Diagnostic`]: `E0609` for a block naming both keys, neither key, or a path another block
/// already claimed; `E0605` for a key naming something that cannot be examined.
///
pub fn canonicalize(
    config: &mut Config,
    origins: &BTreeMap<String, Origin>,
    files: &dyn Files,
) -> Result<(), Diagnostic> {
    let mut claimed: BTreeMap<PathBuf, usize> = BTreeMap::new();
    for (index, block) in config.app.iter_mut().enumerate() {
        let (field, written) = match (&block.root, &block.entry) {
            (Some(root), None) => ("root", root.clone()),
            (None, Some(entry)) => ("entry", entry.clone()),
            (Some(_), Some(_)) => return Err(both_keys(index, origins)),
            (None, None) => return Err(no_key(index)),
        };
        let written_in = origins.get(&format!("app.{index}.{field}"));
        // `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`: relative to the file it was written in, which is the same rule an `[[include]]` and
        // a `password_file` follow. A key with no origin was written nowhere this merge recorded,
        // so there is nothing but the path itself to resolve against.
        let base = written_in
            .and_then(|origin| origin.path.parent())
            .unwrap_or(Path::new("."));
        let named = crate::resolve::absolute(base, Path::new(&written));

        let canonical = files.canonical_block(&named).map_err(|err| {
            crate::resolve::unreadable(
                &named,
                &err,
                "an `[[app]]` block is keyed on it, and a key that names nothing matches nothing — \
                 every application it was meant to cover would fall back to the global \
                 configuration without saying so",
            )
        })?;
        let text = canonical
            .to_str()
            .ok_or_else(|| not_utf8(&canonical, index, field))?
            .to_string();
        if let Some(first) = claimed.insert(canonical.clone(), index) {
            return Err(duplicate(&canonical, first, index, written_in));
        }
        match field {
            "root" => block.root = Some(text),
            _ => block.entry = Some(text),
        }
    }
    Ok(())
}

/// `rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling`'s bound: an `[[app]]` block may widen `[app.limits]` and lower its own `[app.limits.hard]`,
/// and neither may pass the global `[limits.hard]`.
///
/// Runs beside [`canonicalize`] and for the same reason — the global ceiling and the blocks bounded
/// by it are both properties of the merged tree — but it needs no filesystem, so it takes no
/// [`Files`]. With no global `[limits.hard]` written there is no bound to apply and every block
/// passes: `rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling` bounds a block by the host's answer, and a host that gave none has not answered.
///
/// The refusal is at boot and the value is **not** clamped, which is `rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling` citing `rule:config/three-changeability-classes`'s
/// treatment of `Core\Config::set`: a clamped block still reads as though it got what it asked for,
/// and the operator finds out at the first request that hits the real ceiling.
///
/// A start above its own ceiling, globally or in the tree an entry's blocks fold to, is
/// [`starts`]'s refusal rather than this one's, because only the folded tree knows which start and
/// which ceiling meet.
///
/// # Errors
///
/// `E0610` for a value above the host's ceiling, naming both sides and where each was written, and
/// `E0601` for a value on either side that is not a quantity at all — [`crate::value`]'s refusal,
/// which is the same one `Core\Config::set` will hand back for the same text.
///
pub fn bound(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(host) = config
        .limits
        .as_ref()
        .and_then(|limits| limits.hard.as_ref())
    else {
        return Ok(());
    };
    let host = ceilings(host);
    for (index, block) in config.app.iter().enumerate() {
        let Some(limits) = block.limits.as_ref() else {
            continue;
        };
        let block_path = format!("app.{index}.limits");
        for ((name, ceiling), (_, asked)) in host.into_iter().zip(defaults(limits)) {
            bounded(&block_path, name, asked, ceiling, origins)?;
        }
        if let Some(hard) = limits.hard.as_ref() {
            let hard_path = format!("{block_path}.hard");
            for ((name, ceiling), (_, asked)) in host.into_iter().zip(ceilings(hard)) {
                bounded(&hard_path, name, asked, ceiling, origins)?;
            }
        }
    }
    Ok(())
}

/// `rule:config/one-parser-for-the-boot-path-and-config-set`'s boot refusal: every quantity written
/// in `[limits]`, `[limits.hard]`, an `[app.limits]`, an `[app.limits.hard]` or a `[[schedule]]`'s
/// `limits` parses in its key's unit.
///
/// Through [`Quantity::parse`], the parser every runtime reader and `Core\Config::set` use, so a
/// value this passes is one each of them reads as the number it spells. `memory_high_water` and
/// `disconnect_grace` are left to [`high_water`] and [`disconnect`], which refuse more than a
/// malformed value and word their own refusal.
///
/// # Errors
///
/// `E0601` naming the key, what was written and where.
pub fn quantities(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    /// One limits block's dotted path and its named rows.
    type Block<'a> = (String, Vec<(&'static str, Option<&'a Setting>)>);
    fn limits_of<'a>(blocks: &mut Vec<Block<'a>>, path: String, limits: &'a Limits) {
        let mut rows = defaults(limits).to_vec();
        rows.extend(system(limits));
        if let Some(hard) = limits.hard.as_ref() {
            blocks.push((format!("{path}.hard"), ceilings(hard).to_vec()));
        }
        blocks.push((path, rows));
    }
    let mut blocks = Vec::new();
    if let Some(limits) = config.limits.as_ref() {
        limits_of(&mut blocks, "limits".to_string(), limits);
    }
    for (index, block) in config.app.iter().enumerate() {
        if let Some(limits) = block.limits.as_ref() {
            limits_of(&mut blocks, format!("app.{index}.limits"), limits);
        }
    }
    for (index, schedule) in config.schedule.iter().enumerate() {
        if let Some(limits) = schedule.limits.as_ref() {
            blocks.push((
                format!("schedule.{index}.limits"),
                ceilings(limits).to_vec(),
            ));
        }
    }
    for (block, rows) in blocks {
        for (name, written) in rows {
            let Some(written) = written else {
                continue;
            };
            let key = format!("{block}.{name}");
            let Some(unit) = unit_of(&key) else {
                continue;
            };
            Quantity::parse(&key, unit, written)
                .map_err(|invalid| invalid.diagnostic(origins.get(&key)))?;
        }
    }
    Ok(())
}

/// `rule:config/ceilings-are-their-own-directives`'s cap: the value a request starts with is not
/// above the `[limits.hard]` ceiling written beside it.
///
/// The ceiling is the enforceable cap, and the runtime arms the start, so a start above a written
/// ceiling runs with more than the operator allowed. It is refused, not clamped, for [`bound`]'s
/// reason. The resolve calls this over the global tree, and
/// [`Snapshot::build`](crate::snapshot::Snapshot::build) over the tree an entry's `[[app]]` blocks
/// fold to, where a start from one block meets a ceiling from another. An unwritten ceiling is the
/// start itself, so there is nothing to compare, and the default ceilings of the two body caps and
/// the part count are [`body_caps`]'s.
///
/// # Errors
///
/// `E0601` for a start above its ceiling, naming both values and where the start was written.
pub fn starts(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let Some(limits) = config.limits.as_ref() else {
        return Ok(());
    };
    let Some(hard) = limits.hard.as_ref() else {
        return Ok(());
    };
    for ((name, asked), (_, ceiling)) in defaults(limits).into_iter().zip(ceilings(hard)) {
        let (Some(asked), Some(ceiling)) = (asked, ceiling) else {
            continue;
        };
        let key = format!("limits.{name}");
        if crate::value::within_ceiling(&key, asked, ceiling)
            .map_err(|invalid| invalid.diagnostic(origins.get(&key)))?
            == Some(false)
        {
            return Err(start_above_ceiling(&key, asked, ceiling, origins.get(&key)));
        }
    }
    Ok(())
}

/// `rule:observability/memory-high-water-writes-a-warn`'s boot refusal: a `memory_high_water`
/// written in `[limits]` or any `[app.limits]` must be a fraction between `0` and `1`.
///
/// Through [`Quantity::parse`], the typed-value path that refuses a malformed size, so the refusal
/// is `E0601` in that parser's own words. `false` is refused here as well. It parses as no ceiling
/// for every other limit, but this key is not a ceiling, and unwritten is already how it is off.
///
/// # Errors
///
/// `E0601` naming the key, what was written and where.
pub fn high_water(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let global = config
        .limits
        .as_ref()
        .map(|limits| ("limits".to_owned(), limits));
    let apps = config.app.iter().enumerate().filter_map(|(index, block)| {
        block
            .limits
            .as_ref()
            .map(|limits| (format!("app.{index}.limits"), limits))
    });
    for (block_path, limits) in global.into_iter().chain(apps) {
        let Some(written) = limits.memory_high_water.as_ref() else {
            continue;
        };
        let key = format!("{block_path}.memory_high_water");
        let invalid = match Quantity::parse(&key, crate::value::Unit::Ratio, written) {
            Ok(Quantity::Ratio(_)) => continue,
            Ok(_) => crate::value::Invalid {
                key: key.clone(),
                written: crate::value::as_written(written),
                unit: crate::value::Unit::Ratio,
                reason: "this key is a share of the ceiling, and leaving it out is how it is off",
            },
            Err(invalid) => invalid,
        };
        return Err(invalid.diagnostic(origins.get(&key)));
    }
    Ok(())
}

/// `disconnect_grace` with nothing written: finite, as
/// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` requires of every wait.
pub const DISCONNECT_GRACE: std::time::Duration = crate::defaults::LIMITS_DISCONNECT_GRACE;

/// What `rule:http-server/a-request-outlives-a-client-that-goes-away` reads from one block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disconnect {
    /// The methods cancelled at the drop, as written: exact, case-sensitive, no repeats.
    pub cancel: Vec<String>,
    /// How long a request with no `wall_time` runs on after its client went away.
    pub grace: std::time::Duration,
}

impl Disconnect {
    /// Whether a request with this method is cancelled when its client goes away.
    #[must_use]
    pub fn cancels(&self, method: &str) -> bool {
        self.cancel.iter().any(|listed| listed == method)
    }
}

/// The two disconnect keys of `config`'s `[limits]`, with their defaults for a key left out.
///
/// A per-app [`Snapshot`](crate::snapshot::Snapshot) has already folded every matching
/// `[app.limits]` onto `[limits]`, so this one reader answers for the host and for an application.
///
/// # Errors
///
/// `E0601` for a `cancel_on_disconnect` that is not a list of HTTP tokens or repeats one, and for a
/// `disconnect_grace` that is not a duration; `E0619` for a grace of `false` or `0`.
pub fn disconnect_for(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<Disconnect, Diagnostic> {
    read_disconnect("limits", config.limits.as_ref(), origins)
}

/// The boot refusal for both disconnect keys, in `[limits]` and in every `[app.limits]`, so a bad
/// value in a block no request has matched yet still stops the boot.
///
/// # Errors
///
/// [`disconnect_for`]'s, naming the block the value was written in.
pub fn disconnect(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    read_disconnect("limits", config.limits.as_ref(), origins)?;
    for (index, block) in config.app.iter().enumerate() {
        read_disconnect(
            &format!("app.{index}.limits"),
            block.limits.as_ref(),
            origins,
        )?;
    }
    Ok(())
}

fn read_disconnect(
    block_path: &str,
    limits: Option<&Limits>,
    origins: &BTreeMap<String, Origin>,
) -> Result<Disconnect, Diagnostic> {
    let cancel = match limits.and_then(|limits| limits.cancel_on_disconnect.as_ref()) {
        None => Vec::new(),
        Some(written) => methods(
            &format!("{block_path}.cancel_on_disconnect"),
            written,
            origins,
        )?,
    };
    let grace = match limits.and_then(|limits| limits.disconnect_grace.as_ref()) {
        None => DISCONNECT_GRACE,
        Some(written) => grace(&format!("{block_path}.disconnect_grace"), written, origins)?,
    };
    Ok(Disconnect { cancel, grace })
}

/// `cancel_on_disconnect` as a list of method names. A method is an HTTP token (RFC 9110 § 5.6.2),
/// so a custom method can be listed, and a name no request can carry is refused rather than kept
/// as a row that never matches.
fn methods(
    key: &str,
    written: &Setting,
    origins: &BTreeMap<String, Origin>,
) -> Result<Vec<String>, Diagnostic> {
    let refused = |message: String, help: &str| {
        Diagnostic::error(code::E_BAD_DIRECTIVE, message)
            .with_note(format!(
                "this key lists the request methods that are cancelled when the client goes away{}",
                origin_note(origins.get(key))
            ))
            .with_help(help.to_string())
    };
    let Setting::List(names) = written else {
        return Err(refused(
            format!(
                "`{key}` is `{}`, which is not a list of methods",
                crate::value::as_written(written)
            ),
            "write a list, as `[\"GET\", \"HEAD\"]`; an empty list cancels no request",
        ));
    };
    let mut seen: Vec<String> = Vec::with_capacity(names.len());
    for name in names {
        let token = !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte));
        if !token {
            return Err(refused(
                format!("`{key}` lists `{name}`, which is not an HTTP method name"),
                "a method name is letters, digits and `!#$%&'*+-.^_`|~` with no space, as `GET` or `PURGE`",
            ));
        }
        if seen.contains(name) {
            return Err(refused(
                format!("`{key}` lists `{name}` twice"),
                "list each method once; methods are matched exactly, so `get` and `GET` are two methods",
            ));
        }
        seen.push(name.clone());
    }
    Ok(seen)
}

/// `disconnect_grace` as a duration. `false` and `0` are both a request with no `wall_time` that
/// is never stopped once its client has gone, which is the unbounded default the key exists to
/// close.
fn grace(
    key: &str,
    written: &Setting,
    origins: &BTreeMap<String, Origin>,
) -> Result<std::time::Duration, Diagnostic> {
    let quantity = Quantity::parse(key, crate::value::Unit::Duration, written)
        .map_err(|invalid| invalid.diagnostic(origins.get(key)))?;
    match quantity {
        Quantity::Nanos(nanos) if nanos > 0 => Ok(std::time::Duration::from_nanos(nanos)),
        _ => Err(Diagnostic::error(
            code::E_UNBOUNDED_WAIT,
            format!(
                "`{key}` is `{}`, which never stops a request whose client has gone",
                crate::value::as_written(written)
            ),
        )
        .with_note(format!(
            "a request with no `wall_time` is bounded only by this after its client goes away{}",
            origin_note(origins.get(key))
        ))
        .with_help(format!(
            "write how long such a request may run on, as `30s`, or set `wall_time`; leaving \
             `{key}` out keeps the default of 30 seconds"
        ))),
    }
}

/// One key of one block against the host's ceiling for it.
///
/// A key with no unit is left alone rather than compared: [`unit_of`] answers for every limit the
/// tree spells, and a limit added to [`Limits`](crate::tree::Limits) without a unit is a gap in
/// that table rather than a reason to panic here.
fn bounded(
    block_path: &str,
    name: &str,
    asked: Option<&Setting>,
    ceiling: Option<&Setting>,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    let (Some(asked), Some(ceiling)) = (asked, ceiling) else {
        return Ok(());
    };
    let key = format!("{block_path}.{name}");
    let host_key = format!("limits.hard.{name}");
    let Some(unit) = unit_of(&key) else {
        return Ok(());
    };
    let wanted = Quantity::parse(&key, unit, asked)
        .map_err(|invalid| invalid.diagnostic(origins.get(&key)))?;
    let allowed = Quantity::parse(&host_key, unit, ceiling)
        .map_err(|invalid| invalid.diagnostic(origins.get(&host_key)))?;
    if wanted.within(allowed) {
        return Ok(());
    }
    Err(above_ceiling(
        &key,
        asked,
        ceiling,
        origins.get(&key),
        origins.get(&host_key),
    ))
}

/// The `[limits]` keys of a per-app block, named and in the tree's own order — the same keys
/// [`ceilings`] returns, in the same order, so the two zip row for row.
fn defaults(limits: &Limits) -> [(&'static str, Option<&Setting>); 9] {
    [
        ("memory", limits.memory.as_ref()),
        ("cpu_time", limits.cpu_time.as_ref()),
        ("wall_time", limits.wall_time.as_ref()),
        ("max_tasks", limits.max_tasks.as_ref()),
        ("max_output", limits.max_output.as_ref()),
        ("request_body", limits.request_body.as_ref()),
        ("upload_total", limits.upload_total.as_ref()),
        ("max_multipart_parts", limits.max_multipart_parts.as_ref()),
        ("max_regex_steps", limits.max_regex_steps.as_ref()),
    ]
}

/// The `System`-class quantities of a `[limits]` block that have no ceiling, which [`quantities`]
/// parses beside [`defaults`]' rows.
fn system(limits: &Limits) -> [(&'static str, Option<&Setting>); 5] {
    [
        ("fatal_reserve_memory", limits.fatal_reserve_memory.as_ref()),
        ("fatal_reserve_time", limits.fatal_reserve_time.as_ref()),
        ("max_script_depth", limits.max_script_depth.as_ref()),
        ("max_decompressed", limits.max_decompressed.as_ref()),
        (
            "max_decompression_ratio",
            limits.max_decompression_ratio.as_ref(),
        ),
    ]
}

/// The same keys of a `[limits.hard]`, carrying their names — one table, so a new limit is a row
/// here rather than a scattering of places to forget.
pub(crate) fn ceilings(set: &LimitSet) -> [(&'static str, Option<&Setting>); 9] {
    [
        ("memory", set.memory.as_ref()),
        ("cpu_time", set.cpu_time.as_ref()),
        ("wall_time", set.wall_time.as_ref()),
        ("max_tasks", set.max_tasks.as_ref()),
        ("max_output", set.max_output.as_ref()),
        ("request_body", set.request_body.as_ref()),
        ("upload_total", set.upload_total.as_ref()),
        ("max_multipart_parts", set.max_multipart_parts.as_ref()),
        ("max_regex_steps", set.max_regex_steps.as_ref()),
    ]
}

/// `rule:http-server/request-body-and-upload-total-are-two-caps`'s two caps and
/// `rule:errors/multipart-part-count`'s part count against their ceilings, which are the only
/// `[limits.hard]` keys with a default of their own: `64M`, `2G` and `10000`.
///
/// [`bound`] applies a ceiling only where the operator wrote one, because every other ceiling
/// defaults to the value the run started with. These three default to a number, so a `[limits]` or
/// `[app.limits]` value past it is refused here whether or not `[limits.hard]` is written. An
/// `[app.limits.hard]` value is held to the same ceiling, for [`bound`]'s reason.
///
/// # Errors
///
/// `E0610` for a value above the ceiling, and `E0601` for a value that is not a quantity.
pub fn body_caps(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    let hard = config
        .limits
        .as_ref()
        .and_then(|limits| limits.hard.as_ref());
    for (name, default) in [
        (
            "request_body",
            crate::defaults::size(crate::defaults::LIMITS_HARD_REQUEST_BODY),
        ),
        (
            "upload_total",
            crate::defaults::size(crate::defaults::LIMITS_HARD_UPLOAD_TOTAL),
        ),
        (
            "max_multipart_parts",
            crate::defaults::LIMITS_HARD_MAX_MULTIPART_PARTS.to_string(),
        ),
    ] {
        let written = hard.and_then(|hard| field(&ceilings(hard), name));
        let ceiling = written.unwrap_or(Setting::Text(default));
        if let Some(asked) = config
            .limits
            .as_ref()
            .and_then(|limits| field(&defaults(limits), name))
        {
            let key = format!("limits.{name}");
            if crate::value::within_ceiling(&key, &asked, &ceiling)
                .map_err(|invalid| invalid.diagnostic(origins.get(&key)))?
                == Some(false)
            {
                return Err(start_above_ceiling(
                    &key,
                    &asked,
                    &ceiling,
                    origins.get(&key),
                ));
            }
        }
        for (index, block) in config.app.iter().enumerate() {
            let Some(limits) = block.limits.as_ref() else {
                continue;
            };
            let block_path = format!("app.{index}.limits");
            bounded(
                &block_path,
                name,
                field(&defaults(limits), name).as_ref(),
                Some(&ceiling),
                origins,
            )?;
            if let Some(own) = limits.hard.as_ref() {
                let hard_path = format!("{block_path}.hard");
                bounded(
                    &hard_path,
                    name,
                    field(&ceilings(own), name).as_ref(),
                    Some(&ceiling),
                    origins,
                )?;
            }
        }
    }
    Ok(())
}

/// The global `[limits] upload_total` in bytes, which is the cap the server's door applies
/// before a mount is selected (`rule:http-server/request-body-and-upload-total-are-two-caps`).
///
/// The shipped default where the tree writes none, or writes a value that is not a size: the
/// resolve already refused a malformed one at boot, where it could name the file and the line.
#[must_use]
pub fn upload_total(config: &Config) -> u64 {
    let written = config
        .limits
        .as_ref()
        .and_then(|limits| limits.upload_total.as_ref());
    match written
        .map(|setting| Quantity::parse("limits.upload_total", crate::value::Unit::Bytes, setting))
    {
        Some(Ok(Quantity::Bytes(bytes))) => bytes,
        _ => crate::defaults::LIMITS_UPLOAD_TOTAL,
    }
}

/// `E0601` for a `[limits]` start above its ceiling, from [`starts`] or [`body_caps`]. The message
/// names both values, and the help names the one key that raises the ceiling.
fn start_above_ceiling(
    key: &str,
    asked: &Setting,
    ceiling: &Setting,
    written_in: Option<&Origin>,
) -> Diagnostic {
    let leaf = key.trim_start_matches("limits.");
    Diagnostic::error(
        code::E_BAD_DIRECTIVE,
        format!(
            "`[limits] {leaf}` is `{}`, which is above its ceiling of `{}`",
            crate::value::as_written(asked),
            crate::value::as_written(ceiling)
        ),
    )
    .with_note(format!(
        "a request may raise its own `{leaf}` up to the ceiling, so the value it starts with \
         must not be above it{}",
        origin_note(written_in)
    ))
    .with_help(format!(
        "lower `[limits] {leaf}`, or write a higher `[limits.hard] {leaf}`"
    ))
}

/// The value one row of [`defaults`] or [`ceilings`] carries for `name`, owned.
fn field(rows: &[(&'static str, Option<&Setting>)], name: &str) -> Option<Setting> {
    rows.iter()
        .find(|(row, _)| *row == name)
        .and_then(|(_, written)| written.cloned())
}

/// `rule:config/every-matching-app-block-applies-least-specific-first`'s matching blocks for one entry file, **least-specific first** — shortest `root` first,
/// longest last, an `entry` match last of all.
///
/// Indices into `apps` rather than references, because [`layer`] needs to reach the block's own
/// `toml::Table` beside it and a reference cannot say which one it is.
///
/// `apps` must have come through [`canonicalize`], which [`resolve`](crate::resolve::resolve) does
/// for every tree it returns; a block still holding what the operator typed compares against the
/// wrong thing and is the failure `rule:config/app-keys-are-canonicalized-before-matching` is written to prevent.
///
/// Ordering is by the key path's component count alone, and that already puts an `entry` match
/// last: an `entry` block matches only the entry file itself, which is longer than every `root`
/// that can be a proper prefix of it. Two matching blocks cannot tie — equal-length paths that are
/// both prefixes of one entry are the same path, which [`canonicalize`] has already refused as a
/// duplicate — so the sort is total and the stable-sort tiebreak is unreachable.
///
/// # Errors
///
/// `E0605` when `entry` itself cannot be examined.
pub fn matching(apps: &[App], entry: &Path, files: &dyn Files) -> Result<Vec<usize>, Diagnostic> {
    let entry = files.canonical(entry).map_err(|err| {
        crate::resolve::unreadable(
            entry,
            &err,
            "it is the entry file whose `[[app]]` blocks were being looked up",
        )
    })?;
    // A block keyed on the entry file or on any directory above it would apply here, whether or
    // not one is written today, so each of those paths is what this program uses of the roster.
    for above in entry.ancestors() {
        nvs_footprint::app(above);
    }
    let mut matched: Vec<usize> = (0..apps.len())
        .filter(|index| covers(&apps[*index], &entry))
        .collect();
    matched.sort_by_key(|index| key_of(&apps[*index]).map_or(0, |key| key.components().count()));
    Ok(matched)
}

/// The app folder of the canonical `entry`: the `root` of the most specific block that covers it,
/// or else the folder `entry` is in. A capability default reads it ([`crate::capability::Run`]).
///
/// `keys` are the keys of the blocks that match `entry`, least-specific first, as [`matching`]
/// orders them and [`Snapshot::blocks`](crate::Snapshot::blocks) records them. A block keyed on
/// `entry` itself names no folder, so it is passed over for the next `root`: a block that sets
/// one file's limits does not shrink the application that file belongs to. An entry in `public/`
/// under `root = "/srv/www/shop"` has the folder `/srv/www/shop`.
///
/// This is the one place the app folder is decided. Pure: `entry` and `keys` are canonical
/// already, so nothing here reads the disk.
#[must_use]
pub fn folder(entry: &Path, keys: &[PathBuf]) -> PathBuf {
    keys.iter()
        .rev()
        .map(PathBuf::as_path)
        .find(|key| *key != entry)
        .or_else(|| entry.parent())
        .unwrap_or(entry)
        .to_path_buf()
}

/// The keys of the blocks [`matching`] named, in its order: what [`folder`] reads.
#[must_use]
pub fn keys(apps: &[App], matched: &[usize]) -> Vec<PathBuf> {
    matched
        .iter()
        .filter_map(|index| apps.get(*index).and_then(key_of))
        .map(Path::to_path_buf)
        .collect()
}

/// Records that the caller uses every `[[app]]` block of `config`, which must have come through
/// [`canonicalize`]: the whole roster, and the path each block is keyed on, since a key whose path
/// is gone refuses the boot. A reader that prints or compares the roster calls this, and so does
/// the test that resolves the repository's own configuration. A program run for one entry file
/// records only the paths [`matching`] could match.
pub fn record_roster(config: &Config) {
    nvs_footprint::every_app();
    for block in &config.app {
        if let Some(key) = key_of(block) {
            nvs_footprint::exists(key);
        }
    }
}

/// What `rule:config/every-matching-app-block-applies-least-specific-first`'s layering produced for one entry file.
#[derive(Clone, Debug, Default)]
pub struct Layered {
    /// The effective block: every matching block's directives folded into one, later-wins by
    /// specificity. Its `root` and `entry` are cleared — the blocks it came from are below, and a
    /// key carried over from the most specific of them would read as a block that was written.
    pub app: App,
    /// The blocks that matched, least-specific first, by the canonical path each is keyed on.
    /// This is `rule:config/every-matching-app-block-applies-least-specific-first`'s `info: app blocks: …` line.
    pub blocks: Vec<PathBuf>,
    /// Every directive one block took from another, in the order it happened, carrying **both**
    /// origins exactly as `rule:config/later-wins-and-every-override-is-recorded`'s own record does — which is `rule:config/every-matching-app-block-applies-least-specific-first`'s whole argument for
    /// being a fourth ordering rather than a third precedence rule.
    ///
    pub overrides: Vec<Override>,
}

/// `rule:config/every-matching-app-block-applies-least-specific-first`: every block matching `entry` applied, least-specific first.
///
/// The fold is [`resolve`](crate::resolve)'s own `merge_table` over the blocks' tables, not a
/// second implementation of later-wins. That is the section's central claim — "this is not a third
/// precedence rule: it is `rule:config/later-wins-and-every-override-is-recorded`'s later wins, ordered by specificity instead of by file
/// position, and every override is reported the same way" — and it is only true if one function
/// decides both. It is also why [`Resolved::table`](crate::resolve::Resolved::table) is kept:
/// `merge_table` folds `toml::Table`s, and the typed tree cannot be turned back into one.
///
/// An [`Override`] names the two **files**, because that is what an [`Origin`] holds; which
/// *blocks* they were is [`Layered::blocks`], in the same order.
///
/// # Errors
///
/// `E0605` when `entry` cannot be examined, and `E0601` if the folded block does not deserialize —
/// unreachable, since every block was typed as an [`App`] in its own file before it was merged.
///
pub fn layer(resolved: &Resolved, entry: &Path, files: &dyn Files) -> Result<Layered, Diagnostic> {
    let mut layered = Layered::default();
    let mut merged = toml::Table::new();
    let mut origins: BTreeMap<String, Origin> = BTreeMap::new();
    for index in matching(&resolved.config.app, entry, files)? {
        let block = &resolved.config.app[index];
        if let Some(key) = key_of(block) {
            layered.blocks.push(key.to_path_buf());
        }
        // The block as it was written, which is where its untyped `toml` value still is. `root` and
        // `entry` are dropped rather than merged: they are what selected these blocks, and an
        // override record about them would report the match rather than a directive.
        let Some(mut table) = block_table(resolved, index) else {
            continue;
        };
        table.remove("root");
        table.remove("entry");
        let Some(origin) = block_origin(resolved, index) else {
            continue;
        };
        crate::resolve::merge_table(
            &mut merged,
            &table,
            origin,
            "",
            &mut origins,
            &mut layered.overrides,
        );
    }
    layered.app = toml::Value::Table(merged)
        .try_into::<App>()
        .map_err(|err| {
            Diagnostic::error(code::E_BAD_DIRECTIVE, err.message().to_string()).with_note(
            "every `[[app]]` block was typed on its own before it was merged, so this can only be \
             two blocks writing one key in two shapes"
                .to_string(),
        )
        })?;
    Ok(layered)
}

/// The file the `index`th `[[app]]` block was written in — its own, falling back to the array's.
///
/// A block reached the typed tree by being claimed or appended, and both of those record an origin,
/// so the `None` arm is unreachable rather than a case with an answer. Which of the two keys it is
/// keyed on decides which origin names it, because that key is the one [`canonicalize`] resolved
/// against the file (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`).
///
pub(crate) fn block_origin(resolved: &Resolved, index: usize) -> Option<&Origin> {
    let field = if resolved.config.app.get(index)?.root.is_some() {
        "root"
    } else {
        "entry"
    };
    resolved
        .origins
        .get(&format!("app.{index}.{field}"))
        .or_else(|| resolved.origins.get("app"))
}

/// The `index`th `[[app]]` block as the untyped table it was written as, or `None` when the merged
/// table has no such entry — which the typed tree having one makes unreachable.
pub(crate) fn block_table(resolved: &Resolved, index: usize) -> Option<toml::Table> {
    Some(
        resolved
            .table
            .get("app")?
            .as_array()?
            .get(index)?
            .as_table()?
            .clone(),
    )
}

/// Whether this block's key covers `entry`, which is canonical.
///
/// `root` is a **proper** prefix: a root that equals the entry file would mean a directory used as
/// a file, and `entry` is the spelling for one file. `Path::starts_with` is the component-wise
/// test `rule:config/an-application-is-its-entry-file-path` asks for, so `/srv/www/shopfront/index.nvs` does not match `root = "/srv/www/shop"`.
fn covers(block: &App, entry: &Path) -> bool {
    if let Some(root) = &block.root {
        let root = Path::new(root);
        entry.starts_with(root) && entry != root
    } else {
        block
            .entry
            .as_ref()
            .is_some_and(|exact| entry == Path::new(exact))
    }
}

/// The path this block is keyed on, whichever of the two keys spelled it.
pub(crate) fn key_of(block: &App) -> Option<&Path> {
    block
        .root
        .as_deref()
        .or(block.entry.as_deref())
        .map(Path::new)
}

/// `E0609` for a block naming both keys: `rule:config/an-application-is-its-entry-file-path` gives one application one key, and a block naming a
/// directory *and* a file has not said which of the two it means.
fn both_keys(index: usize, origins: &BTreeMap<String, Origin>) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_APP_BLOCK,
        format!("{} names both `root` and `entry`", ordinal(index)),
    )
    .with_note(format!(
        "a block names one or the other. `root` covers every entry file in a directory, and \
         `entry` is exactly one file{}",
        origin_note(origins.get(&format!("app.{index}.root")))
    ))
    .with_help(
        "keep `root` and drop `entry` to cover a tree, or split the block in two so the file gets \
         its own directives"
            .to_string(),
    )
}

/// `E0609` for a block naming neither key: there is no default application, because a block that
/// matched everything would be the global configuration written twice.
fn no_key(index: usize) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_APP_BLOCK,
        format!("{} names neither `root` nor `entry`", ordinal(index)),
    )
    .with_note(
        "a block with no `root` and no `entry` matches no entry file, so its directives never \
         apply"
            .to_string(),
    )
    .with_help(
        "add the directory this application lives in as `root`, or the one file as `entry`; a \
         host-wide default is a block with the widest `root`, which applies before the others"
            .to_string(),
    )
}

/// `E0609` for two blocks on one path — `rule:config/every-matching-app-block-applies-least-specific-first`'s duplicate, which is not a refinement.
///
/// Keyed on the canonical path rather than on the spelling, so the same directory reached through
/// two relative paths, or through a symlink, is caught as well: `rule:config/every-matching-app-block-applies-least-specific-first`'s argument is about which
/// block's value wins, and two blocks on one path have no order between them to answer with.
fn duplicate(path: &Path, first: usize, second: usize, written_in: Option<&Origin>) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_APP_BLOCK,
        format!(
            "{} and {} are both keyed on `{}`",
            ordinal(first),
            ordinal(second),
            path.display()
        ),
    )
    .with_note(format!(
        "matching blocks apply from the least specific path to the most specific one. Two blocks \
         on the same path have no order, so neither value can win{}",
        written_in.map_or_else(String::new, |origin| format!(
            "; the second is written in `{}`",
            origin.path.display()
        ))
    ))
    .with_help(
        "merge the two blocks, or narrow one of them to the subdirectory or the entry file it was \
         meant for"
            .to_string(),
    )
}

/// `E0610` for `rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling`'s bound: a block asking for more than the host's `[limits.hard]` allows, named
/// on both sides with where each was written.
///
/// The message says *what the block asked for* rather than "exceeds the ceiling", because the two
/// values side by side are what an operator acts on, and the help offers both directions: lower the
/// block, or raise the host's ceiling if every application may have it.
///
/// The block is named by the key's own `app.N` segment rather than by [`ordinal`], which counts
/// from one: the dotted key is what the origins map and `rule:config/check-and-dump-audit-the-tree-offline`'s `nvs config dump` both spell, and two
/// numberings for one block in one sentence is a worse reading than either alone.
fn above_ceiling(
    key: &str,
    asked: &Setting,
    ceiling: &Setting,
    written_in: Option<&Origin>,
    host_written_in: Option<&Origin>,
) -> Diagnostic {
    Diagnostic::error(
        code::E_APP_ABOVE_CEILING,
        format!(
            "`{key}` is `{}`, above the host's ceiling of `{}`",
            crate::value::as_written(asked),
            crate::value::as_written(ceiling)
        ),
    )
    .with_note(format!(
        "a block may raise `[app.limits]` and lower its own `[app.limits.hard]`. No application \
         may go above the host's `[limits.hard]`{}{}",
        origin_note(written_in),
        host_written_in.map_or_else(String::new, |origin| format!(
            "; the ceiling is set in `{}`",
            origin.path.display()
        ))
    ))
    .with_help(
        "lower the block to the host's ceiling or below. Or raise `[limits.hard]` if every \
         application on this host may have this value"
            .to_string(),
    )
}

/// `E0609` for a canonical path that is not UTF-8, which is the one shape a `[[app]]` key cannot be
/// written back as: it arrived from TOML as text and must go back as text.
fn not_utf8(path: &Path, index: usize, field: &str) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_APP_BLOCK,
        format!(
            "{}'s `{field}` canonicalizes to `{}`, which is not valid UTF-8",
            ordinal(index),
            path.display()
        ),
    )
    .with_note(
        "a block matches the canonical path, and a configuration file is UTF-8 text. A path that \
         is not UTF-8 cannot be compared with any block"
            .to_string(),
    )
}

/// `` `[[app]]` block 3 `` — blocks have no names, so they are counted in the order the tree read
/// them, which is `rule:config/later-wins-and-every-override-is-recorded`'s order.
///
fn ordinal(index: usize) -> String {
    format!("`[[app]]` block {}", index + 1)
}
