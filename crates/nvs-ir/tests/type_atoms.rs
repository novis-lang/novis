//! The other half of [`refusals`](../refusals.rs)' gate: **every type ADR 0007
//! § 3 lets a program spell reaches a diagnostic or an IR, and never a panic.**
//!
//! # Why counting refusal sites was not enough
//!
//! `refusals.rs` counts *sites* and attributes each to the goal item whose
//! anchors sit in the same file. Both halves are blind in the same place: a
//! catch-all arm is one site however many shapes fall into it, and the item
//! that claims it is whichever one the file anchors — not whichever one the
//! shapes belong to. `crates/nvs-ir/src/lower/mod.rs`'s `lower_checked_ty` was
//! exactly that. It counted as one site, item 25 claimed it because item 25
//! anchors that file, and item 25 is about `object` — which had a
//! representation arm already. Three unrelated shapes were reaching it, all
//! three panicking a compiler that had type-checked them, and the gate read
//! green because the ceiling was a number of arms rather than a list of shapes.
//!
//! So this table names the shapes. A site may hide as many as it likes; a
//! *shape* that panics is one row here that cannot be anything else.
//!
//! # What it does
//!
//! Every atom below, in both declaration positions this can build without also
//! having to build a value of the type — a parameter and a return — through
//! `parse` → `resolve` → `check` → `lower_program`, with the panic hook
//! silenced and the unwind caught. Three outcomes, and only the third is
//! interesting:
//!
//! *   **Diagnosed** — the front end refused it. A pass: the goal's own rule is
//!     that a numbered diagnostic naming its ADR closes a shape as surely as
//!     lowering it does.
//! *   **Lowered** — it reached IR. A pass.
//! *   **Panicked** — it type-checked and then died. A fail, unless the shape
//!     is on [`KNOWN_ICE`].
//!
//! [`KNOWN_ICE`] is a ratchet with the same rule as `refusals.rs`'s `CEILING`:
//! **it may never grow**, every entry is an open item in
//! `docs/agent/loop-goal.md`, and an entry that stops panicking fails this test
//! until it is deleted — so a shape cannot be fixed and left on the list, and a
//! new one cannot be added to make a run go green.
//!
//! # What it does not cover yet
//!
//! The **local-declaration** position, which reaches `lower_decl_type` rather
//! than `lower_checked_ty` — the other of item 25's two sites. Building one
//! needs a *value* of each type as well as the annotation, which is a fixture
//! per atom rather than a table row. The two positions here share
//! `erase_checked_ty`, so they already cover what both catch-alls receive; a
//! session adding the third position should say so here and delete this
//! paragraph.

use std::panic::{AssertUnwindSafe, catch_unwind};

use nvs_diagnostics::{Diagnostics, SourceMap};

/// One row of ADR 0007 § 3's `atom` production, plus the three composite forms
/// its `union`/`intersection`/`qualified` levels build.
///
/// `self`/`static`/`parent` are absent on purpose: all three are *resolution*
/// spellings of the enclosing class, and what reaches lowering is whatever the
/// checker resolved them to, which is the `Name` row below.
const ATOMS: &[&str] = &[
    // The scalars.
    "null",
    "bool",
    "int",
    "uint",
    "float",
    "decimal",
    "string",
    "bytes",
    // ADR 0024 and ADR 0033's qualifiers, which add no representation and so
    // are the one place this table is asserting a *negative*.
    "tainted string",
    "secret string",
    "secret tainted string",
    "tainted bytes",
    "secret bytes",
    // The tops, the bottoms and the containers.
    "array",
    "array<int>",
    "object",
    "mixed",
    "void",
    "never",
    "true",
    "false",
    "iterable",
    "callable",
    // ADR 0047's literal types, ADR 0036's shape, and a plain class name.
    "\"a\"",
    "7",
    "{a: int}",
    "Marker",
    // The composites: ADR 0066's `?T`, a union, and an intersection.
    "?int",
    "int|string",
    "Marker&Other",
];

/// The shapes that still panic, as `(atom, position)`.
///
/// **This may never grow.** Every entry is an open item in
/// `docs/agent/loop-goal.md`, and the test fails just as loudly on an entry
/// that has *stopped* panicking — the ratchet only turns one way.
const KNOWN_ICE: &[(&str, Position)] = &[
    // Item 25. All three reach `erase_checked_ty`'s `_ => None` and then
    // `lower_checked_ty`'s panic.
    //
    // `never` has a **parameter** row as well as a return one, which is the
    // finding this table exists for: ADR 0007 § 3 says `void` and `never` are
    // return-only, `void` in a parameter is diagnosed, and `never` in a
    // parameter is not — it type-checks and reaches the same panic. Closing
    // this half is the checker's rule, not a representation.
    ("never", Position::Param),
    ("never", Position::Return),
    ("iterable", Position::Param),
    ("iterable", Position::Return),
    ("Marker&Other", Position::Param),
    ("Marker&Other", Position::Return),
];

