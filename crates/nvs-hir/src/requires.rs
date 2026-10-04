//! `require`'s static resolution (M2 item 4 — see the crate's module docs
//! for what's left after this).
//!
//! `rule:statements/require-is-the-only-inclusion-construct`
//! keeps `require` as the only same-frame inclusion construct: no isolation
//! at all, so a required file's declarations must be visible to name
//! resolution exactly as if it had been pasted in at the `require` site.
//! [`resolve_program`] is the entry point that makes that happen for a
//! `require` whose path is statically known. A path that is not (a variable,
//! an interpolated string, and the other shapes the paragraph *A statically
//! known path* lists below) is left alone entirely here: no diagnostic and
//! nothing collected. That is the dynamic fallback. The file graph closes
//! while compiling (`rule:programs/no-runtime-autoload`), so no file is ever
//! loaded for such a path, and `nvs_ir`'s lowering makes the site throw a
//! `RuntimeError` when it is reached, which is
//! `rule:statements/require-is-the-only-inclusion-construct`'s missing target.
//!
//! The mechanism is the multi-file shape [`crate::resolve::Resolver`],
//! [`crate::hierarchy::HierarchyResolver`], [`crate::members::MemberResolver`]
//! and [`crate::aliases::AliasResolver`] were all already built to support
//! (their own module docs call this out as "not wired up yet") — an explicit
//! worklist walks the require graph starting at one entry file: each literal
//! target is resolved relative to the requiring file's own directory,
//! loaded and parsed the first time its canonicalized path is named, fed
//! through every resolver's `collect_*` pass, and then walked for its own
//! `require` statements the same way. Once every reachable file has been
//! collected, the same closing passes `resolve_file` runs for one file
//! (`resolve_imports`, `hierarchy.resolve`, `members.check`, `aliases.resolve`)
//! run once over the whole graph.
//!
//! The walk hands its files back alongside the [`Module`], as a [`Loaded`]
//! per file in entry-first load order. Every phase after name resolution —
//! type-checking, layout, lowering — needs each file's statements again, and
//! this is the only place they were ever parsed, so dropping them here would
//! force the caller to re-read the graph to find out what was in it.
//!
//! The same worklist runs
//! `rule:programs/autoload`'s autoload resolution, as a fixpoint rather than a second pass. Each
//! file's walk harvests more than its `require` targets: its `autoload`
//! declarations too, and every name it uses where a class, interface,
//! enum or `type` alias is meant. When the `require` graph is drained the
//! declarations become an [`crate::autoload::AutoloadMap`] — that module owns
//! the map's own rules — and the first harvested name that is still
//! undeclared and that the map can place is loaded, parsed, checked against
//! § 2's one-declaration-per-file rule, and pushed back onto the worklist,
//! where its own requires and names are harvested in turn. The loop ends when
//! no undeclared name resolves to a file nothing has loaded, so a program
//! that never writes an `autoload` pays one empty-map check and nothing else.
//!
//! **The map is fixed the moment it is first consulted.** Every `autoload`
//! declaration found after that — in an autoloaded file, or in a file one of
//! them required — is `code::E_AUTOLOAD_IN_AUTOLOADED_FILE`, because a map
//! that grows as it is consulted is exactly the self-dependence § 1 forbids,
//! and it is what would make the answer depend on resolution order.
//!
//! A literal path whose spelling differs from the on-disk entry's only in
//! case is `code::E_REQUIRE_PATH_CASE_MISMATCH` — see [`check_path_case`],
//! which is what stops a `require` from compiling on Windows/macOS and then
//! failing on Linux ([ADR 0062](/docs/decisions/0062.md)
//! § 3). A literal path that resolves to nothing loadable is
//! `code::E_REQUIRE_TARGET_NOT_FOUND`. A literal path that leads back to a
//! file already on the current chain is `code::E_CIRCULAR_REQUIRE` rather
//! than infinite recursion — the same cycle-to-diagnostic treatment
//! [`crate::hierarchy::detect_cycles`] and [`crate::aliases`] both already
//! give their own kind of cycle. A file reachable by more than one path
//! through the graph (a diamond, not a cycle — `A` requires `B` and `C`,
//! both of which require `D`) is loaded and collected exactly once, keyed by
//! its canonicalized path, so `D`'s declarations don't collide with
//! themselves under `E_DUPLICATE_DECLARATION`.
//!
//! **A file with no on-disk path resolves no relative `require`.** Every other
//! test fixture in this crate is built with [`nvs_diagnostics::SourceMap::add`]
//! rather than [`nvs_diagnostics::SourceMap::load`], so it has no directory to
//! resolve against and a literal `require` written inside one is left as a
//! dynamic fallback. That is inherent to running from a source with no path — a
//! REPL line, `stdin` — rather than a limitation to fix.
//!
//! **The name harvest is an over-approximation, and every AST variant it
//! walks is named.** Each `match` over an `nvs_syntax::ast` enum here — types
//! and type atoms, statements, class members, destructuring elements, `new`
//! targets, member names and expressions — lists every variant that enum
//! declares, so a position holding nothing to harvest says so rather than
//! falling off the end of the walk, and a position that nests a type or an
//! expression is reached. The wildcard arm each of those matches still
//! carries is what `#[non_exhaustive]` requires of a cross-crate `match`, and
//! a variant added to the AST lands there until it is named: that is the one
//! way a name can still be missed. The cost of missing one is a class that
//! fails to autoload (`rule:programs/autoload`), so the direction to widen in
//! is always "harvest more", never "filter harder".
//!
//! **A statically known path is literal text, class constants, and `.` between
//! them.** [`require_path_segments`] cooks a plain `'...'`/`"..."` token into
//! literal text, reads a `Class::CONST` as the constant standing there, and
//! folds a `.` between two halves that are themselves segments, so
//! `require 'lib/' . 'db.nvs';` and `require Paths::LIB . 'db.nvs';` are
//! resolved and bundled exactly as the one-literal spelling is. Everything
//! else is the dynamic fallback: a heredoc/nowdoc token, an interpolation, an
//! `as` conversion, a variable.
//!
//! **The constant folder runs ahead of the walk, and sees the files the walk
//! has already read.** A class constant is the only constant Novis has
//! (`rule:classes/no-free-functions-or-constants`), so reading one means
//! resolving a class, and the symbol table that answers that is the one this
//! walk is building — out of files a `require` it has not folded yet may be
//! what loads. So [`collect_consts`] is a pass of its own, over declaration
//! positions alone, filling a [`ConstTable`] from each file the moment the walk
//! reaches it, and each `require` target is folded against that table as it
//! stands rather than in one batch taken up front. **That is the bound, and it
//! is the order a running program would see it in:** a file's own constants are
//! there before any of its own paths are folded, and so are the constants of a
//! file an *earlier* `require` in it loaded, while a constant declared in a
//! file this walk reaches later — or reaches only through an `autoload` probe —
//! leaves the path dynamic rather than being chased. The class side of
//! `Class::CONST` is a written name, never `self`, a variable or an expression.
//! Nothing here checks that the class exists or that the constant is visible
//! from the `require`: the checker reports both, against the same expression.

use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, SourceId, SourceMap, Span, code};
use nvs_syntax::ast::{
    Arg, ArrayItem, AttributeGroup, AutoloadDecl, AutoloadKind, BinaryOp, Block, CallArgs,
    ClassMember, ClassMemberKind, ConstMember, DestructureElement, DestructureTarget, Expr,
    ExprKind, FnBody, ImplementsClause, MemberName, MethodMember, Name, NamespaceDecl, NewTarget,
    Param, PropertyHook, PropertyHookBody, Stmt, StmtKind, StringPart, TestOperand, Type, TypeAtom,
    TypeKind,
};
use nvs_syntax::{check_declarations, parse_file};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::aliases::AliasResolver;
use crate::autoload::{self, AutoloadMap, Site};
use crate::hierarchy::{CoreRoster, HierarchyResolver};
use crate::members::{MemberResolver, PhpFunctions};
use crate::qname::QName;
use crate::resolve::{Module, Resolver};

/// One file pulled into the require graph, with the statements it parsed to.
///
/// [`resolve_program`] keeps these (rather than dropping each after its
/// `collect_*` pass) because [`crate::members::MemberResolver::check`] needs
/// every file's statements again, after every file has been collected — and
/// then hands the whole vector back, because every later phase needs the same
/// set: `nvs-types` type-checks each file's own top-level statements, and
/// `nvs-ir` lowers each file's declarations. The entry file's statements are
/// in here too, which is what makes the vector a complete description of the
/// program rather than "the files the entry pulled in".
#[derive(Debug)]
pub struct Loaded {
    /// The file, as [`nvs_diagnostics::SourceMap`] knows it.
    pub id: SourceId,
    /// Its whole parsed body, moved here once and never re-parsed.
    pub stmts: Vec<Stmt>,
    /// Every `require` written *in this file* whose literal path this walk
    /// resolved, as the span of the path expression paired with the file it
    /// named.
    ///
    /// This walk is the only place that edge exists: it joins the base
    /// directory, canonicalizes, and decides whether the target had already
    /// been loaded, so a later phase holding only the literal text would have
    /// to re-derive all three — a second copy of the rule, and one that would
    /// disagree the moment [`check_path_case`] or the cycle check changed.
    /// `nvs-ir` needs exactly this to call a required file's own script frame
    /// at the `require` site (`nvs-ir`'s known gap 22), and its span key is
    /// what survives both the `Stmt` move above and the crate boundary.
    ///
    /// A path that is not a literal, that resolves to nothing loadable, or
    /// that closes a cycle contributes no entry — each is already a
    /// diagnostic, or the dynamic fallback `rule:statements/require-is-the-only-inclusion-construct` leaves alone. A path
    /// naming a file some *other* file already required does contribute one:
    /// the file is loaded once, and both sites name it.
    pub requires: Vec<(Span, SourceId)>,
}

/// Resolves the `require` graph reachable from one entry file into a single
/// [`Module`], the same way [`crate::resolve::resolve_file`] resolves one
/// self-contained file — see the module docs for the mechanism.
///
/// `entry_stmts` is consumed rather than borrowed, matching every file
/// discovered by a `require` along the way: this function owns every file's
/// statements for as long as the graph walk needs them, since a later file
/// can't be loaded while an earlier one's `&SourceFile` is still borrowed
/// from `map`. They come back out in the [`Loaded`] vector, because the
/// caller's next phase needs them and nothing else in the pipeline is willing
/// to re-parse a file this one already read.
///
/// **The order is the entry file first, then load order** — the order the
/// walk finished collecting each file, which is a deterministic function of
/// the graph and each file's own `require` order. A caller that must run a
/// per-file phase in a stable sequence (a diagnostic's file order, a codegen
/// unit's) can take this vector as given rather than sorting it. `rule:programs/no-runtime-autoload`'s
/// program id is the strictest reader of that contract: it hashes each file's
/// content digest in this order, so a walk that returned the same files in
/// another one would rename a program nobody had edited.
///
/// The third element is the [`AutoloadMap`] the walk consulted, handed back
/// rather than dropped so `nvs check --autoload-map` can print it (`rule:programs/autoload`). It is complete for any program that reached the first probe — which
/// is every program, since the walk consults the map once the `require` graph
/// drains, whether or not a name is still waiting on it.
///
/// `core` is which names the `Core` namespace declares — a question no
/// [`crate::SymbolTable`] can answer, because no source file may declare one.
/// A caller holding the stdlib passes [`CoreRoster::Names`] and a link naming
/// something outside it is refused; one with no stdlib in hand passes
/// [`CoreRoster::Trusted`].
///
/// This entry point passes no [`PhpFunctions`] lookup, so a call to a PHP
/// function gets `E0320`'s general help. The two below take one.
#[must_use]
pub fn resolve_program(
    entry_id: SourceId,
    entry_stmts: Vec<Stmt>,
    map: &mut SourceMap,
    core: CoreRoster<'_>,
    diags: &mut Diagnostics,
) -> (Module, Vec<Loaded>, AutoloadMap) {
    let stdlib = Stdlib { core, php: None };
    walk(entry_id, entry_stmts, map, stdlib, diags, false, &[])
}

/// [`resolve_program`] with `rule:tooling/strict-docs`'s lint in front of it —
/// the same walk, reporting a public member with no doc comment as it goes.
///
/// `strict_docs` is true only under `nvs check --strict-docs`, which mirrors
/// the way `nvs-cli`'s `front_end_granted` carries the `[capabilities]`
/// question: one entry point per caller that asks, rather than a flag every
/// caller of the plain one has to pass. It reaches every file the walk loaded,
/// not the entry point alone — the program is what a package publishes.
///
/// `php` is what PHP's built-in functions became, which `E0320`'s help names
/// the replacement from.
///
/// `borrowed` is what [`resolve_program_borrowing`] takes, and `nvs check`
/// passes it when the file it checks is one a program autoloads or requires
/// ([`crate::lenders`]). It is empty for a file a program starts from.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the walk's own inputs plus the two `nvs check` alone sets; a struct would be a second spelling of `walk`'s parameters"
)]
pub fn resolve_program_linted(
    entry_id: SourceId,
    entry_stmts: Vec<Stmt>,
    map: &mut SourceMap,
    core: CoreRoster<'_>,
    php: PhpFunctions,
    diags: &mut Diagnostics,
    strict_docs: bool,
    borrowed: &[Site],
) -> (Module, Vec<Loaded>, AutoloadMap) {
    let stdlib = Stdlib { core, php };
    walk(
        entry_id,
        entry_stmts,
        map,
        stdlib,
        diags,
        strict_docs,
        borrowed,
    )
}

/// [`resolve_program`] for an entry that is not where its program starts —
/// the same walk, with `borrowed` behind whatever `autoload` the entry's own
/// `require` chain writes.
///
/// An editor analyses the file that is open, and a file a program autoloads
/// can never carry the map that reached it: `rule:programs/autoload` makes an
/// `autoload` inside one an error. `borrowed` is that program's declarations,
/// each still resolving against the directory of the file that wrote it, so
/// the open file's names land on the files the real build lands them on.
/// [`AutoloadMap::build_borrowing`] is what keeps a borrowed site from
/// reporting anything. [`crate::lenders`] says which program lends, and
/// [`resolve_program_linted`] takes the same input for `nvs check`.
#[must_use]
pub fn resolve_program_borrowing(
    entry_id: SourceId,
    entry_stmts: Vec<Stmt>,
    map: &mut SourceMap,
    core: CoreRoster<'_>,
    php: PhpFunctions,
    diags: &mut Diagnostics,
    borrowed: &[Site],
) -> (Module, Vec<Loaded>, AutoloadMap) {
    let stdlib = Stdlib { core, php };
    walk(entry_id, entry_stmts, map, stdlib, diags, false, borrowed)
}

