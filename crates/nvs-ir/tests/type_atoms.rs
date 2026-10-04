//! The other half of [`refusals`](/crates/nvs-ir/tests/refusals.rs)' gate: **every shape a
//! program can spell reaches a diagnostic or an IR, and never a panic** — the
//! *types* `rule:types/grammar` admits, in the table this file opened with, and the
//! *expressions and statements* `nvs_syntax::ast` admits, in the roster below
//! it. One harness, two rosters, the same three outcomes and the same ratchet.
//!
//! # Why counting refusal sites was not enough
//!
//! `refusals.rs` counts *sites* and attributes each to the goal item whose
//! anchors sit in the same file. Both halves are blind in the same place: a
//! catch-all arm is one site however many shapes fall into it, and the item
//! that claims it is whichever one the file anchors — not whichever one the
//! shapes belong to. `crates/nvs-ir/src/lower/mod.rs`'s `erase_checked_ty` was
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
//! **it may never grow**, every entry is an open item in a goal's prose under
//! `docs/agent/goals/`, and an entry that stops panicking fails this test
//! until it is deleted — so a shape cannot be fixed and left on the list, and a
//! new one cannot be added to make a run go green.
//!
//! # What it does not cover yet
//!
//! The **local-declaration** position *per atom*, which reaches
//! `lower_decl_type` rather than `erase_checked_ty` — the other of item 25's
//! two sites. Building one needs a *value* of each type as well as the
//! annotation, which is a fixture per atom rather than a table row; the
//! expression roster below declares locals of a dozen types and so reaches
//! that site, but it is not the atom table walking through it. The two
//! positions here share `erase_checked_ty`, so they already cover what both
//! catch-alls receive; a session adding the third position should say so here
//! and delete this paragraph.

use std::panic::{AssertUnwindSafe, catch_unwind};

use nvs_diagnostics::{Diagnostics, SourceMap};

/// One row of `rule:types/grammar`'s `atom` production, plus the three composite forms
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
    // `rule:security/tainted-qualifier` and `rule:security/secret-qualifier`'s qualifiers, which add no representation and so
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
    // `rule:types/single-value-types`'s single-value types, `rule:types/object-top`'s shape, and a plain class name.
    "\"a\"",
    "7",
    "{a: int}",
    "Marker",
    // The composites: `rule:expressions/nullable-conversion`'s `?T`, a union, and an intersection.
    "?int",
    "int|string",
    "Marker&Other",
];

/// The shapes that still panic, as `(atom, position)`.
///
/// **This may never grow.** Every entry is an open item in a goal's prose
/// under `docs/agent/goals/`, and the test fails just as loudly on an entry
/// that has *stopped* panicking — the ratchet only turns one way.
///
/// **It is empty, and that is this table's finished state**: every atom ADR
/// 0007 § 3 spells reaches a diagnostic or an IR in both positions. The list
/// stays because the ratchet needs somewhere to name a row and because
/// emptying it is what closed the last one, not what retires the test.
///
/// The last two rows, for the reader who wonders what it was for. `never` in a
/// **parameter** is the finding it exists for — `rule:types/grammar` says `void` and
/// `never` are return-only and neither was refused there, so `never` panicked
/// here while `void` lowered and died one crate further down; `E0742` refuses
/// both at the declaration now (`nvs_types::signatures`). `never` in a
/// **return** was item 34's P4: it erases to `Ty::Void`, the representation of
/// a caller that receives nothing, and `nvs_ir::lower::erase_checked_ty`'s own
/// arm owns why the call site keeps its ordinary fall-through rather than
/// gaining a terminator of its own.
const KNOWN_ICE: &[(&str, Position)] = &[];

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
            // members are global names (`rule:classes/no-free-functions-or-constants`), and `new Core\Error(…)`
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

// ── The expression-and-statement half ────────────────────────────────────────
//
// Same three outcomes, same ratchet, a different roster: what a *program* is
// made of rather than what its declarations say. The rows below are read off
// `nvs_syntax::ast::ExprKind` and `StmtKind` — every variant a source file can
// spell has at least one — because a table assembled from memory covers the
// shapes its author happened to think of, which are the ones already working.