/// Where in a declaration the atom is written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Position {
    /// `public static function m(ATOM $p): void {}` — never called, so the
    /// annotation alone is what reaches lowering.
    Param,
    /// `public static function m(): ATOM { throw …; }` — a body that leaves
    /// the frame satisfies every declared return type, `void` and `never`
    /// included, so one body serves the whole table.
    Return,
}

impl Position {
    /// The whole fixture for `atom` in this position.
    ///
    /// `Marker` and `Other` are declared in every fixture so the two rows that
    /// name a class have one to resolve to, and the rest pay one unused
    /// declaration each.
    fn fixture(self, atom: &str) -> String {
        let head = "\
<?nvs
interface Other {
    public function other(): int;
}

class Marker implements Other {
    public function other(): int {
        return 1;
    }
}

class T {
";
        let body = match self {
            Self::Param => format!("    public static function m({atom} $p): void {{}}\n"),
            // `LogicError` rather than `Core\Error`: the exception tree's
            // members are global names (ADR 0011 § 1), and `new Core\Error(…)`
            // is `nvs-types`' own known zero-arity gap rather than anything
            // this table is asking about.
            Self::Return => format!(
                "    public static function m(): {atom} {{\n        \
                 throw new LogicError(\"unreached\");\n    }}\n"
            ),
        };
        format!("{head}{body}}}\n")
    }
}

/// What the compiler did with one fixture.
#[derive(PartialEq, Eq, Debug)]
enum Outcome {
    /// The front end refused it, naming a rule. A pass.
    Diagnosed,
    /// It reached IR. A pass.
    Lowered,
    /// It type-checked and then died below the front end. The finding.
    Panicked,
}

/// Runs one fixture all the way to `lower_program`, catching an unwind.
///
/// The panic hook is replaced for the duration so a failing row does not print
/// a backtrace per atom; the message is not needed, since the row itself names
/// the shape better than any panic string does.
fn outcome(src: &str) -> Outcome {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let done = catch_unwind(AssertUnwindSafe(|| compile(src)));
    std::panic::set_hook(previous);
    match done {
        Ok(reached) => reached,
        Err(_) => Outcome::Panicked,
    }
}

/// Parse, resolve, check, lower — stopping at the first phase that reports an
/// error, exactly as `nvs-cli`'s own `front_end` does.
fn compile(src: &str) -> Outcome {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();

    let stmts = nvs_syntax::parse_file(map.file(id), &mut diags);
    nvs_syntax::check_declarations(&stmts, map.file(id), &mut diags);
    if diags.has_errors() {
        return Outcome::Diagnosed;
    }

    let module = nvs_hir::resolve_file(&stmts, map.file(id), &mut diags);
    if diags.has_errors() {
        return Outcome::Diagnosed;
    }

    let files = [nvs_types::ProgramFile {
        src: map.file(id),
        stmts: &stmts,
    }];
    let mut interner = nvs_types::TypeInterner::new();
    let mut exprs = nvs_types::ExprTypeTable::new();
    let enums = nvs_types::check_program(&files, &module, &mut interner, &mut exprs, &mut diags);
    if diags.has_errors() {
        return Outcome::Diagnosed;
    }

    let layouts = nvs_types::build_class_layouts(&files, &module.graph);
    let _ = nvs_ir::lower::lower_program("<script>", &files, &exprs, &interner, &enums, &layouts);
    Outcome::Lowered
}

#[test]
fn every_spellable_type_reaches_a_diagnostic_or_an_ir() {
    let mut surprises = Vec::new();
    let mut fixed = Vec::new();

    for &atom in ATOMS {
        for position in [Position::Param, Position::Return] {
            let expected_ice = KNOWN_ICE.contains(&(atom, position));
            let got = outcome(&position.fixture(atom));
            match (got, expected_ice) {
                (Outcome::Panicked, false) => {
                    surprises.push(format!("  `{atom}` as a {position:?} panics the compiler"));
                }
                (Outcome::Diagnosed | Outcome::Lowered, true) => {
                    fixed.push(format!("  `{atom}` as a {position:?}"));
                }
                _ => {}
            }
        }
    }

    assert!(
        surprises.is_empty(),
        "{} shape(s) type-check and then panic below the front end, with no row in \
         KNOWN_ICE and so no item in docs/agent/loop-goal.md owning them. Give each a \
         representation or a diagnostic — do NOT add a row here to make this pass; that \
         list may never grow.\n{}",
        surprises.len(),
        surprises.join("\n")
    );
    assert!(
        fixed.is_empty(),
        "{} shape(s) on KNOWN_ICE no longer panic. Delete their row(s), in the slice that \
         closed them, so the ratchet cannot slip back.\n{}",
        fixed.len(),
        fixed.join("\n")
    );
}
