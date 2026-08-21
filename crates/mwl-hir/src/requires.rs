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
//! A literal path that resolves to nothing loadable is
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

use std::path::{Path, PathBuf};

use mwl_diagnostics::{Diagnostic, Diagnostics, SourceFile, SourceId, SourceMap, Span, code};
use mwl_syntax::ast::{
    Arg, ArrayItem, Block, CallArgs, ClassMember, ClassMemberKind, DestructureElement,
    DestructureTarget, Expr, ExprKind, MemberName, NamespaceDecl, Stmt, StmtKind, StringPart,
};
use mwl_syntax::parse_file;
use rustc_hash::FxHashSet;

use crate::aliases::AliasResolver;
use crate::hierarchy::HierarchyResolver;
use crate::members::MemberResolver;
use crate::resolve::{Module, Resolver};

/// One file already pulled into the require graph, kept around (rather than
/// dropped after its `collect_*` pass) because [`crate::members::MemberResolver::check`]
/// needs every file's statements again, after every file has been collected.
struct Loaded {
    id: SourceId,
    stmts: Vec<Stmt>,
}

/// Resolves the `require` graph reachable from one entry file into a single
/// [`Module`], the same way [`crate::resolve::resolve_file`] resolves one
/// self-contained file — see the module docs for the mechanism.
///
/// `entry_stmts` is consumed rather than borrowed, matching every file
/// discovered by a `require` along the way: this function owns every file's
/// statements for as long as the graph walk needs them, since a later file
/// can't be loaded while an earlier one's `&SourceFile` is still borrowed
/// from `map`.
#[must_use]
pub fn resolve_program(
    entry_id: SourceId,
    entry_stmts: Vec<Stmt>,
    map: &mut SourceMap,
    diags: &mut Diagnostics,
) -> Module {
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

    while let Some((id, stmts, chain)) = work.pop() {
        {
            let src = map.file(id);
            resolver.collect_declarations(&stmts, src, diags);
            hierarchy.collect_links(&stmts, src);
            members.collect_members(&stmts, src);
            aliases.collect_aliases(&stmts, src);
        }

        let mut targets = Vec::new();
        find_require_literals(&stmts, map.file(id), &mut targets);
        let base_dir = map
            .file(id)
            .path()
            .and_then(|p| p.parent().map(Path::to_path_buf));

        if let Some(base_dir) = base_dir {
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
                let mut new_chain = chain.clone();
                new_chain.push(canonical);
                work.push((new_id, new_stmts, new_chain));
            }
        }

        loaded.push(Loaded { id, stmts });
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

    module
}

fn canonical_path(src: &SourceFile) -> Option<PathBuf> {
    src.path().and_then(|p| p.canonicalize().ok())
}

/// Walks `stmts` looking for every `require` expression whose path is a
/// plain string literal, appending each one's cooked path text and span to
/// `out`. A `require` whose path is anything else (a variable, a
/// concatenation, an interpolated string) is left out entirely — that is
/// the dynamic-fallback case this module does not touch.
fn find_require_literals(stmts: &[Stmt], src: &SourceFile, out: &mut Vec<(String, Span)>) {
    for stmt in stmts {
        walk_stmt(stmt, src, out);
    }
}

fn walk_stmt(stmt: &Stmt, src: &SourceFile, out: &mut Vec<(String, Span)>) {
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
        StmtKind::Foreach { subject, body, .. } => {
            e!(subject);
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
        StmtKind::LocalDecl {
            value: Some(value), ..
        } => e!(value),
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
        StmtKind::ClassDecl(decl) => walk_class_members(&decl.members, src, out),
        StmtKind::InterfaceDecl(decl) => walk_class_members(&decl.members, src, out),
        StmtKind::TraitDecl(decl) => walk_class_members(&decl.members, src, out),
        StmtKind::EnumDecl(decl) => {
            for case in &decl.cases {
                if let Some(value) = &case.value {
                    e!(value);
                }
            }
            walk_class_members(&decl.members, src, out);
        }
        StmtKind::NamespaceDecl(NamespaceDecl {
            body: Some(block), ..
        }) => find_require_literals(&block.stmts, src, out),
        _ => {}
    }
}

fn walk_class_members(members: &[ClassMember], src: &SourceFile, out: &mut Vec<(String, Span)>) {
    for member in members {
        match &member.kind {
            ClassMemberKind::Method(m) => {
                for param in &m.params {
                    if let Some(default) = &param.default {
                        walk_expr(default, src, out);
                    }
                }
                if let Some(body) = &m.body {
                    find_require_literals(&body.stmts, src, out);
                }
            }
            ClassMemberKind::Const(c) => walk_expr(&c.value, src, out),
            ClassMemberKind::Property(p) => {
                if let Some(default) = &p.default {
                    walk_expr(default, src, out);
                }
            }
            ClassMemberKind::UseTrait(_) | ClassMemberKind::Error => {}
            _ => {}
        }
    }
}

fn walk_block(block: &Block, src: &SourceFile, out: &mut Vec<(String, Span)>) {
    find_require_literals(&block.stmts, src, out);
}

fn walk_destructure_target(
    target: &DestructureTarget,
    src: &SourceFile,
    out: &mut Vec<(String, Span)>,
) {
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

fn walk_member_name(member: &MemberName, src: &SourceFile, out: &mut Vec<(String, Span)>) {
    if let MemberName::Variable(e) | MemberName::Expr(e) = member {
        walk_expr(e, src, out);
    }
}

fn walk_args(args: &CallArgs, src: &SourceFile, out: &mut Vec<(String, Span)>) {
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
fn walk_expr(expr: &Expr, src: &SourceFile, out: &mut Vec<(String, Span)>) {
    macro_rules! e {
        ($expr:expr) => {
            walk_expr($expr, src, out)
        };
    }

    match &expr.kind {
        ExprKind::Require { path } => {
            if let Some(literal) = literal_require_path(path, src) {
                out.push((literal, path.span));
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
        | ExprKind::Cast { expr, .. }
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
        ExprKind::Conversion { expr, .. } => e!(expr),
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
            args,
            ..
        } => {
            e!(object);
            walk_member_name(method, src, out);
            walk_args(args, src, out);
        }
        ExprKind::StaticCall {
            class,
            method,
            args,
        } => {
            e!(class);
            walk_member_name(method, src, out);
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
        ExprKind::New { args, .. } => walk_args(args, src, out),
        ExprKind::Closure(closure) => {
            for param in &closure.params {
                if let Some(default) = &param.default {
                    e!(default);
                }
            }
            walk_block(&closure.body, src, out);
        }
        ExprKind::ArrowFn(arrow) => {
            for param in &arrow.params {
                if let Some(default) = &param.default {
                    e!(default);
                }
            }
            e!(&arrow.body);
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
        ExprKind::ExitOrDie(Some(x)) => e!(x),
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
        let mut map = SourceMap::new();
        let entry_path = dir.path.join(entry_name);
        let entry_id = map.load(&entry_path).expect("load entry fixture");
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(entry_id), &mut diags);
        let module = resolve_program(entry_id, stmts, &mut map, &mut diags);
        (module, diags)
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
        let _module = resolve_program(id, stmts, &mut map, &mut diags);
        assert!(!diags.has_errors(), "{diags:?}");
    }
}
