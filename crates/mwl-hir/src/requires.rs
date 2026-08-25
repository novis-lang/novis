//! `require`'s static resolution (M2 item 4 — see the crate's module docs
//! for what's left after this).
//!
//! [ADR 0021](../../../docs/adr/0021-single-file-inclusion-construct.md)
//! keeps `require` as the only same-frame inclusion construct: no isolation
//! at all, so a required file's declarations must be visible to name
//! resolution exactly as if it had been pasted in at the `require` site.
//! [`resolve_program`] is the entry point that makes that happen for a
//! `require` whose path is written as a plain string literal — ADR 0021's
//! own M2 verification line calls this "statically resolved where the path
//! is a literal; a dynamic path falls back to a runtime resolve," so a
//! non-literal path (a variable, a concatenation, an interpolated string) is
//! left alone entirely here: no diagnostic, nothing collected, exactly the
//! dynamic fallback the ADR describes.
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
//! [ADR 0061](../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
//! § 1's autoload resolution, as a fixpoint rather than a second pass. Each
//! file's walk harvests three things, not one: its `require` targets, its
//! `autoload` declarations, and every name it uses where a class, interface,
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
//! failing on Linux ([ADR 0062](../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)
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
//! **Known gaps:**
//! - Only a plain `'...'`/`"..."` string literal (with no interpolation) is
//!   recognised as statically known. Heredoc/nowdoc and any expression built
//!   out of one — concatenation, a `const`, an `as` conversion — is treated
//!   as dynamic here even where a human reader could work out the value;
//!   widening this is a constant-folding problem for a later milestone, not
//!   a name-resolution one.
//! - A file with no on-disk path — every other test fixture in this crate,
//!   built with [`mwl_diagnostics::SourceMap::add`] rather than
//!   [`mwl_diagnostics::SourceMap::load`] — has no directory to resolve a
//!   relative `require` against, so a literal `require` written inside one
//!   is also left as a dynamic fallback. This is inherent to running from a
//!   source with no path (a REPL line, `stdin`), not a limitation to fix.
//! - The escape sequences a double-quoted literal's cooking recognises are a
//!   practical subset (`\\`, `\"`, `\$`, `\n`, `\r`, `\t`, `\v`, `\f`, `\e`)
//!   good enough for a file path — octal/hex/unicode escapes are left
//!   un-cooked (the backslash survives verbatim), which only matters for a
//!   `require` path containing one, vanishingly rare in practice. The real
//!   string-literal cooker belongs to a later milestone once something
//!   besides this module needs it.
//! - The name harvest is an over-approximation on purpose, and it reaches
//!   every declaration site's `#[...]` groups as well as its types and its
//!   bodies ([`walk_attributes`]) — but a `Name` in a still-unwalked corner
//!   of the AST would reach nobody. A missed name costs a class that fails
//!   to autoload, so the direction to widen in is always "harvest more",
//!   never "filter harder".
//! - ADR 0061 § 5's probe trace is produced ([`crate::autoload::Probe`]) and
//!   then dropped. Folding it into the artifact cache's key needs
//!   [ADR 0042](../../../docs/adr/0042-on-disk-artifact-cache-format.md)'s
//!   `PathEntry` table, which does not exist yet; that is the cache slice's
//!   work, not this one's.

use std::path::{Path, PathBuf};

use mwl_diagnostics::{Diagnostic, Diagnostics, SourceFile, SourceId, SourceMap, Span, code};
use mwl_syntax::ast::{
    Arg, ArrayItem, AttributeGroup, AutoloadDecl, AutoloadKind, Block, CallArgs, ClassMember,
    ClassMemberKind, DestructureElement, DestructureTarget, Expr, ExprKind, FnBody,
    ImplementsClause, MemberName, Name, NamespaceDecl, NewTarget, Param, PropertyHook,
    PropertyHookBody, Stmt, StmtKind, StringPart, Type, TypeAtom, TypeKind,
};
use mwl_syntax::{check_declarations, parse_file};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::aliases::AliasResolver;
use crate::autoload::{self, AutoloadMap, Site};
use crate::hierarchy::HierarchyResolver;
use crate::members::MemberResolver;
use crate::qname::QName;
use crate::resolve::{Module, Resolver};