/// What a front end tells the walk about the stdlib, which this crate does not
/// depend on: the names `Core` declares, and what PHP's functions became.
#[derive(Clone, Copy)]
struct Stdlib<'a> {
    core: CoreRoster<'a>,
    php: PhpFunctions,
}

/// The walk behind every `resolve_program` entry point.
fn walk(
    entry_id: SourceId,
    entry_stmts: Vec<Stmt>,
    map: &mut SourceMap,
    stdlib: Stdlib<'_>,
    diags: &mut Diagnostics,
    strict_docs: bool,
    borrowed: &[Site],
) -> (Module, Vec<Loaded>, AutoloadMap) {
    let Stdlib { core, php } = stdlib;
    let mut resolver = Resolver::new();
    let mut hierarchy = HierarchyResolver::new(core);
    let mut members = MemberResolver::with_core(core).with_php(php);
    let mut aliases = AliasResolver::new();

    let mut loaded: Vec<Loaded> = Vec::new();
    let mut done: FxHashSet<PathBuf> = FxHashSet::default();
    let entry_chain: Vec<PathBuf> = canonical_path(map.file(entry_id)).into_iter().collect();
    // `done` answers "has this path been walked"; `by_path` answers "as which
    // file", which is what a `require` naming an already-loaded path needs to
    // record its own edge. They are two maps rather than one because a path
    // can be marked done and then fail to load, and an entry with no id is
    // exactly what such a `require` must not be handed.
    let mut by_path: FxHashMap<PathBuf, SourceId> = FxHashMap::default();
    for path in &entry_chain {
        done.insert(path.clone());
        by_path.insert(path.clone(), entry_id);
    }
    // The constant folder's table: every foldable constant the files walked so
    // far declare, which is what a `require` path naming one is resolved
    // against. Keyed by declaring class, because
    // `rule:classes/no-free-functions-or-constants` leaves a class constant as
    // the only constant a path can name.
    let mut consts = ConstTable::default();

    let mut work: Vec<(SourceId, Vec<Stmt>, Vec<PathBuf>)> =
        vec![(entry_id, entry_stmts, entry_chain)];

    // `rule:programs/no-runtime-autoload`'s accumulators: every `autoload` declaration the
    // bootstrap chain wrote, every name that might need one, and the names
    // already probed, so a miss costs one probe rather than one per mention.
    let mut sites: Vec<Site> = Vec::new();
    let mut wanted: Vec<(QName, Span)> = Vec::new();
    // § 3's opt-in: the first `implementing<T>()` site seen, and `None` once
    // the scan it asked for has run. A program writing no such call leaves it
    // `None` for the whole walk and never lists a directory.
    let mut scan: Option<Span> = None;
    let mut probed: FxHashSet<QName> = FxHashSet::default();
    let mut autoload_map: Option<AutoloadMap> = None;

    loop {
        while let Some((id, stmts, chain)) = work.pop() {
            {
                let src = map.file(id);
                resolver.collect_declarations(&stmts, src, diags);
                hierarchy.collect_links(&stmts, src);
                members.collect_members(&stmts, src);
                aliases.collect_aliases(&stmts, src);
                // Before this file's own `require` paths are folded below, so
                // a constant it declares is one of them however far down the
                // file it is written.
                collect_consts(&stmts, src, &mut consts);
            }

            let mut harvest = Harvest::default();
            find_written_requires(&stmts, map.file(id), &mut harvest);
            wanted.append(&mut harvest.names);
            scan = scan.or_else(|| harvest.scans.first().copied());
            let targets = std::mem::take(&mut harvest.requires);
            let base_dir = map
                .file(id)
                .path()
                .and_then(|p| p.parent().map(Path::to_path_buf));

            let mut file_requires: Vec<(Span, SourceId)> = Vec::new();
            if let Some(base_dir) = base_dir {
                let file_sites: Vec<Site> = harvest
                    .autoloads
                    .into_iter()
                    .map(|(kind, span)| Site {
                        base_dir: base_dir.clone(),
                        kind,
                        span,
                    })
                    .collect();
                // The map is fixed the moment it is first consulted. A
                // declaration found after that is one the map's own answer led
                // to, which is the self-dependence § 1 exists to stop — so the
                // rule reaches the whole autoloaded sub-graph, not only the
                // autoloaded file itself.
                if autoload_map.is_some() {
                    autoload::reject_declarations(&file_sites, diags);
                } else {
                    sites.extend(file_sites);
                }

                // Canonicalized once per file rather than once per `require`, so
                // `check_path_case` can line its simulated walk up positionally
                // against each target's own canonical path. An entry file named
                // on the command line as `tests/main.nvs` has a relative,
                // as-typed parent; every other file in the graph was already
                // loaded by its canonical path.
                let canonical_base = canonicalize(&base_dir);
                for (segments, span) in targets {
                    // Folded one target at a time rather than all of them
                    // before the loop: a constant declared in the file a
                    // previous target loaded is in the table by now. A
                    // constant nothing walked so far declares makes the path
                    // not statically known, which the runtime fallback owns.
                    let Some(literal) = resolve_segments(&segments, &consts) else {
                        continue;
                    };
                    let target = base_dir.join(&literal);
                    let Some(canonical) = canonicalize(&target) else {
                        map.note_missing(&target);
                        diags.report(
                            Diagnostic::error(
                                code::E_REQUIRE_TARGET_NOT_FOUND,
                                format!("`{}` cannot be loaded", target.display()),
                            )
                            .with_primary(span, "no file found at this path"),
                        );
                        continue;
                    };
                    if let Some(base) = &canonical_base {
                        check_path_case(base, &literal, &canonical, span, diags);
                    }
                    if chain.contains(&canonical) {
                        let mut names: Vec<String> =
                            chain.iter().map(|p| p.display().to_string()).collect();
                        names.push(canonical.display().to_string());
                        diags.report(
                            Diagnostic::error(
                                code::E_CIRCULAR_REQUIRE,
                                format!("circular require: {}", names.join(" -> ")),
                            )
                            .with_primary(span, "part of this cycle"),
                        );
                        continue;
                    }
                    if done.contains(&canonical) {
                        // Loaded once, named twice: the second site still owns
                        // an edge to it, because it is still a `require` that
                        // runs that file's frame.
                        if let Some(&already) = by_path.get(&canonical) {
                            file_requires.push((span, already));
                        }
                        continue;
                    }
                    done.insert(canonical.clone());
                    let Ok(new_id) = map.load(&canonical) else {
                        diags.report(
                            Diagnostic::error(
                                code::E_REQUIRE_TARGET_NOT_FOUND,
                                format!("`{}` cannot be loaded", canonical.display()),
                            )
                            .with_primary(span, "not valid UTF-8, or too large to load"),
                        );
                        continue;
                    };
                    by_path.insert(canonical.clone(), new_id);
                    file_requires.push((span, new_id));
                    let new_stmts = parse_file(map.file(new_id), diags);
                    check_declarations(&new_stmts, map.file(new_id), diags);
                    // Read here rather than when this file is popped off
                    // `work`, so the next target of the file being walked can
                    // name a constant this one just loaded.
                    collect_consts(&new_stmts, map.file(new_id), &mut consts);
                    let mut new_chain = chain.clone();
                    new_chain.push(canonical);
                    work.push((new_id, new_stmts, new_chain));
                }
            }

            loaded.push(Loaded {
                id,
                stmts,
                requires: file_requires,
            });
        }

        // Every file the `require` chain can reach is collected, so the map is
        // complete and this is the first moment it can be consulted.
        let built = autoload_map
            .get_or_insert_with(|| AutoloadMap::build_borrowing(&sites, borrowed, diags));
        if built.is_empty() {
            break;
        }

        // § 3's scan: for a program that asked for it — `implementing<T>()`
        // or an `rule:routing/table-is-opt-in` router link — every
        // name the roots declare becomes a file to load, whether or not
        // anything mentions it — the one place resolution is not lazy. It
        // runs once, after the `require` chain has drained, because that is
        // when the map is complete; the files it queues are drained by the
        // loop above on the next turn, and any `autoload` they write is
        // refused the same way an autoloaded file's is.
        //
        // Each file arrives under the name `AutoloadMap::resolve` would have
        // found it by, so `check_file_shape` holds a scanned file to exactly
        // the rule a probed one already answers to. The scan records every
        // directory it listed, for the reason the resolve below records what it
        // probed: a module added to one of them changes the program.
        if let Some(site) = scan.take() {
            for (name, path) in built.enumerate_recording() {
                if !done.insert(path.clone()) {
                    continue;
                }
                let Ok(new_id) = map.load(&path) else {
                    continue;
                };
                by_path.insert(path.clone(), new_id);
                let new_stmts = parse_file(map.file(new_id), diags);
                check_declarations(&new_stmts, map.file(new_id), diags);
                autoload::check_file_shape(&new_stmts, map.file(new_id), &name, site, diags);
                work.push((new_id, new_stmts, vec![path]));
            }
            continue;
        }

        let mut next: Option<(QName, Span, PathBuf)> = None;
        while let Some((name, span)) = wanted.pop() {
            if resolver.module().symbols.contains(&name) || !probed.insert(name.clone()) {
                continue;
            }
            // `rule:packaging/autoload-probes-fold-into-the-cache-key` keys a unit on the whole probe trace,
            // misses included, so that adding a file which *shadows* one already
            // resolved invalidates it. Resolving through `resolve_recording`
            // rather than `resolve` is what keeps that trace: it rides back out
            // inside the `AutoloadMap`, which is where the key reads it from.
            if let Some(path) = built.resolve_recording(&name).hit {
                next = Some((name, span, path));
                break;
            }
        }
        let Some((name, span, path)) = next else {
            break;
        };
        if !done.insert(path.clone()) {
            continue;
        }
        // An entry that canonicalized but will not load is not valid UTF-8; the
        // name then stays undeclared and the checker reports it as any other
        // unknown class, which is a better place to say so than here.
        let Ok(new_id) = map.load(&path) else {
            continue;
        };
        // An autoloaded file is reachable by a written `require` too, and such
        // a site owns the same edge a first-loading one does.
        by_path.insert(path.clone(), new_id);
        let new_stmts = parse_file(map.file(new_id), diags);
        check_declarations(&new_stmts, map.file(new_id), diags);
        autoload::check_file_shape(&new_stmts, map.file(new_id), &name, span, diags);
        work.push((new_id, new_stmts, vec![path]));
    }

    resolver.resolve_imports(diags);
    let mut module = resolver.into_module();
    module.graph = hierarchy.resolve(&module.symbols, diags);

    for file in &loaded {
        members.check(
            &file.stmts,
            map.file(file.id),
            &module.symbols,
            &module.graph,
            strict_docs,
            diags,
        );
    }
    module.members = members.into_table();
    module.aliases = aliases.resolve(diags);

    (module, loaded, autoload_map.unwrap_or_default())
}

fn canonical_path(src: &SourceFile) -> Option<PathBuf> {
    src.path().and_then(canonicalize)
}

/// The canonical form of a path the graph walk is about to key a file on, and
/// the one [`crate::autoload`] probes with for the same reason.
///
/// One function rather than a bare `Path::canonicalize` because a bundled
/// executable has no filesystem to canonicalize against: its payload *is* the
/// closed world (`rule:packaging/a-bundle-is-found-by-its-footer-before-argv-is-read`
/// ), so `nvs_diagnostics::embedded` answers first and a path it does not
/// carry is exactly as unloadable as a missing file — which is § 3's rule, and
/// it arrives here as the same `E_REQUIRE_TARGET_NOT_FOUND` an ordinary run
/// would report. Outside a bundle the table is empty and this is the plain
/// syscall.
pub(crate) fn canonicalize(path: &Path) -> Option<PathBuf> {
    if nvs_diagnostics::embedded::is_active() {
        return nvs_diagnostics::embedded::canonicalize(path);
    }
    nvs_footprint::exists(path);
    path.canonicalize().ok()
}

/// Reports a `require` whose literal path resolved only because the
/// filesystem folds case —
/// [ADR 0062](/docs/decisions/0062.md)
/// § 3, extending `rule:programs/autoload`'s exact-name rule from `autoload` to `require`.
///
/// The comparison is free of extra syscalls: `canonicalize` on Windows and
/// macOS already hands back the entry's true on-disk spelling, so this is a
/// component-wise string compare of a path both sides already hold. On a
/// case-sensitive filesystem it can only ever pass — a mis-cased path failed
/// to resolve at all and was reported as `E_REQUIRE_TARGET_NOT_FOUND` one
/// branch earlier.
///
/// `base` must be canonical. The literal's components are replayed onto it —
/// `.` skipped, `..` popped — which reproduces exactly what `canonicalize`
/// did, *unless* a symlink was crossed or the literal was absolute. Both show
/// up as a length or shape mismatch against `canonical`, and both mean the
/// positional alignment this relies on is gone, so the check is skipped
/// rather than guessed at: this diagnostic only ever fires on a difference
/// that is purely one of case.
fn check_path_case(
    base: &Path,
    literal: &str,
    canonical: &Path,
    span: Span,
    diags: &mut Diagnostics,
) {
    if Path::new(literal).is_absolute() {
        return;
    }
    // `true` marks a component the literal wrote, and so the only kind this
    // may report on: `base`'s own components are canonical already, and a
    // mismatch there would be about how the *entry* file was named.
    let mut walked: Vec<(String, bool)> = base
        .components()
        .map(|c| (c.as_os_str().to_string_lossy().into_owned(), false))
        .collect();
    for component in Path::new(literal).components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if walked.pop().is_none() {
                    return;
                }
            }
            std::path::Component::Normal(name) => {
                walked.push((name.to_string_lossy().into_owned(), true));
            }
            std::path::Component::Prefix(_) | std::path::Component::RootDir => return,
        }
    }

    let actual: Vec<String> = canonical
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if actual.len() != walked.len() {
        return;
    }
    for ((written, from_literal), on_disk) in walked.iter().zip(&actual) {
        if written == on_disk {
            continue;
        }
        if !*from_literal || !written.eq_ignore_ascii_case(on_disk) {
            return;
        }
        diags.report(
            Diagnostic::error(
                code::E_REQUIRE_PATH_CASE_MISMATCH,
                format!("`{written}` is spelled `{on_disk}` on disk"),
            )
            .with_primary(
                span,
                "this path resolves only on a case-insensitive filesystem",
            ),
        );
        return;
    }
}

