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
//! [`resolve`](crate::resolve) the way § 7's secrets do. What they need from that tree is the merged `toml::Table` and its origins, which is
//! why [`Resolved`] keeps both.
//!
//! **[`layer`] answers what the effective block *is*; it is not how the snapshot is built.**
//! [`Snapshot::build`](crate::snapshot::Snapshot::build) folds each matching block over the global
//! tree in [`matching`]'s order instead, by the same [`merge_table`](crate::resolve) — same values,
//! and every key still named by the file the block that wrote it lives in, which an effective block
//! folded from three files cannot do with one origin for all of its keys. This function is § 9's
//! per-app `nvs config dump`.
//!
//! **The comparison is canonicalize-then-prefix, and the canonicalization is
//! [`trust::canonical`](crate::trust::canonical)'s.** That is the whole security content of § 1:
//! without it `/srv/www/shop/../other/x.nvs` matches `root = "/srv/www/shop"` and a symlink planted
//! inside an application's tree inherits that application's capabilities. Both sides are
//! canonicalized — the block's key by [`canonicalize`] once at boot, the entry file by [`matching`]
//! per run — and the prefix test is `Path::starts_with`, which compares whole components, so
//! `/srv/www/shopfront` is not under `/srv/www/shop`.
//!
//! **A block's key is canonicalized in place**, replacing what the operator wrote, exactly as § 7
//! writes a secret file's content into `password`. Two reasons: a comparison against a path that has
//! not been through [`trust::canonical`](crate::trust::canonical) is the bug this module exists to
//! prevent, and § 9's `nvs config dump` then prints the path the match will actually use rather than
//! a relative fragment the reader has to resolve by hand.
//!
//! **A key that cannot be canonicalized refuses the boot** (`E0605`) rather than never matching.
//! The ADR does not decide between those, and the reason to refuse is the priority ordering: a
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

/// § 1's keys, resolved: each `[[app]]` block's `root` or `entry` made absolute against the file
/// that wrote it (`rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`) and canonicalized, written back into the block.
///
/// Runs once over the flattened tree, beside § 7's secrets and for the same reason: `[[app]]`
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
        // § 5: relative to the file it was written in, which is the same rule an `[[include]]` and
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