/// Where in a program the shape is written.
///
/// Three, because two of the shapes cannot be written in the same place as the
/// rest: a `yield` needs a body whose declared return makes it a generator, and
/// a declaration needs file scope.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Slot {
    /// Inside `Sub::run`, an ordinary method with `$this`, four parameters
    /// covering the receiver kinds that need a value, and an `int` return.
    Body,
    /// Inside `Sub::gen`, declared `Iterator<int>` and so a generator.
    Generator,
    /// After every declaration, at the script frame's own scope.
    File,
}

/// The declarations every fixture carries, whichever slot the row goes in.
///
/// One hierarchy of two levels (so `parent::`, `self::` and `static::` each
/// name something different), an interface holding the constant, an enum, and
/// a generator to drain — an unused declaration costs a row nothing, and a
/// missing one costs it a resolution error that reads like a finding.
const PREAMBLE: &str = r#"<?nvs
interface Greets {
    public const int LIMIT = 3;

    public function greet(): string;
}

enum Suit: int {
    Hearts = 1,
    Spades = 2,
}

class Base implements Greets {
    public static int $seen = 0;
    public int $count;

    public function constructor(int $count) {
        $this->count = $count;
    }

    public function greet(): string {
        return "base";
    }

    public static function make(): static {
        return new static(0);
    }

    public static function two(): Iterator<int> {
        yield 1;
        yield 2;
    }
}

final class Sub extends Base {
"#;

impl Slot {
    /// The whole fixture for `shape` in this slot.
    ///
    /// Every slot is present in every fixture, filled with something trivial
    /// where the row is not going, so the three differ only in where the row
    /// itself lands.
    fn fixture(self, shape: &str) -> String {
        let (body, generated, top) = match self {
            Self::Body => (shape, "yield 1;", ""),
            Self::Generator => ("", shape, ""),
            Self::File => ("", "yield 1;", shape),
        };
        format!(
            "{PREAMBLE}    public function run(?Base $maybe, mixed $any, array<int> $list, \
             callable $fn): int {{\n        {body}\n        return 0;\n    }}\n\n    \
             public function gen(): Iterator<int> {{\n        {generated}\n    }}\n}}\n\n{top}\n"
        )
    }

    /// How the row reads in a failure message.
    fn described(self) -> &'static str {
        match self {
            Self::Body => "in a method body",
            Self::Generator => "in a generator body",
            Self::File => "at file scope",
        }
    }
}