/// Everything one file's walk yields. The lists answer the questions the graph
/// walk asks of a file: what does it pull in, what does it say about where
/// *other* names live, which names does it use that something will have to
/// declare, and does it ask for the scan.
///
/// The namespace and imports are carried here rather than threaded as a
/// parameter because a `namespace`/`use` declaration only ever appears in a
/// file's own top-level statement sequence: every nested walk below inherits
/// what the enclosing sequence set and can never change it, so a field the
/// sequence writes as it goes is the same thing a parameter would be, minus a
/// parameter on every walk function's signature.
#[derive(Default)]
struct Harvest {
    /// Each statically-known `require` target's path, as the segments it is
    /// built out of, and its span. Segments rather than one cooked string
    /// because a constant in the path cannot be answered where it is read —
    /// see [`Segment`].
    requires: Vec<(Vec<Segment>, Span)>,
    /// Each `autoload` declaration, cooked, still needing the declaring
    /// file's own directory bound to it — [`autoload::Site`]'s `base_dir`.
    autoloads: Vec<(autoload::SiteKind, Span)>,
    /// Every name used where a class, interface, enum or `type` alias is
    /// meant, already resolved through the namespace and imports in force
    /// where it was written.
    ///
    /// Deliberately an over-approximation: a name nothing declares and no
    /// autoload prefix matches costs one failed map lookup and no
    /// diagnostic, while a name this walk *misses* is a program that does
    /// not compile. `Core`'s own names are dropped, since nothing on disk
    /// declares them.
    names: Vec<(QName, Span)>,
    /// Every call site written in this file that asks for § 3's scan — see
    /// [`is_program_scan`] for which those are. `rule:programs/implementing`'s opt-in lives
    /// here rather than in the checker
    /// because the scan has to happen while the walk can still load files —
    /// by the time `nvs-types` reaches the call, the graph it expands
    /// against is already closed.
    scans: Vec<Span>,
    /// The namespace in force at the statement being walked.
    namespace: Vec<String>,
    /// The `use` imports in force at the statement being walked, keyed by
    /// the short name each binds.
    imports: FxHashMap<String, QName>,
}

/// Records one written name against the namespace and imports in force,
/// exactly as [`crate::hierarchy::resolve_ref`] resolves an
/// `extends`/`implements` reference — the same answer, reached the same way,
/// so an autoload lookup can never disagree with the resolver that will
/// later look the name up in the symbol table.
fn record_name(name: &Name, src: &SourceFile, out: &mut Harvest) {
    let Some(text) = src.span_text(name.span) else {
        return;
    };
    let qname = crate::hierarchy::resolve_ref(text, &out.namespace, &out.imports);
    if qname.is_core() {
        return;
    }
    out.names.push((qname, name.span));
}

/// Records the class a string literal under `as class<T>` or `as ?class<T>`
/// names, so the literal loads its class while compiling the way `X::class`
/// does (`rule:types/class-reference`). The text is the class's whole name
/// ([`QName::from_literal`]): the namespace and imports in force do not apply.
///
/// Only a literal written as the operand itself. A class constant, a
/// concatenation and a variable are values built at run time, and nothing is
/// loaded for them (`rule:programs/no-runtime-autoload`).
fn record_written_class_name(operand: &Expr, ty: &Type, src: &SourceFile, out: &mut Harvest) {
    let target = match &ty.kind {
        TypeKind::Nullable(inner) => &inner.kind,
        kind => kind,
    };
    if !matches!(target, TypeKind::Atom(TypeAtom::ClassRef(_))) {
        return;
    }
    let ExprKind::Str(span) = operand.unparenthesized().kind else {
        return;
    };
    let text = nvs_syntax::string_lit::cook_string_literal(src, span);
    if let Some(qname) = QName::from_literal(&text).filter(|qname| !qname.is_core()) {
        out.names.push((qname, span));
    }
}

/// `rule:programs/implementing`'s enumeration, spelled out: the calls whose appearance
/// anywhere in the program turn every autoload root's whole tree into files
/// to load.
///
/// Three classes ask for the same scan. `Core\Program`'s enumeration members
/// are § 3's own query. `Core\Router`'s and `Core\Request`'s route readers
/// need the compile-time route table, which `rule:routing/table-is-opt-in`
/// builds by filtering *this* enumeration by a `#[Core\Route]` attribute
/// rather than by an implemented interface. That is why all three are member
/// lists here and not separate walks: a program calling any of them pays § 5's
/// directory-listing dependency once, and a program calling none of them
/// performs no scan.
const PROGRAM_CLASS: &str = r"Core\Program";
/// The `Core\Program` members that expand to the enumeration, and so need
/// every class the autoload roots declare. Each one alone opts the program
/// in: a program that calls only `implementingWith` or only `constructors`
/// lists the same classes `implementing` would.
const PROGRAM_SCAN_MEMBERS: &[&str] = &["implementing", "implementingWith", "constructors"];
const ROUTER_CLASS: &str = r"Core\Router";
/// The `Core\Router` members whose answer comes from the route table, and so
/// need the scan that builds it: the three link builders resolve a route name
/// against it, `match` and `methodsFor` walk it, and `signedRoute` verifies
/// against the match the door took from it. With no table that match is
/// always absent and every signed link is refused.
const ROUTER_SCAN_MEMBERS: &[&str] = &[
    "url",
    "urlAbsolute",
    "urlSigned",
    "match",
    "methodsFor",
    "signedRoute",
];
const REQUEST_CLASS: &str = r"Core\Request";
/// The `Core\Request` members whose answer comes from the route table. `route`
/// returns the match the door took from it, and is the only one: `mount`
/// comes from the server's mount table, and every `Core\Router\Match` reader
/// is an instance method reached through `route`, `match` or `signedRoute`.
const REQUEST_SCAN_MEMBERS: &[&str] = &["route"];

/// Whether this static call opts a program into the scan: a
/// [`PROGRAM_SCAN_MEMBERS`], [`ROUTER_SCAN_MEMBERS`] or
/// [`REQUEST_SCAN_MEMBERS`] call.
///
/// Matched nominally against the *resolved* class name, so a `use Core;` plus
/// `Program::implementing<Module>()` is the same call as the fully written
/// one and a userland class named `Program` is not any of them. The member
/// name is compared as written: a computed `::{$m}()` is not this call, and
/// treating it as one would mean scanning the tree for a program that may
/// never make it.
fn is_program_scan(class: &Expr, method: &MemberName, src: &SourceFile, out: &Harvest) -> bool {
    let ExprKind::ConstFetch(name) = &class.kind else {
        return false;
    };
    let MemberName::Ident(member) = method else {
        return false;
    };
    let (Some(class_text), Some(member_text)) = (src.span_text(name.span), src.span_text(*member))
    else {
        return false;
    };
    let class_name = crate::hierarchy::resolve_ref(class_text, &out.namespace, &out.imports);
    let members = if class_name == QName::parse(PROGRAM_CLASS) {
        PROGRAM_SCAN_MEMBERS
    } else if class_name == QName::parse(ROUTER_CLASS) {
        ROUTER_SCAN_MEMBERS
    } else if class_name == QName::parse(REQUEST_CLASS) {
        REQUEST_SCAN_MEMBERS
    } else {
        return false;
    };
    members.contains(&member_text)
}

/// Records every name a type expression mentions. A shape type's fields, a
/// union's members and an `array<T>`'s argument all nest types, and any of
/// them may be the one class reference that pulls a file in.
fn walk_type(ty: &Type, src: &SourceFile, out: &mut Harvest) {
    match &ty.kind {
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => walk_type(inner, src, out),
        TypeKind::Union(members) | TypeKind::Intersection(members) => {
            for member in members {
                walk_type(member, src, out);
            }
        }
        TypeKind::Atom(atom) => match atom {
            TypeAtom::Array(Some(inner))
            | TypeAtom::ClassRef(inner)
            | TypeAtom::PropertyKey(inner) => {
                walk_type(inner, src, out);
            }
            TypeAtom::Shape(fields) => {
                for field in fields {
                    walk_type(&field.ty, src, out);
                }
            }
            // `rule:types/callable-signature`: every position inside a written
            // signature is an ordinary type position, so a class named only
            // there — `callable(User): Row` — is a reference that pulls its
            // file in exactly as a shape field's is.
            TypeAtom::CallableSig { params, ret } => {
                for param in params {
                    walk_type(param, src, out);
                }
                walk_type(ret, src, out);
            }
            TypeAtom::Member(name, _) => record_name(name, src, out),
            TypeAtom::Name(name, args) => {
                record_name(name, src, out);
                for arg in args {
                    walk_type(arg, src, out);
                }
            }
            // Every remaining atom is a keyword or a literal: it nests no
            // type and spells no name, so there is nothing under it to reach.
            TypeAtom::Null
            | TypeAtom::Bool
            | TypeAtom::Int
            | TypeAtom::Uint
            | TypeAtom::Float
            | TypeAtom::Decimal
            | TypeAtom::String
            | TypeAtom::Bytes
            | TypeAtom::TaintedString
            | TypeAtom::TaintedBytes
            | TypeAtom::SecretString
            | TypeAtom::SecretBytes
            | TypeAtom::SecretTaintedString
            | TypeAtom::SecretTaintedBytes
            | TypeAtom::Array(None)
            | TypeAtom::Object
            | TypeAtom::Mixed
            | TypeAtom::Void
            | TypeAtom::Never
            | TypeAtom::True
            | TypeAtom::False
            | TypeAtom::SingleValueString(_)
            | TypeAtom::SingleValueInt(_)
            | TypeAtom::Iterable
            | TypeAtom::Callable
            | TypeAtom::SelfTy
            | TypeAtom::StaticTy
            | TypeAtom::Parent => {}
            // `TypeAtom` is `#[non_exhaustive]`, so this arm is what the
            // compiler requires of a cross-crate `match`, not an atom the
            // walk declines: one added in `nvs-syntax` lands here until it is
            // named above.
            _ => {}
        },
        // Every `TypeKind` is named above; this is the same `#[non_exhaustive]`
        // arm.
        _ => {}
    }
}

fn walk_types(types: &[Type], src: &SourceFile, out: &mut Harvest) {
    for ty in types {
        walk_type(ty, src, out);
    }
}

fn record_names(names: &[Name], src: &SourceFile, out: &mut Harvest) {
    for name in names {
        record_name(name, src, out);
    }
}

fn record_implements(clauses: &[ImplementsClause], src: &SourceFile, out: &mut Harvest) {
    for clause in clauses {
        record_name(&clause.name, src, out);
        walk_types(&clause.type_args, src, out);
    }
}

/// Walks `stmts` looking for every `require` expression whose path is
/// statically known, appending each one's cooked path text and span to
/// `out`. A `require` whose path is anything else (a variable, an
/// interpolated string) is left out entirely — that is the
/// dynamic-fallback case this module does not touch.
///
/// The same walk harvests `rule:programs/no-runtime-autoload`'s other inputs — every `autoload`
/// declaration, and every name that might need one — because they are found
/// in the same places by the same recursion, and a second walker over the
/// whole AST would be a second walker to keep in step with the first.
fn find_written_requires(stmts: &[Stmt], src: &SourceFile, out: &mut Harvest) {
    for stmt in stmts {
        walk_stmt(stmt, src, out);
    }
}