/// One file pulled into the require graph, with the statements it parsed to.
///
/// [`resolve_program`] keeps these (rather than dropping each after its
/// `collect_*` pass) because [`crate::members::MemberResolver::check`] needs
/// every file's statements again, after every file has been collected — and
/// then hands the whole vector back, because every later phase needs the same
/// set: `mwl-types` type-checks each file's own top-level statements, and
/// `mwl-ir` lowers each file's declarations. The entry file's statements are
/// in here too, which is what makes the vector a complete description of the
/// program rather than "the files the entry pulled in".
#[derive(Debug)]
pub struct Loaded {
    /// The file, as [`mwl_diagnostics::SourceMap`] knows it.
    pub id: SourceId,
    /// Its whole parsed body, moved here once and never re-parsed.
    pub stmts: Vec<Stmt>,
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
/// unit's) can take this vector as given rather than sorting it.
///
/// The third element is the [`AutoloadMap`] the walk consulted, handed back
/// rather than dropped so `mwl check --autoload-map` can print it (ADR 0061
/// § 1). It is complete for any program that reached the first probe — which
/// is every program, since the walk consults the map once the `require` graph
/// drains, whether or not a name is still waiting on it.
#[must_use]
pub fn resolve_program(
    entry_id: SourceId,
    entry_stmts: Vec<Stmt>,
    map: &mut SourceMap,
    diags: &mut Diagnostics,
) -> (Module, Vec<Loaded>, AutoloadMap) {
    let mut resolver = Resolver::new();
    let mut hierarchy = HierarchyResolver::new();
    let mut members = MemberResolver::new();
    let mut aliases = AliasResolver::new();

    let mut loaded: Vec<Loaded> = Vec::new();
    let mut done: FxHashSet<PathBuf> = FxHashSet::default();
    let entry_chain: Vec<PathBuf> = canonical_path(map.file(entry_id)).into_iter().collect();
    for path in &entry_chain {
        done.insert(path.clone());
    }

    let mut work: Vec<(SourceId, Vec<Stmt>, Vec<PathBuf>)> =
        vec![(entry_id, entry_stmts, entry_chain)];

    // ADR 0061's three accumulators: every `autoload` declaration the
    // bootstrap chain wrote, every name that might need one, and the names
    // already probed, so a miss costs one probe rather than one per mention.
    let mut sites: Vec<Site> = Vec::new();
    let mut wanted: Vec<(QName, Span)> = Vec::new();
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
            }

            let mut harvest = Harvest::default();
            find_require_literals(&stmts, map.file(id), &mut harvest);
            wanted.append(&mut harvest.names);
            let targets = std::mem::take(&mut harvest.requires);
            let base_dir = map
                .file(id)
                .path()
                .and_then(|p| p.parent().map(Path::to_path_buf));

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
                // on the command line as `tests/main.mwl` has a relative,
                // as-typed parent; every other file in the graph was already
                // loaded by its canonical path.
                let canonical_base = base_dir.canonicalize().ok();
                for (literal, span) in targets {
                    let target = base_dir.join(&literal);
                    let Ok(canonical) = target.canonicalize() else {
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
                    let new_stmts = parse_file(map.file(new_id), diags);
                    check_declarations(&new_stmts, map.file(new_id), diags);
                    let mut new_chain = chain.clone();
                    new_chain.push(canonical);
                    work.push((new_id, new_stmts, new_chain));
                }
            }

