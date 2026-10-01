//! Fixtures shared by `nvs-types`'s checker tests.
//!
//! Each `tests/*.rs` file is its own binary, so the four helpers these tests
//! were written against live here and are reached through `mod common;`. They
//! moved out of `check.rs`'s inline `mod tests` unchanged except for becoming
//! `pub` — nothing about what they do is new.

#![allow(
    dead_code,
    reason = "each test binary compiles all of this module but uses only the \
              helpers its own area needs; which ones those are differs per file"
)]

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_hir::resolve_file;
use nvs_syntax::parse_file;
use nvs_types::check::check_program;
use nvs_types::expr_table::ExprTypeTable;
use nvs_types::ty::TypeInterner;

/// Wraps `body` inside `class T { function m(): void { ... } }` and
/// checks it — the common shape for a definite-assignment/expression
/// fixture that doesn't need its own class.
pub(crate) fn check_in_method(body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\nclass T {{\n  function m(): void {{\n{body}\n  }}\n}}\n"
    ))
}

/// Like [`check_src`], but hands back the typed-expression table too —
/// what a closure fixture asserts on, since `rule:types/callable-absorbs-closure`'s `callable`
/// carries none of what was resolved.
pub(crate) fn check_src_table(src: &str) -> (Diagnostics, ExprTypeTable) {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let module = resolve_file(&stmts, map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    check_program(
        &[nvs_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }],
        &module,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    (diags, exprs)
}

/// [`check_src_table`] without its two assertions — for a caller handing the
/// checker source it did not write itself and so cannot promise parses and
/// resolves, which wants the table whatever came back. The caller reads the
/// diagnostics to decide what silence in the table means.
pub(crate) fn check_src_table_allowing_errors(src: &str) -> (Diagnostics, ExprTypeTable) {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    let module = resolve_file(&stmts, map.file(file), &mut diags);
    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    check_program(
        &[nvs_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }],
        &module,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    (diags, exprs)
}

/// Like [`check_src_table`], but hands back the interner too, and a way to name
/// a written annotation's span — what a fixture asserting *which type an atom
/// interned to* needs, since a `TypeId` means nothing without the interner that
/// issued it.
///
/// The span is found by locating the annotation's own source text, which is
/// what [`nvs_types::lower::lower_type`] keys its record by
/// (`ExprTypeTable::declared_ty`). Written out rather than counted, so a
/// fixture says which annotation it means.
pub(crate) fn check_src_declared(src: &str) -> (Diagnostics, DeclaredTypes) {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let module = resolve_file(&stmts, map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    check_program(
        &[nvs_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }],
        &module,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    (
        diags,
        DeclaredTypes {
            src: src.to_owned(),
            file,
            exprs,
            interner,
        },
    )
}

/// What [`check_src_declared`] hands back: every annotation the run interned,
/// reachable by the text it was written as.
pub(crate) struct DeclaredTypes {
    src: String,
    file: nvs_diagnostics::SourceId,
    exprs: ExprTypeTable,
    pub interner: TypeInterner,
}

impl DeclaredTypes {
    /// The table this run built — what a fixture asserting a recorded
    /// `TypeId` needs beside [`Self::interner`], a `TypeId` meaning nothing
    /// without the interner that issued it.
    pub(crate) fn exprs(&self) -> &ExprTypeTable {
        &self.exprs
    }

    /// The `TypeId` the annotation `ty`, written on the binding `var`,
    /// interned to — `of("Mode", "$whole")`.
    ///
    /// Located by the pair rather than by `ty` alone, because a type's own
    /// text is rarely unique in a fixture (`Mode` appears in the `enum` header
    /// too) while `Mode $whole` is.
    ///
    /// # Panics
    /// Panics if `ty var` appears nowhere in the fixture, or if nothing was
    /// recorded at `ty`'s span — both mean the fixture and the assertion have
    /// drifted apart, which is worth failing loudly for.
    pub(crate) fn of(&self, ty: &str, var: &str) -> nvs_types::ty::TypeId {
        let needle = format!("{ty} {var}");
        let start = self
            .src
            .find(&needle)
            .unwrap_or_else(|| panic!("the fixture does not contain `{needle}`"));
        let start = u32::try_from(start).expect("fixtures are small");
        let span = nvs_diagnostics::Span::new(
            self.file,
            start,
            start + u32::try_from(ty.len()).expect("fixtures are small"),
        );
        self.exprs
            .declared_ty(span)
            .unwrap_or_else(|| panic!("no type was recorded for the annotation `{needle}`"))
    }

    /// The [`nvs_types::expr_table::ExprInfo`] recorded for the call written
    /// as `call` — `None` where the run recorded nothing for it.
    ///
    /// Located by the call's own text for [`Self::of`]'s reason, and it is the
    /// only handle a fixture has on a *folded* call: the answer is keyed by
    /// the call's span, and a fold leaves no name, no local and no diagnostic
    /// behind to find it by. `call` is written out whole, from the class name
    /// through the closing `)`, which is exactly the span the parser gives a
    /// static call.
    ///
    /// # Panics
    /// Panics if `call` appears nowhere in the fixture — the assertion and the
    /// fixture have drifted apart, which is worth failing loudly for.
    pub(crate) fn folded_at(&self, call: &str) -> Option<&nvs_types::expr_table::ExprInfo> {
        let start = self
            .src
            .find(call)
            .unwrap_or_else(|| panic!("the fixture does not contain `{call}`"));
        let start = u32::try_from(start).expect("fixtures are small");
        let end = start + u32::try_from(call.len()).expect("fixtures are small");
        self.exprs
            .lookup(nvs_diagnostics::Span::new(self.file, start, end))
    }
}

/// The capture names a fixture's one closure recorded, in order.
pub(crate) fn captures_of(src: &str) -> Vec<String> {
    let (diags, exprs) = check_src_table(src);
    assert!(!diags.has_errors(), "{diags:?}");
    exprs
        .closures()
        .next()
        .expect("the fixture declares one closure")
        .1
        .iter()
        .map(|(name, _)| name.clone())
        .collect()
}

/// [`check_src`] for a fixture the *parser* itself reports on — a construct
/// refused at parse time and then parsed whole, so every pass below still sees
/// what is inside it. Neither of [`check_src`]'s two assertions can hold for
/// one of those, and the caller asserts on the codes it expects instead.
pub(crate) fn check_src_allowing_parse_errors(src: &str) -> Diagnostics {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    let module = resolve_file(&stmts, map.file(file), &mut diags);
    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    check_program(
        &[nvs_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }],
        &module,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    diags
}

pub(crate) fn check_src(src: &str) -> Diagnostics {
    check_src_granted(src, None)
}

/// The declaration pass on its own — `nvs_syntax::check_declarations`, which
/// answers identifier casing (`rule:core-api/identifier-casing`) and written visibility off the
/// bare AST, before anything is resolved.
///
/// Every other helper here starts at `resolve_file`, so a casing refusal is
/// invisible to all of them: a fixture asking for one calls this instead of
/// [`check_src`], and gets that pass's diagnostics and no others.
pub(crate) fn check_declarations_only(src: &str) -> Diagnostics {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    nvs_syntax::check_declarations(&stmts, map.file(file), &mut diags);
    diags
}

/// [`check_src`] with a deployment's `[capabilities]` block in front of the
/// checker — `rule:core-classes/db-literal-query-checking`'s "read at boot on the machine that compiles",
/// which is the only input to a check that is not the program.
///
/// `None` is what every other fixture passes and is *not* an empty grant set;
/// `nvs_types::check_program_granted` owns that distinction, and a fixture
/// asserting a capability refusal has to hand over a real one.
pub(crate) fn check_src_granted(
    src: &str,
    grants: Option<&nvs_config::tree::Capabilities>,
) -> Diagnostics {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let module = resolve_file(&stmts, map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    nvs_types::check_program_granted(
        &[nvs_types::ProgramFile {
            src: map.file(file),
            stmts: &stmts,
        }],
        &module,
        grants,
        &mut interner,
        &mut exprs,
        &mut diags,
    );
    diags
}

/// A whole *program* checked, rather than one source: the files are written to
/// a temporary directory and walked by [`nvs_hir::resolve_program`], so the
/// table every whole-program pass builds is filled from the same
/// `require`/`autoload` graph the compiler fills it from.
///
/// `files` is `(name, source)` with the **entry first**, which is the order
/// `resolve_program` hands its loaded files back in and therefore the order
/// every collected table is in.
///
/// The single-source [`check_src_table`] cannot stand in for this: one
/// `resolve_file` resolves one file's declarations, so a class declared in a
/// second file would not be found however the sources were concatenated into
/// the `ProgramFile` slice.
pub(crate) fn check_program_table(files: &[(&str, &str)]) -> (Diagnostics, ExprTypeTable) {
    let dir = TempDir::new(files[0].0);
    for (name, src) in files {
        dir.write(name, src);
    }
    let mut map = SourceMap::new();
    let entry = map
        .load(dir.path.join(files[0].0))
        .expect("load the entry fixture");
    let mut diags = Diagnostics::new();
    let stmts = nvs_syntax::parse_file(map.file(entry), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let core = nvs_stdlib::registry::link_targets();
    let (module, loaded, _autoload) = nvs_hir::resolve_program(
        entry,
        stmts,
        &mut map,
        nvs_hir::CoreRoster::Names(&core),
        &mut diags,
    );
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let program: Vec<nvs_types::ProgramFile<'_>> = loaded
        .iter()
        .map(|file| nvs_types::ProgramFile {
            src: map.file(file.id),
            stmts: &file.stmts,
        })
        .collect();
    let mut interner = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    check_program(&program, &module, &mut interner, &mut exprs, &mut diags);
    (diags, exprs)
}

/// A directory of fixture files that removes itself, the shape
/// `nvs_hir::requires`' own tests use — copied rather than shared because that
/// one lives inside a `#[cfg(test)]` module of another crate.
struct TempDir {
    path: std::path::PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "nvs-types-program-test-{}-{}",
            name.replace('.', "-"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn write(&self, name: &str, contents: &str) {
        let path = self.path.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create the fixture's folder");
        }
        std::fs::write(path, contents).expect("write fixture");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