/// § 3's bound: an `[[app]]` block may widen `[app.limits]` and lower its own `[app.limits.hard]`,
/// and neither may pass the global `[limits.hard]`.
///
/// Runs beside [`canonicalize`] and for the same reason — the global ceiling and the blocks bounded
/// by it are both properties of the merged tree — but it needs no filesystem, so it takes no
/// [`Files`]. With no global `[limits.hard]` written there is no bound to apply and every block
/// passes: § 3 bounds a block by the host's answer, and a host that gave none has not answered.
///
/// The refusal is at boot and the value is **not** clamped, which is § 3 citing `rule:config/three-changeability-classes`'s
/// treatment of `Core\Config::set`: a clamped block still reads as though it got what it asked for,
/// and the operator finds out at the first request that hits the real ceiling.
///
/// What this deliberately does not check, because § 3 does not state it: a per-app default above
/// the block's *own* lowered ceiling, and the same shape globally. Each is incoherent rather than
/// unsafe — the value in force is still bounded by the ceiling the request is held to.
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
fn defaults(limits: &Limits) -> [(&'static str, Option<&Setting>); 6] {
    [
        ("memory", limits.memory.as_ref()),
        ("cpu_time", limits.cpu_time.as_ref()),
        ("wall_time", limits.wall_time.as_ref()),
        ("max_tasks", limits.max_tasks.as_ref()),
        ("max_output", limits.max_output.as_ref()),
        ("max_regex_steps", limits.max_regex_steps.as_ref()),
    ]
}

/// The same keys of a `[limits.hard]`, carrying their names — one table, so a new limit is a row
/// here rather than a scattering of places to forget.
pub(crate) fn ceilings(set: &LimitSet) -> [(&'static str, Option<&Setting>); 6] {
    [
        ("memory", set.memory.as_ref()),
        ("cpu_time", set.cpu_time.as_ref()),
        ("wall_time", set.wall_time.as_ref()),
        ("max_tasks", set.max_tasks.as_ref()),
        ("max_output", set.max_output.as_ref()),
        ("max_regex_steps", set.max_regex_steps.as_ref()),
    ]
}

/// § 2's matching blocks for one entry file, **least-specific first** — shortest `root` first,
/// longest last, an `entry` match last of all.
///
/// Indices into `apps` rather than references, because [`layer`] needs to reach the block's own
/// `toml::Table` beside it and a reference cannot say which one it is.
///
/// `apps` must have come through [`canonicalize`], which [`resolve`](crate::resolve::resolve) does
/// for every tree it returns; a block still holding what the operator typed compares against the
/// wrong thing and is the failure § 1 is written to prevent.
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

/// What § 2's layering produced for one entry file.
#[derive(Clone, Debug, Default)]
pub struct Layered {
    /// The effective block: every matching block's directives folded into one, later-wins by
    /// specificity. Its `root` and `entry` are cleared — the blocks it came from are below, and a
    /// key carried over from the most specific of them would read as a block that was written.
    pub app: App,
    /// The blocks that matched, least-specific first, by the canonical path each is keyed on.
    /// This is § 2's `info: app blocks: …` line.
    pub blocks: Vec<PathBuf>,
    /// Every directive one block took from another, in the order it happened, carrying **both**
    /// origins exactly as `rule:config/later-wins-and-every-override-is-recorded`'s own record does — which is § 2's whole argument for
    /// being a fourth ordering rather than a third precedence rule.
    ///
    pub overrides: Vec<Override>,
}

/// § 2: every block matching `entry` applied, least-specific first.
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
/// test § 1 asks for, so `/srv/www/shopfront/index.nvs` does not match `root = "/srv/www/shop"`.
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

/// `E0609` for a block naming both keys: § 1 gives one application one key, and a block naming a
/// directory *and* a file has not said which of the two it means.
fn both_keys(index: usize, origins: &BTreeMap<String, Origin>) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_APP_BLOCK,
        format!("{} names both `root` and `entry`", ordinal(index)),
    )
    .with_note(format!(
        "`rule:config/an-application-is-its-entry-file-path` keys an application on one or the other: `root` is every entry file beneath \
         a directory, `entry` is one file exactly{}",
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
        "`rule:config/an-application-is-its-entry-file-path` keys an application on an entry file path, so a block with no key matches no \
         entry file and its directives would never apply"
            .to_string(),
    )
    .with_help(
        "add the directory this application lives in as `root`, or the one file as `entry`; a \
         host-wide default is a block with the widest `root`, which § 2 layers under the others"
            .to_string(),
    )
}

/// `E0609` for two blocks on one path — § 2's duplicate, which is not a refinement.
///
/// Keyed on the canonical path rather than on the spelling, so the same directory reached through
/// two relative paths, or through a symlink, is caught as well: § 2's argument is about which
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
        "`rule:config/every-matching-app-block-applies-least-specific-first` layers matching blocks by specificity, so two blocks on the same path are a \
         duplicate rather than a refinement and there is no order between them to decide which \
         value wins{}",
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

/// `E0610` for § 3's bound: a block asking for more than the host's `[limits.hard]` allows, named
/// on both sides with where each was written.
///
/// The message says *what the block asked for* rather than "exceeds the ceiling", because the two
/// values side by side are what an operator acts on, and the help offers both directions: lower the
/// block, or raise the host's ceiling if every application may have it.
///
/// The block is named by the key's own `app.N` segment rather than by [`ordinal`], which counts
/// from one: the dotted key is what the origins map and § 9's `nvs config dump` both spell, and two
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
        "`rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling` lets a block widen `[app.limits]` and lower its own `[app.limits.hard]`, but \
         `[limits.hard]` is the host's answer and an application cannot exceed it{}{}",
        origin_note(written_in),
        host_written_in.map_or_else(String::new, |origin| format!(
            "; the ceiling is set in `{}`",
            origin.path.display()
        ))
    ))
    .with_help(
        "lower the block to the host's ceiling or below, or raise `[limits.hard]` if every \
         application on this host may have it — the value is refused rather than clamped, so that \
         a block never reads as though it got what it asked for"
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
        "`rule:config/an-application-is-its-entry-file-path` matches on the canonical path, and a configuration file is UTF-8 text, so a \
         path this process cannot spell back is one no block could be compared against"
            .to_string(),
    )
}

/// `` `[[app]]` block 3 `` — blocks have no names, so they are counted in the order the tree read
/// them, which is `rule:config/later-wins-and-every-override-is-recorded`'s order.
///
fn ordinal(index: usize) -> String {
    format!("`[[app]]` block {}", index + 1)
}