            loaded.push(Loaded { id, stmts });
        }

        // Every file the `require` chain can reach is collected, so the map is
        // complete and this is the first moment it can be consulted.
        let built = autoload_map.get_or_insert_with(|| AutoloadMap::build(&sites, diags));
        if built.is_empty() {
            break;
        }

        let mut next: Option<(QName, Span, PathBuf)> = None;
        while let Some((name, span)) = wanted.pop() {
            if resolver.module().symbols.contains(&name) || !probed.insert(name.clone()) {
                continue;
            }
            // ADR 0061 § 5 keys the artifact cache on the whole probe trace,
            // misses included, so that adding a file which *shadows* one already
            // resolved invalidates the unit. `Probe::tried` carries it; nothing
            // records it yet, because ADR 0042's `PathEntry` table is the cache
            // slice's, not this one's.
            if let Some(path) = built.resolve(&name).hit {
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
        let new_stmts = parse_file(map.file(new_id), diags);
        check_declarations(&new_stmts, map.file(new_id), diags);
        autoload::check_file_shape(&new_stmts, map.file(new_id), name.short_name(), span, diags);
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
            diags,
        );
    }
    module.members = members.into_table();
    module.aliases = aliases.resolve(diags);

    (module, loaded, autoload_map.unwrap_or_default())
}

fn canonical_path(src: &SourceFile) -> Option<PathBuf> {
    src.path().and_then(|p| p.canonicalize().ok())
}

/// Reports a `require` whose literal path resolved only because the
/// filesystem folds case —
/// [ADR 0062](../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)
/// § 3, extending [ADR 0061](../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
/// § 1's exact-name rule from `autoload` to `require`.
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

/// Everything one file's walk yields. The three lists answer the three
/// questions the graph walk asks of a file: what does it pull in, what does
/// it say about where *other* names live, and which names does it use that
/// something will have to declare.
///
/// The namespace and imports are carried here rather than threaded as a
/// parameter because a `namespace`/`use` declaration only ever appears in a
/// file's own top-level statement sequence: every nested walk below inherits
/// what the enclosing sequence set and can never change it, so a field the
/// sequence writes as it goes is the same thing a parameter would be, minus
/// nine signatures.
#[derive(Default)]
struct Harvest {
    /// Each statically-known `require` target's cooked path text and span.
    requires: Vec<(String, Span)>,
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
            TypeAtom::Array(Some(inner)) => walk_type(inner, src, out),
            TypeAtom::Shape(fields) => {
                for field in fields {
                    walk_type(&field.ty, src, out);
                }
            }
            TypeAtom::Member(name, _) => record_name(name, src, out),
            TypeAtom::Name(name, args) => {
                record_name(name, src, out);
                for arg in args {
                    walk_type(arg, src, out);
                }
            }
            _ => {}
        },
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

/// Walks `stmts` looking for every `require` expression whose path is a
/// plain string literal, appending each one's cooked path text and span to
/// `out`. A `require` whose path is anything else (a variable, a
/// concatenation, an interpolated string) is left out entirely — that is
/// the dynamic-fallback case this module does not touch.
///
/// The same walk harvests ADR 0061's two other inputs — every `autoload`
/// declaration, and every name that might need one — because they are found
/// in the same places by the same recursion, and a second walker over the
/// whole AST would be a second walker to keep in step with the first.
fn find_require_literals(stmts: &[Stmt], src: &SourceFile, out: &mut Harvest) {
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
        StmtKind::Block(b) => find_require_literals(&b.stmts, src, out),
        StmtKind::If { cond, then, else_ } => {
            e!(cond);
            s!(then);
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
            for x in init.iter().chain(cond).chain(step) {
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
                if let Some(ty) = &binding.ty {
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
                find_require_literals(&case.body, src, out);
            }
        }
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            find_require_literals(&body.stmts, src, out);
            for catch in catches {
                walk_type(&catch.ty, src, out);
                find_require_literals(&catch.body.stmts, src, out);
            }
            if let Some(finally) = finally {
                find_require_literals(&finally.stmts, src, out);
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
                find_require_literals(&block.stmts, src, out);
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
        _ => {}
    }
}

/// Cooks one `autoload` declaration's spans into an [`autoload::SiteKind`].
///
/// The parser records spans rather than values — `ExprKind::Str`'s own shape
/// — so this is where a prefix and its roots become strings, through the same
/// [`cook_quoted`] a `require` path goes through. A literal that will not
/// cook (a heredoc) is dropped: the parser already reported
/// `E_AUTOLOAD_PATH_NOT_LITERAL` for every path that is not a plain string.
fn record_autoload(decl: &AutoloadDecl, src: &SourceFile, out: &mut Harvest) {
    let cook = |span: Span| src.span_text(span).and_then(cook_quoted);
    let kind = match &decl.kind {
        AutoloadKind::Prefix { prefix, roots } => {
            let Some(prefix) = cook(*prefix) else {
                return;
            };
            let roots: Vec<String> = roots.iter().filter_map(|r| cook(*r)).collect();
            if roots.is_empty() {
                return;
            }
            autoload::SiteKind::Prefix { prefix, roots }
        }
        AutoloadKind::Discover { glob } => {
            let Some(glob) = cook(*glob) else {
                return;
            };
            autoload::SiteKind::Discover { glob }
        }
    };
    out.autoloads.push((kind, decl.span));
}

fn walk_class_members(members: &[ClassMember], src: &SourceFile, out: &mut Harvest) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Method(m) => {
                walk_attributes(&m.attributes, src, out);
                walk_params(&m.params, src, out);
                if let Some(ty) = &m.return_type {
                    walk_type(ty, src, out);
                }
                if let Some(body) = &m.body {
                    find_require_literals(&body.stmts, src, out);
                }
            }
            ClassMemberKind::Const(c) => {
                walk_attributes(&c.attributes, src, out);
                if let Some(ty) = &c.ty {
                    walk_type(ty, src, out);
                }
                walk_expr(&c.value, src, out);
            }
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
            ClassMemberKind::Error => {}
            _ => {}
        }
    }
}