/// The shapes that reach IR from an ordinary method body.
///
/// Grouped by what they are, and each group is meant to be *complete* over its
/// `ExprKind`/`StmtKind` variants rather than representative.
const LOWERS_IN_A_BODY: &[&str] = &[
    // The literals, one per `ExprKind` that is one.
    "?int $v = null;",
    "bool $v = true;",
    "int $v = 7;",
    "float $v = 1.5;",
    "string $v = \"a\";",
    "string $v = \"n={$this->count}\";",
    "Core\\Time\\Duration $v = 1h;",
    "array<int> $v = [1, 2];",
    "var $v = {x: 1, y: 2};",
    "callable $v = fn(int $x): int => $x + 1;",
    // The operators.
    "int $v = -$this->count;",
    "int $v = 0; $v++; ++$v; $v--; --$v;",
    "int $v = 1 + 2 * 3;",
    "int $v = 1; $v += 2;",
    "int $v = $this->count > 0 ? 1 : 2;",
    "string $v = $this->count as string;",
    "bool $v = $this is Greets;",
    "int $v = (1 + 2);",
    "int $v = match ($this->count) { 0 => 1, default => 2 };",
    // Reads that name a member rather than a variable.
    "int $v = Base::LIMIT;",
    "int $v = self::LIMIT;",
    "string $v = Base::class;",
    "Base::$seen = 1; int $v = Base::$seen;",
    "Suit $v = Suit::Hearts;",
    "int $v = $this->count;",
    "int $v = $list[\"0\"];",
    // `new`, through each of the three spellings of the class.
    "Base $v = new Sub(1);",
    "Base $v = new self(1);",
    "Base $v = new static(1);",
    "LogicError $v = new LogicError(\"x\");",
    "Base $b = new Base(1); Base $v = clone $b;",
    // A call through each receiver kind. `$this`, a local, a temporary, the
    // three class-name spellings, a nullsafe chain, an erased receiver, and a
    // `Core` member — the row this table's own item names.
    "string $v = $this->greet();",
    "Base $b = new Base(1); string $v = $b->greet();",
    "string $v = Base::make()->greet();",
    "string $v = parent::greet();",
    "Base $v = self::make();",
    "Base $v = static::make();",
    "Base $v = Base::make();",
    "string $v = $maybe?->greet() ?? \"none\";",
    "mixed $v = $any->greet();",
    "string $v = Core\\Str::upper(\"a\");",
    "mixed $v = $fn();",
    "callable $v = Base::make(...);",
    // The remaining `ExprKind`s that are constructs rather than operators.
    "print \"x\";",
    "bool $v = isset($this->count);",
    "bool $v = empty($this->count);",
    "exit(1);",
    "mixed $v = require \"cfg.nvs\";",
    // Every statement form that is not a declaration.
    "$this->greet();",
    "return 1;",
    "{ int $v = 1; }",
    ";",
    "if ($this->count > 0) { echo \"a\"; } elseif ($this->count < 0) { echo \"b\"; } \
     else { echo \"c\"; }",
    "while ($this->count > 0) { break; }",
    "do { break; } while ($this->count > 0);",
    "for (int $i = 0; $i < 2; $i++) { continue; }",
    "foreach ($list as int $v) { echo $v; }",
    "foreach ($list as string $k => int $v) { echo $k, $v; }",
    "while ($this->count > 0) { foreach ($list as int $v) { break 2; } }",
    "switch ($this->count) { case 0: break; default: break; }",
    "try { throw new LogicError(\"x\"); } catch (LogicError $e) { echo \"c\"; } \
     finally { echo \"f\"; }",
    "throw new LogicError(\"x\");",
    "echo \"x\", \"\\n\";",
    "array<int> $l = [1]; unset($l[\"0\"]);",
    "array<int> $p = [1, 2]; [int $a, int $b] = $p;",
    // `rule:security/isolate-shares-nothing`'s two constructs, which lower to one `InstKind::CoreCall` each.
    "spawn script \"cfg.nvs\";",
    "var $h = spawn script \"cfg.nvs\"; var $r = await $h;",
];

/// The shapes an ordinary method body may spell and the front end refuses.
///
/// A refusal closes a shape as surely as a lowering does — the goal's own rule
/// — so these belong in the same table as the rows above, on the far side of
/// the same claim.
const REFUSED_IN_A_BODY: &[&str] = &[
    // `rule:security/isolate-shares-nothing`'s spawn whose `grants:` is a scalar where the option
    // takes a list of capability names — the construct itself lowers, one table up.
    "spawn script \"cfg.nvs\" with(grants: 7);",
    // The three PHP statement forms the AST still carries a variant for
    // because refusing a shape means parsing it first.
    "global $g;",
    "goto end;",
    "static int $n = 0;",
    // A bare global name is not a constant read.
    "int $v = LIMIT;",
];

/// The shapes that need a generator body to be spellable at all.
const LOWERS_IN_A_GENERATOR: &[&str] = &[
    "yield 1;",
    // `rule:iteration/one-way-only`'s own replacement for the row below.
    "foreach (Base::two() as int $v) { yield $v; }",
];

/// The delegation form `rule:iteration/one-way-only` refuses, in the one place it parses.
const REFUSED_IN_A_GENERATOR: &[&str] = &["yield from Base::two();"];