fn walk_stmt(stmt: &Stmt, src: &SourceFile, out: &mut Harvest) {
    macro_rules! e {
        ($expr:expr) => {
            walk_expr($expr, src, out)
        };
    }
    macro_rules! s {
        ($stmt:expr) => {
            walk_stmt($stmt, src, out)
        };
    }

    match &stmt.kind {
        StmtKind::Expr(x) => e!(x),
        StmtKind::Return(Some(x)) | StmtKind::Break(Some(x)) | StmtKind::Continue(Some(x)) => {
            e!(x);
        }
        StmtKind::Block(b) => find_written_requires(&b.stmts, src, out),
        StmtKind::If { arms, else_ } => {
            for arm in arms {
                e!(&arm.cond);
                s!(&arm.then);
            }
            if let Some(else_) = else_ {
                s!(else_);
            }
        }
        StmtKind::While { cond, body } => {
            e!(cond);
            s!(body);
        }
        StmtKind::DoWhile { body, cond } => {
            s!(body);
            e!(cond);
        }
        StmtKind::For {
            init,
            cond,
            step,
            body,
        } => {
            // `rule:iteration/for-init-clause`: an init clause may be a declaration, whose type
            // and initializer both need harvesting.
            if let Some(decl) = init.decl() {
                s!(decl);
            }
            for x in init.exprs().iter().chain(cond).chain(step) {
                e!(x);
            }
            s!(body);
        }
        StmtKind::Foreach {
            subject,
            key,
            value,
            body,
            ..
        } => {
            e!(subject);
            for binding in key.iter().chain(std::iter::once(value)) {
                if let Some(ty) = binding.written_ty() {
                    walk_type(ty, src, out);
                }
            }
            s!(body);
        }
        StmtKind::Switch { subject, cases } => {
            e!(subject);
            for case in cases {
                if let Some(cond) = &case.cond {
                    e!(cond);
                }
                find_written_requires(&case.body, src, out);
            }
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            find_written_requires(&body.stmts, src, out);
            for catch in catches {
                walk_type(&catch.ty, src, out);
                find_written_requires(&catch.body.stmts, src, out);
            }
            if let Some(finally) = finally {
                find_written_requires(&finally.stmts, src, out);
            }
        }
        StmtKind::Echo(xs) | StmtKind::Unset(xs) => {
            for x in xs {
                e!(x);
            }
        }
        StmtKind::LocalDecl { ty, value, .. } => {
            if let Some(ty) = ty {
                walk_type(ty, src, out);
            }
            if let Some(value) = value {
                e!(value);
            }
        }
        StmtKind::Destructure { target, value } => {
            walk_destructure_target(target, src, out);
            e!(value);
        }
        StmtKind::StaticLocal { vars, .. } => {
            for var in vars {
                if let Some(default) = &var.default {
                    e!(default);
                }
            }
        }
        StmtKind::ClassDecl(decl) => {
            walk_attributes(&decl.attributes, src, out);
            if let Some(extends) = &decl.extends {
                record_name(extends, src, out);
            }
            record_implements(&decl.implements, src, out);
            walk_class_members(&decl.members, src, out);
        }
        StmtKind::InterfaceDecl(decl) => {
            walk_attributes(&decl.attributes, src, out);
            record_names(&decl.extends, src, out);
            walk_class_members(&decl.members, src, out);
        }
        StmtKind::EnumDecl(decl) => {
            walk_attributes(&decl.attributes, src, out);
            record_names(&decl.implements, src, out);
            for case in &decl.cases {
                walk_attributes(&case.attributes, src, out);
                if let Some(value) = &case.value {
                    e!(value);
                }
            }
            walk_class_members(&decl.members, src, out);
        }
        StmtKind::TypeAliasDecl(decl) => walk_type(&decl.ty, src, out),
        StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
            let outer_ns = std::mem::take(&mut out.namespace);
            let outer_imports = std::mem::take(&mut out.imports);
            out.namespace = name
                .as_ref()
                .and_then(|n| src.span_text(n.span))
                .map(|text| QName::parse(text).segments().to_vec())
                .unwrap_or_default();
            // A `namespace Name { ... }` block scopes its own imports and
            // hands the enclosing sequence back what it had; a bare
            // `namespace Name;` changes both for the rest of the file, which
            // is what *not* restoring them does.
            if let Some(block) = body {
                find_written_requires(&block.stmts, src, out);
                out.namespace = outer_ns;
                out.imports = outer_imports;
            }
        }
        StmtKind::UseDecl(use_decl) => {
            let Some(text) = src.span_text(use_decl.path.span) else {
                return;
            };
            let target = QName::parse(text);
            out.imports
                .insert(target.short_name().to_owned(), target.clone());
            if !target.is_core() {
                out.names.push((target, use_decl.path.span));
            }
        }
        StmtKind::AutoloadDecl(decl) => record_autoload(decl, src, out),
        // `rule:classes/no-free-functions-or-constants` refuses both of these
        // and the parser keeps what it refused, so a name written inside one
        // is still a name this file uses and is harvested exactly as the
        // class-body form is.
        StmtKind::TopLevelFunction(method) => walk_method(method, src, out),
        StmtKind::TopLevelConst(constants) => {
            for constant in constants {
                walk_const(constant, src, out);
            }
        }
        // Nothing under these spells a name: a bare `;`, the literal text
        // between `?>` and `<?nvs`, `global`'s and `goto`'s bare identifiers,
        // a valueless `return`/`break`/`continue`, and a recovery placeholder.
        StmtKind::Empty
        | StmtKind::InlineHtml(_)
        | StmtKind::Global(_)
        | StmtKind::Goto(_)
        | StmtKind::Return(None)
        | StmtKind::Break(None)
        | StmtKind::Continue(None)
        | StmtKind::Error => {}
        // `StmtKind` is `#[non_exhaustive]`, so this arm is what the compiler
        // requires of a cross-crate `match`, not a statement the walk
        // declines.
        _ => {}
    }
}

/// Cooks one `autoload` declaration's spans into an [`autoload::SiteKind`].
///
/// The parser records spans rather than values — `ExprKind::Str`'s own shape
/// — so this is where a prefix and its roots become strings, through the same
/// [`cook_quoted`] a `require` path goes through. A literal that will not
/// cook (a heredoc) is dropped: the parser already reported
/// `E_AUTOLOAD_PATH_NOT_WRITTEN_DIRECTLY` for every path that is not a plain string.
fn record_autoload(decl: &AutoloadDecl, src: &SourceFile, out: &mut Harvest) {
    let cook = |span: Span| cook_quoted(src, span);
    let kind = match &decl.kind {
        AutoloadKind::Prefix {
            prefix: literal,
            roots,
        } => {
            let Some(prefix) = cook(*literal) else {
                return;
            };
            let roots: Vec<(String, Span)> = roots
                .iter()
                .filter_map(|literal| Some((cook(*literal)?, *literal)))
                .collect();
            if roots.is_empty() {
                return;
            }
            autoload::SiteKind::Prefix {
                prefix,
                literal: *literal,
                roots,
            }
        }
        AutoloadKind::Discover { glob: literal } => {
            let Some(glob) = cook(*literal) else {
                return;
            };
            autoload::SiteKind::Discover {
                glob,
                literal: *literal,
            }
        }
    };
    out.autoloads.push((kind, decl.span));
}

/// Records every name a method declaration mentions: its attribute groups,
/// its parameters, its return type and its body.
///
/// A rejected top-level `function` is this same [`MethodMember`], so the
/// declaration inside a class body and the one at file scope are harvested by
/// one walk rather than by two that can drift apart.
fn walk_method(method: &MethodMember, src: &SourceFile, out: &mut Harvest) {
    walk_attributes(&method.attributes, src, out);
    walk_params(&method.params, src, out);
    if let Some(ty) = &method.return_type {
        walk_type(ty, src, out);
    }
    if let Some(body) = &method.body {
        find_written_requires(&body.stmts, src, out);
    }
}

/// Records every name a constant declaration mentions: its attribute groups,
/// its written type and its value. A rejected top-level `const` is this same
/// [`ConstMember`], for the reason [`walk_method`] gives.
fn walk_const(constant: &ConstMember, src: &SourceFile, out: &mut Harvest) {
    walk_attributes(&constant.attributes, src, out);
    if let Some(ty) = &constant.ty {
        walk_type(ty, src, out);
    }
    walk_expr(&constant.value, src, out);
}

fn walk_class_members(members: &[ClassMember], src: &SourceFile, out: &mut Harvest) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Method(m) => walk_method(m, src, out),
            ClassMemberKind::Const(c) => walk_const(c, src, out),
            ClassMemberKind::Property(p) => {
                walk_attributes(&p.attributes, src, out);
                walk_type(&p.ty, src, out);
                if let Some(default) = &p.default {
                    walk_expr(default, src, out);
                }
                for hook in p.hooks.iter().flatten() {
                    walk_property_hook(hook, src, out);
                }
            }
            // A recovery placeholder stands in for a member that was refused
            // or never written, and carries nothing to walk.
            ClassMemberKind::Error => {}
            // `ClassMemberKind` is `#[non_exhaustive]`, so this arm is what
            // the compiler requires of a cross-crate `match`, not a member
            // the walk declines.
            _ => {}
        }
    }
}

fn walk_block(block: &Block, src: &SourceFile, out: &mut Harvest) {
    find_written_requires(&block.stmts, src, out);
}

fn walk_params(params: &[Param], src: &SourceFile, out: &mut Harvest) {
    for param in params {
        walk_attributes(&param.attributes, src, out);
        if let Some(ty) = &param.ty {
            walk_type(ty, src, out);
        }
        if let Some(default) = &param.default {
            walk_expr(default, src, out);
        }
    }
}

fn walk_property_hook(hook: &PropertyHook, src: &SourceFile, out: &mut Harvest) {
    walk_attributes(&hook.attributes, src, out);
    if let Some(param) = &hook.param {
        walk_params(std::slice::from_ref(param), src, out);
    }
    match &hook.body {
        Some(PropertyHookBody::Expr(expr)) => walk_expr(expr, src, out),
        Some(PropertyHookBody::Block(block)) => walk_block(block, src, out),
        None => {}
    }
}

fn walk_destructure_target(target: &DestructureTarget, src: &SourceFile, out: &mut Harvest) {
    for element in &target.elements {
        match element {
            // A leaf's declared type is an ordinary type position, and it is
            // the only place `[Framework\Row $row] = $pair;` writes `Row` —
            // so both halves of a leaf are walked, not just its key.
            DestructureElement::Leaf { key, ty, .. } => {
                if let Some(key) = key {
                    walk_expr(key, src, out);
                }
                if let Some(ty) = ty {
                    walk_type(ty, src, out);
                }
            }
            DestructureElement::Nested { key, target, .. } => {
                if let Some(key) = key {
                    walk_expr(key, src, out);
                }
                walk_destructure_target(target, src, out);
            }
            // An empty slot binds nothing.
            DestructureElement::Skip => {}
            // `DestructureElement` is `#[non_exhaustive]`, so this arm is what
            // the compiler requires of a cross-crate `match`, not an element
            // the walk declines.
            _ => {}
        }
    }
}

fn walk_member_name(member: &MemberName, src: &SourceFile, out: &mut Harvest) {
    match member {
        MemberName::Variable(e) | MemberName::Expr(e) => walk_expr(e, src, out),
        // A written identifier and one the parser invented in its place are
        // both plain member names, and neither can name a class.
        MemberName::Ident(_) | MemberName::Missing(_) => {}
        // `MemberName` is `#[non_exhaustive]`, so this arm is what the
        // compiler requires of a cross-crate `match`, not a spelling the walk
        // declines.
        _ => {}
    }
}

/// Records every name an `#[...]` group mentions: the attribute class itself,
/// and whatever its argument expressions name.
///
/// An attribute is a class reference like any other — `rule:programs/autoload` places it
/// by prefix the same way — but it is the one such reference that reaches no
/// type position and no expression, so without this the file declaring
/// `#[Route(...)]`'s `Route` is never pulled in. Every declaration site that
/// carries a `Vec<AttributeGroup>` is fed through here by the walk that
/// already visits it.
fn walk_attributes(groups: &[AttributeGroup], src: &SourceFile, out: &mut Harvest) {
    for group in groups {
        for attr in &group.attributes {
            if let Some(name) = &attr.name {
                record_name(name, src, out);
            }
            for field in &attr.fields {
                walk_expr(&field.value, src, out);
            }
        }
    }
}