fn walk_block(block: &Block, src: &SourceFile, out: &mut Harvest) {
    find_require_literals(&block.stmts, src, out);
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
            DestructureElement::Leaf { key: Some(key), .. } => walk_expr(key, src, out),
            DestructureElement::Nested { key, target, .. } => {
                if let Some(key) = key {
                    walk_expr(key, src, out);
                }
                walk_destructure_target(target, src, out);
            }
            _ => {}
        }
    }
}

fn walk_member_name(member: &MemberName, src: &SourceFile, out: &mut Harvest) {
    if let MemberName::Variable(e) | MemberName::Expr(e) = member {
        walk_expr(e, src, out);
    }
}

/// Records every name an `#[...]` group mentions: the attribute class itself,
/// and whatever its argument expressions name.
///
/// An attribute is a class reference like any other — ADR 0061 § 1 places it
/// by prefix the same way — but it is the one such reference that reaches no
/// type position and no expression, so without this the file declaring
/// `#[Route(...)]`'s `Route` is never pulled in. Every declaration site that
/// carries a `Vec<AttributeGroup>` is fed through here by the walk that
/// already visits it.
fn walk_attributes(groups: &[AttributeGroup], src: &SourceFile, out: &mut Harvest) {
    for group in groups {
        for attr in &group.attributes {
            record_name(&attr.name, src, out);
            if let Some(args) = &attr.args {
                walk_args(args, src, out);
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
            if let Some(literal) = literal_require_path(path, src) {
                out.requires.push((literal, path.span));
            }
            // A dynamic path may still nest its own sub-expressions worth
            // walking for a further, statically-resolvable `require` inside
            // them (e.g. a ternary choosing between two literal paths, one
            // arm still worth statically pulling in) — that generality isn't
            // needed yet, so this stops at the top-level path expression.
        }
        ExprKind::Interpolated(parts) => {
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
        ExprKind::Conversion { expr, ty } => {
            e!(expr);
            walk_type(ty, src, out);
        }
        ExprKind::ConstFetch(name) => record_name(name, src, out),
        ExprKind::InstanceOf { expr, class } => {
            e!(expr);
            e!(class);
        }
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
        ExprKind::New { target, args } => {
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
                _ => {}
            }
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
        _ => {}
    }
}

/// Extracts a `require` path's literal text, if it was written as a plain
/// `'...'`/`"..."` string with no interpolation — unwrapping any surrounding
/// `(...)` first, so `require ('config.mwl');` resolves the same as
/// `require 'config.mwl';`.
fn literal_require_path(expr: &Expr, src: &SourceFile) -> Option<String> {
    let mut inner = expr;
    while let ExprKind::Paren(next) = &inner.kind {
        inner = next;
    }
    let ExprKind::Str(span) = &inner.kind else {
        return None;
    };
    cook_quoted(src.span_text(*span)?)
}

/// Cooks a lexed string token's raw text (quotes included) into its value.
/// Only single- and double-quoted forms are recognised — a heredoc/nowdoc
/// token's raw text starts with `<`, which falls through to `None`, the
/// dynamic-fallback case. See the module docs for the escape subset a
/// double-quoted literal supports.
fn cook_quoted(raw: &str) -> Option<String> {
    let quote = raw.chars().next()?;
    if quote != '\'' && quote != '"' || raw.len() < 2 || !raw.ends_with(quote) {
        return None;
    }
    let body = &raw[quote.len_utf8()..raw.len() - quote.len_utf8()];

    if quote == '\'' {
        let mut out = String::with_capacity(body.len());
        let mut chars = body.chars();
        while let Some(c) = chars.next() {
            if c == '\\'
                && let Some(n @ ('\\' | '\'')) = chars.clone().next()
            {
                out.push(n);
                chars.next();
                continue;
            }
            out.push(c);
        }
        return Some(out);
    }

    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.clone().next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('$') => out.push('$'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('v') => out.push('\u{0B}'),
            Some('f') => out.push('\u{0C}'),
            Some('e') => out.push('\u{1B}'),
            _ => {
                out.push('\\');
                continue;
            }
        }
        chars.next();
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use mwl_diagnostics::SourceMap;

    use super::*;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "mwl-hir-requires-test-{name}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn write(&self, name: &str, contents: &str) -> PathBuf {
            let path = self.path.join(name);
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
        let (module, loaded, _autoload) = resolve_program(entry_id, stmts, &mut map, &mut diags);
        (module, loaded, map, diags)
    }

    /// ADR 0062 § 3's whole point, stated as the invariant that holds on
    /// every OS: a mis-cased `require` never compiles clean. *Which*
    /// diagnostic it gets is the filesystem's business — Linux never finds
    /// the file at all, Windows/macOS find it and reject the spelling.
    #[test]
    fn a_mis_cased_require_never_compiles_clean() {
        let dir = TempDir::new("case");
        dir.write(
            "Lib.mwl",
            "<?mwl
class Helper {}
",
        );
        dir.write(
            "main.mwl",
            "<?mwl
require 'lib.mwl';
",
        );

        let (_, diags) = resolve_entry(&dir, "main.mwl");
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
            "Lib/Helper.mwl",
            "<?mwl
class Helper {}
",
        );
        dir.write(
            "main.mwl",
            "<?mwl
require './Lib/Helper.mwl';
",
        );

        let (module, diags) = resolve_entry(&dir, "main.mwl");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            module
                .symbols
                .contains(&crate::qname::QName::parse("Helper"))
        );
    }

    /// `check_path_case` drives off two paths it is handed, so its whole
    /// decision table is testable without a case-insensitive filesystem to
    /// run on.
    #[test]
    fn check_path_case_only_fires_on_a_pure_case_difference() {
        fn run(base: &str, literal: &str, canonical: &str) -> bool {
            let mut map = SourceMap::new();
            let id = map.add("case.mwl", "");
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
        assert!(run("/app", "lib.mwl", "/app/Lib.mwl"));
        assert!(run("/app", "lib/helper.mwl", "/app/Lib/helper.mwl"));
        // `.` and `..` are replayed, not compared.
        assert!(run("/app/src", "../lib.mwl", "/app/Lib.mwl"));
        assert!(!run("/app/src", "./lib.mwl", "/app/src/lib.mwl"));
        // An exact spelling, and a difference that is not one of case (a
        // symlink resolved elsewhere), are both silent.
        assert!(!run("/app", "Lib.mwl", "/app/Lib.mwl"));
        assert!(!run("/app", "lib.mwl", "/app/other.mwl"));
        assert!(!run("/app", "lib.mwl", "/elsewhere/deeper/Lib.mwl"));
        // A component `base` contributed is never reported: how the entry
        // file was spelled on the command line is not this diagnostic's
        // business.
        assert!(!run("/App", "lib.mwl", "/app/lib.mwl"));
    }

    #[test]
    fn a_literal_require_merges_the_target_files_declarations() {
        let dir = TempDir::new("merge");
        dir.write("lib.mwl", "<?mwl\nclass Helper {}\n");
        dir.write("main.mwl", "<?mwl\nrequire 'lib.mwl';\nclass App {}\n");

        let (module, diags) = resolve_entry(&dir, "main.mwl");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            module
                .symbols
                .contains(&crate::qname::QName::parse("Helper"))
        );
        assert!(module.symbols.contains(&crate::qname::QName::parse("App")));
    }

    #[test]
    fn a_required_class_is_visible_to_member_resolution() {
        let dir = TempDir::new("members");
        dir.write(
            "lib.mwl",
            "<?mwl\nclass Helper { public static function go(): void {} }\n",
        );
        dir.write(
            "main.mwl",
            "<?mwl\nrequire 'lib.mwl';\nclass App { function run(): void { Helper::go(); } }\n",
        );

        let (_module, diags) = resolve_entry(&dir, "main.mwl");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_missing_require_target_is_diagnosed() {
        let dir = TempDir::new("missing");
        dir.write("main.mwl", "<?mwl\nrequire 'nope.mwl';\n");

        let (_module, diags) = resolve_entry(&dir, "main.mwl");
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
        dir.write("a.mwl", "<?mwl\nrequire 'b.mwl';\n");
        dir.write("b.mwl", "<?mwl\nrequire 'a.mwl';\n");

        let (_module, diags) = resolve_entry(&dir, "a.mwl");
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
        dir.write("d.mwl", "<?mwl\nclass Shared {}\n");
        dir.write("b.mwl", "<?mwl\nrequire 'd.mwl';\nclass B {}\n");
        dir.write("c.mwl", "<?mwl\nrequire 'd.mwl';\nclass C {}\n");
        dir.write(
            "main.mwl",
            "<?mwl\nrequire 'b.mwl';\nrequire 'c.mwl';\nclass App {}\n",
        );

        let (module, diags) = resolve_entry(&dir, "main.mwl");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(
            module
                .symbols
                .contains(&crate::qname::QName::parse("Shared"))
        );
    }

    /// The contract [`resolve_program`]'s callers rely on: the entry file is
    /// first, every file in the graph is there exactly once, and each carries
    /// its own parsed statements. The order of the four is the walk's and is
    /// deterministic, but only the entry's position is a promise — a caller
    /// that needs more than "entry first" should say so here.
    #[test]
    fn the_walk_hands_back_every_file_it_loaded_entry_first() {
        let dir = TempDir::new("loaded");
        dir.write("d.mwl", "<?mwl\nclass Shared {}\n");
        dir.write("b.mwl", "<?mwl\nrequire 'd.mwl';\nclass B {}\n");
        dir.write("c.mwl", "<?mwl\nrequire 'd.mwl';\nclass C {}\n");
        dir.write(
            "main.mwl",
            "<?mwl\nrequire 'b.mwl';\nrequire 'c.mwl';\nclass App {}\n",
        );

        let (_module, loaded, map, diags) = resolve_entry_loaded(&dir, "main.mwl");
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
            names[0], "main.mwl",
            "the entry file comes first: {names:?}"
        );
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(sorted, ["b.mwl", "c.mwl", "d.mwl", "main.mwl"], "{names:?}");
    }

    #[test]
    fn a_dynamic_require_path_is_left_for_the_runtime_fallback() {
        let dir = TempDir::new("dynamic");
        dir.write(
            "main.mwl",
            "<?mwl\nstring $path = 'lib.mwl';\nrequire $path;\n",
        );

        let (_module, diags) = resolve_entry(&dir, "main.mwl");
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn a_require_with_no_on_disk_path_is_left_for_the_runtime_fallback() {
        let mut map = SourceMap::new();
        let id = map.add("virtual.mwl", "<?mwl\nrequire 'lib.mwl';\n");
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(id), &mut diags);
        let (_module, _loaded, _autoload) = resolve_program(id, stmts, &mut map, &mut diags);
        assert!(!diags.has_errors(), "{diags:?}");
    }

    // --- ADR 0061: the autoload map ----------------------------------------

    /// A [`Site`] over `dir`, with a throwaway span: every assertion below is
    /// about which file was found, never about where the declaration sat.
    fn site(dir: &TempDir, id: SourceId, kind: autoload::SiteKind) -> Site {
        Site {
            base_dir: dir.path.clone(),
            kind,
            span: Span::at(id, 0),
        }
    }

    fn prefix(prefix: &str, roots: &[&str]) -> autoload::SiteKind {
        autoload::SiteKind::Prefix {
            prefix: prefix.to_owned(),
            roots: roots.iter().map(|r| (*r).to_owned()).collect(),
        }
    }

    fn scratch_id(map: &mut SourceMap) -> SourceId {
        map.add("autoload.mwl", "")
    }

    /// ADR 0061 § 1 end to end: a class nothing `require`s, reached only by
    /// name through a prefix the bootstrap file declared.
    #[test]
    fn a_class_reached_only_through_autoload_is_collected() {
        let dir = TempDir::new("autoload-hit");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.mwl",
            "<?mwl\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Core.mwl",
            "<?mwl\nnamespace Framework;\nclass Core {}\n",
        );
        dir.write(
            "main.mwl",
            "<?mwl\nrequire './Bootstrap.mwl';\nvar $app = new Framework\\Core();\n",
        );

        let (module, diags) = resolve_entry(&dir, "main.mwl");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(module.symbols.contains(&QName::parse(r"Framework\Core")));
    }

    /// ADR 0061 § 1's output rather than its effect: the name resolves to a
    /// *file*, and the [`AutoloadMap`] the fixpoint hands back is what says
    /// which one. [`a_class_reached_only_through_autoload_is_collected`]
    /// above asserts the symbol arrived; this asserts where from, and that
    /// the walk loaded that file rather than only computing its path.
    #[test]
    fn an_autoload_declaration_resolves_a_name_to_its_file() {
        let dir = TempDir::new("autoload-name-to-file");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write(
            "Bootstrap.mwl",
            "<?mwl\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Core.mwl",
            "<?mwl\nnamespace Framework;\nclass Core {}\n",
        );
        dir.write(
            "main.mwl",
            "<?mwl\nrequire './Bootstrap.mwl';\nvar $app = new Framework\\Core();\n",
        );

        let mut map = SourceMap::new();
        let entry_id = map
            .load(dir.path.join("main.mwl"))
            .expect("load entry fixture");
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(entry_id), &mut diags);
        let (module, loaded, autoload) = resolve_program(entry_id, stmts, &mut map, &mut diags);
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
            Some("Core.mwl"),
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
                .is_some_and(|found| found == "Core.mwl")),
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
            "override/Thing.mwl",
            "<?mwl\nnamespace Acme;\nclass Thing {}\n",
        );
        dir.write(
            "vendor/Thing.mwl",
            "<?mwl\nnamespace Acme;\nclass Thing {}\n",
        );
        dir.write("vendor/Only.mwl", "<?mwl\nnamespace Acme;\nclass Only {}\n");

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
                .ends_with(Path::new("override").join("Thing.mwl"))
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
            "Acme/src/Thing.mwl",
            "<?mwl\nnamespace Acme;\nclass Thing {}\n",
        );
        dir.write(
            "Other/src/Thing.mwl",
            "<?mwl\nnamespace Other;\nclass Thing {}\n",
        );
        fs::create_dir_all(dir.path.join("override")).expect("create override");
        dir.write(
            "override/Thing.mwl",
            "<?mwl\nnamespace Acme;\nclass Thing {}\n",
        );

        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let built = AutoloadMap::build(
            &[
                site(&dir, id, prefix("Acme", &["./override"])),
                site(
                    &dir,
                    id,
                    autoload::SiteKind::Discover {
                        glob: "./*/src".to_owned(),
                    },
                ),
            ],
            &mut diags,
        );
        assert!(!diags.has_errors(), "{diags:?}");

        let acme = built.resolve(&QName::parse(r"Acme\Thing"));
        assert!(
            acme.hit
                .expect("explicit root wins")
                .ends_with(Path::new("override").join("Thing.mwl"))
        );
        assert!(built.resolve(&QName::parse(r"Other\Thing")).hit.is_some());
    }

    /// § 1's last sentence: `mwl check --autoload-map` prints the resolved
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
                site(
                    &dir,
                    id,
                    autoload::SiteKind::Discover {
                        glob: "./*/src".to_owned(),
                    },
                ),
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
                "prefixes (2)\n",
                "  Acme   explicit  override\n",
                "  Other  discover  Other/src\n",
                "shadowed (1)\n",
                "  Acme   discover  Acme/src\n",
                "skipped (3)\n",
                "  .git      not a PascalCase namespace segment\n",
                "  override  not a PascalCase namespace segment\n",
                "  vendor    not a PascalCase namespace segment\n",
            )
        );
    }

    /// § 1's exact-name rule: a case-insensitive filesystem must not accept
    /// `thing.mwl` for `Thing` and then fail on Linux. Both legs answer
    /// "miss" — Linux never finds it, Windows finds it and refuses the
    /// spelling — so this asserts the answer rather than the mechanism.
    #[test]
    fn a_mis_cased_entry_is_a_miss_on_every_filesystem() {
        let dir = TempDir::new("autoload-case");
        fs::create_dir_all(dir.path.join("src")).expect("create root");
        dir.write("src/thing.mwl", "<?mwl\nnamespace Acme;\nclass thing {}\n");

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
            "Bootstrap.mwl",
            "<?mwl\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Core.mwl",
            "<?mwl\nnamespace Framework;\nautoload 'Extra' from './more';\nclass Core {}\n",
        );
        dir.write(
            "main.mwl",
            "<?mwl\nrequire './Bootstrap.mwl';\nvar $app = new Framework\\Core();\n",
        );

        let (_module, diags) = resolve_entry(&dir, "main.mwl");
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
            "Bootstrap.mwl",
            "<?mwl\nautoload 'Framework' from './src';\n",
        );
        dir.write(
            "src/Core.mwl",
            "<?mwl\nnamespace Framework;\nclass Core {}\nclass Helper {}\n",
        );
        dir.write(
            "main.mwl",
            "<?mwl\nrequire './Bootstrap.mwl';\nvar $app = new Framework\\Core();\n",
        );

        let (_module, diags) = resolve_entry(&dir, "main.mwl");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_AUTOLOAD_FILE_SHAPE)),
            "{diags:?}"
        );
    }

    /// A `discover` glob has to be one `*` occupying a whole segment, and one
    /// that silently discovers nothing is the outcome worth diagnosing.
    #[test]
    fn a_malformed_discovery_glob_is_diagnosed() {
        let dir = TempDir::new("autoload-glob");
        let mut map = SourceMap::new();
        let id = scratch_id(&mut map);
        let mut diags = Diagnostics::new();
        let _ = AutoloadMap::build(
            &[site(
                &dir,
                id,
                autoload::SiteKind::Discover {
                    glob: "./src".to_owned(),
                },
            )],
            &mut diags,
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_AUTOLOAD_GLOB_SHAPE)),
            "{diags:?}"
        );
    }

    /// A program with no `autoload` declaration pays for nothing: the map is
    /// never built past the empty check, and a name nothing declares is left
    /// for the checker to report.
    #[test]
    fn a_name_with_no_matching_prefix_is_left_alone() {
        let dir = TempDir::new("autoload-none");
        dir.write("main.mwl", "<?mwl\nvar $app = new Framework\\Core();\n");

        let (module, diags) = resolve_entry(&dir, "main.mwl");
        assert!(!diags.has_errors(), "{diags:?}");
        assert!(!module.symbols.contains(&QName::parse(r"Framework\Core")));
    }
}