/// The declarations, plus the statement forms only a script frame has.
const LOWERS_AT_FILE_SCOPE: &[&str] = &[
    "class Extra { public static function f(): int { return 1; } }",
    "interface Ix { public function f(): int; }",
    "enum Ex { A = 1, B = 2 }",
    "type Alias = {path: string};",
    "echo \"top\";",
    "?>tail",
    "use Core\\Time\\Zone;",
    "autoload 'App' from './';",
];

/// The declarations a file may spell and the front end refuses.
const REFUSED_AT_FILE_SCOPE: &[&str] = &[
    // `rule:classes/no-free-functions-or-constants`: a function and a constant are class members, and there is
    // no file-scope spelling of either.
    "function f(): int { return 1; }",
    "const int X = 1;",
    // A file has one namespace, declared before every declaration, and this
    // slot comes after the preamble's classes.
    "namespace App;",
];

/// The five tables, each with the slot its rows go in and what it claims.
///
/// The claim is asserted in both directions: a row that stops lowering fails,
/// and so does a row that stops being refused — a table of shapes that all
/// quietly became parse errors would otherwise pass this test perfectly.
const SHAPE_TABLES: &[(&[&str], Slot, Outcome)] = &[
    (LOWERS_IN_A_BODY, Slot::Body, Outcome::Lowered),
    (REFUSED_IN_A_BODY, Slot::Body, Outcome::Diagnosed),
    (LOWERS_IN_A_GENERATOR, Slot::Generator, Outcome::Lowered),
    (REFUSED_IN_A_GENERATOR, Slot::Generator, Outcome::Diagnosed),
    (LOWERS_AT_FILE_SCOPE, Slot::File, Outcome::Lowered),
    (REFUSED_AT_FILE_SCOPE, Slot::File, Outcome::Diagnosed),
];

/// The shapes that still panic, by their source text.
///
/// **This may never grow**, on [`KNOWN_ICE`]'s terms exactly: every entry is an
/// open item in a goal's prose under `docs/agent/goals/`, and an entry that has stopped
/// panicking fails this test until it is deleted.
const SHAPE_ICE: &[&str] = &[];

#[test]
fn every_spellable_expression_reaches_a_diagnostic_or_an_ir() {
    let mut surprises = Vec::new();
    let mut fixed = Vec::new();
    let mut disagreed = Vec::new();

    for &(table, slot, want) in SHAPE_TABLES {
        for &shape in table {
            let expected_ice = SHAPE_ICE.contains(&shape);
            let got = outcome(&slot.fixture(shape));
            let at = slot.described();
            match (got, expected_ice) {
                (Outcome::Panicked, true) => {}
                (Outcome::Panicked, false) => {
                    surprises.push(format!("  `{shape}` {at} panics the compiler"));
                }
                (_, true) => fixed.push(format!("  `{shape}` {at}")),
                (got, false) if got != want => {
                    disagreed.push(format!("  `{shape}` {at} is {got:?}, not {want:?}"));
                }
                _ => {}
            }
        }
    }

    assert!(
        surprises.is_empty(),
        "{} shape(s) type-check and then panic below the front end, with no row in \
         SHAPE_ICE and so no item in docs/agent/loop-goal.md owning them. Give each a \
         lowering or a diagnostic — do NOT add a row here to make this pass; that list \
         may never grow.\n{}",
        surprises.len(),
        surprises.join("\n")
    );
    assert!(
        fixed.is_empty(),
        "{} shape(s) on SHAPE_ICE no longer panic. Delete their row(s), in the slice that \
         closed them, so the ratchet cannot slip back.\n{}",
        fixed.len(),
        fixed.join("\n")
    );
    assert!(
        disagreed.is_empty(),
        "{} shape(s) did what the table does not say they do. A row that lowered and now \
         reports a diagnostic is a regression; a row that was refused and now lowers is a \
         feature to move between the two tables in the slice that landed it.\n{}",
        disagreed.len(),
        disagreed.join("\n")
    );
}