fn walk_args(args: &CallArgs, src: &SourceFile, out: &mut Harvest) {
    let CallArgs::List(list) = args else {
        return;
    };
    for Arg { value, .. } in list {
        walk_expr(value, src, out);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one match arm per AST expression variant, each a couple of lines \
              (mirrors crate::members's own walker)"
)]
fn walk_expr(expr: &Expr, src: &SourceFile, out: &mut Harvest) {
    macro_rules! e {
        ($expr:expr) => {
            walk_expr($expr, src, out)
        };
    }

    match &expr.kind {
        ExprKind::Require { path } => {
            if let Some(segments) = require_path_segments(path, src, &out.namespace, &out.imports) {
                out.requires.push((segments, path.span));
            }
            // A dynamic path may still nest its own sub-expressions worth
            // walking for a further, statically-resolvable `require` inside
            // them (e.g. a ternary choosing between two literal paths, one
            // arm still worth statically pulling in) — that generality isn't
            // needed yet, so this stops at the top-level path expression.
        }
        // `rule:core-classes/html-template`'s html template carries the same
        // parts an interpolated string does, and a hole in either is an
        // ordinary expression.
        ExprKind::Interpolated(parts) | ExprKind::HtmlTemplate(parts) => {
            for part in parts {
                if let StringPart::Expr(x) = part {
                    e!(x);
                }
            }
        }
        ExprKind::ArrayLiteral(items) => {
            for ArrayItem { key, value, .. } in items {
                if let Some(key) = key {
                    e!(key);
                }
                e!(value);
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::PreIncDec { expr, .. }
        | ExprKind::PostIncDec { expr, .. }
        | ExprKind::Clone(expr)
        | ExprKind::YieldFrom(expr)
        | ExprKind::Print(expr)
        | ExprKind::Throw(expr)
        | ExprKind::Empty(expr)
        | ExprKind::Paren(expr) => e!(expr),
        ExprKind::Binary { lhs, rhs, .. } => {
            e!(lhs);
            e!(rhs);
        }
        ExprKind::Assign { target, value, .. } => {
            e!(target);
            e!(value);
        }
        ExprKind::Ternary { cond, then, else_ } => {
            e!(cond);
            if let Some(then) = then {
                e!(then);
            }
            e!(else_);
        }
        ExprKind::Conversion { expr, ty, .. } => {
            record_written_class_name(expr, ty, src, out);
            e!(expr);
            walk_type(ty, src, out);
        }
        ExprKind::TypeTest { expr, against } => {
            e!(expr);
            match against {
                TestOperand::Type(ty) => walk_type(ty, src, out),
                TestOperand::Value(operand) => e!(operand),
            }
        }
        ExprKind::ConstFetch(name) => record_name(name, src, out),
        ExprKind::Call { callee, args } => {
            e!(callee);
            walk_args(args, src, out);
        }
        ExprKind::MethodCall {
            object,
            method,
            type_args,
            args,
            ..
        } => {
            e!(object);
            walk_member_name(method, src, out);
            walk_types(type_args, src, out);
            walk_args(args, src, out);
        }
        ExprKind::StaticCall {
            class,
            method,
            type_args,
            args,
        } => {
            if is_program_scan(class, method, src, out) {
                out.scans.push(expr.span);
            }
            e!(class);
            walk_member_name(method, src, out);
            walk_types(type_args, src, out);
            walk_args(args, src, out);
        }
        ExprKind::PropertyAccess {
            object, property, ..
        } => {
            e!(object);
            walk_member_name(property, src, out);
        }
        ExprKind::StaticPropertyAccess { class, .. } => e!(class),
        ExprKind::ClassConstAccess { class, .. } => e!(class),
        ExprKind::ClassNameConst { class } => e!(class),
        ExprKind::Index { base, index } => {
            e!(base);
            if let Some(index) = index {
                e!(index);
            }
        }
        ExprKind::New {
            target,
            type_args,
            args,
        } => {
            match target {
                NewTarget::Name(name) => record_name(name, src, out),
                NewTarget::Expr(expr) => e!(expr),
                NewTarget::AnonClass(decl) => {
                    if let Some(extends) = &decl.extends {
                        record_name(extends, src, out);
                    }
                    record_names(&decl.implements, src, out);
                    walk_class_members(&decl.members, src, out);
                }
                // `self`, `static` and `parent` each name the enclosing
                // class, which the file that wrote them already declares.
                NewTarget::SelfTy | NewTarget::StaticTy | NewTarget::ParentTy => {}
                // `NewTarget` is `#[non_exhaustive]`, so this arm is what the
                // compiler requires of a cross-crate `match`, not a target
                // the walk declines.
                _ => {}
            }
            walk_types(type_args, src, out);
            walk_args(args, src, out);
        }
        ExprKind::Fn(fn_expr) => {
            walk_params(&fn_expr.params, src, out);
            if let Some(ty) = &fn_expr.return_type {
                walk_type(ty, src, out);
            }
            match &fn_expr.body {
                FnBody::Block(block) => walk_block(block, src, out),
                FnBody::Expr(body) => e!(body),
            }
        }
        ExprKind::Catch { guarded, arms } => {
            e!(guarded);
            for arm in arms {
                walk_type(&arm.ty, src, out);
                e!(&arm.body);
            }
        }
        ExprKind::Match { subject, arms } => {
            e!(subject);
            for arm in arms {
                if let Some(conds) = &arm.conditions {
                    for cond in conds {
                        e!(cond);
                    }
                }
                e!(&arm.body);
            }
        }
        ExprKind::Yield { key, value } => {
            if let Some(key) = key {
                e!(key);
            }
            if let Some(value) = value {
                e!(value);
            }
        }
        ExprKind::Exit(Some(x)) => e!(x),
        ExprKind::Isset(xs) => {
            for x in xs {
                e!(x);
            }
        }
        ExprKind::SpawnScript { path, options } => {
            e!(path);
            for option in options {
                e!(&option.value);
            }
        }
        ExprKind::Await(inner) => e!(inner),
        ExprKind::AnonObject(fields) => {
            for field in fields {
                e!(&field.value);
            }
        }
        // A scalar literal, a variable, the three class keywords, a valueless
        // `exit` and a recovery placeholder each name nothing.
        ExprKind::Null
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Duration(_)
        | ExprKind::Str(_)
        | ExprKind::Variable(_)
        | ExprKind::SelfExpr
        | ExprKind::StaticExpr
        | ExprKind::ParentExpr
        | ExprKind::Exit(None)
        | ExprKind::Error(_) => {}
        // `ExprKind` is `#[non_exhaustive]`, so this arm is what the compiler
        // requires of a cross-crate `match`, not an expression the walk
        // declines.
        _ => {}
    }
}

/// One piece of a statically known `require` path: literal text, or the class
/// constant whose value stands there.
///
/// A path is kept as segments rather than as a finished string because the
/// constant half cannot be answered where the path is read: the class
/// declaring it may live in a file a `require` earlier in this same file
/// loads. So the read records what it saw, and [`resolve_segments`] answers it
/// against the [`ConstTable`] as it stands when that path's own target is
/// resolved.
#[derive(Debug)]
enum Segment {
    /// Cooked literal text, escapes and all.
    Text(String),
    /// `Class::CONST`, as the resolved class and the constant's written name.
    Const(QName, String),
}

/// The constants a `require` path may name, keyed by the class declaring them:
/// `Paths::LIB` is `table[Paths]["LIB"]`.
///
/// Only the ones that fold are in here. A constant whose value is not literal
/// text — an `int`, an array, an interpolation, a call — is not a path segment
/// at all, so leaving it out is what makes a missing entry mean "this path is
/// not statically known" and nothing narrower.
type ConstTable = FxHashMap<QName, FxHashMap<String, String>>;

/// Fills `out` with every foldable constant one file declares, in declaration
/// order — so a constant naming one declared above it resolves, and one naming
/// a constant below it does not.
///
/// A pass of its own rather than part of [`find_written_requires`], because it
/// reads declaration positions alone: the `namespace`/`use` sequence a class
/// name resolves through, the class and interface declarations, and their
/// `const` members. That is what lets a whole file's constants be in the table
/// before the first of its `require` paths is folded. An `enum`'s members are
/// not read, because `rule:enums/no-class-machinery` refuses a constant on one.
fn collect_consts(stmts: &[Stmt], src: &SourceFile, out: &mut ConstTable) {
    let mut namespace: Vec<String> = Vec::new();
    let mut imports: FxHashMap<String, QName> = FxHashMap::default();
    collect_consts_in(stmts, src, &mut namespace, &mut imports, out);
}

/// [`collect_consts`] over one statement sequence, carrying the namespace and
/// imports in force through it the way [`Harvest`] carries them for the name
/// walk.
fn collect_consts_in(
    stmts: &[Stmt],
    src: &SourceFile,
    namespace: &mut Vec<String>,
    imports: &mut FxHashMap<String, QName>,
    out: &mut ConstTable,
) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let outer_ns = std::mem::take(namespace);
                let outer_imports = std::mem::take(imports);
                *namespace = name
                    .as_ref()
                    .and_then(|n| src.span_text(n.span))
                    .map(|text| QName::parse(text).segments().to_vec())
                    .unwrap_or_default();
                // A `namespace Name { ... }` block scopes its own imports and
                // hands the enclosing sequence back what it had; a bare
                // `namespace Name;` changes both for the rest of the file,
                // which is what *not* restoring them does.
                if let Some(block) = body {
                    collect_consts_in(&block.stmts, src, namespace, imports, out);
                    *namespace = outer_ns;
                    *imports = outer_imports;
                }
            }
            StmtKind::UseDecl(use_decl) => {
                if let Some(text) = src.span_text(use_decl.path.span) {
                    let target = QName::parse(text);
                    imports.insert(target.short_name().to_owned(), target);
                }
            }
            StmtKind::ClassDecl(decl) => {
                collect_class_consts(&decl.name, &decl.members, src, namespace, imports, out);
            }
            StmtKind::InterfaceDecl(decl) => {
                collect_class_consts(&decl.name, &decl.members, src, namespace, imports, out);
            }
            // Nothing else declares a constant a path can name. A statement
            // that *nests* a declaration is not read either: a class written
            // inside a block is not the shape this folder answers for, and a
            // path naming its constant stays dynamic.
            _ => {}
        }
    }
}

/// Records the foldable constants of one class or interface declaration under
/// the [`QName`] it declares, the same name [`crate::resolve`] enters in the
/// symbol table.
///
/// Each value is folded against `out` as it stands, which is what keeps the
/// declaration order above meaningful and makes a constant naming itself, or a
/// pair naming each other, terminate with no entry rather than recursing.
fn collect_class_consts(
    declared: &Name,
    members: &[ClassMember],
    src: &SourceFile,
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
    out: &mut ConstTable,
) {
    let Some(text) = src.span_text(declared.span) else {
        return;
    };
    let class = QName::join(namespace, text);
    for member in members {
        let ClassMemberKind::Const(constant) = &member.kind else {
            continue;
        };
        let Some(name) = src.span_text(constant.name) else {
            continue;
        };
        let Some(segments) = require_path_segments(&constant.value, src, namespace, imports) else {
            continue;
        };
        let Some(value) = resolve_segments(&segments, out) else {
            continue;
        };
        out.entry(class.clone())
            .or_default()
            .insert(name.to_owned(), value);
    }
}

/// Joins a path's segments into the one path text they denote, or `None` when a
/// constant among them is not in `consts` — the dynamic fallback, which is the
/// same answer a variable in the path gets.
fn resolve_segments(segments: &[Segment], consts: &ConstTable) -> Option<String> {
    let mut path = String::new();
    for segment in segments {
        match segment {
            Segment::Text(text) => path.push_str(text),
            Segment::Const(class, name) => path.push_str(consts.get(class)?.get(name.as_str())?),
        }
    }
    Some(path)
}

/// Reads the segments a `require` path is built out of, if every one of them is
/// statically known — unwrapping any surrounding `(...)` first, so
/// `require ('config.nvs');` resolves the same as `require 'config.nvs';`.
///
/// Three spellings are a segment and nothing else is. A plain `'...'`/`"..."`
/// token is literal text. A `Class::CONST` whose class side is a written name
/// is that constant, resolved through the namespace and imports in force here
/// exactly as [`record_name`] resolves any other written name. And a `.`
/// between two halves that are themselves segments is their concatenation:
/// `require __DIR__` is not the spelling Novis has, so a program building a
/// path out of a prefix writes the prefix as a literal or a constant and
/// concatenates, and folding the operator here is what keeps that program's
/// target statically known rather than a dynamic fallback. A chain of any
/// length resolves, and one dynamic operand anywhere in it makes the whole
/// path dynamic again.
fn require_path_segments(
    expr: &Expr,
    src: &SourceFile,
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
) -> Option<Vec<Segment>> {
    let mut inner = expr;
    while let ExprKind::Paren(next) = &inner.kind {
        inner = next;
    }
    match &inner.kind {
        ExprKind::Str(span) => Some(vec![Segment::Text(cook_quoted(src, *span)?)]),
        ExprKind::ClassConstAccess { class, name } => {
            let ExprKind::ConstFetch(written) = &class.kind else {
                return None;
            };
            let class =
                crate::hierarchy::resolve_ref(src.span_text(written.span)?, namespace, imports);
            Some(vec![Segment::Const(
                class,
                src.span_text(*name)?.to_owned(),
            )])
        }
        ExprKind::Binary {
            op: BinaryOp::Concat,
            lhs,
            rhs,
        } => {
            let mut folded = require_path_segments(lhs, src, namespace, imports)?;
            folded.extend(require_path_segments(rhs, src, namespace, imports)?);
            Some(folded)
        }
        _ => None,
    }
}

/// Cooks a lexed string token into its value, for the two spellings a
/// `require` resolves statically.
///
/// The cooking itself is [`nvs_syntax::string_lit::cook_string_literal`] — the
/// front end's one escape grammar, so a path and an ordinary string literal
/// written the same way denote the same bytes, octal, hex and `\u{...}`
/// escapes included. What this adds is the *spelling* filter: a heredoc/nowdoc
/// token's raw text starts with `<`, and that falls through to `None`, which is
/// the dynamic fallback.
///
/// A literal whose escapes do not fully cook — a `\u{...}` past the Unicode
/// ceiling, a byte escape that is not valid UTF-8 — still yields a value here,
/// the same best-effort one the checker keeps checking against; the escape's
/// own diagnostic belongs to `nvs_types::expr`'s `ExprKind::Str` arm, and a
/// best-effort path that names no file is already
/// `code::E_REQUIRE_TARGET_NOT_FOUND`'s case.
fn cook_quoted(src: &SourceFile, span: Span) -> Option<String> {
    let raw = src.span_text(span)?;
    let quote = raw.chars().next()?;
    if quote != '\'' && quote != '"' || raw.len() < 2 || !raw.ends_with(quote) {
        return None;
    }
    Some(nvs_syntax::string_lit::cook_string_literal(src, span))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use nvs_diagnostics::SourceMap;

    use super::*;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "nvs-hir-requires-test-{name}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn write(&self, name: &str, contents: &str) -> PathBuf {
            let path = self.path.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create fixture directory");
            }
            fs::write(&path, contents).expect("write fixture");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn resolve_entry(dir: &TempDir, entry_name: &str) -> (Module, Diagnostics) {
        let (module, _loaded, _map, diags) = resolve_entry_loaded(dir, entry_name);
        (module, diags)
    }

    /// The same walk, keeping what `resolve_entry` throws away: the files it
    /// loaded, and the [`SourceMap`] their [`SourceId`]s index into.
    fn resolve_entry_loaded(
        dir: &TempDir,
        entry_name: &str,
    ) -> (Module, Vec<Loaded>, SourceMap, Diagnostics) {
        let mut map = SourceMap::new();
        let entry_path = dir.path.join(entry_name);
        let entry_id = map.load(&entry_path).expect("load entry fixture");
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(entry_id), &mut diags);
        let (module, loaded, _autoload) =
            resolve_program(entry_id, stmts, &mut map, CoreRoster::Trusted, &mut diags);
        (module, loaded, map, diags)
    }

    /// The edge `nvs-ir` cannot re-derive: which file a written `require`
    /// actually named. Two files require the same target, so one of those
    /// edges is recorded from the already-loaded branch rather than
    /// from the load itself, and both must still name the one `SourceId`
    /// that file was loaded as — the whole point of keeping `by_path`
    /// beside `done`. The target that does not exist contributes none.
    #[test]
    fn every_resolved_require_records_the_file_it_named() {
        let dir = TempDir::new("edges");
        dir.write("shared.nvs", "<?nvs\nclass Shared {}\n");
        dir.write("a.nvs", "<?nvs\nrequire 'shared.nvs';\nclass A {}\n");
        dir.write("b.nvs", "<?nvs\nrequire 'shared.nvs';\nclass B {}\n");
        dir.write(
            "main.nvs",
            "<?nvs
require 'a.nvs';
require 'b.nvs';
require 'nope.nvs';
",
        );

        let (_, loaded, map, _diags) = resolve_entry_loaded(&dir, "main.nvs");
        let name_of = |id: SourceId| -> String {
            map.file(id)
                .path()
                .and_then(Path::file_name)
                .expect("every fixture was loaded from disk")
                .to_string_lossy()
                .into_owned()
        };

        let mut edges: Vec<(String, String)> = Vec::new();
        for file in &loaded {
            for &(span, target) in &file.requires {
                // The span is the key `nvs-ir` will look this up by, so it
                // has to belong to the file that wrote the `require`.
                assert_eq!(span.file, file.id, "a require's span is its own file's");
                edges.push((name_of(file.id), name_of(target)));
            }
        }
        edges.sort();

        assert_eq!(
            edges,
            [
                ("a.nvs".to_owned(), "shared.nvs".to_owned()),
                ("b.nvs".to_owned(), "shared.nvs".to_owned()),
                ("main.nvs".to_owned(), "a.nvs".to_owned()),
                ("main.nvs".to_owned(), "b.nvs".to_owned()),
            ]
        );
    }

    /// `rule:programs/path-case`'s whole point, stated as the invariant that holds on
    /// every OS: a mis-cased `require` never compiles clean. *Which*
    /// diagnostic it gets is the filesystem's business — Linux never finds
    /// the file at all, Windows/macOS find it and reject the spelling.
    #[test]
    fn a_mis_cased_require_never_compiles_clean() {
        let dir = TempDir::new("case");
        dir.write(
            "Lib.nvs",
            "<?nvs
class Helper {}
",
        );
        dir.write(
            "main.nvs",
            "<?nvs
require 'lib.nvs';
",
        );

        let (_, diags) = resolve_entry(&dir, "main.nvs");
        let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
        assert!(
            codes.contains(&code::E_REQUIRE_PATH_CASE_MISMATCH)
                || codes.contains(&code::E_REQUIRE_TARGET_NOT_FOUND),
            "expected a case-mismatch or not-found diagnostic, got {diags:?}"
        );
    }

    #[test]
    fn an_exactly_spelled_require_in_a_subdirectory_is_clean() {
        let dir = TempDir::new("subdir");
        fs::create_dir_all(dir.path.join("Lib")).expect("create subdir");
        dir.write(
            "Lib/Helper.nvs",
            "<?nvs
class Helper {}
",
        );
        dir.write(
            "main.nvs",
            "<?nvs
require './Lib/Helper.nvs';
",
        );

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            module
                .symbols
                .contains(&crate::qname::QName::parse("Helper"))
        );
    }

    /// A `require` path is a string literal, so it denotes exactly what the
    /// front end's one escape grammar says a string literal denotes
    /// ([`nvs_syntax::string_lit`], the routine the checker diagnoses through
    /// and the lowering emits from). Asserted as agreement on both sides of
    /// the grammar rather than on one decoded byte: the target is named with
    /// the three escapes only the full cooker decodes — hex, octal and
    /// `\u{...}` — and a single-quoted literal, which has exactly two escapes,
    /// still leaves `\x61` the four characters written.
    #[test]
    fn a_require_path_decodes_every_escape_its_string_does() {
        let dir = TempDir::new("escapes");
        dir.write(
            "esc-aBé.nvs",
            "<?nvs
class Escaped {}
",
        );
        dir.write(
            "main.nvs",
            r#"<?nvs
require "esc-\x61\102\u{e9}.nvs";
"#,
        );

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            module
                .symbols
                .contains(&crate::qname::QName::parse("Escaped")),
            "the hex/octal/unicode-escaped path named no file"
        );

        let single = TempDir::new("escapes-single");
        single.write(
            "esc-a.nvs",
            "<?nvs
class Unreached {}
",
        );
        single.write("main.nvs", "<?nvs\nrequire 'esc-\\x61.nvs';\n");

        let (module, diags) = resolve_entry(&single, "main.nvs");
        assert!(diags.has_errors(), "a single-quoted `\\x61` cooked");
        assert!(
            !module
                .symbols
                .contains(&crate::qname::QName::parse("Unreached"))
        );
    }

    /// `check_path_case` drives off two paths it is handed, so its whole
    /// decision table is testable without a case-insensitive filesystem to
    /// run on.
    #[test]
    fn check_path_case_only_fires_on_a_pure_case_difference() {
        fn run(base: &str, literal: &str, canonical: &str) -> bool {
            let mut map = SourceMap::new();
            let id = map.add("case.nvs", "");
            let mut diags = Diagnostics::new();
            check_path_case(
                Path::new(base),
                literal,
                Path::new(canonical),
                Span::new(id, 0, 0),
                &mut diags,
            );
            diags.has_errors()
        }

        // The case the ADR exists for, at the file and at a directory.
        assert!(run("/app", "lib.nvs", "/app/Lib.nvs"));
        assert!(run("/app", "lib/helper.nvs", "/app/Lib/helper.nvs"));
        // `.` and `..` are replayed, not compared.
        assert!(run("/app/src", "../lib.nvs", "/app/Lib.nvs"));
        assert!(!run("/app/src", "./lib.nvs", "/app/src/lib.nvs"));
        // An exact spelling, and a difference that is not one of case (a
        // symlink resolved elsewhere), are both silent.
        assert!(!run("/app", "Lib.nvs", "/app/Lib.nvs"));
        assert!(!run("/app", "lib.nvs", "/app/other.nvs"));
        assert!(!run("/app", "lib.nvs", "/elsewhere/deeper/Lib.nvs"));
        // A component `base` contributed is never reported: how the entry
        // file was spelled on the command line is not this diagnostic's
        // business.
        assert!(!run("/App", "lib.nvs", "/app/lib.nvs"));
    }

    #[test]
    fn a_literal_require_merges_the_target_files_declarations() {
        let dir = TempDir::new("merge");
        dir.write("lib.nvs", "<?nvs\nclass Helper {}\n");
        dir.write("main.nvs", "<?nvs\nrequire 'lib.nvs';\nclass App {}\n");

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            module
                .symbols
                .contains(&crate::qname::QName::parse("Helper"))
        );
        assert!(module.symbols.contains(&crate::qname::QName::parse("App")));
    }

    #[test]
    fn a_concatenated_literal_require_resolves_like_one_literal() {
        let dir = TempDir::new("concat");
        dir.write("lib/db.nvs", "<?nvs\nclass Db {}\n");
        dir.write("main.nvs", "<?nvs\nrequire 'lib/' . 'db' . '.nvs';\n");

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(module.symbols.contains(&crate::qname::QName::parse("Db")));
    }

    #[test]
    fn a_concatenation_with_a_dynamic_half_stays_dynamic() {
        let dir = TempDir::new("concat-dynamic");
        dir.write("lib/db.nvs", "<?nvs\nclass Db {}\n");
        dir.write(
            "main.nvs",
            "<?nvs\n$name = 'db';\nrequire 'lib/' . $name . '.nvs';\n",
        );

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            !module.symbols.contains(&crate::qname::QName::parse("Db")),
            "a dynamic operand makes the whole path dynamic"
        );
    }

    #[test]
    fn a_require_path_folds_a_const_and_a_literal_concatenation() {
        let dir = TempDir::new("concat-const");
        dir.write("lib/db.nvs", "<?nvs\nclass Db {}\n");
        dir.write(
            "main.nvs",
            "<?nvs\nclass Paths { public const string LIB = 'lib/'; }\nrequire Paths::LIB . 'db.nvs';\n",
        );

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            module.symbols.contains(&crate::qname::QName::parse("Db")),
            "a constant the file declares folds like the literal it holds"
        );
    }

    #[test]
    fn a_const_an_earlier_require_loaded_folds_the_next_path() {
        let dir = TempDir::new("concat-const-earlier");
        dir.write(
            "paths.nvs",
            "<?nvs\nclass Paths { public const string LIB = 'lib/'; }\n",
        );
        dir.write("lib/db.nvs", "<?nvs\nclass Db {}\n");
        dir.write(
            "main.nvs",
            "<?nvs\nrequire 'paths.nvs';\nrequire Paths::LIB . 'db.nvs';\n",
        );

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            module.symbols.contains(&crate::qname::QName::parse("Db")),
            "the folder reads each file as the walk reaches it, in target order"
        );
    }

    #[test]
    fn a_require_path_naming_an_undeclared_const_stays_dynamic() {
        let dir = TempDir::new("concat-const-unknown");
        dir.write("lib/db.nvs", "<?nvs\nclass Db {}\n");
        dir.write("main.nvs", "<?nvs\nrequire Paths::LIB . 'db.nvs';\n");

        let (module, _diags) = resolve_entry(&dir, "main.nvs");
        assert!(
            !module.symbols.contains(&crate::qname::QName::parse("Db")),
            "a constant no file walked declares leaves the path for the runtime"
        );
    }

    #[test]
    fn a_required_class_is_visible_to_member_resolution() {
        let dir = TempDir::new("members");
        dir.write(
            "lib.nvs",
            "<?nvs\nclass Helper { public static function go(): void {} }\n",
        );
        dir.write(
            "main.nvs",
            "<?nvs\nrequire 'lib.nvs';\nclass App { function run(): void { Helper::go(); } }\n",
        );

        let (_module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_missing_require_target_is_diagnosed() {
        let dir = TempDir::new("missing");
        dir.write("main.nvs", "<?nvs\nrequire 'nope.nvs';\n");

        let (_module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_REQUIRE_TARGET_NOT_FOUND)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_direct_require_cycle_is_diagnosed_not_looped() {
        let dir = TempDir::new("cycle");
        dir.write("a.nvs", "<?nvs\nrequire 'b.nvs';\n");
        dir.write("b.nvs", "<?nvs\nrequire 'a.nvs';\n");

        let (_module, diags) = resolve_entry(&dir, "a.nvs");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_CIRCULAR_REQUIRE)),
            "{diags:?}"
        );
    }

    #[test]
    fn a_diamond_require_collects_the_shared_target_once() {
        let dir = TempDir::new("diamond");
        dir.write("d.nvs", "<?nvs\nclass Shared {}\n");
        dir.write("b.nvs", "<?nvs\nrequire 'd.nvs';\nclass B {}\n");
        dir.write("c.nvs", "<?nvs\nrequire 'd.nvs';\nclass C {}\n");
        dir.write(
            "main.nvs",
            "<?nvs\nrequire 'b.nvs';\nrequire 'c.nvs';\nclass App {}\n",
        );

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            module
                .symbols
                .contains(&crate::qname::QName::parse("Shared"))
        );
    }

    /// The contract [`resolve_program`]'s callers rely on: the entry file is
    /// first, every file in the graph is there exactly once, and each carries
    /// its own parsed statements. The order of the rest is the walk's and is
    /// deterministic, but only the entry's position is a promise — a caller
    /// that needs more than "entry first" should say so here.
    #[test]
    fn the_walk_hands_back_every_file_it_loaded_entry_first() {
        let dir = TempDir::new("loaded");
        dir.write("d.nvs", "<?nvs\nclass Shared {}\n");
        dir.write("b.nvs", "<?nvs\nrequire 'd.nvs';\nclass B {}\n");
        dir.write("c.nvs", "<?nvs\nrequire 'd.nvs';\nclass C {}\n");
        dir.write(
            "main.nvs",
            "<?nvs\nrequire 'b.nvs';\nrequire 'c.nvs';\nclass App {}\n",
        );

        let (_module, loaded, map, diags) = resolve_entry_loaded(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");

        let names: Vec<String> = loaded
            .iter()
            .map(|file| {
                assert!(!file.stmts.is_empty(), "a loaded file kept no statements");
                map.file(file.id)
                    .path()
                    .and_then(|p| p.file_name())
                    .expect("every fixture has an on-disk name")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();

        assert_eq!(
            names[0], "main.nvs",
            "the entry file comes first: {names:?}"
        );
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(sorted, ["b.nvs", "c.nvs", "d.nvs", "main.nvs"], "{names:?}");
    }

    #[test]
    fn a_dynamic_require_path_is_left_for_the_runtime_fallback() {
        let dir = TempDir::new("dynamic");
        dir.write(
            "main.nvs",
            "<?nvs\nstring $path = 'lib.nvs';\nrequire $path;\n",
        );

        let (_module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_require_with_no_on_disk_path_is_left_for_the_runtime_fallback() {
        let mut map = SourceMap::new();
        let id = map.add("virtual.nvs", "<?nvs\nrequire 'lib.nvs';\n");
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(id), &mut diags);
        let (_module, _loaded, _autoload) =
            resolve_program(id, stmts, &mut map, CoreRoster::Trusted, &mut diags);
        assert!(!diags.has_errors(), "{diags:?}");
    }

    // --- `rule:programs/no-runtime-autoload`: the autoload map ----------------------------------------

    /// A [`Site`] over `dir`, with one throwaway span for the declaration and
    /// each of its literals: every assertion below is about which file was
    /// found, never about where the declaration sat.
    fn site(dir: &TempDir, id: SourceId, kind: impl FnOnce(Span) -> autoload::SiteKind) -> Site {
        let span = Span::at(id, 0);
        Site {
            base_dir: dir.path.clone(),
            kind: kind(span),
            span,
        }
    }

    fn prefix(prefix: &str, roots: &[&str]) -> impl FnOnce(Span) -> autoload::SiteKind {
        let prefix = prefix.to_owned();
        let roots: Vec<String> = roots.iter().map(|r| (*r).to_owned()).collect();
        move |span| autoload::SiteKind::Prefix {
            prefix,
            literal: span,
            roots: roots.into_iter().map(|root| (root, span)).collect(),
        }
    }

    fn discover(glob: &str) -> impl FnOnce(Span) -> autoload::SiteKind {
        let glob = glob.to_owned();
        move |literal| autoload::SiteKind::Discover { glob, literal }
    }

    fn scratch_id(map: &mut SourceMap) -> SourceId {
        map.add("autoload.nvs", "")
    }

    /// `rule:programs/autoload` end to end: a class nothing `require`s, reached only by
    /// name through a prefix the bootstrap file declared.
    #[test]
    fn a_class_reached_only_through_autoload_is_collected() {
        let dir = TempDir::new("autoload-hit");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.nvs",
            "<?nvs\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Core.nvs",
            "<?nvs\nnamespace Framework;\nclass Core {}\n",
        );
        dir.write(
            "main.nvs",
            "<?nvs\nrequire './Bootstrap.nvs';\nvar $app = new Framework\\Core();\n",
        );

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(module.symbols.contains(&QName::parse(r"Framework\Core")));
    }

    /// `rule:types/class-reference`: a string literal under `as class<T>` or
    /// `as ?class<T>` loads its class while compiling, as `X::class` does, and
    /// the literal is the whole name even inside a namespace. A concatenation
    /// is a value built at run time and loads nothing.
    #[test]
    fn a_written_class_name_conversion_loads_its_class() {
        let dir = TempDir::new("autoload-written-class-name");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write("Bootstrap.nvs", "<?nvs\nautoload 'Shop' from './src';\n");
        dir.write(
            "src/Animal.nvs",
            "<?nvs\nnamespace Shop;\ninterface Animal {}\n",
        );
        dir.write(
            "src/Dog.nvs",
            "<?nvs\nnamespace Shop;\nclass Dog implements Animal {}\n",
        );
        dir.write(
            "src/Cat.nvs",
            "<?nvs\nnamespace Shop;\nclass Cat implements Animal {}\n",
        );
        dir.write(
            "src/Bird.nvs",
            "<?nvs\nnamespace Shop;\nclass Bird implements Animal {}\n",
        );
        dir.write(
            "main.nvs",
            "<?nvs\nnamespace Blog;\nrequire './Bootstrap.nvs';\n\
             var $dog = 'Shop\\Dog' as class<Shop\\Animal>;\n\
             var $cat = ('Shop\\Cat') as ?class<Shop\\Animal>;\n\
             var $bird = ('Shop\\\\' . 'Bird') as ?class<Shop\\Animal>;\n",
        );

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(module.symbols.contains(&QName::parse(r"Shop\Dog")));
        assert!(module.symbols.contains(&QName::parse(r"Shop\Cat")));
        assert!(!module.symbols.contains(&QName::parse(r"Shop\Bird")));
    }

    /// The name a destructuring leaf's declared type writes is a class
    /// reference like any other, and a pattern is the one place it can be the
    /// *only* mention in the file — `$pair` says nothing about what is in it.
    #[test]
    fn a_destructuring_leafs_declared_type_is_autoloaded() {
        let dir = TempDir::new("autoload-destructure-leaf");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.nvs",
            "<?nvs\nautoload 'Framework' from './src';\n",
        );
        dir.write("src/Row.nvs", "<?nvs\nnamespace Framework;\nclass Row {}\n");
        dir.write(
            "main.nvs",
            "<?nvs\nrequire './Bootstrap.nvs';\nvar $pair = [];\n[Framework\\Row $row] = $pair;\n",
        );

        let (module, _diags) = resolve_entry(&dir, "main.nvs");
        assert!(module.symbols.contains(&QName::parse(r"Framework\Row")));
    }

    /// `rule:packaging/autoload-probes-fold-into-the-cache-key`'s shadowing
    /// edge, which is the whole reason a trace keeps its misses. Between the
    /// two resolutions here, every file the first one hashed is byte-for-byte
    /// what it was — so nothing *but* the recorded miss could tell the two
    /// units apart, and a cache keyed on the compiled files alone would serve
    /// the stale one.
    #[test]
    fn a_file_created_where_autoload_probed_invalidates_the_unit() {
        let dir = TempDir::new("autoload-shadowing");
        fs::create_dir_all(dir.path.join("src")).expect("create src root");
        fs::create_dir_all(dir.path.join("vendor")).expect("create vendor root");
        dir.write(
            "Bootstrap.nvs",
            "<?nvs\nautoload 'Framework' from './src', './vendor';\n",
        );
        dir.write(
            "vendor/Core.nvs",
            "<?nvs\nnamespace Framework;\nclass Core {}\n",
        );
        dir.write(
            "main.nvs",
            "<?nvs\nrequire './Bootstrap.nvs';\nvar $app = new Framework\\Core();\n",
        );

        let walk = |dir: &TempDir| -> Vec<PathBuf> {
            let mut map = SourceMap::new();
            let entry_id = map
                .load(dir.path.join("main.nvs"))
                .expect("load entry fixture");
            let mut diags = Diagnostics::new();
            let stmts = parse_file(map.file(entry_id), &mut diags);
            let (_module, _loaded, autoload) =
                resolve_program(entry_id, stmts, &mut map, CoreRoster::Trusted, &mut diags);
            assert!(!diags.has_errors(), "{diags:?}");
            autoload.probe_trace().probed().to_vec()
        };

        let before = walk(&dir);
        assert_eq!(
            before.len(),
            2,
            "probed {before:?}, not `src` then `vendor`"
        );
        assert!(
            before[0].ends_with(Path::new("src/Core.nvs")) && !before[0].exists(),
            "{:?} is not a recorded miss under `./src`",
            before[0]
        );
        assert!(
            before[1].ends_with(Path::new("vendor/Core.nvs")),
            "the hit came from {:?}, not `./vendor`",
            before[1]
        );

        // The one edit: a file written exactly where the first resolution
        // probed and found nothing. `Bootstrap.nvs`, `main.nvs` and
        // `vendor/Core.nvs` are untouched.
        dir.write(
            "src/Core.nvs",
            "<?nvs\nnamespace Framework;\nclass Core {}\n",
        );

        let after = walk(&dir);
        assert_ne!(
            before, after,
            "the trace did not notice the file written where it probed"
        );
        assert_eq!(
            after,
            before[..1].to_vec(),
            "`./src` now hits, so the probe stops there"
        );
    }

    /// `rule:programs/autoload`'s output rather than its effect: the name resolves to a
    /// *file*, and the [`AutoloadMap`] the fixpoint hands back is what says
    /// which one. [`a_class_reached_only_through_autoload_is_collected`]
    /// above asserts the symbol arrived; this asserts where from, and that
    /// the walk loaded that file rather than only computing its path.
    #[test]
    fn an_autoload_declaration_resolves_a_name_to_its_file() {
        let dir = TempDir::new("autoload-name-to-file");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.nvs",
            "<?nvs\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Core.nvs",
            "<?nvs\nnamespace Framework;\nclass Core {}\n",
        );
        dir.write(
            "main.nvs",
            "<?nvs\nrequire './Bootstrap.nvs';\nvar $app = new Framework\\Core();\n",
        );

        let mut map = SourceMap::new();
        let entry_id = map
            .load(dir.path.join("main.nvs"))
            .expect("load entry fixture");
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(entry_id), &mut diags);
        let (module, loaded, autoload) =
            resolve_program(entry_id, stmts, &mut map, CoreRoster::Trusted, &mut diags);
        assert!(!diags.has_errors(), "{diags:?}");

        let name = QName::parse(r"Framework\Core");
        let probe = autoload.resolve(&name);
        let hit = probe.hit.as_ref().unwrap_or_else(|| {
            panic!(
                "`Framework\\Core` resolved to no file; probed {:?}",
                probe.tried
            )
        });

        // Compared by shape and not by string: a probe hit is canonicalized,
        // which on Windows carries a `\\?\` prefix and on macOS resolves the
        // temp directory's own symlink.
        assert_eq!(
            hit.file_name().and_then(|n| n.to_str()),
            Some("Core.nvs"),
            "{hit:?}"
        );
        assert_eq!(
            hit.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str()),
            Some("src"),
            "{hit:?}"
        );

        assert!(
            loaded.iter().any(|file| map
                .file(file.id)
                .path()
                .and_then(|p| p.file_name())
                .is_some_and(|found| found == "Core.nvs")),
            "the file the map named is not one the walk loaded",
        );
        assert!(module.symbols.contains(&name));
    }

    /// § 1's Composer rule: roots are probed in declaration order and the
    /// first hit wins, which is what makes an override root work. The probe
    /// trace records the misses that got there — § 5's cache input.
    #[test]
    fn roots_are_probed_in_declaration_order_and_the_first_hit_wins() {
        let dir = TempDir::new("autoload-order");
        for root in ["override", "vendor"] {
            fs::create_dir_all(dir.path.join(root)).expect("create root");
        }
        dir.write(
            "override/Thing.nvs",
            "<?nvs\nnamespace Acme;\nclass Thing {}\n",
        );
        dir.write(
            "vendor/Thing.nvs",
            "<?nvs\nnamespace Acme;\nclass Thing {}\n",
        );
        dir.write("vendor/Only.nvs", "<?nvs\nnamespace Acme;\nclass Only {}\n");

        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let built = AutoloadMap::build(
            &[site(&dir, id, prefix("Acme", &["./override", "./vendor"]))],
            &mut diags,
        );
        assert!(!diags.has_errors(), "{diags:?}");

        let first = built.resolve(&QName::parse(r"Acme\Thing"));
        assert_eq!(first.tried.len(), 1, "{first:?}");
        assert!(
            first
                .hit
                .expect("override hit")
                .ends_with(Path::new("override").join("Thing.nvs"))
        );

        let second = built.resolve(&QName::parse(r"Acme\Only"));
        assert_eq!(second.tried.len(), 2, "the miss is recorded: {second:?}");
        assert!(second.hit.is_some());

        let absent = built.resolve(&QName::parse(r"Acme\Absent"));
        assert!(absent.hit.is_none());
        assert_eq!(absent.tried.len(), 2);
    }

    /// § 1's "one prefix has one home".
    #[test]
    fn two_explicit_declarations_of_one_prefix_are_diagnosed() {
        let dir = TempDir::new("autoload-dup");
        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let _ = AutoloadMap::build(
            &[
                site(&dir, id, prefix("Acme", &["./a"])),
                site(&dir, id, prefix("Acme", &["./b"])),
            ],
            &mut diags,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_DUPLICATE_AUTOLOAD_PREFIX)),
            "{diags:?}"
        );
    }

    /// A borrowed declaration sits behind the program's own and never reports:
    /// a prefix both declare is the program's, a borrowed glob with nothing to
    /// scan says nothing, and only the program's own roots say which files it
    /// autoloads.
    #[test]
    fn a_borrowed_declaration_yields_to_the_programs_own_and_is_silent() {
        let dir = TempDir::new("autoload-borrowed");
        for root in ["own", "lent", "other"] {
            fs::create_dir_all(dir.path.join(root)).expect("create root");
        }
        dir.write("own/Thing.nvs", "<?nvs\nnamespace Acme;\nclass Thing {}\n");
        dir.write("lent/Thing.nvs", "<?nvs\nnamespace Acme;\nclass Thing {}\n");
        dir.write("other/Part.nvs", "<?nvs\nnamespace Other;\nclass Part {}\n");
        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let built = AutoloadMap::build_borrowing(
            &[site(&dir, id, prefix("Acme", &["./own"]))],
            &[
                site(&dir, id, prefix("Acme", &["./lent"])),
                site(&dir, id, prefix("Other", &["./other"])),
                site(&dir, id, discover("./missing/*/src")),
            ],
            &mut diags,
        );

        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(built.sites().len(), 1);
        let thing = built.resolve(&QName::parse("Acme\\Thing")).hit;
        assert_eq!(
            thing,
            canonicalize(&dir.path.join("own/Thing.nvs")),
            "the program's own root is probed, not the borrowed one"
        );
        assert!(built.resolve(&QName::parse("Other\\Part")).hit.is_some());
        assert!(built.claims(&dir.path.join("own/Thing.nvs")));
        assert!(!built.claims(&dir.path.join("other/Part.nvs")));
    }

    /// § 1: an explicit prefix beats a `discover` glob that would produce the
    /// same one — the glob skips the name rather than colliding with it — and
    /// a matched directory that is not a legal namespace segment is skipped
    /// in silence.
    #[test]
    fn an_explicit_prefix_shadows_a_discover_glob_that_would_repeat_it() {
        let dir = TempDir::new("autoload-discover");
        for module in ["Acme", "Other", ".git", "vendor"] {
            fs::create_dir_all(dir.path.join(module).join("src")).expect("create module");
        }
        dir.write(
            "Acme/src/Thing.nvs",
            "<?nvs\nnamespace Acme;\nclass Thing {}\n",
        );
        dir.write(
            "Other/src/Thing.nvs",
            "<?nvs\nnamespace Other;\nclass Thing {}\n",
        );
        fs::create_dir_all(dir.path.join("override")).expect("create override");
        dir.write(
            "override/Thing.nvs",
            "<?nvs\nnamespace Acme;\nclass Thing {}\n",
        );

        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let built = AutoloadMap::build(
            &[
                site(&dir, id, prefix("Acme", &["./override"])),
                site(&dir, id, discover("./*/src")),
            ],
            &mut diags,
        );
        assert!(!diags.has_errors(), "{diags:?}");

        let acme = built.resolve(&QName::parse(r"Acme\Thing"));
        assert!(
            acme.hit
                .expect("explicit root wins")
                .ends_with(Path::new("override").join("Thing.nvs"))
        );
        assert!(built.resolve(&QName::parse(r"Other\Thing")).hit.is_some());
    }

    /// § 1's last sentence: `nvs check --autoload-map` prints the resolved
    /// map *including what was skipped and what was shadowed*, which is the
    /// only way to tell a glob that discovered nothing from a glob nobody
    /// wrote. The whole rendering is asserted rather than sampled, since its
    /// shape is the contract — `crate::autoload`'s module doc owns it.
    #[test]
    fn the_rendered_map_names_what_was_skipped_and_what_was_shadowed() {
        let dir = TempDir::new("autoload-render");
        for module in ["Acme", "Other", ".git", "vendor"] {
            fs::create_dir_all(dir.path.join(module).join("src")).expect("create module");
        }
        fs::create_dir_all(dir.path.join("override")).expect("create override");

        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let built = AutoloadMap::build(
            &[
                site(&dir, id, prefix("Acme", &["./override"])),
                site(&dir, id, prefix("Gone", &["./absent"])),
                site(&dir, id, discover("./*/src")),
            ],
            &mut diags,
        );
        assert!(!diags.has_errors(), "{diags:?}");

        // `override` is swept by the glob like any other directory and passed
        // over for the same reason `.git` and `vendor` are; that it is also
        // `Acme`'s explicit root is not something the glob knows.
        assert_eq!(
            built.render(&dir.path),
            concat!(
                "prefixes (3)\n",
                "  Acme   explicit  override\n",
                "  Gone   explicit  absent\n",
                "  Other  discover  Other/src\n",
                "shadowed (1)\n",
                "  Acme   discover  Acme/src\n",
                "skipped (3)\n",
                "  .git      not a PascalCase namespace segment\n",
                "  override  not a PascalCase namespace segment\n",
                "  vendor    not a PascalCase namespace segment\n",
                "missing (1)\n",
                "  Gone   absent\n",
            )
        );
    }

    /// § 1's exact-name rule: a case-insensitive filesystem must not accept
    /// `thing.nvs` for `Thing` and then fail on Linux. Both legs answer
    /// "miss" — Linux never finds it, Windows finds it and refuses the
    /// spelling — so this asserts the answer rather than the mechanism.
    #[test]
    fn a_mis_cased_entry_is_a_miss_on_every_filesystem() {
        let dir = TempDir::new("autoload-case");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write("src/thing.nvs", "<?nvs\nnamespace Acme;\nclass thing {}\n");

        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let built = AutoloadMap::build(&[site(&dir, id, prefix("Acme", &["./src"]))], &mut diags);
        assert!(built.resolve(&QName::parse(r"Acme\Thing")).hit.is_none());
    }

    /// § 1: an `autoload` inside a file the map itself found would let the
    /// map depend on its own answers.
    #[test]
    fn an_autoload_inside_an_autoloaded_file_is_diagnosed() {
        let dir = TempDir::new("autoload-nested");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.nvs",
            "<?nvs\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Core.nvs",
            "<?nvs\nnamespace Framework;\nautoload 'Extra' from './more';\nclass Core {}\n",
        );
        dir.write(
            "main.nvs",
            "<?nvs\nrequire './Bootstrap.nvs';\nvar $app = new Framework\\Core();\n",
        );

        let (_module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_AUTOLOAD_IN_AUTOLOADED_FILE)),
            "{diags:?}"
        );
    }

    /// § 2: a file reached through an autoload root declares exactly one
    /// thing, named after it.
    #[test]
    fn an_autoloaded_file_declaring_a_second_thing_is_diagnosed() {
        let dir = TempDir::new("autoload-shape");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.nvs",
            "<?nvs\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Core.nvs",
            "<?nvs\nnamespace Framework;\nclass Core {}\nclass Helper {}\n",
        );
        dir.write(
            "main.nvs",
            "<?nvs\nrequire './Bootstrap.nvs';\nvar $app = new Framework\\Core();\n",
        );

        let (_module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_AUTOLOAD_FILE_SHAPE)),
            "{diags:?}"
        );
    }

    /// [`Site::directories`], which an editor's link reads: each written path
    /// of a declaration and the directory it resolves to, where there is one.
    #[test]
    fn each_written_autoload_path_names_the_directory_it_resolves_to() {
        let dir = TempDir::new("autoload-literals");
        dir.write("src/Thing.nvs", "<?nvs\n");
        dir.write("modules/Shop/src/Cart.nvs", "<?nvs\n");
        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let canonical = |relative: &str| {
            dir.path
                .join(relative)
                .canonicalize()
                .expect("fixture directory")
        };
        let at = |start: u32, end: u32| Span::new(id, start, end);

        // A root that exists names its directory under its own literal's span.
        // A missing root is allowed and names nothing. The prefix names the
        // first root that exists.
        let roots = Site {
            base_dir: dir.path.clone(),
            kind: autoload::SiteKind::Prefix {
                prefix: "App".to_owned(),
                literal: at(9, 14),
                roots: vec![
                    ("./gone".to_owned(), at(20, 28)),
                    ("./src".to_owned(), at(30, 37)),
                ],
            },
            span: at(0, 38),
        };
        assert_eq!(
            roots.directories(),
            vec![
                (at(9, 14), canonical("src")),
                (at(30, 37), canonical("src"))
            ]
        );
        // `roots` keeps the missing root, in probe order.
        let probed: Vec<(Span, String, bool)> = roots
            .roots()
            .into_iter()
            .map(|root| (root.literal, root.shown, root.exists))
            .collect();
        assert_eq!(
            probed,
            vec![
                (at(20, 28), "gone".to_owned(), false),
                (at(30, 37), "src".to_owned(), true)
            ]
        );
        assert_eq!(roots.namespace(), Some(vec!["App".to_owned()]));

        // A `{..}` segment is the name of the directory its steps reach.
        let braced = Site {
            base_dir: canonical("modules/Shop/src"),
            kind: autoload::SiteKind::Prefix {
                prefix: "App\\{..}".to_owned(),
                literal: at(9, 19),
                roots: vec![(".".to_owned(), at(25, 28))],
            },
            span: at(0, 29),
        };
        assert_eq!(
            braced.namespace(),
            Some(vec!["App".to_owned(), "Shop".to_owned()])
        );

        // A glob names the directory it lists, the part before its `*`.
        let glob = site(&dir, id, discover("./modules/*/src"));
        assert_eq!(
            glob.directories(),
            vec![(Span::at(id, 0), canonical("modules"))]
        );

        // A glob of the wrong shape lists nothing, so it names nothing.
        let malformed = site(&dir, id, discover("./modules/src"));
        assert_eq!(malformed.directories(), Vec::new());
        let missing = site(&dir, id, discover("./gone/*"));
        assert_eq!(missing.directories(), Vec::new());
    }

    /// A `discover` glob has to be one `*` occupying a whole segment, and one
    /// that silently discovers nothing is the outcome worth diagnosing.
    #[test]
    fn a_malformed_discovery_glob_is_diagnosed() {
        let dir = TempDir::new("autoload-glob");
        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let _ = AutoloadMap::build(&[site(&dir, id, discover("./src"))], &mut diags);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_AUTOLOAD_GLOB_SHAPE)),
            "{diags:?}"
        );
    }

    /// `rule:programs/implementing`'s scan, asserted whole: every name the roots declare, in
    /// one fully-qualified order, with a nested directory becoming a
    /// namespace segment the same way [`AutoloadMap::resolve`] turns one back
    /// into a directory. The fixture holds every entry the walk must pass
    /// over in silence — a directory that cannot name a segment, a file that
    /// is not a `.nvs`, a stem that is not `PascalCase` — because that half
    /// is the reason § 1 refuses to diagnose what a glob sweeps.
    #[test]
    fn the_scan_enumerates_every_name_the_roots_declare() {
        let dir = TempDir::new("autoload-enumerate");
        for sub in ["src", "src/Http", "src/.git", "vendor"] {
            fs::create_dir_all(dir.path.join(sub)).expect("create root");
        }
        dir.write(
            "src/Core.nvs",
            "<?nvs\nnamespace Framework;\nclass Core {}\n",
        );
        dir.write("src/Http/Router.nvs", "<?nvs\n");
        dir.write("src/.git/Head.nvs", "<?nvs\n");
        dir.write("src/README.md", "not a declaration\n");
        dir.write("src/lowercase.nvs", "<?nvs\n");
        dir.write("vendor/Compat.nvs", "<?nvs\n");

        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let autoload = AutoloadMap::build(
            &[site(&dir, id, prefix("Framework", &["./src", "./vendor"]))],
            &mut diags,
        );
        assert!(!diags.has_errors(), "{diags:?}");

        let names: Vec<String> = autoload
            .enumerate()
            .into_iter()
            .map(|(qname, _path)| qname.to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                r"Framework\Compat".to_owned(),
                r"Framework\Core".to_owned(),
                r"Framework\Http\Router".to_owned(),
            ],
        );
    }

    /// `rule:packaging/autoload-probes-fold-into-the-cache-key`'s discovery
    /// half: the recording scan keeps every directory it listed, each with the
    /// names it could act on, and the plain scan keeps nothing.
    #[test]
    fn the_recording_scan_keeps_every_directory_it_listed() {
        let dir = TempDir::new("autoload-enumerate-listed");
        for sub in ["src/Http", "src/.git"] {
            fs::create_dir_all(dir.path.join(sub)).expect("create root");
        }
        dir.write("src/Core.nvs", "<?nvs\n");
        dir.write("src/README.md", "not a declaration\n");
        dir.write("src/Http/Router.nvs", "<?nvs\n");

        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let mut autoload = AutoloadMap::build(
            &[site(&dir, id, prefix("Framework", &["./src", "./gone"]))],
            &mut diags,
        );
        assert!(!diags.has_errors(), "{diags:?}");

        let _ = autoload.enumerate();
        assert!(autoload.probe_trace().listed().is_empty());

        let _ = autoload.enumerate_recording();
        let listed: Vec<(String, Option<Vec<String>>)> = autoload
            .probe_trace()
            .listed()
            .iter()
            .map(|listing| {
                let name = listing.dir.file_name().unwrap_or_default();
                (name.to_string_lossy().into_owned(), listing.names.clone())
            })
            .collect();
        let names = |names: &[&str]| Some(names.iter().map(|n| (*n).to_owned()).collect());
        assert_eq!(
            listed,
            vec![
                ("src".to_owned(), names(&["Core.nvs", "Http"])),
                ("Http".to_owned(), names(&["Router.nvs"])),
                ("gone".to_owned(), None),
            ],
        );
        for listing in autoload.probe_trace().listed() {
            assert_eq!(
                crate::autoload::listed_names(&listing.dir),
                listing.names,
                "a listing taken again through the same filter agrees",
            );
        }
    }

    /// `rule:programs/implementing`'s opt-in, asserted as the difference it makes: one
    /// program writes `implementing<T>()` and collects a class nothing
    /// requires and nothing names, and the other differs only by not writing
    /// the call and never sees that class at all. Asserted as a pair because
    /// either half alone is satisfied by a walk that always scans, or by one
    /// that never does.
    #[test]
    fn only_a_program_that_asks_for_the_enumeration_scans_the_roots() {
        let dir = TempDir::new("autoload-scan");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.nvs",
            "<?nvs\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Mailer.nvs",
            "<?nvs\nnamespace Framework;\nclass Mailer {}\n",
        );
        dir.write(
            "scanning.nvs",
            concat!(
                "<?nvs\n",
                "require './Bootstrap.nvs';\n",
                "interface Module {}\n",
                "var $modules = Core\\Program::implementing<Module>();\n",
            ),
        );
        dir.write(
            "quiet.nvs",
            concat!(
                "<?nvs\n",
                "require './Bootstrap.nvs';\n",
                "interface Module {}\n",
            ),
        );

        let mailer = QName::parse(r"Framework\Mailer");

        let (scanned, diags) = resolve_entry(&dir, "scanning.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(scanned.symbols.contains(&mailer));

        let (quiet, diags) = resolve_entry(&dir, "quiet.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(!quiet.symbols.contains(&mailer));
    }

    /// `rule:programs/implementing-with` is the same enumeration as
    /// `implementing`, so it opts the program into the same scan on its own.
    /// The program below writes no `implementing` call, and still collects a
    /// class nothing requires and nothing names.
    #[test]
    fn implementing_with_alone_opts_the_program_into_the_scan() {
        let dir = TempDir::new("autoload-scan-with");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.nvs",
            "<?nvs\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Mailer.nvs",
            "<?nvs\nnamespace Framework;\nclass Mailer {}\n",
        );
        dir.write(
            "with.nvs",
            concat!(
                "<?nvs\n",
                "require './Bootstrap.nvs';\n",
                "interface Module {}\n",
                "var $rows = Core\\Program::implementingWith<Module, {path: string}>();\n",
            ),
        );

        let (module, diags) = resolve_entry(&dir, "with.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(module.symbols.contains(&QName::parse(r"Framework\Mailer")));
    }

    /// `rule:programs/constructors` lists the classes `implementing` would, so
    /// it opts the program into the same scan on its own, as
    /// `implementing_with_alone_opts_the_program_into_the_scan` pins for
    /// `implementingWith`.
    #[test]
    fn constructors_alone_opts_the_program_into_the_scan() {
        let dir = TempDir::new("autoload-scan-constructors");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.nvs",
            "<?nvs\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Mailer.nvs",
            "<?nvs\nnamespace Framework;\nclass Mailer {}\n",
        );
        dir.write(
            "make.nvs",
            concat!(
                "<?nvs\n",
                "require './Bootstrap.nvs';\n",
                "interface Module {}\n",
                "var $rows = Core\\Program::constructors<Module, callable(): Module>();\n",
            ),
        );

        let (module, diags) = resolve_entry(&dir, "make.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(module.symbols.contains(&QName::parse(r"Framework\Mailer")));
    }

    /// `rule:routing/table-is-opt-in`'s opt-in is 0061 § 3's, reused: every
    /// `Core\Router` or `Core\Request` member whose answer comes from the route
    /// table needs the scan, and the table is built from this same
    /// enumeration. Each member is asserted alone, because a member list is the
    /// kind of list that ships with one entry missing. The negatives are other
    /// `Core` static calls rather than no call at all, one of them on
    /// `Core\Request` itself. The pair above already pins that half, and what
    /// could go wrong *here* is a rule that scans for any `Core::` call it walks
    /// past, or for any member of a class it lists.
    #[test]
    fn a_router_link_asks_for_the_same_enumeration() {
        let dir = TempDir::new("autoload-scan-router");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.nvs",
            "<?nvs\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Mailer.nvs",
            "<?nvs\nnamespace Framework;\nclass Mailer {}\n",
        );
        for (file, call) in [
            ("url.nvs", r"Core\Router::url('Users::show', [])"),
            (
                "absolute.nvs",
                r"Core\Router::urlAbsolute('Users::show', [])",
            ),
            (
                "match.nvs",
                r"Core\Router::match(Core\Http\Method::Get, '/')",
            ),
            ("methods.nvs", r"Core\Router::methodsFor('/')"),
            ("signed.nvs", r"Core\Router::signedRoute([])"),
            ("route.nvs", r"Core\Request::route()"),
            ("other.nvs", r"Core\Str::length('Users::show')"),
            ("path.nvs", r"Core\Request::path()"),
        ] {
            dir.write(
                file,
                &format!("<?nvs\nrequire './Bootstrap.nvs';\nvar $x = {call};\n"),
            );
        }

        let mailer = QName::parse(r"Framework\Mailer");

        for file in [
            "url.nvs",
            "absolute.nvs",
            "match.nvs",
            "methods.nvs",
            "signed.nvs",
            "route.nvs",
        ] {
            let (module, diags) = resolve_entry(&dir, file);
            assert!(!diags.has_errors(), "{file}: {diags:?}");
            assert!(module.symbols.contains(&mailer), "{file} did not scan");
        }

        for file in ["other.nvs", "path.nvs"] {
            let (quiet, diags) = resolve_entry(&dir, file);
            assert!(!diags.has_errors(), "{file}: {diags:?}");
            assert!(!quiet.symbols.contains(&mailer), "{file} scanned");
        }
    }

    /// The two directions have to agree about which file declares a name, or
    /// § 3's enumeration would instantiate a class the rest of the compiler
    /// resolves somewhere else. A name under two roots of one prefix is the
    /// case where they could part: `resolve` probes in declaration order and
    /// takes the first hit, so the scan lists that same file once.
    #[test]
    fn a_name_under_two_roots_is_enumerated_at_the_file_resolve_would_pick() {
        let dir = TempDir::new("autoload-enumerate-shadow");
        for sub in ["override", "src"] {
            fs::create_dir_all(dir.path.join(sub)).expect("create root");
        }
        dir.write("override/Core.nvs", "<?nvs\n");
        dir.write("src/Core.nvs", "<?nvs\n");

        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let autoload = AutoloadMap::build(
            &[site(
                &dir,
                id,
                prefix("Framework", &["./override", "./src"]),
            )],
            &mut diags,
        );
        assert!(!diags.has_errors(), "{diags:?}");

        let found = autoload.enumerate();
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0, QName::parse(r"Framework\Core"));
        assert_eq!(
            found[0].1,
            autoload
                .resolve(&QName::parse(r"Framework\Core"))
                .hit
                .expect("the name resolves to a file"),
        );
    }

    /// A program with no `autoload` declaration pays for nothing: the map is
    /// never built past the empty check, and a name nothing declares is left
    /// for the checker to report.
    #[test]
    fn a_name_with_no_matching_prefix_is_left_alone() {
        let dir = TempDir::new("autoload-none");
        dir.write("main.nvs", "<?nvs\nvar $app = new Framework\\Core();\n");

        let (module, diags) = resolve_entry(&dir, "main.nvs");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(!module.symbols.contains(&QName::parse(r"Framework\Core")));
    }
}
