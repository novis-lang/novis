//! Lowering's own tests, one module deep so that `mod.rs` is the lowering
//! and not the evidence for it. The module path is `lower::tests`, so
//! `super` is `lower` here.

use insta::assert_snapshot;
use nvs_diagnostics::{Diagnostics, SourceId, SourceMap};
use nvs_syntax::ast::{ClassMemberKind, StmtKind as TopStmtKind};
use nvs_syntax::parse_file;

use super::*;
use crate::print::{print_function, print_program};

/// Parses `src`, runs it through `nvs_hir::resolve_file` and
/// `nvs_types::check_program` — lowering a call/`new` needs a real
/// [`ExprTypeTable`], and only a real check run produces one — then pulls
/// out `T`'s first method and lowers it.
fn lower_first_method(src: &str) -> (Function, SourceMap, SourceId) {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let module = nvs_hir::resolve_file(&stmts, map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let mut checked_types = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    let files = [nvs_types::ProgramFile {
        src: map.file(file),
        stmts: &stmts,
    }];
    let enums =
        nvs_types::check_program(&files, &module, &mut checked_types, &mut exprs, &mut diags);
    assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");

    let decl = stmts
        .iter()
        .find_map(|s| match &s.kind {
            TopStmtKind::ClassDecl(decl) if span_text(map.file(file), decl.name.span) == "T" => {
                Some(decl)
            }
            _ => None,
        })
        .expect("fixture must declare a class `T`");
    let method = decl
        .members
        .iter()
        .find_map(|m| match &m.kind {
            ClassMemberKind::Method(method) => Some(method),
            _ => None,
        })
        .expect("fixture class must declare a method");

    let name = span_text(map.file(file), method.name).to_owned();
    let f = lower_method(
        &name,
        method,
        map.file(file),
        &exprs,
        &checked_types,
        &enums,
    );
    (f.function, map, file)
}

/// [`lower_script_src`] as the program's entry frame, which is what every
/// snapshot below reads.
fn lower_script_src(src: &str) -> (Function, SourceMap, SourceId) {
    lower_script_src_as(src, ScriptRole::Entry)
}

/// Parses, resolves and checks `src` exactly as [`lower_first_method`]
/// does, then lowers the file's *own* top-level statements through
/// [`lower_script`] instead of pulling a method out of a class.
fn lower_script_src_as(src: &str, role: ScriptRole) -> (Function, SourceMap, SourceId) {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let module = nvs_hir::resolve_file(&stmts, map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let mut checked_types = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    let files = [nvs_types::ProgramFile {
        src: map.file(file),
        stmts: &stmts,
    }];
    let enums =
        nvs_types::check_program(&files, &module, &mut checked_types, &mut exprs, &mut diags);
    assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");

    let f = lower_script(
        "<script>",
        &stmts,
        map.file(file),
        &exprs,
        &checked_types,
        &enums,
        role,
    );
    (f.function, map, file)
}

/// The whole file lowered — every function and every class, which is
/// what a generator needs: one declaration becomes three functions plus a
/// synthesized class, and a snapshot of any one of them alone would hide
/// how they fit together.
fn lower_program(src: &str) -> (crate::ir::Program, SourceMap, SourceId) {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let module = nvs_hir::resolve_file(&stmts, map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let mut checked_types = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    let files = [nvs_types::ProgramFile {
        src: map.file(file),
        stmts: &stmts,
    }];
    let enums =
        nvs_types::check_program(&files, &module, &mut checked_types, &mut exprs, &mut diags);
    assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
    let layouts = nvs_types::layout::build_class_layouts(&files, &module.graph);
    let p = lower_file(
        "<script>",
        &stmts,
        map.file(file),
        &exprs,
        &checked_types,
        &enums,
        &layouts,
    );
    (p, map, file)
}

/// Every function kind polls at its entry right after its last parameter,
/// and every poll has an error edge, so a stop there releases the arguments
/// (`rule:errors/stack-depth`). The fixture has a script frame, a method, a
/// property hook, an anonymous function, a method reference, a generator and
/// two delegation forwards. Every program also has the exception
/// constructors.
#[test]
fn every_function_kind_polls_with_an_error_edge() {
    let (p, _, _) = lower_program(concat!(
        "<?nvs\n",
        "interface Greeter {\n",
        "  public function greet(string $s): string;\n",
        "}\n",
        "class Polite implements Greeter {\n",
        "  public function greet(string $s): string { return $s; }\n",
        "}\n",
        "class Plain implements Greeter by $inner {\n",
        "  public function constructor(public Polite $inner) {}\n",
        "}\n",
        "class Late implements Greeter by $inner {\n",
        "  public lateinit Polite $inner;\n",
        "}\n",
        "class T {\n",
        "  public int $n = 0;\n",
        "  public int $twice {\n",
        "    get => $this->n * 2;\n",
        "  }\n",
        "  public function m(string $s): string { return $s; }\n",
        "  public function count(int $to): Iterator<int> {\n",
        "    for (int $i = 0; $i < $to; $i++) {\n",
        "      yield $i;\n",
        "    }\n",
        "  }\n",
        "}\n",
        "T $t = new T();\n",
        "var $f = fn(string $s): string => $s;\n",
        "var $r = $t->m(...);\n",
        "echo $f(\"a\"), $r(\"b\"), $t->twice;\n",
    ));

    let mut kinds = std::collections::BTreeSet::new();
    for f in &p.functions {
        let entry = f
            .blocks
            .iter()
            .find(|b| b.id == f.entry)
            .unwrap_or_else(|| panic!("`{}` has no entry block", f.name));
        // The captures an anonymous function reads may sit between, because
        // no such read can fail. What matters is that every parameter is
        // bound and nothing that can stop the frame runs before the poll.
        let poll = entry
            .insts
            .iter()
            .position(|inst| matches!(inst.kind, InstKind::Safepoint))
            .unwrap_or_else(|| panic!("`{}`'s entry block has no poll", f.name));
        assert!(
            entry.insts[poll..]
                .iter()
                .all(|inst| !matches!(inst.kind, InstKind::Param(_))),
            "`{}` binds a parameter after its entry poll",
            f.name
        );
        assert!(
            entry.insts[..poll]
                .iter()
                .all(|inst| inst.on_error.is_none()),
            "`{}` runs a fallible instruction before its entry poll",
            f.name
        );
        for block in &f.blocks {
            for inst in &block.insts {
                if matches!(inst.kind, InstKind::Safepoint) {
                    assert!(
                        inst.on_error.is_some(),
                        "`{}` polls with no error edge",
                        f.name
                    );
                }
            }
        }
        let kind = match f.name.rsplit_once("::").map_or("", |(_, member)| member) {
            "" => "script",
            "m" => "method",
            "constructor" if f.name.starts_with("Plain") => "constructor",
            "constructor" => "exception constructor",
            "greet" if !f.name.starts_with("Polite") => "delegation forward",
            FN_INVOKE => "anonymous function or method reference",
            GEN_ADVANCE | GEN_CURRENT_METHOD | GEN_UNWIND_METHOD | "count" => "generator",
            "get" if f.name.contains("$twice") => "property hook",
            _ => "other",
        };
        kinds.insert(kind);
    }
    for kind in [
        "script",
        "method",
        "constructor",
        "exception constructor",
        "delegation forward",
        "anonymous function or method reference",
        "generator",
        "property hook",
    ] {
        assert!(
            kinds.contains(kind),
            "the fixture lowered no {kind}: {kinds:?}"
        );
    }
}

/// `echo "Hello, World!";` as a whole script, lowered: one
/// synthesized frame, no receiver parameter, a `ConstStr` handed straight
/// to `Helper::EchoStr`, and the literal released right after the write
/// reads it (nothing else ever owns it).
#[test]
fn a_script_body_echoes_a_string_literal() {
    let (f, map, file) = lower_script_src("<?nvs\necho \"Hello, World!\";\n");
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `throw new LogicError(…)` allocates an ordinary object, runs the
/// synthesized root constructor on it, stamps the throw site into
/// `location`, hands it to the context, and enters a landing block that
/// names the frame the throw is leaving.
#[test]
fn a_throw_builds_an_exception_object_and_enters_its_landing_block() {
    let (f, map, file) = lower_script_src("<?nvs\nthrow new LogicError(\"boom\");\n");
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Every failing call inside a `try` reaches the same dispatch block,
/// each through its own landing block — which is what keeps a handler
/// phi's predecessors distinct. The exception is taken once by
/// `take.thrown`, tested by one class test per clause, and the clause's
/// binding is released where its body ends.
#[test]
fn a_try_gives_every_protected_call_its_own_landing_block() {
    let (f, map, file) = lower_script_src(
        "<?nvs\nclass T {\n  public static function go(): void { }\n}\n\
             try {\n  T::go();\n  echo \"fine\";\n} catch (Throwable $e) {\n  \
             echo $e->message;\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Two clauses on one `try`: a class-test chain in source order, and a
/// re-raise of the very same reference when neither matches.
#[test]
fn two_catch_clauses_lower_to_a_class_test_chain_ending_in_a_rethrow() {
    let (f, map, file) = lower_script_src(
        "<?nvs\nclass T {\n  public static function go(): void { }\n}\n\
             try {\n  T::go();\n} catch (LogicError $a) {\n  echo \"logic\";\n\
             } catch (IOError $b) {\n  echo \"io\";\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `try { return … } finally { … }` — the `finally` body is lowered once
/// per exit, so the `return` runs its own copy before leaving the frame
/// and the exception path runs a second one before re-raising.
#[test]
fn a_finally_is_lowered_once_per_exit_out_of_the_protected_region() {
    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
  function m(): int {
    try {
      return T::inner();
    } finally {
      echo \"done\";
    }
  }
  static function inner(): int { return 1; }
}
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Outside a `try`, a landing block releases the frame's live refcounted
/// locals before the status travels onward — stated as IR rather than left
/// to the backend, so a `THROWN` sweeps the frame without `nvs-codegen`
/// having to know which locals are live. The tests below extend the same
/// sweep to every other non-`OK` status and to the `try`-protected exit.
#[test]
fn a_propagating_landing_block_releases_the_frames_live_strings() {
    let (f, map, file) = lower_script_src(
        "<?nvs\nclass T {\n  public static function go(): void { }\n}\n\
             string $s = \"held\";\nT::go();\necho $s;\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `Core` member borrows its arguments, so a materialized `string`
/// literal is this frame's own temporary — and the helper is exactly the
/// thing that can throw while it is in flight.
/// [`Lowering::landing_block`] releases the whole
/// [`Lowering::owned_temporaries`] stack on the error edge, and this pins
/// the harder half: the edge that reaches a `catch` in this same frame,
/// where the locals sweep deliberately releases nothing. A temporary has
/// no `Env` entry for the handler to find it through, so the landing block
/// is the last place anything can drop it.
#[test]
fn a_landing_block_releases_the_call_temporaries_still_in_flight() {
    let (f, map, file) = lower_script_src(
        "<?nvs\ntry {\n  bytes $b = Core\\Encoding::fromHex(\"ff\");\n}\
             \ncatch (Throwable $e) {\n  echo \"caught\";\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A file-scope local is an ordinary local of the synthesized frame
/// (`rule:statements/storage-that-outlives-a-call`) — declared, reassigned and read with exactly the
/// machinery a method body already uses. `echo` of an `int` converts
/// through the same `Helper::IntToString` `.` concatenation uses.
#[test]
fn a_script_body_local_is_an_ordinary_local() {
    let (f, map, file) = lower_script_src("<?nvs\nint $n = 1;\n$n = $n + 2;\necho $n;\n");
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A declaration is skipped by the script frame — `T`'s method is
/// `lower_method`'s job, not this walk's. The walk still enters a
/// `namespace X { ... }` block, since a namespace scopes names and not
/// storage, but no such block reaches this pass: the parser refuses the
/// braced form outright (`E0243`), so that arm is defensive and has no
/// fixture to pin it.
#[test]
fn a_script_body_skips_declarations() {
    let (f, map, file) =
        lower_script_src("<?nvs\nclass T {\n  function m(): void { }\n}\necho \"after\";\n");
    assert_snapshot!(print_function(&f, map.file(file)));
}

#[test]
fn straight_line_arithmetic_and_return() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function add(int $a, int $b): int {\n    int $sum = $a + $b;\n    return $sum;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `rule:types/arithmetic`'s "either operand a `float`" row is settled *here*, by a
/// conversion emitted ahead of the operator, and not by `nvs-codegen`
/// repairing a `BinOp` whose two operands disagree — that crate's "a
/// `BinOp` has one representation" invariant stays intact, and its
/// mismatch refusal stays a genuine internal error.
///
/// The conversion is the checked one `$n as float` writes
/// (`Helper::IntToFloat`/`UintToFloat`), so it carries `rule:errors/propagation`'s error
/// edge — `rule:types/conversion` names this as the language's one implicit
/// conversion and says it throws above 2^53 rather than rounding.
///
/// Read off the rendering rather than snapshotted: what is pinned is the
/// *order* of two instructions and which of them can throw, and a
/// snapshot would go red for an unrelated renumbering while saying
/// nothing about either.
#[test]
fn a_mixed_numeric_pair_converts_before_the_operator() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function widen(int $i, uint $u, float $f): float {\n",
        "    float $a = $i + $f;\n",
        "    float $b = $f * $u;\n",
        "    return $a + $b;\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle} is not in the lowering: {text}"))
    };
    assert!(
        at("int_to_float") < at("= add "),
        "the `int` side widened after its operator: {text}"
    );
    assert!(
        at("uint_to_float") < at("= mul "),
        "the `uint` side widened after its operator: {text}"
    );
    for line in text.lines() {
        if line.contains("int_to_float") {
            assert!(
                line.contains(" ! bb"),
                "the implicit widening lost its error edge: {line}"
            );
        }
    }
}

/// An `int` stored at a union that holds `float` and not `int` is converted
/// before it is tagged, and a `?int` value stored there goes through the
/// helper that converts an integer tag. Both carry the error edge, because
/// both throw above 2^53. A union that names `int` converts nothing.
#[test]
fn an_int_at_a_union_holding_float_converts_before_it_is_tagged() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function widen(int $i, ?int $m): ?float {\n",
        "    int|float $kept = $i;\n",
        "    ?float $a = $i;\n",
        "    float|string $b = $m ?? 'none';\n",
        "    return $m;\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    let lines = |needle: &str| {
        text.lines()
            .filter(|line| line.contains(needle))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        lines("helper.int_to_float").len(),
        1,
        "only the `?float` local converts the `int`: {text}"
    );
    assert_eq!(
        lines("tagged_widen_to_float").len(),
        2,
        "the `float|string` local and the `?float` return each convert a tag: {text}"
    );
    for line in lines("helper.int_to_float")
        .into_iter()
        .chain(lines("tagged_widen_to_float"))
    {
        assert!(
            line.contains(" ! bb"),
            "the widening lost its error edge: {line}"
        );
    }
}

/// The other side of that bound: `rule:types/arithmetic`'s *ordering* rows are
/// exact in the mathematical integers, so a mixed numeric pair under
/// `<` is answered by `Helper::NumericLt` and pays no conversion at
/// all — a widening there would raise `ArithmeticError` past 2^53 for
/// a pair that orders perfectly well.
#[test]
fn a_mixed_numeric_comparison_pays_no_widening() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function order(int $i, float $f): bool {\n",
        "    return $i < $f;\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    assert!(text.contains("numeric_lt"), "{text}");
    assert!(
        !text.contains("int_to_float"),
        "an ordering row widened an operand that can throw: {text}"
    );
}

/// `rule:types/arithmetic`'s bitwise rows all reach an instruction, and only the ones
/// PHP can refuse carry an error edge.
///
/// Read off the rendering rather than snapshotted, because what is being
/// pinned is *which* rows are fallible — a snapshot would go red for any
/// unrelated renumbering and say nothing about that.
#[test]
fn every_bitwise_operator_lowers() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function mix(int $a, int $b): int {\n",
        "    int $out = ((($a & $b) | 8) ^ 1) << 2;\n",
        "    return ($out >> 1) + ~$a;\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    for name in ["band", "bor", "bxor", "shl", "shr", "bnot"] {
        assert!(text.contains(name), "{name} did not lower: {text}");
    }
    // A shift's *count* is the only thing PHP refuses here — a negative
    // one throws `ArithmeticError` — so `<<` and `>>` take `rule:errors/propagation`'s edge
    // while the total operators and `~` do not.
    for line in text.lines() {
        let fallible = line.contains(" ! bb");
        for (name, expected) in [
            ("= band ", false),
            ("= bor ", false),
            ("= bxor ", false),
            ("= bnot ", false),
            ("= shl ", true),
            ("= shr ", true),
        ] {
            if line.contains(name) {
                assert_eq!(fallible, expected, "{name}error edge: {line}");
            }
        }
    }
}

/// `$x op= e` is `$x = $x op e`, so the bitwise compound forms need no
/// lowering of their own — `lower_compound_assignment`'s rewrite is what
/// gives them one, and this is the test that says so rather than a second
/// table in the lowering.
#[test]
fn a_bitwise_compound_assignment_lowers_through_its_binary_form() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function mask(int $x): int {\n",
        "    $x &= 3;\n    $x |= 8;\n    $x ^= 1;\n    $x <<= 2;\n    $x >>= 1;\n",
        "    return $x;\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    for name in ["band", "bor", "bxor", "shl", "shr"] {
        assert!(
            text.contains(name),
            "the compound form of {name} did not lower: {text}"
        );
    }
}

/// `$x++` and `++$x` are the same *statement*: the operator's position
/// decides which of the read-modify-write's two values a surrounding
/// expression sees, and an expression statement sees neither. So all four
/// spellings go through the one [`Lowering::lower_incdec_stmt`] and two
/// adds and two subs is the whole shape.
///
/// Counted rather than snapshotted, for
/// [`a_bitwise_compound_assignment_lowers_through_its_binary_form`]'s
/// reason: a snapshot would go red for a renumbering while saying nothing
/// about the two spellings agreeing.
#[test]
fn an_increment_lowers_in_either_position() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function step(int $x): int {\n",
        "    $x++;\n    ++$x;\n    $x--;\n    --$x;\n",
        "    return $x;\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("= add ").count(), 2, "{text}");
    assert_eq!(text.matches("= sub ").count(), 2, "{text}");
}

/// The `$t = $t ⊕ e` rewrite writes its target down twice, so a target
/// with a call in it would *call* twice — `f()->count += 1` incrementing
/// the field of one object and then discarding a second one.
/// [`Lowering::lower_read_modify_write`] lowers the address once and
/// stages it ([`Lowering::staged_targets`]), which is what this counts:
/// one call, and the field read and written off the value it produced.
#[test]
fn a_compound_assignment_evaluates_its_target_once() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  public int $count = 0;\n",
        "  function bump(): void {\n",
        "    $this->box()->count += 1;\n",
        "  }\n",
        "  function box(): T { return $this; }\n",
        "}\n",
    ));
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("call T::box").count(), 1, "{text}");
    assert_eq!(text.matches("field.get").count(), 1, "{text}");
    assert_eq!(text.matches("field.set").count(), 1, "{text}");
}

/// `rule:types/arithmetic` gives `**` a row for every numeric representation but the
/// `decimal` `rule:types/arithmetic` refuses, and lists it beside `+`, `-` and `*` — so the two integer
/// rows throw and the `float` one, being `f64::powf`, cannot.
///
/// Counted rather than snapshotted, and for the reason
/// [`every_bitwise_operator_lowers`] gives: what is pinned is *which* rows
/// carry `rule:errors/propagation`'s edge, and a snapshot would go red for a renumbering
/// while saying nothing about that. `**=` is in the same body because it
/// has no lowering of its own — `lower_compound_assignment`'s rewrite is
/// what gives it one — so the count is what says it arrived.
#[test]
fn a_power_operator_lowers_over_every_numeric_row() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function rows(int $i, uint $u, float $x): float {\n",
        "    int $a = $i ** 3;\n",
        "    uint $b = $u ** $u;\n",
        "    float $c = $x ** 2.0;\n",
        "    $a **= 2;\n",
        "    return $c;\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    let powers: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("= pow "))
        .collect();
    assert_eq!(
        powers.len(),
        4,
        "one `pow` per row and one for `**=`: {text}"
    );
    let fallible = powers.iter().filter(|line| line.contains(" ! bb")).count();
    assert_eq!(
        fallible, 3,
        "the two `int` rows and the `uint` one throw, the `float` one does not: {text}"
    );
}

/// A plain local reassignment gets a fresh SSA value rather than mutating
/// the one already bound to `$n` — the point of routing even
/// straight-line reassignment through `lower_reassignment`.
#[test]
fn reassignment_produces_a_fresh_ssa_value() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function bump(int $n): int {\n    int $out = $n;\n    $out = $out + 1;\n    return $out;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Unary negation/not and a comparison operator, over a `uint`-defaulted
/// bare literal (`rule:types/arithmetic`) — exercises the operators the arithmetic
/// test above doesn't.
#[test]
fn unary_and_comparison_operators() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function check(int $n): bool {\n    bool $neg = -$n < 0;\n    return !$neg;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `uint` local initialized from a bare integer literal takes the
/// literal as `uint`, not `int` — `rule:types/arithmetic`'s target-directed rule,
/// mirrored from `nvs_types::expr::infer`.
#[test]
fn a_bare_literal_targeting_uint_is_lowered_as_uint() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): uint {\n    uint $n = 1;\n    return $n;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `if`/`else`, both branches reassigning the same local — the plain
/// two-predecessor merge, needing one real phi.
#[test]
fn if_else_merges_with_a_phi() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function pick(bool $c, int $a, int $b): int {\n    int $r = 0;\n    if ($c) {\n      $r = $a;\n    } else {\n      $r = $b;\n    }\n    return $r;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `if` with no `else` — the false edge lands on the merge block
/// directly, carrying the pre-branch environment.
#[test]
fn if_with_no_else_merges_the_implicit_edge() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function clamp(int $n): int {\n    int $r = $n;\n    if ($r < 0) {\n      $r = 0;\n    }\n    return $r;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Both branches of an `if` `return` — the merge block has no real
/// predecessor and is dead, but still needs a well-formed terminator.
#[test]
fn if_else_both_returning_leaves_a_dead_merge_block() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function abs(int $n): int {\n    if ($n < 0) {\n      return -$n;\n    } else {\n      return $n;\n    }\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `while` loop reassigning two pre-existing locals in its body — each
/// needs its own loop-header phi, patched with the back-edge value.
#[test]
fn while_loop_carries_locals_through_a_header_phi() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function sum(int $n): int {\n    int $total = 0;\n    int $i = 0;\n    while ($i < $n) {\n      $total = $total + $i;\n      $i = $i + 1;\n    }\n    return $total;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `if ($n)` with an `int` parameter — `rule:expressions/truthy-positions`'s truthy table for a
/// scalar condition, converted through `Helper::IntTruthy` rather than
/// requiring `$n` already be `bool`.
#[test]
fn an_int_condition_converts_through_a_truthy_helper() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(int $n): bool {\n    if ($n) {\n      return true;\n    }\n    return false;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `while ($s)` with a `string` parameter — the same table's `string`
/// row (`Helper::StrTruthy`), exercised through `while` rather than
/// `if` to confirm `Lowering::lower_while` routes through the same
/// `Lowering::lower_truthy_cond` helper.
#[test]
fn a_string_while_condition_converts_through_a_truthy_helper() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(string $s): void {\n    while ($s) {\n      $s = \"\";\n    }\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `if ($a)` with an `array<int>` parameter — `rule:expressions/truthy-positions`'s "empty is
/// falsy, regardless of element type" row, via `Helper::ArrayTruthy`.
/// `$a` is a bare variable read (`is_aliasing_read`), so no release
/// follows the helper call — the array is still the parameter's own
/// slot, released normally at scope exit.
#[test]
fn an_array_condition_converts_through_a_truthy_helper() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(array<int> $a): bool {\n    if ($a) {\n      return true;\n    }\n    return false;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `if (self::make())` where `make` returns a fresh `array<int>` — unlike
/// the parameter case above, this array has no other owner once the
/// truthy check reads it, so `Lowering::lower_truthy_cond` must release
/// it right after, the same "release a fresh value once its one and only
/// use is done" precedent `Self::concat_operand`'s own caller already
/// sets for `.` concatenation.
#[test]
fn a_fresh_array_condition_is_released_after_the_truthy_check() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): bool {\n    if (self::make()) {\n      return true;\n    }\n    return false;\n  }\n  static function make(): array<int> {\n    return [1];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `if ($f)` with a class-typed parameter — `rule:enums/truthiness` makes a class
/// instance always truthy, so this needs no `HelperCall` at all: it
/// folds straight to a fresh `const.bool true`.
#[test]
fn an_object_condition_is_always_truthy() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(Foo $f): bool {\n    if ($f) {\n      return true;\n    }\n    return false;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// The narrowest `Ty::Tagged` round-trip: a `mixed`-typed parameter,
/// returned straight back through the bare-`$name`-return transfer-out
/// path, exercising `lower_decl_type`'s `TypeAtom::Mixed` arm for both the
/// parameter and the return type with `Ty::is_refcounted` correctly
/// reporting `false` (no retain/release appears anywhere in the snapshot).
#[test]
fn a_mixed_parameter_round_trips_through_return() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function pick(mixed $x): mixed {\n    return $x;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `mixed $y = $x;` — an explicitly `mixed`-typed local declared from a
/// `mixed` parameter, then returned. Exercises `Lowering::bind_local`
/// with a `Ty::Tagged` binding: still no retain, since `Ty::Tagged` is not
/// `is_refcounted`, and the local correctly excludes itself from
/// `release_all_locals`'s exit sweep by transferring out on `return`,
/// exactly like any other bare-variable return.
#[test]
fn a_typed_mixed_local_round_trips() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function pick(mixed $x): mixed {\n    mixed $y = $x;\n    return $y;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `var $y = $x;` (`rule:types/var-inference`) with a `mixed`-typed initializer — `var`'s
/// own inference path (`lower_expr` with `expected: None`) picks up
/// `Ty::Tagged` from the initializer exactly the way it already does for
/// any other representation, needing no `var`-specific handling.
#[test]
fn a_var_local_infers_mixed_from_a_mixed_initializer() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function pick(mixed $x): mixed {\n    var $y = $x;\n    return $y;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Passing a `mixed` local as a call argument round-trips too — the
/// callee's own `mixed` parameter is just another local, released at the
/// callee's own (trivial, no-op) exit, mirroring every other
/// representation's call-argument boundary.
#[test]
fn passing_a_mixed_local_as_a_call_argument_round_trips() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(mixed $x): mixed {\n    return $this->identity($x);\n  }\n  function identity(mixed $v): mixed {\n    return $v;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `mixed` condition is `rule:expressions/truthy-table`'s last table row: the dispatch it
/// names moves into `Helper::ValueTruthy`, which reads the operand's tag
/// and applies whichever of the rows above it names. What the snapshot
/// pins is that one helper call, and no untag anywhere: an unchecked one
/// over an `int` payload is a pointer the next instruction would
/// dereference.
#[test]
fn a_mixed_condition_dispatches_the_truthy_table_on_the_tag() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(mixed $x): bool {\n    if ($x) {\n      return true;\n    }\n    return false;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `bytes` is the one row `rule:expressions/truthy-table` does not take from PHP, which has
/// no such type: falsy iff empty, dropping the one-octet `"0"` case that
/// exists for a `string` only because PHP reads one as a possible number.
/// The snapshot pins that a declared `bytes` reaches `Helper::BytesTruthy`
/// rather than `Helper::StrTruthy`, which is the whole of the difference —
/// the two heap shapes are identical, so a mis-routed operand would still
/// run and answer wrongly on one input.
#[test]
fn a_bytes_condition_takes_its_own_truthy_helper() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bytes $b): bool {\n    if ($b) {\n      return true;\n    }\n    return false;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `for` loop: the initializer runs before the header, the step runs in
/// a block of its own between the body and the header, and the loop
/// variable's header phi is patched from that step block — see
/// [`Lowering::lower_for`].
#[test]
fn for_loop_runs_its_step_in_a_block_between_body_and_header() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function sum(int $n): int {\n    int $total = 0;\n    int $i = 0;\n    for ($i = 0; $i < $n; $i += 1) {\n      $total += $i;\n    }\n    return $total;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `continue` in a `for` jumps to the step block rather than the header,
/// so the step still runs on that path — the one structural difference
/// between [`Lowering::lower_for`] and [`Lowering::lower_while`].
#[test]
fn for_loop_continue_reaches_the_step_block() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function count(int $n): int {\n    int $hits = 0;\n    int $i = 0;\n    for ($i = 0; $i < $n; $i += 1) {\n      if ($i == 2) {\n        continue;\n      }\n      $hits += 1;\n    }\n    return $hits;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `for` whose body always returns: nothing reaches the step block, so
/// it is sealed with the environment the loop was entered with rather
/// than a merge of edges that do not exist. See [`Lowering::lower_for`].
#[test]
fn for_loop_whose_body_always_returns_leaves_a_dead_step_block() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function head(int $n): int {\n    int $i = 0;\n    for ($i = 0; $i < $n; $i += 1) {\n      return $i;\n    }\n    return -1;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `switch`: one subject, an equality chain over the labels, and a
/// `default` reached by the chain's own fall-off — see
/// [`Lowering::lower_switch`].
#[test]
fn switch_lowers_to_an_equality_chain_ending_at_the_default() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function rank(int $n): int {\n    switch ($n) {\n      case 1:\n        return 10;\n      case 2:\n        return 20;\n      default:\n        return 0;\n    }\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Fallthrough is the absence of a `break`, so a body that reaches its end
/// jumps into the next body block rather than past the switch, and a
/// `break` jumps to the after-block the frame carries.
#[test]
fn switch_falls_through_a_body_with_no_break() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function pick(int $n): int {\n    int $hits = 0;\n    switch ($n) {\n      case 1:\n      case 2:\n        $hits += 1;\n        break;\n      default:\n        $hits += 9;\n    }\n    return $hits;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `string` subject is retained for the length of the switch and
/// released once in the after-block, the same shape
/// [`Lowering::lower_foreach`] gives the array it walks.
#[test]
fn a_switch_over_a_string_holds_one_reference_to_its_subject() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function tier(string $s): int {\n    int $out = 0;\n    switch ($s) {\n      case \"a\":\n        $out = 1;\n        break;\n    }\n    return $out;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `continue` written inside a `switch` inside a loop continues the
/// **loop** — the switch's frame carries no continue target, so
/// [`Lowering::lower_continue`] walks past it. PHP's own bare `continue`
/// there means `break`; see [`Lowering::lower_switch`] for why Novis takes
/// the meaning PHP's warning points at instead.
#[test]
fn continue_inside_a_switch_reaches_the_enclosing_loops_header() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function count(int $n): int {\n    int $hits = 0;\n    int $i = 0;\n    while ($i < $n) {\n      $i += 1;\n      switch ($i) {\n        case 2:\n          continue;\n        default:\n          $hits += 1;\n      }\n    }\n    return $hits;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `match` is [`Lowering::lower_switch`]'s chain producing a value: each
/// arm ends in a jump to one merge block, and the arms join in a phi the
/// way a ternary's two branches do.
#[test]
fn match_arms_join_in_a_phi_at_one_merge_block() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function name(int $n): string {\n    return match ($n) {\n      1, 2 => \"low\",\n      default => \"high\",\n    };\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// With no `default` arm the chain's fall-off raises a `LogicError`
/// instead of reaching the merge — `nvs_hir::errors`' closed tree has no
/// `UnhandledMatchError` to raise.
#[test]
fn a_match_with_no_default_throws_where_the_chain_runs_out() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function name(int $n): string {\n    return match ($n) {\n      1 => \"one\",\n    };\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `match` that names every case of its enum subject lowers no fallback
/// throw, and does not compare its last label
/// (`rule:expressions/a-match-over-an-enum-covers-every-case`,
/// `rule:programs/less-work-is-taken`). The count is the proof: three cases
/// make two branches and no throw, where the same `match` with a `default`
/// arm makes three branches.
#[test]
fn a_match_over_every_case_lowers_no_fallback_throw() {
    let count = |src: &str| {
        let (f, _, _) = lower_first_method(src);
        let branches = f
            .blocks
            .iter()
            .filter(|b| matches!(b.term, Terminator::Branch { .. }))
            .count();
        let throws = f
            .blocks
            .iter()
            .filter(|b| matches!(b.term, Terminator::Throw { .. }))
            .count();
        (branches, throws)
    };
    let covering = count(
        "<?nvs\nenum Rank { Bronze, Silver, Gold }\nclass T {\n  function name(Rank $r): string {\n    return match ($r) {\n      Rank::Bronze => \"b\",\n      Rank::Silver => \"s\",\n      Rank::Gold => \"g\",\n    };\n  }\n}\n",
    );
    let with_default = count(
        "<?nvs\nenum Rank { Bronze, Silver, Gold }\nclass T {\n  function name(Rank $r): string {\n    return match ($r) {\n      Rank::Bronze => \"b\",\n      Rank::Silver => \"s\",\n      Rank::Gold => \"g\",\n      default => \"x\",\n    };\n  }\n}\n",
    );
    assert_eq!(
        covering,
        (2, 0),
        "a covering match compares two labels and throws nothing"
    );
    assert_eq!(
        with_default,
        (3, 0),
        "a `default` arm keeps every comparison"
    );
}

/// `new Foo(1)` with a resolved one-parameter constructor — the class's
/// own resolved target and the constructor's argument both come from the
/// typed-expression table (`ExprInfo::New`), not from re-deriving `Foo`'s
/// signature by hand.
#[test]
fn new_with_a_resolved_constructor() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  function constructor(int $x) {}\n}\nclass T {\n  function make(): Foo {\n    return new Foo(1);\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `new Foo()` against a class with no explicit `constructor` — the
/// `ExprInfo::New` entry's `ctor` is `None`, so lowering emits an empty
/// argument list rather than looking one up.
#[test]
fn new_with_no_declared_constructor() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {}\nclass T {\n  function make(): Foo {\n    return new Foo();\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `self::make()` — a static call with no receiver, resolved to `T`'s own
/// method via the typed-expression table.
#[test]
fn a_self_static_call_with_a_scalar_return() {
    // `m` declared first, forward-referencing `make` — `lower_first_method`
    // lowers `T`'s *first* method, and Novis resolves a same-class method
    // call regardless of declaration order (its signature table is built
    // in a pass ahead of body-checking; see `nvs_types::signatures`).
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): int {\n    return self::make(1);\n  }\n  static function make(int $n): int {\n    return $n;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A local declared with a class type, initialized from `new` and
/// returned — exercises `lower_decl_type`'s `TypeAtom::Name` arm
/// alongside `ExprInfo::New`.
#[test]
fn a_class_typed_local_initialized_from_new() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {}\nclass T {\n  function make(): Foo {\n    Foo $x = new Foo();\n    return $x;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$this->a(1)` — the implicit receiver seeded by `lower_method` (SSA
/// value 0, parameter index 0) flows into `InstKind::Call`'s `receiver`
/// field via the same `ExprKind::Variable`/`Env` lookup any other local
/// uses; nothing about `MethodCall`'s own lowering is `$this`-specific.
#[test]
fn a_this_method_call() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): int {\n    return $this->a(1);\n  }\n  function a(int $x): int {\n    return $x;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj->greet()` — an instance call through a receiver that isn't
/// `$this` at all, on a class with no explicit parameters, to exercise
/// the general `object` lowering path rather than only the `$this`
/// special case.
#[test]
fn an_instance_method_call_through_a_local_receiver() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  function greet(): int {\n    return 1;\n  }\n}\nclass T {\n  function m(): int {\n    Foo $obj = new Foo();\n    return $obj->greet();\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj?->greet()` on a receiver that may be `null` — one `is.null` over
/// the tagged receiver, the call in the arm where it isn't, and a `null`
/// in the arm where it is, merged by a `phi`. See
/// `Lowering::open_nullsafe`.
#[test]
fn a_nullsafe_method_call_on_a_nullable_receiver() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  function greet(): int {\n    return 1;\n  }\n}\nclass T {\n  function m(?Foo $obj): ?int {\n    return $obj?->greet();\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// The same call on a receiver whose *representation* rules `null` out
/// costs nothing: no branch, no tag test, exactly the instructions `->`
/// emits — which is also why `nvs_types` gives it no `null` in its type.
#[test]
fn a_nullsafe_method_call_on_a_receiver_that_cannot_be_null() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  function greet(): int {\n    return 1;\n  }\n}\nclass T {\n  function m(): int {\n    Foo $obj = new Foo();\n    return $obj?->greet();\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `if ($obj != null) { $obj->greet(); }` — the receiver's slot is still
/// one tagged value, so the plain `->` reads it back with a single
/// unchecked `untag` and no test of its own. The `!== null` in the
/// condition is the *only* tag test, and it is one `is.null` rather than
/// a comparison against a `null` constant. See
/// `Lowering::untag_receiver`.
#[test]
fn a_narrowed_receiver_untags_once_with_no_guard_of_its_own() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  function greet(): int {\n    return 1;\n  }\n}\nclass T {\n  function m(?Foo $obj): int {\n    if ($obj != null) {\n      return $obj->greet();\n    }\n    return 0;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$this->count` — a property access through the implicit receiver,
/// resolved to its declaring class via `ExprInfo::Property`.
#[test]
fn a_this_property_access() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  public int $count = 0;\n  function m(): int {\n    return $this->count;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj->count` — a property access through a receiver that isn't
/// `$this`, to exercise the general `object` lowering path rather than
/// only the `$this` special case.
#[test]
fn a_property_access_through_a_local_receiver() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public int $count = 0;\n}\nclass T {\n  function m(): int {\n    Foo $obj = new Foo();\n    return $obj->count;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj?->count` — the same guard a nullsafe *call* opens, wrapped
/// around a field read instead: the `field.get` runs only in the arm
/// where the tag says the receiver is not `null`.
#[test]
fn a_nullsafe_property_access_on_a_nullable_receiver() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public int $count = 0;\n}\nclass T {\n  function m(?Foo $obj): ?int {\n    return $obj?->count;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$i->path` on an `rule:types/erased-member-access` shape receiver — no class, no label and
/// no layout table: one `slot.get` keyed on the field's *name*, carrying
/// its position in the shape's sorted field list (which puts `path` after
/// `message`) as the runtime's hint. Fallible, so it has a landing block
/// of its own: `rule:types/erased-member-access` makes a name the concrete
/// receiver does not carry a catchable throw. The receiver is a parameter, so it is an aliasing
/// read and the value read out of its slot is retained by whoever keeps
/// it, exactly as for a class field.
#[test]
fn a_shape_property_access_reads_its_slot_by_name() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m({path: string, message: string} $i): string {\n    return $i->path;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A property access through a plain-`object` receiver is `rule:types/erased-member-access`'s
/// *fully* erased half: there is no declaring class and no layout either,
/// so what the checker records is the written name alone and the read
/// lowers to the same name-keyed `SlotGet` a shape's does — with a hint
/// of slot 0, which the runtime's own by-name search corrects, and a
/// result at `Ty::Tagged` because nothing knows what the field holds.
#[test]
fn a_property_access_through_a_plain_object_receiver_reads_by_name() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(object $o): mixed {\n    return $o->x;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `var $n = 1;` (`rule:types/var-inference`) — no declared type at all, so the local's
/// type is whatever `lower_expr` synthesizes from the initializer alone,
/// exactly as `nvs_types::locals::check_stmt`'s own `var` arm fixes it.
/// A bare integer literal with no `expected` type defaults to `int`
/// (`rule:types/arithmetic`), so `$n` ends up `int` here even though nothing in the
/// source spells that out.
#[test]
fn a_var_local_infers_its_type_from_the_initializer() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): int {\n    var $n = 1;\n    return $n;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A hex/octal/binary integer literal cooks to the same value its
/// decimal spelling would — `nvs-syntax`'s lexer accepts every prefixed
/// form as one `IntLiteral` token (see
/// `crates/nvs-syntax/src/lexer.rs`'s `lex_number`), and `nvs-ir` cooks
/// the prefix rather than only a plain decimal run, so `0x1F` lowers
/// to `31`.
#[test]
fn multi_base_integer_literals_cook_to_the_same_value() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): int {\n    int $hex = 0x1F;\n    int $oct = 0o17;\n    int $bin = 0b101;\n    return $hex + $oct + $bin;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `string` literal cooks to `InstKind::ConstStr` — a single-quoted
/// literal only unescapes `\\`/`\'`, a double-quoted one additionally
/// unescapes `\n`/`\t`/`\"`. Neither local is ever aliased or returned,
/// so both get exactly one release at the implicit `void` fallback
/// return — no retain anywhere in this fixture.
#[test]
fn string_literals_cook_their_escapes_and_release_at_scope_exit() {
    let (f, map, file) = lower_first_method(
        r#"<?nvs
class T {
  function m(): void {
    string $single = 'it\'s a \\ test';
    string $double = "line1\nline2\t\"quoted\"";
  }
}
"#,
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A double-quoted literal's numeric escapes (`\101` octal, `\x2A` hex,
/// `\u{1F600}` a multi-byte Unicode codepoint) cook to the actual
/// byte/codepoint they name, delegated to
/// `nvs_types::string_lit::cook_double_quoted_text` — see
/// `cook_str_literal`'s own doc comment for why this crate shares that
/// routine with the checker rather than duplicating it.
#[test]
fn numeric_escapes_cook_to_their_byte_or_codepoint() {
    let (f, map, file) = lower_first_method(
        r#"<?nvs
class T {
  function m(): void {
    string $s = "\101\x2A\u{1F600}";
  }
}
"#,
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `"pre$mid post"` — a double-quoted literal with one interpolation
/// site surrounded by literal text on both sides — lowers to the same
/// single three-piece `InstKind::Concat` a written-out
/// `"pre" . $mid . " post"` does, per
/// `Lowering::lower_interpolated_parts`, so the whole string is one
/// allocation rather than a fold's two. `$mid`'s own read is an aliasing
/// one, so it's left unreleased (its slot still owns it); the trailing
/// literal text piece is fresh and released once the `Concat` has read it,
/// while the leading one is handed over instead — its release is the
/// reference the instruction consumes. The whole method's own `void` exit
/// then releases `$s`.
#[test]
fn interpolated_string_with_text_on_both_sides_is_one_concat() {
    let (f, map, file) = lower_first_method(
        r#"<?nvs
class T {
  function m(): void {
    string $mid = "middle";
    string $s = "pre$mid post";
  }
}
"#,
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `"$x"` alone — no literal text around the one interpolation site —
/// is `ExprKind::Interpolated`'s degenerate single-part case:
/// `nvs_syntax::parser::collapse_string_parts` still picks `Interpolated`
/// over `Str` (the one part isn't `StringPart::Text`), but no
/// `InstKind::Concat` ever runs to copy `$x`'s value into a fresh
/// buffer. `Lowering::lower_interpolated_parts` has to retain `$x`'s
/// value itself in exactly this shape — the snapshot should show a
/// retain on `$x`'s own value immediately after it's read, with no
/// `Concat` instruction anywhere in the function, and the usual single
/// release of `$s` (now the sole owner of that retained reference,
/// alongside `$x`'s own still-live slot) at the implicit return.
#[test]
fn interpolated_string_with_only_a_variable_retains_it() {
    let (f, map, file) = lower_first_method(
        r#"<?nvs
class T {
  function m(): void {
    string $x = "hello";
    string $s = "$x";
  }
}
"#,
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A heredoc with no interpolation site used at all collapses to a plain
/// `ExprKind::Str` (`nvs_syntax::parser::collapse_string_parts`), just
/// like a double-quoted literal — the same `InstKind::ConstStr` shape,
/// cooked through `cook_heredoc_str` instead of the quote-delimited
/// branch. Its closing marker is flush left, so PHP 7.3's
/// flexible-indentation strip is a no-op here; the numeric escape still
/// cooks, since a heredoc runs the same escape grammar a double-quoted
/// literal does.
#[test]
fn a_flush_left_heredoc_cooks_like_a_double_quoted_literal() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    string $s = <<<EOT\nline1\\nline2\nEOT;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// The closing marker's own indentation is stripped from every body
/// line — PHP 7.3's "flexible heredoc" rule. The cooked `ConstStr`
/// should show `"hello\nworld"` with no leading spaces baked in, even
/// though the source itself indents both body lines and the marker to
/// match this function's own brace nesting.
#[test]
fn an_indented_heredoc_strips_the_closing_markers_indentation() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    string $s = <<<EOT\n        hello\n        world\n        EOT;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A nowdoc (`<<<'EOT'`) applies no escape grammar at all — unlike the
/// flush-left heredoc fixture above, `\n` here must cook to two literal
/// characters, backslash and `n`, not a newline — while still getting
/// its closing marker's indentation stripped exactly like a heredoc
/// does.
#[test]
fn a_nowdoc_strips_indentation_but_applies_no_escapes() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    string $s = <<<'EOT'\n        raw \\n text\n        EOT;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A heredoc with an interpolation site lowers through the exact same
/// `InstKind::Concat` over every piece that `lower_interpolated_parts`
/// already builds for a double-quoted literal — the only difference is
/// each `Text` run
/// getting dedented first. The middle line picks up right after `$x`'s
/// interpolation site, so it has to be recognized as a fresh line of
/// its own for the indentation strip to apply to it at all.
#[test]
fn an_indented_interpolated_heredoc_strips_indentation_from_every_run() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    string $x = \"hi\";\n    string $s = <<<EOT\n        pre $x\n        post\n        EOT;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `string $b = $a;` aliases `$a`'s already-owned value rather than
/// constructing a fresh one — `Lowering::bind_local` retains it. `return
/// $b;` then transfers `$b`'s reference out directly (excluded from
/// `Lowering::release_all_locals`'s sweep), leaving exactly one release
/// for `$a`'s slot — one retain, one release, never zero and never two,
/// for a value that in fact has exactly one owner (the caller) once this
/// function returns.
#[test]
fn assigning_one_string_local_to_another_retains_the_shared_value() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function pick(): string {\n    string $a = \"hello\";\n    string $b = $a;\n    return $b;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Reassigning a `string` local to a fresh literal releases the value it
/// previously held — `$x`'s `\"a\"` is released the moment `\"b\"`
/// overwrites it, well before the function's own exit sweep releases
/// `\"b\"` in turn.
#[test]
fn reassigning_a_string_local_releases_its_previous_value() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    string $x = \"a\";\n    $x = \"b\";\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Passing a `string` local as a call argument retains it first —
/// `Lowering::lower_call_args`'s own aliasing check — since the callee's
/// own parameter is bound like any other local and released at the
/// callee's exit (not visible in this snapshot, since `lower_first_method`
/// only lowers `T`'s first method). `take` returns `int`, not `string`,
/// so the call's own result needs no refcount treatment — this fixture
/// isolates the argument-side retain from the return-side question the
/// two tests below cover. Net effect on `$s`'s own slot: one retain right
/// before the call, one release at `m`'s own exit sweep. The call is a
/// statement of its own, so it is not the last-use move that
/// `return self::take($s)` takes.
#[test]
fn passing_a_string_local_as_a_call_argument_retains_it() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): int {\n    string $s = \"hi\";\n    self::take($s);\n    return 1;\n  }\n  static function take(string $x): int {\n    return 1;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `var $s = $obj->name;` — a `string`-typed property read is an
/// aliasing read exactly like a bare variable read
/// (`lower::is_aliasing_read`), so binding it to a new local retains the
/// field's own value; `$obj` itself is `Ty::Object`, not yet refcounted,
/// so only `$s`'s slot is released at the exit sweep.
#[test]
fn binding_a_string_property_read_to_a_local_retains_it() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public string $name = \"hi\";\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo();\n    var $s = $obj->name;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `return $obj->name;` — a property read has no local slot for
/// `Lowering::release_all_locals` to exclude the way a bare `$name`
/// return does, so `StmtKind::Return`'s own arm retains it explicitly
/// instead: exactly one retain, no release, leaving the caller with
/// exactly one owned reference once this function returns.
#[test]
fn returning_a_string_property_read_retains_it() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public string $name = \"hi\";\n}\nclass T {\n  function m(): string {\n    Foo $obj = new Foo();\n    return $obj->name;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `return self::make();` where `make` returns `string` — a call's own
/// result is a fresh-like producer, same as `new` or a literal
/// (`lower::is_aliasing_read` is `false` for `ExprKind::StaticCall`), so
/// returning it directly needs no retain at all: it already has exactly
/// one owner, which just transfers out to the caller.
#[test]
fn returning_a_string_returning_calls_result_needs_no_retain() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): string {\n    return self::make();\n  }\n  static function make(): string {\n    return \"hi\";\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `self::helper();` with no assignment at all — the ordinary way to
/// invoke a `void`-returning method. `Lowering::lower_expr_stmt` routes a
/// bare call/`new` expression statement through `lower_expr` for its side
/// effect alone; `helper` returns `void`, so there is nothing to release
/// afterward.
#[test]
fn a_bare_void_call_used_as_a_statement_lowers_with_no_release() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    self::helper();\n  }\n  static function helper(): void {}\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `self::make();` with no assignment, where `make` returns `string` —
/// the call's `string` result is refcounted and nothing ever binds it, so
/// `Lowering::lower_expr_stmt` releases it immediately, right after the
/// call, rather than leaking it: exactly one release, no retain (a call's
/// own result is a fresh producer, per `is_aliasing_read`).
#[test]
fn a_bare_call_used_as_a_statement_releases_a_discarded_string_result() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    self::make();\n  }\n  static function make(): string {\n    return \"hi\";\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `new Foo();` with no assignment at all — a bare `new` used purely for
/// a constructor's side effect. The fresh instance has exactly one owner
/// and nothing ever binds it, so `Lowering::lower_expr_stmt` releases it
/// straight after the construction, the same way it already did a
/// discarded `string` result above.
#[test]
fn a_bare_new_used_as_a_statement_releases_the_discarded_instance() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  function constructor() {}\n}\nclass T {\n  function m(): void {\n    new Foo();\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj->greet();` with no assignment — a bare *instance* call as a
/// statement, not just a static one, to make sure `lower_expr_stmt`'s
/// dispatch isn't accidentally `StaticCall`-only.
#[test]
fn a_bare_instance_call_used_as_a_statement_lowers_too() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  function greet(): void {}\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo();\n    $obj->greet();\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj->name = "new";` — a `string`-typed property write from a fresh
/// literal. `Lowering::lower_reassignment`'s property-target arm reads
/// the field's previous value back with a `FieldGet` and releases it, but
/// needs no retain of the new value: a literal already has exactly one
/// natural owner (`is_aliasing_read` is `false` for `ExprKind::Str`),
/// same as any other durable-slot bind.
#[test]
fn writing_a_fresh_string_literal_to_a_property_releases_its_previous_value() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public string $name = \"orig\";\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo();\n    $obj->name = \"new\";\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj->make()->name` — a field read whose base is a *temporary*. The
/// call hands back the only reference to the object, and `FieldGet`
/// borrows out of its slot, so the read retains its own result before the
/// base is released: the two together make the whole expression a fresh
/// producer, which is what `Lowering::aliasing_read` reports it as.
#[test]
fn a_field_read_off_a_temporary_retains_its_result_and_releases_the_base() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public string $name = \"orig\";\n}\nclass Maker {\n  function make(): Foo { return new Foo(); }\n}\nclass T {\n  function m(): void {\n    Maker $obj = new Maker();\n    string $s = $obj->make()->name;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj->rows()["k"]` — an *element* read whose base is a temporary, the
/// same shape as the field read above one storage kind along.
/// `InstKind::ArrayGet` borrows out of the array the call handed back, so
/// the read retains its own result before the array is released, and the
/// whole expression is a fresh producer.
#[test]
fn an_index_read_off_a_temporary_retains_its_result_and_releases_the_base() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Maker {\n  function rows(): array<string> { return [\"k\" => \"v\"]; }\n}\nclass T {\n  function m(): void {\n    Maker $obj = new Maker();\n    string $s = $obj->rows()[\"k\"];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj->name = $s;` — writing an aliasing local into a property retains
/// the new value first (same order `Lowering::bind_local` uses for a
/// local target), then reads and releases the field's previous value —
/// retain before release, so a self-assignment through the same slot
/// would never observe a transient zero refcount.
#[test]
fn writing_a_string_local_to_a_property_retains_it_before_releasing_the_old_value() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public string $name = \"orig\";\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo();\n    string $s = \"hi\";\n    $obj->name = $s;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$this->name = "new";` — a property write through the implicit
/// receiver, exercising the same `$this`/`Env` lookup path
/// `Lowering::lower_expr`'s `PropertyAccess` read arm already shares with
/// an ordinary local receiver.
#[test]
fn writing_through_this_lowers_too() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  public string $name = \"orig\";\n  function m(): void {\n    $this->name = \"new\";\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A property write through a plain-`object` receiver takes the same
/// name-keyed `SlotSet` the read side's `SlotGet` mirrors — the whole of
/// `rule:types/erased-member-access`'s erased write. The value is widened to `Ty::Tagged`
/// first: the field's real type is the receiving class's to state, and
/// `nvs_runtime::nvs_object_slot_set` is where it is checked.
#[test]
fn writing_through_a_plain_object_receiver_writes_by_name() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(object $o): void {\n    $o->x = 1;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `object` is the opaque top of every class type (`rule:types/grammar`) and it
/// costs **no** representation of its own: [`erase_checked_ty`] answers
/// [`Ty::Object`] for a named class, for a shape and for the top type
/// alike, so nothing below this boundary can read a class label off a
/// parameter, a return or a receiver. That is an *agreement* claim across
/// three receivers rather than anything one lowering prints, which is why
/// it is asserted here and not in a snapshot: the two neighbouring tests
/// above pin how an erased access *reads* (by name, at `Ty::Tagged`), and
/// a snapshot of one receiver could not have caught the other two drifting
/// away from it. What the erasure drops is the field list, and `SlotGet`
/// finding a name on the concrete descriptor is what replaces it —
/// [`erase_checked_ty`]'s own arm is the one home for the rule.
#[test]
fn the_object_top_type_erases_to_the_pointer_a_class_does() {
    let (f, ..) = lower_first_method(
        "<?nvs\nclass T {\n  function m(T $named, object $top, {x: int} $shape): object {\n    return $top;\n  }\n}\n",
    );
    // Index 0 is the implicit receiver, itself a `T` and so erased by the
    // same arm; 1..=3 are the three written spellings.
    assert_eq!(
        f.params,
        vec![Ty::Object, Ty::Object, Ty::Object, Ty::Object],
        "a class, the top type and a shape must share one representation"
    );
    assert_eq!(f.ret, Ty::Object, "and a return position is not special");
}

/// `"a" . "b"` — two fresh literal operands lower to a single
/// `InstKind::Concat`, with no retain of either. Each is a fresh,
/// non-aliasing value with no durable slot of its own — a bare `Str`
/// literal isn't `is_aliasing_read` — so the *trailing* one gets exactly
/// one release right after `Concat` reads it, the same "release a fresh
/// value once its one and only use is done" precedent a bare call/`new`
/// statement already sets. The **leading** one gets none: that release is
/// the reference the instruction consumes, which is what
/// `Lowering::emit_concat` hands over rather than emitting. The
/// concatenation's own result needs no retain or release at all when it's
/// returned directly — a fresh producer, same as a literal or a call's
/// result, transferring straight out.
#[test]
fn concatenating_two_string_literals_needs_no_retain_of_either_operand() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): string {\n    return \"a\" . \"b\";\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$a . $b` — both operands are aliasing reads of an existing local.
/// `$b` is only *read* to build the result, so it is not retained: a
/// concatenation never becomes a second durable owner of a trailing piece
/// the way binding one to a new local would. `$a` **is** retained, and
/// only because the instruction consumes one reference to its leading
/// piece — `$a`'s own slot keeps the one it holds and this is the extra,
/// which is `InstKind::Concat`'s protocol and `Lowering::emit_concat`'s
/// choice between the two ways of supplying it. Both slots still get
/// their ordinary one release each at `m`'s exit sweep, and the
/// concatenation's own result — bound to `$c` here, an aliasing read of
/// nothing — needs no retain either, only the release
/// `release_all_locals` gives every refcounted local still live at return.
#[test]
fn concatenating_two_string_locals_retains_only_the_leading_one() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    string $a = \"x\";\n    string $b = \"y\";\n    string $c = $a . $b;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `1 . "x"` — an `int` operand on the `.` side that `nvs_types::expr::
/// check_expr`'s own `require_stringable` happily accepts (PHP-style
/// implicit to-string) converts through a new `InstKind::HelperCall`
/// (`Helper::IntToString`) before reaching `InstKind::Concat`. Both the
/// helper-call result and the `"x"` literal are fresh, non-aliasing
/// values with no durable slot of their own; the literal is released
/// right after `Concat` reads it, and the conversion result is not,
/// because it leads and its release *is* the reference the instruction
/// consumes. The concatenation's own result is returned directly and
/// needs no release at all.
#[test]
fn concatenating_an_int_literal_with_a_string_uses_a_helper_call() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): string {\n    return 1 . \"x\";\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// An `int` hole of an interpolated string is an `int` piece of the join, and
/// an escaped hole of an html template is an `html` piece. Neither is
/// converted or escaped into a string of its own first, and the join consumes
/// no piece, so the leading literal is released after it like every other.
#[test]
fn an_int_hole_and_an_escaped_hole_are_typed_pieces_of_the_join() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(int $n, string $s): Core\\Html\\Markup {\n    \
         string $t = \"n=$n, s=$s\";\n    return html`<b>{$s}</b>{$n}{$t}`;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$flag . "!"` — a `bool` local read (an aliasing read of its own
/// slot) converts through `Helper::BoolToString`; the conversion result
/// is still a fresh, non-aliasing `Ty::Str` value (the `bool` itself was
/// never refcounted, so there was nothing to alias into the conversion),
/// and it leads — so it is handed to `Concat` rather than released after
/// it, same as the `int` case above. `$flag`'s own slot needs no release
/// from `Concat` at all — it isn't `Ty::is_refcounted`, so
/// `release_all_locals` skips it too.
#[test]
fn concatenating_a_bool_local_with_a_string_uses_a_helper_call() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bool $flag): string {\n    return $flag . \"!\";\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj . "x"` where `$obj`'s class implements `Stringable` — `rule:classes/stringable`'s implicit conversion, desugared to the `toString()`
/// `nvs_types::expr::operators::require_stringable` resolved under the
/// operand's own span. A `.` operand is not itself a call expression, so
/// there is no `ExprInfo::Call` for it the way an actual
/// `$obj->toString()` site would have; the checker's own side map
/// (`ExprTypeTable::to_string_call`) is what carries the target across.
///
/// It dispatches through `InstKind::ClassDescOf`/`CallVirtual` so an
/// override wins, carries `rule:errors/propagation`'s error edge because a `toString` body
/// may throw, and retains `$n` first — the parameter's slot still owns it,
/// and the callee releases every refcounted parameter at its own exit. The
/// `toString` result then leads the concatenation, so it is handed over
/// rather than released after it, exactly as a converted scalar is.
#[test]
fn concatenating_a_stringable_object_operand_calls_its_to_string() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Name implements Stringable {\n  public function toString(): string { return \"x\"; }\n}\nclass T {\n  public function m(Name $n): string {\n    return $n . \"x\";\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj as string` — `rule:types/conversion`'s explicit spelling of the very same
/// conversion, reaching the very same `Self::lower_to_string_call` rather than
/// getting a second answer of its own, exactly as `as bool` reuses
/// `rule:expressions/truthy-positions`'s truthy table.
#[test]
fn converting_a_stringable_object_to_string_calls_its_to_string() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Name implements Stringable {\n  public function toString(): string { return \"x\"; }\n}\nclass T {\n  public function m(Name $n): string {\n    return $n as string;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

// `bytes` is the mechanical follow-on to `string` the crate docs named:
// same `Ty::Bytes` representation, same `Lowering::bind_local`/
// `lower_call_args`/`release_all_locals`/`lower_reassignment` insertion
// points `Ty::Str` already uses. `nvs-syntax`'s grammar has no `bytes`
// literal syntax at all (no `b"..."` form), so unlike the `string` tests
// above, every fixture below sources its `bytes` value from a parameter
// or a property read rather than a literal — both already-covered
// `is_aliasing_read` shapes, so this still exercises the same policy a
// literal-sourced fixture would.

/// `bytes $b = $a;` aliases the parameter `$a`'s already-owned value —
/// `Lowering::bind_local` retains it, exactly like the `string` analog
/// above. `return $b;` transfers `$b`'s reference out directly (excluded
/// from `Lowering::release_all_locals`'s sweep), leaving exactly one
/// release for `$a`'s own slot at the exit sweep.
#[test]
fn a_bytes_parameter_bound_to_a_local_transfers_out_on_return() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function pick(bytes $a): bytes {\n    bytes $b = $a;\n    return $b;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `bytes $x = $a; $x = $b;` — reassigning a `bytes` local to a second
/// aliasing parameter retains the new value first, then releases the
/// value `$x` previously held, the same order `Lowering::bind_local`
/// always uses. At the exit sweep every local still live — `$a`, `$b` and
/// `$x` (now aliasing `$b`'s storage) — gets its own release: `$a`'s
/// storage ends up released twice in total (once when `$x` moves off it,
/// once for `$a`'s own slot), which is correct rather than a double free
/// — two live slots (`$a`, and `$x` before the reassignment) really did
/// hold two independent references to it.
#[test]
fn reassigning_a_bytes_local_retains_the_new_value_and_releases_the_old() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bytes $a, bytes $b): void {\n    bytes $x = $a;\n    $x = $b;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Passing a `bytes` local as a call argument retains it first —
/// `Lowering::lower_call_args`'s aliasing check, exactly mirroring the
/// `string` analog above. `take` returns `int`, isolating the
/// argument-side retain from any return-side question.
#[test]
fn passing_a_bytes_local_as_a_call_argument_retains_it() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bytes $s): int {\n    return self::take($s);\n  }\n  static function take(bytes $x): int {\n    return 1;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `var $s = $obj->data;` — a `bytes`-typed property read is an aliasing
/// read exactly like a `string` one, so binding it to a new local retains
/// the field's own value. `Foo`'s `bytes` property has no literal default
/// available (see this block's own note), so its constructor assigns it
/// from a `bytes` parameter instead — `rule:classes/definite-property-initialization`'s definite-initialization
/// obligation either way.
#[test]
fn binding_a_bytes_property_read_to_a_local_retains_it() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public bytes $data;\n  function constructor(bytes $data) {\n    $this->data = $data;\n  }\n}\nclass T {\n  function m(bytes $seed): void {\n    Foo $obj = new Foo($seed);\n    var $s = $obj->data;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj->data = $other;` — writing an aliasing `bytes` local into a
/// property retains the new value first, then reads and releases the
/// field's previous value, the same order `Lowering::lower_reassignment`'s
/// property-target arm always uses for a refcounted field.
#[test]
fn writing_a_bytes_local_to_a_property_retains_it_before_releasing_the_old_value() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public bytes $data;\n  function constructor(bytes $data) {\n    $this->data = $data;\n  }\n}\nclass T {\n  function m(bytes $seed, bytes $other): void {\n    Foo $obj = new Foo($seed);\n    $obj->data = $other;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

// `array<T>`: `Ty::Array` is a bare, opaque representation exactly like
// `Ty::Object` (see that variant's own doc comment), and
// `InstKind::ArrayNew` is the fixed-shape array literal instruction it needs.
// The fixtures immediately below are all *positional* literals — no
// explicit `key =>` — which keep the single-`ArrayNew` shape; the
// explicit-`key =>` fixtures further down cover the `ArrayNew` (empty) +
// `ArraySet`* shape, which a `...spread` element takes too. `&value`
// never reaches here at all, `nvs_types` refusing it as `E0483`.

/// `[]` — an empty array literal lowers to `InstKind::ArrayNew` with no
/// entries at all, still a well-formed fresh `Ty::Array` value.
#[test]
fn an_empty_array_literal_lowers_with_no_entries() {
    let (f, map, file) =
        lower_first_method("<?nvs\nclass T {\n  function m(): array {\n    return [];\n  }\n}\n");
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `[1, 2, 3]` — three fresh, non-aliasing `int` elements, auto-numbered
/// `"0"`/`"1"`/`"2"`. None of them is `Ty::is_refcounted`, so no retain is
/// emitted for any entry — only the array's own slot gets a release at
/// `m`'s exit sweep.
#[test]
fn a_literal_with_fresh_scalar_elements_needs_no_retain() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    array $a = [1, 2, 3];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `[$s]` — a `string` local read is an aliasing read
/// (`lower::is_aliasing_read`), so the element is retained before the
/// array durably owns it, the same policy `Lowering::lower_call_args`
/// already applies at a call-argument boundary. `$s`'s own slot still
/// gets its ordinary release at `m`'s exit sweep, alongside the array's.
#[test]
fn a_literal_with_an_aliasing_element_retains_it() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    string $s = \"hi\";\n    array $a = [$s];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `["k" => 1]` with the cheaper forms off — a literal with an explicit
/// `key =>` element lowers to an empty `ArrayNew` plus one `ArraySet`: `"k"`
/// is a fresh `ConstStr` (a string literal is never an aliasing read), so it
/// needs no retain of its own, mirroring the value `1`. With the cheaper
/// forms on, row `constant-key-literal` builds it in one step
/// (`a_literal_with_constant_keys_is_built_in_one_step`); the chain is what a
/// computed key still takes.
#[test]
fn a_string_literal_keyed_array_element_lowers_to_array_new_then_array_set() {
    let (f, map, file) = crate::lower::without_cheaper_forms(|| {
        lower_first_method(
            "<?nvs\nclass T {\n  function m(): void {\n    array $a = [\"k\" => 1];\n  }\n}\n",
        )
    });
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `[5 => "a"]` with the cheaper forms off — an explicit `int` key travels
/// to the `InstKind::ArraySet` chain unrendered, exactly as an `$arr[$i]`
/// subscript does: this is the same `Lowering::lower_array_key` on both
/// sides, which is why the two can never drift.
#[test]
fn an_int_literal_keyed_array_element_carries_the_integer_unrendered() {
    let (f, map, file) = crate::lower::without_cheaper_forms(|| {
        lower_first_method(
            "<?nvs\nclass T {\n  function m(): void {\n    array $a = [5 => \"a\"];\n  }\n}\n",
        )
    });
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `[$k => 1]` — a `string` local used as an explicit key is an aliasing
/// read, so it is retained before the array durably owns it, exactly the
/// policy `Lowering::lower_reassignment`'s `Index`-target arm already
/// gives `$a[$k] = 1;`.
#[test]
fn a_dynamic_string_keyed_array_element_retains_the_key() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(string $k): void {\n    array $a = [$k => 1];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `[1, "k" => 2, 3]` — in a literal with an explicit key, each positional
/// element is an `InstKind::ArrayAppend`, so it takes the array's next
/// integer key at run time rather than a key this pass computes.
#[test]
fn a_positional_element_after_an_explicit_key_appends() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    array $a = [1, \"k\" => 2, 3];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `[...$a]` — one `InstKind::ArraySpread` per spread element, over the
/// same empty-`ArrayNew` shape an explicit key already takes. The subject
/// is a local read, so it is *borrowed* and no retain is emitted beside
/// the copy: what the destination ends up owning is a fresh reference per
/// entry, and the runtime takes it.
#[test]
fn a_spread_array_element_lowers() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    array $a = [1];\n    array $b = [...$a];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `[0, ...$a, 2]` — a keyless element of a literal that contains a
/// spread is an `InstKind::ArrayAppend`, not a lowering-time index: how
/// many entries the spread contributed is the subject's own run-time
/// length.
#[test]
fn a_keyless_element_beside_a_spread_appends_instead_of_numbering() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    array $a = [1];\n    array $b = [0, ...$a, 2];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A spread whose subject is a *fresh producer* — a call's result rather
/// than a local read — is staged on `Lowering::owned_temporaries` and
/// released once the copy has been emitted, because
/// `InstKind::ArraySpread` borrows its subject rather than consuming it.
/// The array under construction is on that same stack throughout, which
/// is what gives the copy's own error edge something to release.
#[test]
fn a_spread_of_a_call_result_releases_the_subject_after_the_copy() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): void {\n    array $b = [...self::rows()];\n  }\n  static function rows(): array {\n    return [1];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Passing an `array` local as a call argument retains it first —
/// `Lowering::lower_call_args`'s aliasing check, exactly mirroring the
/// `string`/`bytes` analogs above. `erase_checked_ty`'s
/// `CheckedTy::Array(_) => Ty::Array` arm is what makes this boundary
/// work with no insertion point of its own. The call is a statement of its
/// own, so it is not the last-use move that `return self::take($a)` takes.
#[test]
fn passing_an_array_local_as_a_call_argument_retains_it() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): int {\n    array $a = [1];\n    self::take($a);\n    return 1;\n  }\n  static function take(array $x): int {\n    return 1;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `var $s = $obj->data;` — an `array`-typed property read is an
/// aliasing read exactly like `string`/`bytes`, so binding it to a new
/// local retains the field's own value. `Foo`'s `array` property has no
/// literal default available in a property initializer the way a scalar
/// one would, so its constructor assigns it from an `array` parameter
/// instead — `rule:classes/definite-property-initialization`'s definite-initialization obligation either way.
/// The constructor call itself also exercises an array literal
/// (`[1]`) passed as a resolved call argument, not just a local bind.
#[test]
fn binding_an_array_property_read_to_a_local_retains_it() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public array $data;\n  function constructor(array $data) {\n    $this->data = $data;\n  }\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo([1]);\n    var $s = $obj->data;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$obj->data = $other;` — writing an aliasing `array` local into a
/// property retains the new value first, then reads and releases the
/// field's previous value, the same order
/// `Lowering::lower_reassignment`'s property-target arm always uses for
/// a refcounted field.
#[test]
fn writing_an_array_local_to_a_property_retains_it_before_releasing_the_old_value() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass Foo {\n  public array $data;\n  function constructor(array $data) {\n    $this->data = $data;\n  }\n}\nclass T {\n  function m(): void {\n    Foo $obj = new Foo([1]);\n    array $other = [2];\n    $obj->data = $other;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$a[0]` through an `array<int>` parameter — the simplest array-access
/// read: a fresh, non-refcounted `int` element, and a literal `int` key
/// that reaches `InstKind::ArrayGet` as the `int` it is, with no
/// `helper.int_to_string` and therefore no key allocation and no release
/// of one either. `rule:types/arrays` says the key *is* `"0"`; `nvs-ir`'s
/// module doc § *an array key is a `string`, and an `int` subscript no
/// longer spells it* is why the decimal is not rendered to reach it, and
/// codegen picks `nvs_array_get_index` off this operand's `Ty`.
#[test]
fn reading_an_int_element_through_a_literal_key_carries_the_integer_unrendered() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(array<int> $a): int {\n    return $a[0];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$a[$i]` with a `uint` subscript, which `Lowering::lower_array_key`
/// branches on: the runtime's index ABI is an `i64`, and a `uint` above
/// `i64::MAX` has no `i64` spelling naming the same key. So the output has
/// both arms — a `reinterpret` to `int` tagged without a call where the
/// value is not negative as an `int`, and `helper.uint_to_string` only on the
/// other edge — joined by a tagged phi that is released after the borrow.
#[test]
fn reading_an_element_through_a_uint_subscript_renders_only_past_i64_max() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(array<int> $a, uint $i): int {\n    return $a[$i];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$out .= $piece;` on a plain `string` local — one `str.append` and
/// nothing else. No `concat`, because there is no fresh buffer to build;
/// no retain of the suffix, which `$piece`'s own slot still owns; and no
/// release of the old `$out`, because `InstKind::StrAppend` consumes that
/// reference and yields the one the binding is re-pointed at. That is
/// `InstKind::ArraySet`'s protocol, which that variant's doc comment owns.
#[test]
fn appending_to_a_string_local_appends_in_place_rather_than_concatenating() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(string $piece): string {\n    var $out = \"\";\n    \
             $out .= $piece;\n    return $out;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$this->p .= "x";` on a plain `string` property is load, append, store:
/// the suffix is lowered first, then `field.get` reads the slot,
/// `str.append` grows it, and `field.set` writes it back, with no retain,
/// no `concat` and no second read of the slot. The ownership pass lends the
/// slot, so a debug build also poisons it around the append.
#[test]
fn appending_to_a_plain_property_grows_it_in_place() {
    assert_snapshot!(print_shipped_method(
        "<?nvs\nclass T {\n  public string $p = \"\";\n  function m(): void {\n    \
             $this->p .= \"x\";\n  }\n}\n",
    ));
}

/// `unset($a[0]);` — the other subscript that renders.
/// `InstKind::ArrayUnset` has no index-shaped runtime primitive beside
/// it, so `Lowering::lower_rendered_array_key` forces the decimal here
/// rather than letting codegen discover it cannot. Contrast
/// `writing_an_int_element_through_a_literal_key_carries_the_integer_unrendered`,
/// which is the same literal key one instruction along.
/// A `uint` hole in a joined interpolation is a `uint` part, so the join
/// writes its digits and no string of digits is made. An append of the same
/// interpolation writes the same part into its target.
#[test]
fn a_uint_hole_joins_as_a_part() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(uint $u): string {\n    var $s = \"a{$u}b\";\n    $s .= \"<{$u}>\";\n    return $s;\n  }\n}\n",
    );
    let printed = print_function(&f, map.file(file));
    assert!(!printed.contains("uint_to_string"), "{printed}");
    assert!(printed.contains("concat v"), "{printed}");
    assert_eq!(printed.matches(", uint v").count(), 2, "{printed}");
    assert!(printed.contains("str.append v"), "{printed}");
}

/// `body` as `T::m`'s body, with `params`, beside two Novis callees `f` and
/// `g`, lowered as a whole program so the ownership pass has run. Every
/// function the program lowered is printed, so a generator's own functions
/// are in it too.
fn lower_moves(body: &str, params: &str) -> String {
    let src = format!(
        "<?nvs\nclass T {{\n  static function m({params}): array<int> {{\n{body}\n  }}\n  \
         static function f(array<int> $a): array<int> {{ return $a; }}\n  \
         static function g(array<int> $a, string $s): array<int> {{ return $a; }}\n}}\n"
    );
    let (program, map, file) = lower_whole_file_with_src(&src);
    program
        .functions
        .iter()
        .filter(|f| f.name.starts_with("T::m"))
        .map(|f| print_function(f, map.file(file)))
        .collect()
}

const XS: &str = "array<int> $xs, bool $c";

/// A value passed at its last use goes to the callee with no retain, and a
/// subject passed to an owning `Core` member goes to its owning entry. That
/// holds for a different target, for a nested call and inside a `try` whose
/// `catch` does not read the value.
#[test]
fn a_value_is_moved_at_its_last_use() {
    for moved in [
        "    $xs = T::f($xs);\n    return $xs;",
        "    return T::f($xs);",
        "    array<int> $ys = T::f($xs);\n    return $ys;",
        "    $xs = T::f(T::f($xs));\n    return $xs;",
    ] {
        let printed = lower_moves(moved, XS);
        assert!(!printed.contains("retain v"), "moved: {moved}\n{printed}");
    }
    for moved in [
        "    $xs = Core\\Arr::slice($xs, 1);\n    return $xs;",
        "    try {\n      $xs = Core\\Arr::slice($xs, 1);\n    } catch (Throwable $e) {\n    }\n    return $xs;",
        // A second holder that is never read again is released at its last
        // use, so the call holds the only reference.
        "    array<int> $ys = $xs;\n    $xs = Core\\Arr::slice($xs, 1);\n    return $xs;",
    ] {
        let printed = lower_moves(moved, XS);
        assert!(printed.contains("_owned"), "moved: {moved}\n{printed}");
    }
}

/// A value that one path reads after the call keeps its retain, and a
/// `Core` member called with it borrows it.
#[test]
fn a_value_read_after_on_one_path_keeps_its_retain() {
    let kept =
        "    array<int> $ys = T::f($xs);\n    if ($c) {\n      return $xs;\n    }\n    return $ys;";
    let printed = lower_moves(kept, XS);
    assert!(printed.contains("retain v"), "{printed}");
    for kept in [
        "    array<int> $ys = Core\\Arr::slice($xs, 1);\n    if ($c) {\n      return $xs;\n    }\n    return $ys;",
        // A second holder releases the value too.
        "    array<int> $ys = $xs;\n    $xs = Core\\Arr::slice($xs, 1);\n    return $ys;",
        // A loop reads it again on its back edge.
        "    array<int> $ys = [];\n    while ($c) {\n      $ys = Core\\Arr::slice($xs, 1);\n    }\n    return $ys;",
    ] {
        let printed = lower_moves(kept, XS);
        assert!(!printed.contains("slice_owned"), "kept: {kept}\n{printed}");
    }
}

/// A Novis callee owns its argument on its error exit, so a `catch` that
/// reads the value keeps it: the value is not moved, and the known callee
/// takes it lent, through its lent body, which never frees it. An owning
/// `Core` member leaves its subject with the caller on an error, so the same
/// `catch` does not stop a move into it.
#[test]
fn a_value_a_catch_reads_is_kept_on_the_error_edge() {
    let kept = "    try {\n      $xs = T::f($xs);\n    } catch (Throwable $e) {\n      return $xs;\n    }\n    return $xs;";
    let printed = lower_moves(kept, XS);
    let m = function_named(&printed, "T::m");
    assert!(m.contains("call T::f#lent1("), "{m}");
    assert!(
        !m.contains("call T::f("),
        "the plain body would free it: {m}"
    );
    let moved = "    try {\n      $xs = Core\\Arr::slice($xs, 1);\n    } catch (Throwable $e) {\n      return $xs;\n    }\n    return $xs;";
    let printed = lower_moves(moved, XS);
    assert!(printed.contains("slice_owned"), "{printed}");
}

/// A generator's body moves a value at its last use as a function does.
#[test]
fn a_generator_moves_like_a_function() {
    let src = "<?nvs\nclass T {\n  static function f(array<int> $a): array<int> { return $a; }\n  \
               static function gen(array<int> $xs): Iterator<uint> {\n    \
               yield 1;\n    $xs = T::f($xs);\n    $xs = Core\\Arr::slice($xs, 1);\n    \
               yield Core\\Arr::count($xs);\n  }\n}\n";
    let (program, map, file) = lower_whole_file_with_src(src);
    let printed: String = program
        .functions
        .iter()
        .filter(|f| f.name.contains("gen"))
        .map(|f| print_function(f, map.file(file)))
        .collect();
    assert!(printed.contains("slice_owned"), "{printed}");
}

/// A generator's functions as printed, for the two `generator-local` tests:
/// `$xs` is parked across a `yield` and changed after it.
fn lower_generator_local() -> String {
    let src = "<?nvs\nclass T {\n  static function gen(array<int> $xs): Iterator<uint> {\n    \
               yield 1;\n    $xs = Core\\Arr::append($xs, 4);\n    \
               yield Core\\Arr::count($xs);\n    $xs = Core\\Arr::append($xs, 5);\n    \
               yield Core\\Arr::count($xs);\n  }\n}\n";
    let (program, map, file) = lower_whole_file_with_src(src);
    program
        .functions
        .iter()
        .filter(|f| f.name.ends_with("::advance"))
        .map(|f| print_function(f, map.file(file)))
        .collect()
}

/// The value ids `printed` reads back out of a field with `op`.
fn field_reads(printed: &str, op: &str) -> Vec<String> {
    printed
        .lines()
        .filter(|line| line.contains(&format!("= {op} ")) && line.contains("::xs"))
        .map(|line| line.trim().split(' ').next().unwrap_or_default().to_owned())
        .collect()
}

/// A resume takes the parked array out of the state object with no retain,
/// so the local is its only reference and the change after the `yield`
/// calls the owning entry.
#[test]
fn a_generator_local_changed_after_a_yield_is_changed_in_place() {
    let printed = lower_generator_local();
    let taken = field_reads(&printed, "field.take");
    // The seeded parameter and the two resumes.
    assert!(taken.len() >= 3, "{printed}");
    // The old reload retained its value on the line after the read. A spill
    // later in the block still retains the value it parks.
    let lines: Vec<&str> = printed.lines().map(str::trim).collect();
    for (i, line) in lines.iter().enumerate() {
        if let Some(v) = taken.iter().find(|v| line.starts_with(&format!("{v} = "))) {
            assert_ne!(lines[i + 1], format!("retain {v}"), "{printed}");
        }
    }
    assert_eq!(printed.matches("append_owned").count(), 2, "{printed}");
}

/// While the generator is parked, the state object has the one reference
/// to each local, and the body has none. So a drop while parked frees each
/// local once: every spill retains the value it parks once, and every read
/// of the field outside a resume is the spill's read of the old value,
/// which it releases.
#[test]
fn a_generator_dropped_while_parked_frees_each_local_once() {
    let printed = lower_generator_local();
    let parked: Vec<&str> = printed
        .lines()
        .filter(|line| line.trim().starts_with("field.set") && line.contains("::xs,"))
        .map(|line| line.rsplit(", ").next().unwrap_or_default())
        .collect();
    assert!(parked.len() >= 2, "{printed}");
    for v in &parked {
        assert_eq!(
            printed.matches(&format!("retain {v}\n")).count(),
            1,
            "{v}\n{printed}"
        );
    }
    for old in field_reads(&printed, "field.get") {
        assert!(
            printed.contains(&format!("release {old}\n")),
            "{old}\n{printed}"
        );
    }
}

/// A by-value loop whose body writes only the entry it is on walks the array
/// its variable has now, so the loop takes no reference of its own and the
/// write is in place.
#[test]
fn a_loop_that_writes_only_its_own_entry_holds_no_reference() {
    for body in [
        "    foreach ($xs as string $k => int $v) {\n      $xs[$k] = $v + 1;\n    }\n    return $xs;",
        "    foreach ($xs as string $k => int $v) {\n      if ($c) {\n        continue;\n      }\n      $xs[$k] = $v * 2;\n    }\n    return $xs;",
    ] {
        // `v1` is `$xs`, the array the loop walks.
        let printed = lower_moves(body, XS);
        assert!(!printed.contains("retain v1\n"), "{body}\n{printed}");
    }
}

/// A loop whose body can change the array another way keeps its own
/// reference: an append, a write at another key, an assigned key, a write to
/// the whole variable and a call that takes the array.
#[test]
fn a_loop_whose_body_can_change_the_array_another_way_keeps_its_reference() {
    for body in [
        "    foreach ($xs as string $k => int $v) {\n      $xs[] = $v;\n    }\n    return $xs;",
        "    foreach ($xs as string $k => int $v) {\n      $xs['0'] = $v;\n    }\n    return $xs;",
        "    foreach ($xs as string $k => int $v) {\n      $k = '0';\n      $xs[$k] = $v;\n    }\n    return $xs;",
        "    foreach ($xs as string $k => int $v) {\n      $xs = [];\n    }\n    return $xs;",
        "    foreach ($xs as string $k => int $v) {\n      $xs[$k] = $v;\n      $xs = T::f($xs);\n    }\n    return $xs;",
    ] {
        let printed = lower_moves(body, XS);
        assert!(printed.contains("retain v1\n"), "{body}\n{printed}");
    }
}

/// The pass reads the IR, so a mention of the variable in a string literal
/// or a comment does not keep a retain.
#[test]
fn the_lowering_no_longer_scans_text_for_moves() {
    for moved in [
        "    $xs = T::g($xs, '$xs');\n    return $xs;",
        "    $xs = T::f($xs /* $xs */);\n    return $xs;",
    ] {
        let printed = lower_moves(moved, XS);
        assert!(!printed.contains("retain v"), "moved: {moved}\n{printed}");
    }
    let lower = include_str!("stmt.rs");
    assert!(!lower.contains("fn mentions_of"), "the text scan is gone");
}

/// `body` as `T::m`'s body, in a class with a plain array property `$p`, a
/// static array property `$s`, a property `$n` holding a `N` whose own `$q` is
/// an array, a property `$h` with hooks, and a property `$w` holding a `W`,
/// whose class is a `PropertyGuard`. Lowered as a whole program, so the
/// ownership pass has run, and `T::m` alone is printed.
fn lower_lends(body: &str) -> String {
    let src = format!(
        "<?nvs\nfinal class N {{\n  public array<int> $q = [];\n}}\n\
         final class W implements PropertyGuard {{\n  public array<int> $p = [];\n  \
         public function beforePropertySet(string $name, mixed $value, array<string> $keys): void {{}}\n  \
         public function beforePropertyUnset(string $name, array<string> $keys): void {{}}\n}}\n\
         final class T {{\n  public array<int> $p = [];\n  public static array<int> $s = [];\n  \
         public array<array<int>> $g = [];\n  public array<string> $u = [];\n  \
         public static array<array<string>> $r = [];\n  public N $n;\n  public W $w;\n  \
         public array<int> $h {{ get => $this->h; set (array<int> $v) {{ $this->h = $v; }} }}\n  \
         public function constructor() {{ $this->n = new N(); $this->w = new W(); $this->h = []; }}\n  \
         public function m(callable(int): bool $cb): void {{\n{body}\n  }}\n  \
         public static function f(): int {{ return 1; }}\n}}\n"
    );
    let (program, map, file) = lower_whole_file_with_src(&src);
    program
        .functions
        .iter()
        .filter(|f| f.name == "T::m")
        .map(|f| print_function(f, map.file(file)))
        .collect()
}

/// How many times `printed` reads `slot`, such as `T::p` or `T::$s`. A lent
/// slot is read once, by the call's subject. A slot that is not lent is read a
/// second time, for the release of its old value after the store.
fn slot_reads(printed: &str, slot: &str) -> usize {
    printed
        .lines()
        .filter(|line| line.contains(".get ") && line.contains(&format!("{slot}  ;")))
        .count()
}

/// Asserts that `body` lends `slot` to an owning `Core` entry.
fn assert_lent(body: &str, slot: &str) {
    let printed = lower_lends(body);
    assert!(printed.contains("_owned("), "{body}\n{printed}");
    assert_eq!(slot_reads(&printed, slot), 1, "{body}\n{printed}");
}

/// Asserts that `body` keeps the slot's own reference: the store still reads
/// the old value and releases it.
fn assert_kept(body: &str, slot: &str) {
    let printed = lower_lends(body);
    assert_eq!(slot_reads(&printed, slot), 2, "{body}\n{printed}");
}

#[test]
fn a_plain_property_is_lent() {
    assert_lent("    $this->p = Core\\Arr::slice($this->p, 1);", "T::p");
    assert_lent("    $this->p = Core\\Arr::sort($this->p);", "T::p");
}

#[test]
fn a_static_property_is_lent() {
    assert_lent("    T::$s = Core\\Arr::slice(T::$s, 1);", "T::$s");
    assert_lent("    self::$s = Core\\Arr::reverse(self::$s);", "T::$s");
}

#[test]
fn a_property_chain_is_lent() {
    assert_lent(
        "    $this->n->q = Core\\Arr::slice($this->n->q, 1);",
        "N::q",
    );
}

/// An element is lent once the array holding it is made sole and stored
/// back, and the write puts the result over the entry without releasing it.
/// Two different keys name two entries, and a read of the array between
/// could share it, so neither is lent.
#[test]
fn an_element_is_lent_through_its_containers() {
    for body in [
        "    $this->g['a'] = Core\\Arr::slice($this->g['a'], 1);",
        "    $this->g[0] = Core\\Arr::reverse($this->g[0]);",
    ] {
        let printed = lower_lends(body);
        assert!(printed.contains("_owned("), "{body}\n{printed}");
        assert!(printed.contains("= array.sole v"), "{body}\n{printed}");
        assert!(printed.contains("= array.set.lent v"), "{body}\n{printed}");
        let sole = printed.find("array.sole").unwrap_or(usize::MAX);
        let get = printed.find("array.get").unwrap_or(0);
        assert!(sole < get, "the array is sole before the read\n{printed}");
    }
    for body in [
        "    $this->g['a'] = Core\\Arr::slice($this->g['b'], 1);",
        "    $this->g['a'] = Core\\Arr::slice($this->g['a'], Core\\Arr::count($this->g) as int);",
    ] {
        let printed = lower_lends(body);
        assert!(!printed.contains("array.sole"), "{body}\n{printed}");
        assert!(!printed.contains("array.set.lent"), "{body}\n{printed}");
    }
}

/// An element of an array a local owns is lent as one a slot holds: the
/// array is made sole just before the element's read, and the write into the
/// sole array is lent. A read of the local between the two, or a different
/// key, keeps the borrowed call.
#[test]
fn an_element_of_a_local_array_is_lent() {
    for body in [
        "    array<array<int>> $a = ['a' => []];\n    \
         $a['a'] = Core\\Arr::append($a['a'], 1);\n    $this->g = $a;",
        "    array<array<int>> $a = [[]];\n    int $i = 0;\n    \
         while ($i < 3) {\n      $a[0] = Core\\Arr::append($a[0], $i);\n      $i = $i + 1;\n    }\n    \
         $this->g = $a;",
    ] {
        let printed = lower_lends(body);
        assert!(printed.contains("_owned("), "{body}\n{printed}");
        let sole = printed.find("= array.sole v").expect(&printed);
        let get = printed[sole..].find("array.get v").map(|at| at + sole);
        assert!(
            get.is_some(),
            "the array is sole before the read\n{printed}"
        );
        assert!(printed.contains("= array.set.lent v"), "{body}\n{printed}");
    }
    for body in [
        "    array<array<int>> $a = ['a' => [], 'b' => []];\n    \
         $a['a'] = Core\\Arr::append($a['b'], 1);\n    $this->g = $a;",
        "    array<array<int>> $a = ['a' => []];\n    \
         $a['a'] = Core\\Arr::append($a['a'], Core\\Arr::count($a) as int);\n    $this->g = $a;",
    ] {
        let printed = lower_lends(body);
        assert!(!printed.contains("array.sole"), "{body}\n{printed}");
        assert!(!printed.contains("array.set.lent"), "{body}\n{printed}");
    }
}

/// A write of one element of a plain property or static property is lent
/// the slot's reference: the write takes no retain, and the store reads the
/// slot only once. With the pass off, the write retains the array and the
/// store reads the old value again to release it.
#[test]
fn an_element_write_into_a_plain_slot_is_lent() {
    for (body, slot) in [
        ("    $this->p[] = 1;", "T::p"),
        ("    $this->p['k'] = 2;", "T::p"),
        ("    T::$s[] = 3;", "T::$s"),
        ("    self::$s[0] = 4;", "T::$s"),
        ("    $this->n->q['a'] = 5;", "N::q"),
    ] {
        let printed = lower_lends(body);
        assert!(!printed.contains("retain"), "{body}\n{printed}");
        assert_eq!(slot_reads(&printed, slot), 1, "{body}\n{printed}");
        let off = crate::lower::without_in_place(|| lower_lends(body));
        assert!(off.contains("retain"), "{body}\n{off}");
        assert_eq!(slot_reads(&off, slot), 2, "{body}\n{off}");
    }
}

/// A nested write and a `.=` on a `string` element are one helper call that
/// consumes the root, and a failure leaves the root with the caller. So a
/// plain property or static property is lent to that call as it is to a
/// one-level write. With the pass off, the call takes a retain of the root
/// and the store reads the old value again to release it.
#[test]
fn a_nested_write_into_a_plain_slot_is_lent() {
    for (body, slot) in [
        ("    $this->g[0][1] = 7;", "T::g"),
        ("    $this->g[0][] = 8;", "T::g"),
        ("    $this->u['k'] .= 'x';", "T::u"),
        ("    T::$r[0]['k'] .= 'y';", "T::$r"),
    ] {
        let printed = lower_lends(body);
        assert!(printed.contains("_through v"), "{body}\n{printed}");
        assert!(!printed.contains("retain"), "{body}\n{printed}");
        assert_eq!(slot_reads(&printed, slot), 1, "{body}\n{printed}");
        let off = crate::lower::without_in_place(|| lower_lends(body));
        assert!(off.contains("retain"), "{body}\n{off}");
        assert_eq!(slot_reads(&off, slot), 2, "{body}\n{off}");
    }
}

/// `body` as `T::m(string $x)`'s body, beside a `string` property `o`, a
/// `string` static property `z` and an `array<string>` property `u`, lowered
/// as a whole program so the ownership pass has run.
fn lower_concats(body: &str) -> String {
    let src = format!(
        "<?nvs\nfinal class T {{\n  public string $o = '';\n  public static string $z = '';\n  \
         public array<string> $u = [];\n  \
         public function m(string $x): string {{\n    string $s = '!' . $x;\n{body}\n    \
         return $s;\n  }}\n}}\n"
    );
    let (program, map, file) = lower_whole_file_with_src(&src);
    program
        .functions
        .iter()
        .filter(|f| f.name == "T::m")
        .map(|f| print_function(f, map.file(file)))
        .collect()
}

/// Asserts that `body`, a statement that replaces a target with a join that
/// opens with it, hands the target's reference to the join: a local is moved
/// in, a slot is lent and read once, and an element's array is made sole.
/// With the pass off, the join takes a retain and a slot is read twice.
fn assert_joined_in_place(body: &str, slot: Option<&str>) {
    let printed = lower_concats(body);
    assert!(printed.contains("concat v"), "{body}\n{printed}");
    assert!(!printed.contains("retain v"), "{body}\n{printed}");
    let off = crate::lower::without_in_place(|| lower_concats(body));
    assert!(off.contains("retain v"), "{body}\n{off}");
    match slot {
        Some("T::u") => {
            assert!(printed.contains("= array.sole v"), "{body}\n{printed}");
            assert!(printed.contains("= array.set.lent v"), "{body}\n{printed}");
        }
        Some(slot) => {
            assert_eq!(slot_reads(&printed, slot), 1, "{body}\n{printed}");
            assert_eq!(slot_reads(&off, slot), 2, "{body}\n{off}");
        }
        None => {}
    }
}

/// `t = t . e` writes into `t`'s own buffer on a local, a plain property, a
/// static property and an element. A join whose later piece is the target,
/// or that reads the slot again, keeps its retain.
#[test]
fn every_concatenation_target_appends_in_place() {
    for (body, slot) in [
        ("    $s = $s . $x;", None),
        ("    $s = $s . $x . 'y';", None),
        ("    $this->o = $this->o . $x;", Some("T::o")),
        ("    T::$z = T::$z . $x . 'y';", Some("T::$z")),
        ("    self::$z = self::$z . 'y';", Some("T::$z")),
        ("    $this->u['a'] = $this->u['a'] . $x;", Some("T::u")),
    ] {
        assert_joined_in_place(body, slot);
    }
    for body in ["    $s = $s . $s;", "    $this->o = $this->o . $this->o;"] {
        let printed = lower_concats(body);
        assert!(printed.contains("retain v"), "{body}\n{printed}");
    }
    // `$s` is read after the join, so the join copies it.
    let printed = lower_concats("    string $t = $s;\n    $s = $s . $x;\n    echo $t;");
    assert!(printed.contains("retain v"), "{printed}");
}

/// `t = "{t}…"` is the same join as `t = t . …`, so it writes into `t`'s own
/// buffer on a local, a plain property and an element. A static property
/// cannot open an interpolation, because a hole opens with `{$`.
#[test]
fn an_interpolation_that_opens_with_its_target_appends_in_place() {
    for (body, slot) in [
        ("    $s = \"{$s}-{$x}\";", None),
        ("    $this->o = \"{$this->o}<{$x}>\";", Some("T::o")),
        ("    $this->u['a'] = \"{$this->u['a']}y\";", Some("T::u")),
    ] {
        assert_joined_in_place(body, slot);
    }
}

/// `$xs = [...$xs, …]` at `$xs`'s last use moves `$xs` into the literal's
/// first spread, which then takes its reference. A spread that is not the
/// literal's first item, a local the same literal or a later statement reads
/// again, and the pass switched off keep the borrowed spread.
#[test]
fn a_literal_that_opens_with_a_spread_of_a_local_takes_its_reference() {
    for body in [
        "    $xs = [...$xs, 9];\n    return $xs;",
        "    return [...$xs, 9];",
    ] {
        let printed = lower_moves(body, XS);
        assert!(
            printed.contains("array.spread.owned v"),
            "{body}\n{printed}"
        );
        let off = crate::lower::without_in_place(|| lower_moves(body, XS));
        assert!(!off.contains("array.spread.owned"), "{body}\n{off}");
    }
    for body in [
        "    array<int> $ys = [9, ...$xs];\n    return $ys;",
        "    $xs = [...$xs, 9, ...$xs];\n    return $xs;",
        "    array<int> $ys = [...$xs, 9];\n    $ys = T::f($ys);\n    return T::f($xs);",
    ] {
        let printed = lower_moves(body, XS);
        assert!(!printed.contains("array.spread.owned"), "{body}\n{printed}");
    }
}

/// An `unset` of an element of a plain property or static property is lent
/// the slot's reference, one level and nested: the removal or the climb's
/// last store takes no retain of the root, and the store reads the slot only
/// once. A guarded root is read again after its guard runs, so the guard
/// cannot free the array the removal changes.
#[test]
fn an_unset_of_a_plain_slot_is_lent() {
    for (body, slot) in [
        ("    unset($this->p[0]);", "T::p"),
        ("    unset(T::$s[1]);", "T::$s"),
        ("    unset($this->g[0][1]);", "T::g"),
        ("    unset($this->n->q['a']);", "N::q"),
    ] {
        let printed = lower_lends(body);
        assert_eq!(slot_reads(&printed, slot), 1, "{body}\n{printed}");
        let off = crate::lower::without_in_place(|| lower_lends(body));
        assert_eq!(slot_reads(&off, slot), 2, "{body}\n{off}");
        assert!(
            off.matches("retain ").count() > printed.matches("retain ").count(),
            "{body}\n{off}"
        );
    }
    let guarded = lower_lends("    unset($this->w->p[0]);");
    let call = guarded.find("beforePropertyUnset").unwrap_or(usize::MAX);
    let last_read = guarded.rfind("W::p  ;").unwrap_or(0);
    assert!(
        call < last_read,
        "the root is read after the guard\n{guarded}"
    );
}

/// A guarded root is read again after its guard runs. That second read is
/// lent to the write and to the removal, so the store reads the slot only
/// once after the guard.
#[test]
fn a_guarded_root_is_lent_after_its_guard() {
    for (body, guard) in [
        ("    $this->w->p[0] = 1;", "beforePropertySet"),
        ("    unset($this->w->p[0]);", "beforePropertyUnset"),
    ] {
        let printed = lower_lends(body);
        let after = &printed[printed.find(guard).unwrap_or(0)..];
        assert_eq!(slot_reads(after, "W::p"), 1, "{body}\n{printed}");
        let off = crate::lower::without_in_place(|| lower_lends(body));
        let after = &off[off.find(guard).unwrap_or(0)..];
        assert_eq!(slot_reads(after, "W::p"), 2, "{body}\n{off}");
    }
}

/// A `lateinit` root is initialized in blocks of its own, and its read and
/// its store share the block after, so it is lent. A union receiver reads
/// and stores back in one block per class, so its write keeps the lowering's
/// store, which takes no retain and reads the slot once.
#[test]
fn a_lateinit_root_is_lent_and_a_union_root_keeps_its_store() {
    let src = "<?nvs\nfinal class A {\n  public array<int> $p = [];\n}\nfinal class B {\n  public array<int> $p = [];\n}\n\
               final class T implements LateInit {\n  public lateinit array<int> $l;\n  \
               public function initProperty(string $name): void { $this->l = []; }\n  \
               public function m(A|B $u): void {\n    $u->p[0] = 1;\n    $this->l[0] = 2;\n  }\n}\n";
    let printed = |in_place: bool| {
        let lower = || lower_whole_file_with_src(src);
        let (program, map, file) = if in_place {
            lower()
        } else {
            crate::lower::without_in_place(lower)
        };
        program
            .functions
            .iter()
            .filter(|f| f.name == "T::m")
            .map(|f| print_function(f, map.file(file)))
            .collect::<String>()
    };
    let lent = printed(true);
    assert_eq!(slot_reads(&lent, "T::l"), 1, "{lent}");
    assert_eq!(slot_reads(&lent, "A::p"), 1, "{lent}");
    assert!(!lent.contains("retain "), "{lent}");
    let kept = printed(false);
    assert_eq!(slot_reads(&kept, "T::l"), 2, "{kept}");
    assert_eq!(slot_reads(&kept, "A::p"), 1, "{kept}");
}

/// A hooked property is read and written by its hooks, so there is no slot to
/// lend. A guarded one runs the guard between the call and the store.
#[test]
fn a_hooked_or_guarded_slot_keeps_the_retain() {
    let printed = lower_lends("    $this->h = Core\\Arr::slice($this->h, 1);");
    assert!(printed.contains("call T::$h::set("), "{printed}");
    assert_eq!(slot_reads(&printed, "T::h"), 0, "{printed}");
    assert_kept(
        "    $this->w->p = Core\\Arr::slice($this->w->p, 1);",
        "W::p",
    );
}

/// A call to user code, a second read of the slot or a write of a container
/// between the read and the store keeps the slot's reference.
#[test]
fn code_between_the_read_and_the_store_keeps_the_retain() {
    for body in [
        "    $this->p = Core\\Arr::slice($this->p, T::f());",
        "    $this->p = Core\\Arr::slice($this->p, Core\\Arr::count($this->p) as int);",
        "    $this->n->q = Core\\Arr::slice($this->n->q, ($this->n = new N()) is N ? 1 : 0);",
    ] {
        let printed = lower_lends(body);
        assert!(
            slot_reads(&printed, "T::p") + slot_reads(&printed, "N::q") >= 2,
            "{body}\n{printed}"
        );
    }
}

/// A callback runs before the member changes anything, so one that only
/// reads the slot sees the old value, and the slot is lent.
#[test]
fn a_callback_that_reads_the_slot_sees_the_old_value() {
    assert_lent(
        "    $this->p = Core\\Arr::filter($this->p, fn(int $x): bool => Core\\Arr::count($this->p) > $x);",
        "T::p",
    );
}

/// `.=` on a plain `string` property or static property is lent to its
/// append: the pass removes the retain, the second read and the release the
/// lowering put around it. With in-place changes off, all three stay.
#[test]
fn a_slot_append_is_lent() {
    let src = "<?nvs\nfinal class T {\n  public string $o = '';\n  public static string $s = '';\n  \
               public function m(string $x): void {\n    $this->o .= $x;\n    T::$s .= \"<{$x}>\";\n  }\n}\n";
    let printed = |in_place: bool| {
        let lower = || lower_whole_file_with_src(src);
        let (program, map, file) = if in_place {
            lower()
        } else {
            crate::lower::without_in_place(lower)
        };
        program
            .functions
            .iter()
            .filter(|f| f.name == "T::m")
            .map(|f| print_function(f, map.file(file)))
            .collect::<String>()
    };
    let lent = printed(true);
    assert!(lent.contains("str.append"), "{lent}");
    assert_eq!(slot_reads(&lent, "T::o"), 1, "{lent}");
    assert_eq!(slot_reads(&lent, "T::$s"), 1, "{lent}");
    assert!(!lent.contains("retain "), "{lent}");
    let kept = printed(false);
    assert_eq!(slot_reads(&kept, "T::o"), 2, "{kept}");
    assert_eq!(slot_reads(&kept, "T::$s"), 2, "{kept}");
    assert_eq!(kept.matches("retain ").count(), 2, "{kept}");
}

/// A callback that writes the slot, or one whose body is not known, keeps
/// the slot's reference, so the write cannot free the array the member is
/// reading.
#[test]
fn a_callback_that_writes_the_slot_keeps_the_retain() {
    assert_kept(
        "    $this->p = Core\\Arr::filter($this->p, fn(int $x): bool => { $this->p = []; return true; });",
        "T::p",
    );
    assert_kept("    $this->p = Core\\Arr::filter($this->p, $cb);", "T::p");
}

/// Two functions that call each other reach one summary: the union of what
/// each body touches. A function outside the cycle keeps its own, and one
/// that makes a virtual call may touch anything.
#[test]
fn recursive_callees_reach_one_summary() {
    use crate::effects::{Summaries, Touches};
    let (p, _, _) = lower_program(concat!(
        "<?nvs\n",
        "class B {\n",
        "  public function m(): void {}\n",
        "}\n",
        "class C extends B {\n",
        "  public function m(): void {}\n",
        "}\n",
        "final class T {\n",
        "  public static array<int> $x = [];\n",
        "  public static array<int> $y = [];\n",
        "  public static function a(int $n): void {\n",
        "    if ($n > 0) { T::b($n - 1); }\n",
        "    T::$x = [];\n",
        "  }\n",
        "  public static function b(int $n): void {\n",
        "    array<int> $v = T::$y;\n",
        "    T::a($n);\n",
        "  }\n",
        "  public static function c(): void {\n",
        "    T::$y = [];\n",
        "  }\n",
        "  public static function d(B $b): void {\n",
        "    $b->m();\n",
        "  }\n",
        "}\n",
    ));
    let summaries = Summaries::of(&p.functions);
    let a = summaries.get("T::a").expect("T::a");
    let b = summaries.get("T::b").expect("T::b");
    assert_eq!(a, b);
    let x = crate::ir::Slot::Static {
        class: "T",
        name: "x",
    };
    let y = crate::ir::Slot::Static {
        class: "T",
        name: "y",
    };
    assert!(a.writes.reaches(x) && !a.writes.reaches(y), "{a:?}");
    // `b`'s read of `y` reaches `a` through the call. A store reads the old
    // value it releases, so `a` reads `x` too.
    assert!(a.reads.reaches(y), "{a:?}");
    let c = summaries.get("T::c").expect("T::c");
    assert!(c.writes.reaches(y) && !c.writes.reaches(x), "{c:?}");
    assert!(!c.reads.reaches(x), "{c:?}");
    let d = summaries.get("T::d").expect("T::d");
    assert_eq!(d.writes, Touches::Anything, "{d:?}");
}

/// A local inside a `try` whose `catch` reads it cannot move into the call,
/// so it is lent to the callee's lent body: the call takes no retain, and
/// the callee appends in place. The `catch` still sees the old value.
#[test]
fn a_local_inside_a_try_is_lent_to_a_callee_that_cannot_fail_after_its_change() {
    let body = "    array<int> $a = [$x];\n    try {\n      $a = $this->add($a, $x);\n    } \
                catch (LogicError $e) {\n      $this->p = $a;\n    }\n    $this->p = $a;";
    let printed = lower_callees(ADD, body);
    let m = function_named(&printed, "T::m");
    let call = m.find("call T::add#lent1(").expect(&printed);
    let line = &m[call..m[call..].find('\n').map_or(m.len(), |end| call + end)];
    let arg = line
        .split(['(', ',', ')'])
        .nth(2)
        .map(str::trim)
        .expect(&printed);
    assert!(
        !m[..call].contains(&format!("retain {arg}\n")),
        "{arg}\n{printed}"
    );
    let lent = function_named(&printed, "T::add#lent1");
    assert!(lent.contains("array.append v1"), "{printed}");
    assert!(!lent.contains("retain v1"), "{printed}");
}

/// A class `T` with an array property `$p`, the methods `methods`, and a
/// method `m(int $x)` whose body is `body`, lowered as a whole program. Every
/// function of `T` is printed, its lent bodies included.
fn lower_callees(methods: &str, body: &str) -> String {
    let src = format!(
        "<?nvs\nclass B {{\n  public function add(array<int> $items, int $x): array<int> {{ return $items; }}\n}}\n\
         class C extends B {{\n  public function add(array<int> $items, int $x): array<int> {{ $items[] = $x; return $items; }}\n}}\n\
         final class T {{\n  public array<int> $p = [];\n  public static array<int> $s = [];\n{methods}\n  \
         public function m(int $x): void {{\n{body}\n  }}\n}}\n"
    );
    let (program, map, file) = lower_whole_file_with_src(&src);
    program
        .functions
        .iter()
        .filter(|f| f.name.starts_with("T::"))
        .map(|f| print_function(f, map.file(file)))
        .collect()
}

/// The printed function `name` out of `printed`, or an empty string.
fn function_named<'a>(printed: &'a str, name: &str) -> &'a str {
    let head = format!("fn {name}(");
    let Some(start) = printed.find(&head) else {
        return "";
    };
    let rest = &printed[start..];
    &rest[..rest.find("\n}").map_or(rest.len(), |end| end + 2)]
}

const ADD: &str = "  public function add(array<int> $items, int $x): array<int> {\n    $items[] = $x;\n    return $items;\n  }";

const ADD_THEN_THROW: &str = "  public function add(array<int> $items, int $x): array<int> {\n    $items[] = $x;\n    \
                              if ($x > 5) { throw new LogicError('no'); }\n    return $items;\n  }";

/// A callee that cannot fail after its append is lent the property: the call
/// takes no retain and reads the slot once, and the lent body appends in
/// place, with no retain of its parameter.
#[test]
fn a_callee_that_cannot_fail_after_its_change_changes_the_slot_in_place() {
    let printed = lower_callees(ADD, "    $this->p = $this->add($this->p, $x);");
    let m = function_named(&printed, "T::m");
    assert!(m.contains("call T::add#lent1("), "{printed}");
    assert_eq!(slot_reads(m, "T::p"), 1, "{printed}");
    assert!(!m.contains("retain v2"), "{printed}");
    let lent = function_named(&printed, "T::add#lent1");
    assert!(lent.contains("array.append v1"), "{printed}");
    assert!(!lent.contains("retain v1"), "{printed}");
    let off = crate::lower::without_in_place(|| {
        lower_callees(ADD, "    $this->p = $this->add($this->p, $x);")
    });
    assert!(!off.contains("#lent"), "{off}");
}

/// A callee that may throw after its append copies at that append: the lent
/// body retains its parameter first, and releases the caller's reference
/// before it returns. Its error exits after the append free only the copy.
#[test]
fn a_callee_that_can_fail_after_its_change_copies_at_that_change_only() {
    let printed = lower_callees(ADD_THEN_THROW, "    $this->p = $this->add($this->p, $x);");
    let lent = function_named(&printed, "T::add#lent1");
    let retain = lent.find("retain v1").unwrap_or(usize::MAX);
    let append = lent.find("array.append v1").unwrap_or(0);
    assert!(
        retain < append,
        "the parameter is retained first\n{printed}"
    );
    assert_eq!(lent.matches("retain v1").count(), 1, "{printed}");
    let ret = lent.find("return v3").unwrap_or(0);
    let release = lent[..ret].rfind("release v1").unwrap_or(0);
    assert!(
        lent[release..ret].lines().count() <= 2,
        "the caller's reference is released before the return\n{printed}"
    );
}

/// The lent body never frees its parameter on a path that ends in a
/// failure: the entry poll's landing and the append's landing leave it with
/// the caller. The callee's own body frees it on both.
#[test]
fn a_lent_parameter_is_not_freed_on_an_error_exit() {
    let printed = lower_callees(ADD, "    $this->p = $this->add($this->p, $x);");
    let own = function_named(&printed, "T::add");
    assert_eq!(own.matches("release v1").count(), 2, "{printed}");
    let lent = function_named(&printed, "T::add#lent1");
    assert_eq!(lent.matches("release v1").count(), 0, "{printed}");
    assert_eq!(lent.matches("propagate").count(), 2, "{printed}");
}

/// A read of the slot before the change sees the old value, so the change is
/// in place. A static property is lent the same way.
#[test]
fn a_callee_that_reads_the_slot_before_its_change_changes_it_in_place() {
    let methods = "  public function add(array<int> $items, int $x): array<int> {\n    \
                   int $n = Core\\Arr::count($this->p) as int;\n    $items[] = $x + $n;\n    return $items;\n  }\n  \
                   public static function put(array<int> $items, int $x): array<int> {\n    \
                   int $n = Core\\Arr::count(T::$s) as int;\n    $items[] = $x + $n;\n    return $items;\n  }";
    let printed = lower_callees(
        methods,
        "    $this->p = $this->add($this->p, $x);\n    T::$s = T::put(T::$s, $x);",
    );
    for name in ["T::add#lent1", "T::put#lent1"] {
        let lent = function_named(&printed, name);
        assert!(lent.contains("array.append"), "{name}\n{printed}");
        assert!(!lent.contains("retain v"), "{name}\n{printed}");
    }
}

/// A read of the slot after the change would see the new value while the
/// slot still has the old one, so the lent body copies at the change.
#[test]
fn a_callee_that_reads_the_slot_after_its_change_copies_at_that_change_only() {
    let methods = "  public function add(array<int> $items, int $x): array<int> {\n    $items[] = $x;\n    \
                   int $n = Core\\Arr::count($this->p) as int;\n    $items[] = $n;\n    return $items;\n  }";
    let printed = lower_callees(methods, "    $this->p = $this->add($this->p, $x);");
    let lent = function_named(&printed, "T::add#lent1");
    assert_eq!(lent.matches("retain v1").count(), 1, "{printed}");
    let retain = lent.find("retain v1").unwrap_or(usize::MAX);
    let append = lent.find("array.append v1").unwrap_or(0);
    assert!(retain < append, "{printed}");
}

/// Every copy the pass keeps is a row of the program's copy table, on the
/// line of the statement that copies: a lent body's change before a failure
/// and before a read of the slot, and a local passed to an owning member and
/// read again. A lent call keeps no copy. Only the read that can move above
/// the change has a fix, and its edit moves that one line.
#[test]
fn every_copy_the_pass_keeps_is_in_the_table() {
    use crate::copies::Reason;
    let src = "<?nvs\nfinal class T {\n  public array<int> $p = [];\n  \
               public function fails(array<int> $items, int $x): array<int> {\n    \
               $items[] = $x;\n    if ($x > 5) { throw new LogicError('no'); }\n    return $items;\n  }\n  \
               public function reads(array<int> $items, int $x): array<int> {\n    \
               $items[] = $x;\n    echo Core\\Arr::count($this->p);\n    return $items;\n  }\n  \
               public function m(int $x): void {\n    \
               $this->p = $this->fails($this->p, $x);\n    \
               $this->p = $this->reads($this->p, $x);\n    \
               array<int> $l = [1, 2];\n    \
               array<int> $k = Core\\Arr::append($l, $x);\n    \
               echo Core\\Arr::count($l), Core\\Arr::count($k);\n  }\n}\n";
    let (program, map, _) = lower_whole_file_with_src(src);
    let table = program.copy_table(&map);
    let at = |line: u32, reason: Reason| {
        table
            .iter()
            .find(|row| row.line == line && row.reason == reason)
            .unwrap_or_else(|| panic!("no {reason:?} row on line {line}: {table:#?}"))
    };
    assert!(
        at(5, Reason::FailureAfterChange).edit.is_none(),
        "{table:#?}"
    );
    let read = at(10, Reason::ReadAfterChange);
    let edit = read
        .edit
        .as_ref()
        .expect("the read can move above the change");
    assert_eq!(edit.text, "    echo Core\\Arr::count($this->p);\n");
    assert_eq!(&src[edit.insert_at as usize..][..18], "    $items[] = $x;");
    assert_eq!(
        &src[edit.delete.0 as usize..edit.delete.1 as usize],
        edit.text
    );
    at(18, Reason::StillUsed);
    assert!(
        table.iter().all(|row| row.line != 15 && row.line != 16),
        "a lent call keeps no copy: {table:#?}"
    );
    assert!(
        table
            .iter()
            .all(|row| row.edit.is_none() || row.reason.warns()),
        "{table:#?}"
    );
    assert!(
        table
            .iter()
            .all(|row| !row.text_hash.is_empty() && row.file == "t.nvs")
    );
    let off = crate::lower::without_in_place(|| lower_whole_file_with_src(src).0);
    assert!(off.copies.is_empty(), "the pass did not run");
}

/// A kept copy is counted where it is made: the refused move of `$l` into
/// `Core\Arr::append` gets a `copy.count` with its row's id right after the
/// call, against the local and the call's result.
#[test]
fn a_kept_copy_is_counted_right_after_its_call() {
    use crate::copies::Reason;
    let src = "<?nvs\nfinal class T {\n  public function m(int $x): void {\n    \
               array<int> $l = [1, 2];\n    \
               array<int> $k = Core\\Arr::append($l, $x);\n    \
               echo Core\\Arr::count($l), Core\\Arr::count($k);\n  }\n}\n";
    let (program, map, file) = lower_whole_file_with_src(src);
    let printed = print_program(&program, map.file(file));
    let index = program
        .copies
        .iter()
        .position(|copy| copy.reason == Reason::StillUsed)
        .unwrap_or_else(|| panic!("no still-used row: {:#?}", program.copies));
    let site = u32::try_from(index + 1).expect("a small table");
    let counted = program.functions.iter().find_map(|f| {
        f.blocks.iter().find_map(|block| {
            block
                .insts
                .iter()
                .enumerate()
                .find_map(|(i, inst)| match inst.kind {
                    InstKind::CopyCount {
                        site: at,
                        ty,
                        before,
                        after,
                    } if at == site => Some((block, i, ty, before, after)),
                    _ => None,
                })
        })
    });
    let (block, i, ty, before, after) =
        counted.unwrap_or_else(|| panic!("no copy.count {site}:\n{printed}"));
    assert_eq!(ty, Ty::Array, "{printed}");
    let call = &block.insts[i - 1];
    assert_eq!(
        call.result,
        Some(after),
        "the count follows the call:\n{printed}"
    );
    let InstKind::CoreCall { args, .. } = &call.kind else {
        panic!("the count follows a Core call:\n{printed}");
    };
    assert_eq!(args.first(), Some(&before), "{printed}");
    assert!(
        printed.contains(&format!("copy.count {site}, v{}", before.index())),
        "{printed}"
    );
}

/// Only a separated change whose offending statement can move above it
/// warns: the read after the change does, with `W1032`, its span on the
/// change and its fix moving the read. The failure that cannot move, the
/// still-used local and the lent calls do not.
#[test]
fn a_warning_is_raised_exactly_where_an_easy_fix_removes_a_copy() {
    let src = "<?nvs\nfinal class T {\n  public array<int> $p = [];\n  \
               public function fails(array<int> $items, int $x): array<int> {\n    \
               $items[] = $x;\n    if ($x > 5) { throw new LogicError('no'); }\n    return $items;\n  }\n  \
               public function reads(array<int> $items, int $x): array<int> {\n    \
               $items[] = $x;\n    echo Core\\Arr::count($this->p);\n    return $items;\n  }\n  \
               public function m(int $x): void {\n    \
               $this->p = $this->fails($this->p, $x);\n    \
               $this->p = $this->reads($this->p, $x);\n    \
               array<int> $l = [1, 2];\n    \
               array<int> $k = Core\\Arr::append($l, $x);\n    \
               echo Core\\Arr::count($l), Core\\Arr::count($k);\n  }\n}\n";
    let (program, map, _) = lower_whole_file_with_src(src);
    let warnings = crate::copies::warnings(&program.copies, &map);
    assert_eq!(warnings.len(), 1, "{warnings:#?}");
    let warning = &warnings[0];
    assert_eq!(
        warning.code,
        Some(nvs_diagnostics::code::W_COPY_READ_AFTER_CHANGE)
    );
    let line = |span: nvs_diagnostics::Span| src[..span.start as usize].matches('\n').count() + 1;
    assert_eq!(line(warning.primary_span().expect("a primary label")), 10);
    assert_eq!(line(warning.labels[1].span), 11);
    let fix = &warning.suggestions[0];
    assert!(!fix.safe, "a moved statement is reviewed: {fix:#?}");
    let mut fixed = src.to_owned();
    fixed.replace_range(
        fix.span.start as usize..fix.span.end as usize,
        &fix.replacement,
    );
    assert!(
        fixed.contains(
            "    echo Core\\Arr::count($this->p);\n    $items[] = $x;\n    return $items;"
        ),
        "{fixed}"
    );
    let (fixed_program, fixed_map, _) = lower_whole_file_with_src(&fixed);
    assert!(
        crate::copies::warnings(&fixed_program.copies, &fixed_map).is_empty(),
        "the fix removes the copy"
    );
}

/// A call that may throw after the change, and can move above it, warns with
/// `W1033`, its span on the change and a second label on the call.
#[test]
fn a_failure_after_the_change_warns_with_its_own_code() {
    let src = "<?nvs\nfinal class T {\n  public array<int> $p = [];\n  \
               public function add(array<int> $items, int $x): array<int> {\n    \
               $items[] = $x;\n    $this->check($x);\n    return $items;\n  }\n  \
               private function check(int $x): void {\n    \
               if ($x > 5) { throw new LogicError('no'); }\n  }\n  \
               public function m(int $x): void {\n    \
               $this->p = $this->add($this->p, $x);\n  }\n}\n";
    let (program, map, _) = lower_whole_file_with_src(src);
    let warnings = crate::copies::warnings(&program.copies, &map);
    assert_eq!(warnings.len(), 1, "{warnings:#?}");
    let warning = &warnings[0];
    assert_eq!(
        warning.code,
        Some(nvs_diagnostics::code::W_COPY_FAILURE_AFTER_CHANGE)
    );
    let line = |span: nvs_diagnostics::Span| src[..span.start as usize].matches('\n').count() + 1;
    assert_eq!(line(warning.primary_span().expect("a primary label")), 5);
    assert_eq!(line(warning.labels[1].span), 6);
}

/// A callee that writes the slot it would be lent keeps the call's retain:
/// its write cannot free the array it is changing, and the caller's store
/// still releases the slot's old value.
#[test]
fn a_callee_that_writes_the_slot_keeps_both_values_alive() {
    let methods = "  public function add(array<int> $items, int $x): array<int> {\n    $this->p = [];\n    \
                   $items[] = $x;\n    return $items;\n  }";
    let printed = lower_callees(methods, "    $this->p = $this->add($this->p, $x);");
    let m = function_named(&printed, "T::m");
    assert!(m.contains("call T::add("), "{printed}");
    assert!(!printed.contains("#lent"), "{printed}");
    assert_eq!(slot_reads(m, "T::p"), 2, "{printed}");
}

/// A method that a subclass overrides may reach either body, so its call has
/// no one callee and keeps its retain.
#[test]
fn an_overridable_or_dynamic_callee_keeps_the_retain() {
    let printed = lower_callees(
        "",
        "    B $b = new C();\n    $this->p = $b->add($this->p, $x);",
    );
    let m = function_named(&printed, "T::m");
    assert!(!printed.contains("#lent"), "{printed}");
    assert_eq!(slot_reads(m, "T::p"), 2, "{printed}");
}

/// A string built after the change can only stop the request when memory
/// runs out, which no `catch` sees. It has no error edge, so the lent body
/// changes the parameter in place.
#[test]
fn a_fatal_is_not_a_failure_the_summary_counts() {
    let methods = "  public function add(array<int> $items, int $x, string $t): array<int> {\n    $items[] = $x;\n    \
                   string $label = $t . 'm';\n    return $items;\n  }";
    let printed = lower_callees(methods, "    $this->p = $this->add($this->p, $x, 'k');");
    assert!(printed.contains("concat"), "{printed}");
    let lent = function_named(&printed, "T::add#lent1");
    assert!(lent.contains("array.append v1"), "{printed}");
    assert!(!lent.contains("retain v1"), "{printed}");
}

/// A second holder that is never read again is released right after its
/// last use, so the call holds the only reference.
#[test]
fn a_dead_binding_is_released_at_its_last_use() {
    let printed = lower_moves(
        "    array<int> $ys = $xs;\n    $xs = Core\\Arr::slice($xs, 1);\n    return $xs;",
        XS,
    );
    assert!(printed.contains("_owned("), "{printed}");
    let printed = lower_moves(
        "    array<int> $ys = $xs;\n    $xs = Core\\Arr::slice($xs, 1);\n    return $ys;",
        XS,
    );
    assert!(!printed.contains("_owned("), "{printed}");
}

#[test]
fn unsetting_an_element_through_an_int_key_still_renders_the_decimal() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(array<int> $a): void {\n    unset($a[0]);\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$a[$k]` where both the array's element and the key are `string`
/// locals — the key is already `Ty::Str` and is a bare variable read
/// (`is_aliasing_read`), so it needs no conversion and, unlike the
/// literal-key case above, is *not* released after the read (`$k`'s own
/// slot still owns it). Binding the `array<string>` element itself to
/// `$s` retains it first, since `ExprKind::Index` is one of
/// `is_aliasing_read`'s recognized shapes — exactly the same policy a
/// property read already gets.
#[test]
fn reading_a_string_element_through_a_string_local_key_retains_the_result() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(array<string> $a, string $k): void {\n    var $s = $a[$k];\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$a[0] = 5;` through an `array<int>` parameter — the simplest
/// array-element write: a fresh, non-refcounted `int` value (no retain)
/// and a literal `int` key carried unrendered exactly the way the read
/// side carries it, with no old-value get/release pair at all
/// (`InstKind::ArraySet`'s own doc comment explains why an ordinary
/// new-or-existing-key write bundles that into one instruction rather
/// than splitting it like `FieldSet` does).
#[test]
fn writing_an_int_element_through_a_literal_key_carries_the_integer_unrendered() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(array<int> $a): void {\n    $a[0] = 5;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$a[$k] = $v;` where the array's element, the key, and the new value
/// are all `string` locals — both the key and the value are aliasing
/// reads of their own slots, so both get retained before
/// `InstKind::ArraySet` runs; `$a`/`$k`/`$v` each still get their
/// ordinary release at `m`'s exit sweep.
#[test]
fn writing_a_string_element_through_a_string_local_key_retains_both_key_and_value() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(array<string> $a, string $k, string $v): void {\n    $a[$k] = $v;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$a[] = 1;` — PHP's append syntax lowers to `InstKind::ArrayAppend`
/// with no key at all, unlike every other `Index`-target write: a fresh,
/// non-refcounted `int` value needs no retain, mirroring
/// `writing_an_int_element_through_a_literal_key_carries_the_integer_unrendered`
/// but with no `lower_array_key` call in the output at all, since there is
/// no key to lower.
///
/// It is the one array write with an error edge (` ! bb1`), so the snapshot
/// also carries a landing block — see `Lowering::emit_array_append`.
#[test]
fn appending_a_fresh_int_value_needs_no_retain() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(array<int> $a): void {\n    $a[] = 1;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$a[] = $v;` where the appended value is a `string` local — an
/// aliasing read of `$v`'s own slot, so it is retained before
/// `InstKind::ArrayAppend` runs, the same policy
/// `writing_a_string_element_through_a_string_local_key_retains_both_key_and_value`
/// already gives an explicit key's value; `$a`/`$v` each still get their
/// ordinary release at `m`'s exit sweep.
///
/// The landing block releases both locals and nothing more: the refusal
/// path inside `nvs_runtime::nvs_array_append` releases the extra reference
/// the retain above staged, and leaves the array's where this frame's own
/// slot still names it.
#[test]
fn appending_an_aliasing_string_value_retains_it() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(array<string> $a, string $v): void {\n    $a[] = $v;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Two writes into one local: the second reads the array the *first*
/// yielded, not the one the parameter arrived as, and the exit sweep
/// releases the last one only. That chain is `rule:types/arrays`'s copy-on-write
/// separation being written back — see `Lowering::write_back_array` — and
/// it is the whole reason `InstKind::ArraySet` defines a value.
#[test]
fn a_second_write_reads_the_array_the_first_one_yielded() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(array<int> $a): void {\n    $a[0] = 5;\n    $a[1] = 6;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$this->rows[$k] = $v;` — the other holder a separation can be written
/// back to. The yielded array goes straight into the property slot with a
/// bare `field.set` and no retain or release: the reference the write
/// consumed was the slot's own, and the one it produced replaces it there.
/// The ownership pass lends the slot, so a debug build also poisons it
/// around the write.
#[test]
fn writing_an_element_through_a_property_base_stores_the_result_back() {
    assert_snapshot!(print_shipped_method(
        "<?nvs\nclass T {\n  public array<int> $rows;\n  function m(): void {\n    $this->rows[0] = 5;\n  }\n  function constructor() { $this->rows = []; }\n}\n",
    ));
}

/// `$grid[0][1] = 5;` lowers to one `array_write_through`, whose result is
/// written back to `$g`. No row is read on the way down and no `array.set`
/// climbs back out, so nothing adds a reference to a row and an unshared one
/// is written where it is stored. The keys are borrowed, so neither is
/// retained. A write into one level is still one `array.set`.
#[test]
fn a_nested_write_lowers_to_one_write_through() {
    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
  function m(array<array<int>> $g, string $k): void {
    $g[0][$k] = 5;
  }
}
",
    );
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("array_write_through").count(), 1, "{text}");
    assert!(!text.contains("array.set"), "a climbing set: {text}");
    assert!(
        !text.contains("array.get"),
        "a row read on the way down: {text}"
    );
    assert!(
        !text.contains("retain"),
        "a borrowed key is retained: {text}"
    );

    // A deeper chain passes its inner keys as one array, freed by the
    // statement, and an inner append is a `null` in it.
    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
  function m(array<array<array<int>>> $g): void {
    $g[0][][\"k\"] = 5;
  }
}
",
    );
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("array_write_through").count(), 1, "{text}");
    assert!(text.contains("const.null"), "an inner append's key: {text}");
    assert!(!text.contains("array.set"), "a climbing set: {text}");

    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
  function m(array<int> $g): void {
    $g[0] = 5;
  }
}
",
    );
    let text = print_function(&f, map.file(file));
    assert!(!text.contains("array_write_through"), "{text}");
    assert_eq!(text.matches("array.set").count(), 1, "{text}");
}

/// A `bool` subscript is not one of `rule:types/arrays`'s legal key source
/// types (`int`/`uint`/`string`) — `nvs_types::expr::check_array_key_type`
/// rejects it at check time (see `nvs_types::check`'s own
/// `a_bool_key_array_literal_is_diagnosed`-style fixtures for the
/// diagnostic side), so `lower_first_method`'s own `check_program` call
/// fails the fixture before lowering ever runs —
/// `Lowering::lower_array_key`'s `other` panic arm is unreachable for
/// this input, not the thing this test demonstrates.
#[test]
#[should_panic(expected = "fixture failed to check")]
fn a_bool_subscript_key_is_rejected_before_lowering_even_runs() {
    lower_first_method(
        "<?nvs\nclass T {\n  function m(array<int> $a, bool $b): void {\n    $a[$b] = 1;\n  }\n}\n",
    );
}

/// `$a && $b` — `rule:expressions/truthy-positions`'s short-circuit `&&`: `$a`'s own truthy
/// test branches straight to a merge block carrying `const.bool false`
/// when falsy, only evaluating `$b` (through its own truthy test) on the
/// truthy path — `Lowering::lower_and`'s branch/`Phi`-merge shape.
#[test]
fn and_short_circuits_to_a_phi() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bool $a, bool $b): bool {\n    return $a && $b;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$a || $b` — `Lowering::lower_or`'s mirror of `and_short_circuits_to_a_phi`:
/// the short-circuit edge (truthy `$a`) carries `const.bool true` instead,
/// and `$b` is only evaluated when `$a` is falsy.
#[test]
fn or_short_circuits_to_a_phi() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bool $a, bool $b): bool {\n    return $a || $b;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `!$s` with a `string` operand — unary `!` always produces `Ty::Bool`,
/// per `rule:expressions/truthy-positions`, whatever representation its operand carries.
/// `$s` converts through `Helper::StrTruthy` first, then negates —
/// `$s` is a bare parameter read (`is_aliasing_read`), so no release
/// follows the helper call, same as any other truthy-tested aliasing
/// read.
#[test]
fn not_converts_a_non_bool_operand_through_the_truthy_table_then_negates() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(string $s): bool {\n    return !$s;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `!($a && $b)` — `!`'s operand is itself a short-circuit `&&`, composing
/// through `Lowering::lower_not`'s own `Lowering::lower_expr` call.
#[test]
fn not_composes_with_a_short_circuit_and() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bool $a, bool $b): bool {\n    return !($a && $b);\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `if ($a && $b)` — an `if`'s own condition is itself a short-circuit
/// `&&`, exercising `Lowering::lower_if`'s call into
/// `Lowering::lower_truthy_cond` with a mutable `cur` that `&&`'s own
/// branch/merge shape gets to redirect before the `if`'s own `Branch`
/// terminator is sealed.
#[test]
fn if_condition_short_circuits_with_and() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bool $a, bool $b): bool {\n    if ($a && $b) {\n      return true;\n    }\n    return false;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `while ($a || $b)` — the loop header hosting a branching condition:
/// the header phi for `$a` lives in the fixed loop-header block, but the
/// loop's own `Branch` terminator seals onto `cond_end` (wherever `||`'s
/// own merge block ended up), not the header block itself.
#[test]
fn while_condition_short_circuits_with_or() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bool $a, bool $b): void {\n    while ($a || $b) {\n      $a = false;\n    }\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$c ? $a : $b` with both branches the same `int` — `Lowering::
/// lower_ternary`'s ordinary (non-elvis) shape: `cond`'s own value is
/// released once `truthy_convert` reads it (nothing reuses it, unlike
/// elvis), and the two branches join through a fresh `Phi`.
#[test]
fn ternary_with_matching_branch_types_merges_with_a_phi() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bool $c, int $a, int $b): int {\n    return $c ? $a : $b;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$a ?: $b` (elvis) where `$a` is a non-refcounted `int` — the truthy
/// path reuses `$a`'s own value as the ternary's result with neither a
/// retain nor a release, since `Ty::Int` isn't `is_refcounted` at all;
/// this is the "nothing to own" half of elvis's reuse rule.
#[test]
fn elvis_with_a_non_refcounted_condition_needs_no_retain() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(int $a, int $b): int {\n    return $a ?: $b;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$s ?: $d` (elvis) where `$s` is a `string` parameter — `$s` is an
/// aliasing read (its own parameter slot still owns it), so reusing it as
/// the truthy path's value needs a retain (a second, independent owner:
/// the ternary's own result) rather than the release every other
/// truthy-tested position would apply here.
#[test]
fn elvis_retains_an_aliased_refcounted_condition() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(string $s, string $d): string {\n    return $s ?: $d;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `self::make() ?: \"x\"` (elvis) where `cond` is a *fresh*, non-aliasing
/// `string` (a call's own result) — the opposite corner from
/// `elvis_retains_an_aliased_refcounted_condition`: reusing it needs
/// neither a retain nor a release, since it already has exactly one
/// owner, which simply transfers to become the ternary's result.
#[test]
fn elvis_transfers_a_fresh_refcounted_condition_with_no_retain_or_release() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(): string {\n    return self::make() ?: \"x\";\n  }\n  static function make(): string {\n    return \"y\";\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A ternary whose `then`/`else` branches lower to two different
/// `crate::ty::Ty` representations (`int` vs `string`) — each branch is
/// tagged in its **own** block, ahead of its jump, and the phi carries
/// `Ty::Tagged`, which is exactly what `erase_checked_ty` gives the union
/// the checker already typed the whole expression as. See
/// `Lowering::join_representations` for why that is the erasure rather
/// than a promotion of one side into the other.
#[test]
fn a_ternary_with_mismatched_branch_types_joins_at_the_tagged_representation() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bool $c): mixed {\n    mixed $r = $c ? 1 : \"x\";\n    return $r;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// The same rule over more than two branches: a `match` whose arms lower
/// to `int`, `float` and `string` tags each of them in its own arm block
/// and joins at one `Ty::Tagged` phi — `Lowering::lower_match` shares
/// `Lowering::join_representations` with the ternary rather than owning a
/// second rule.
#[test]
fn match_arms_in_three_representations_join_at_the_tagged_representation() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(int $k): mixed {\n    mixed $r = match ($k) { 1 => 1, 2 => 2.5, default => \"x\" };\n    return $r;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A short-circuiting `&&` nested inside a **call argument** — a position
/// that needs a `&mut BlockId` of its own to redirect.
/// `Lowering::lower_expr` owns one, so the argument's own branch/merge is
/// spliced into the caller's block chain and the call is emitted in
/// whichever block the merge ended in.
#[test]
fn a_short_circuit_and_nested_in_a_call_argument_composes() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(bool $a, bool $b): void {\n    self::take($a && $b);\n  }\n  static function take(bool $x): void {}\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A plain `break;` inside a nested `if`, with no reassignment along the
/// break path that would ever differ from the loop's own steady-state
/// value — `Lowering::merge_envs` degenerates to a plain clone (the same
/// `[(_, only)]` shape a break-free loop already produced) rather than a
/// spurious phi, but the CFG itself gains the extra break block/edge:
/// this is mostly a shape test confirming `break` lowers to a `Jump`
/// straight to the after-block at all, before the next test exercises a
/// case where the merge actually needs a fresh phi.
#[test]
fn while_loop_with_a_plain_break_exits_early() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(int $n): int {\n    int $i = 0;\n    while ($i < $n) {\n      if ($i == 3) {\n        break;\n      }\n      $i = $i + 1;\n    }\n    return $i;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `break` after a reassignment the loop's own back edge never sees
/// (`$r = $i;` runs every iteration, but the `break` fires before the
/// bottom-of-body value the header phi's back edge would otherwise
/// carry) — the after-block's own environment now needs a real
/// `Lowering::merge_envs`-built phi for `$r`, combining the condition's
/// ordinary false edge (the loop-steady-state phi value) with the
/// break's own edge (that iteration's fresher value), not just a plain
/// clone of `header_env` the way a break-free loop always produced.
#[test]
fn break_merges_a_differing_value_into_the_after_block() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(int $n): int {\n    int $i = 0;\n    int $r = 0;\n    while ($i < $n) {\n      $r = $i;\n      if ($i == 3) {\n        break;\n      }\n      $i = $i + 1;\n    }\n    return $r;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `continue` inside a nested `if` adds a *third* incoming edge to
/// `$sum`'s header phi, alongside the pre-loop edge and the body's own
/// fall-through back edge — `$sum` is skipped (via `continue`) on the
/// iteration where `$i == 3`, so that edge's value genuinely differs
/// from the fall-through edge's, confirming
/// `Lowering::lower_while`'s combined `back_edges` (fall-through plus
/// every recorded `continue`) all reach the same phi, not just the
/// fall-through edge alone.
#[test]
fn continue_adds_another_incoming_edge_to_the_header_phi() {
    let (f, map, file) = lower_first_method(
        "<?nvs\nclass T {\n  function m(int $n): int {\n    int $i = 0;\n    int $sum = 0;\n    while ($i < $n) {\n      $i = $i + 1;\n      if ($i == 3) {\n        continue;\n      }\n      $sum = $sum + $i;\n    }\n    return $sum;\n  }\n}\n",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `break 2;` targets the *outer* loop's after-block, not the inner
/// one's — the whole point of a level, and the half a jump could get
/// wrong while still lowering to something well-formed.
///
/// Read off the rendering rather than snapshotted: what is pinned is
/// which block the jump names, and a snapshot would go red for any
/// unrelated renumbering while saying nothing about that. `break;` in the
/// same position is lowered beside it, so the assertion is that the two
/// name **different** blocks rather than that either names a particular
/// one.
#[test]
fn a_multi_level_break_leaves_the_loop_its_level_names() {
    let nested = |keyword: &str| {
        let (f, map, file) = lower_first_method(&format!(
            "<?nvs\nclass T {{\n  function m(int $n): void {{\n    while ($n > 0) {{\n      \
                 while ($n > 0) {{\n        {keyword};\n      }}\n      echo \"inner-done\";\n    \
                 }}\n  }}\n}}\n",
        ));
        print_function(&f, map.file(file))
    };
    let one = nested("break");
    let two = nested("break 2");
    assert_ne!(
        one, two,
        "`break 2` lowered to the same jump a `break` does"
    );
}

/// `continue 2` written inside a `switch` inside one loop is PHP's own
/// idiomatic spelling for "continue the enclosing loop", and it lowers to
/// exactly the back edge a bare `continue` there already takes — the
/// `switch` counts as a level and the walk outward finds the loop.
///
/// `nvs_ir::lower::Lowering::lower_continue` is where both steps live and
/// why; `rule:statements/continue-always-continues-a-loop` is the
/// decision's home.
#[test]
fn a_continue_level_walks_out_of_a_switch_to_the_loop() {
    let inside_switch = |keyword: &str| {
        let (f, map, file) = lower_first_method(&format!(
            "<?nvs\nclass T {{\n  function m(int $n): void {{\n    while ($n > 0) {{\n      \
                 $n = $n - 1;\n      switch ($n) {{\n        case 1:\n          {keyword};\n      \
                   default:\n          echo \"tick\";\n      }}\n    }}\n  }}\n}}\n",
        ));
        // Without the `; stmt @l:c-l:c` markers, which differ only
        // because `continue 2` is two characters longer than `continue`.
        print_function(&f, map.file(file))
            .lines()
            .filter(|l| !l.trim_start().starts_with("; stmt"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        inside_switch("continue"),
        inside_switch("continue 2"),
        "`continue 2` inside a `switch` did not reach the loop a bare `continue` does"
    );
}

/// Runs the whole front end over `src` and lowers the file — every class
/// method, the script frame, and the class table.
fn lower_whole_file(src: &str) -> crate::ir::Program {
    lower_whole_file_with_src(src).0
}

/// [`lower_whole_file`], keeping the source map so a test can render one
/// of the lowered functions with [`print_function`].
fn lower_whole_file_with_src(src: &str) -> (crate::ir::Program, SourceMap, SourceId) {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
    let module = nvs_hir::resolve_file(&stmts, map.file(file), &mut diags);
    assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
    let mut checked_types = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    let files = [nvs_types::ProgramFile {
        src: map.file(file),
        stmts: &stmts,
    }];
    let enums =
        nvs_types::check_program(&files, &module, &mut checked_types, &mut exprs, &mut diags);
    assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
    let layouts = nvs_types::build_class_layouts(&files, &module.graph);
    let program = lower_file(
        "<script>",
        &stmts,
        map.file(file),
        &exprs,
        &checked_types,
        &enums,
        &layouts,
    );
    (program, map, file)
}

/// `T::m` of the whole program `src`, after the ownership pass, printed: the
/// shape the compiler ships, which `lower_first_method` does not run the pass
/// to show.
fn print_shipped_method(src: &str) -> String {
    let (program, map, file) = lower_whole_file_with_src(src);
    let f = program
        .functions
        .iter()
        .find(|f| f.name == "T::m")
        .expect("the fixture has a `T::m`");
    print_function(f, map.file(file))
}

/// The class table is carried straight through from `nvs-types`, sorted
/// by label so an unchanged file lowers identically every time.
#[test]
fn a_lowered_file_carries_its_class_table_in_label_order() {
    let program = lower_whole_file(concat!(
        "<?nvs\n",
        "interface Greets { public function greet(): string; }\n",
        "class Animal {\n",
        "  public int $legs;\n",
        "  function constructor(int $legs) { $this->legs = $legs; }\n",
        "}\n",
        "class Dog extends Animal implements Greets {\n",
        "  public string $name;\n",
        "  function constructor(string $name) {\n",
        "    $this->name = $name;\n",
        "    parent::constructor(4);\n",
        "  }\n",
        "  public function greet(): string { return $this->name; }\n",
        "}\n",
    ));

    // The seeded exception tree is in every program's table, so the
    // declared classes are checked by name rather than by position.
    let declared: Vec<&str> = program
        .classes
        .iter()
        .map(|class| class.label.as_str())
        .filter(|label| ["Animal", "Dog", "Greets"].contains(label))
        .collect();
    assert_eq!(declared, ["Animal", "Dog", "Greets"]);

    let by_label = |label: &str| {
        program
            .classes
            .iter()
            .find(|class| class.label == label)
            .unwrap_or_else(|| panic!("{label} should be in the class table"))
    };
    let dog = by_label("Dog");
    assert_eq!(dog.fields, ["legs", "name"]);
    assert_eq!(dog.conforms, ["Animal", "Greets"]);
    assert!(by_label("Greets").fields.is_empty());
}

/// Source with a `get`- and a `set`-hooked property, plus one ordinary
/// one — the shape every hook test below reads.
const HOOKED: &str = concat!(
    "<?nvs\n",
    "class Counter {\n",
    "  public int $hits;\n",
    "  public int $doubled {\n",
    "    get => $this->hits * 2;\n",
    "    set(int $v) { $this->hits = $v; }\n",
    "  }\n",
    "  function constructor(int $hits) { $this->hits = $hits; }\n",
    "  public function read(): int { return $this->doubled; }\n",
    "  public function write(int $n): void { $this->doubled = $n; }\n",
    "}\n",
);

/// `rule:classes/property-hooks`'s hooks are ordinary compiled functions, each under the
/// label `nvs_types::signatures::hook_label` spells — the same one the
/// access site's `InstKind::Call` names, which is why nothing here needs a
/// dispatch table entry.
#[test]
fn each_property_hook_is_lowered_as_its_own_function() {
    let program = lower_whole_file(HOOKED);
    let names: Vec<&str> = program
        .functions
        .iter()
        .map(|f| f.name.as_str())
        .filter(|n| n.contains("$doubled"))
        .collect();
    assert_eq!(names, ["Counter::$doubled::get", "Counter::$doubled::set"]);

    let by_name = |name: &str| {
        program
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{name} should have been lowered"))
    };
    // `get` takes only the receiver and returns the property's type;
    // `set` takes the incoming value in slot 1 and returns nothing.
    let get = by_name("Counter::$doubled::get");
    assert_eq!(get.params, [Ty::Object]);
    assert_eq!(get.ret, Ty::Int);
    let set = by_name("Counter::$doubled::set");
    assert_eq!(set.params, [Ty::Object, Ty::Int]);
    assert_eq!(set.ret, Ty::Void);
}

/// The point of a hooked property: `$this->doubled` is a **call**, not a
/// field read — a `FieldGet` here would read a backing slot nothing ever
/// writes, and `examples/hooks.nvs` would print `0`.
#[test]
fn reading_a_get_hooked_property_calls_the_hook_instead_of_reading_the_slot() {
    let (program, map, file) = lower_whole_file_with_src(HOOKED);
    let read = program
        .functions
        .iter()
        .find(|f| f.name == "Counter::read")
        .expect("`read` should have been lowered");
    let text = print_function(read, map.file(file));
    assert!(text.contains("Counter::$doubled::get"), "{text}");
    assert!(!text.contains("field.get"), "{text}");
}

/// The write side, same shape: the assigned value is the accessor's one
/// ordinary argument.
#[test]
fn writing_a_set_hooked_property_calls_the_hook_instead_of_writing_the_slot() {
    let (program, map, file) = lower_whole_file_with_src(HOOKED);
    let write = program
        .functions
        .iter()
        .find(|f| f.name == "Counter::write")
        .expect("`write` should have been lowered");
    let text = print_function(write, map.file(file));
    assert!(text.contains("Counter::$doubled::set"), "{text}");
    assert!(!text.contains("field.set"), "{text}");
}

/// Inside `$doubled`'s own hooks the property is its backing slot, never
/// a re-entrant call — that is what lets a hook transform a stored value
/// and still terminate. `nvs_types::Ctx::current_hook` is the rule; this
/// is the lowering that proves it, on the one hook that touches its own
/// property.
#[test]
fn a_hook_body_reaching_its_own_property_touches_the_slot_directly() {
    let (program, map, file) = lower_whole_file_with_src(concat!(
        "<?nvs\n",
        "class Box {\n",
        "  public int $n {\n",
        "    get => $this->n + 1;\n",
        "    set(int $v) { $this->n = $v * 2; }\n",
        "  }\n",
        "  function constructor() { $this->n = 1; }\n",
        "}\n",
    ));
    for (name, expected) in [("Box::$n::get", "field.get"), ("Box::$n::set", "field.set")] {
        let f = program
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{name} should have been lowered"));
        let text = print_function(f, map.file(file));
        assert!(text.contains(expected), "{name}: {text}");
        // Read for a `call` naming it rather than for the name anywhere:
        // the signature line names the function, and so does the frame
        // label in an `rule:errors/propagation` landing block's `propagate` — which the
        // getter's `+ 1` has, since `rule:types/arithmetic` gives integer
        // arithmetic an overflow edge.
        let recursed = text
            .lines()
            .any(|line| line.contains("call") && line.contains(name));
        assert!(!recursed, "{name} recursed into itself: {text}");
    }
}

/// `foreach` over an `array<T>` with both bindings — the cursor's header
/// phi, the retained loop-owned array reference under its reserved `Env`
/// name, the step at the *top* of the body, and the per-iteration release
/// of the key on the back edge. See `Lowering::lower_foreach`.
#[test]
fn a_foreach_walks_an_array_through_a_cursor_it_owns_a_reference_to() {
    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
  function m(array<int> $a): void {
    foreach ($a as string $k => int $v) {
      echo $k;
    }
  }
}
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Lowers `T::m` once with written binding types and once with `var`, and
/// asserts the two print identically. The printed IR carries source columns,
/// so each `var` fixture pads the keyword to the written type's width.
fn assert_var_binding_lowers_as_written(written: &str, var: &str) {
    let (written_f, written_map, written_file) = lower_first_method(written);
    let (var_f, var_map, var_file) = lower_first_method(var);
    assert_eq!(
        print_function(&var_f, var_map.file(var_file)),
        print_function(&written_f, written_map.file(written_file)),
    );
}

/// `rule:types/var-inference`: a `var` key and value binding over an
/// `array<T>` lower to the IR the written `string` and `T` lower to, because
/// `binding_ty` reads the type the checker recorded under the keyword.
#[test]
fn foreach_var_binding_lowers_as_the_written_type_does() {
    assert_var_binding_lowers_as_written(
        "<?nvs
class T {
  function m(array<string> $a): void {
    foreach ($a as string $k => string $v) {
      echo $k . $v;
    }
  }
}
",
        "<?nvs
class T {
  function m(array<string> $a): void {
    foreach ($a as var    $k => var    $v) {
      echo $k . $v;
    }
  }
}
",
    );
}

/// The same equality over an `Iterator<T>` subject, whose value binding
/// takes the cursor's element type and has no key to bind.
#[test]
fn foreach_var_binding_over_a_cursor_lowers_as_the_written_type_does() {
    assert_var_binding_lowers_as_written(
        "<?nvs
class T {
  function m(Iterator<int> $c): void {
    foreach ($c as int $v) {
      echo $v;
    }
  }
}
",
        "<?nvs
class T {
  function m(Iterator<int> $c): void {
    foreach ($c as var $v) {
      echo $v;
    }
  }
}
",
    );
}

/// `rule:types/var-inference`: a `var` local over a one-type array literal
/// lowers to the IR the written `array<T>` lowers to — flat, nested and
/// through a spread — because the literal is then checked against the
/// inferred type exactly as against a written one.
#[test]
fn var_array_literal_lowers_as_the_written_type_does() {
    assert_var_binding_lowers_as_written(
        "<?nvs
class T {
  function m(array<int> $more): void {
    array<int> $x = [1, 2];
    array<array<int>> $g = [[1, 2], [3]];
    array<int> $all = [...$more, 3];
    echo $x[0] + $g[0][0] + $all[0];
  }
}
",
        "<?nvs
class T {
  function m(array<int> $more): void {
    var        $x = [1, 2];
    var               $g = [[1, 2], [3]];
    var        $all = [...$more, 3];
    echo $x[0] + $g[0][0] + $all[0];
  }
}
",
    );
}

/// A refcounted value binding is retained where a scalar one is not — the
/// binding is a durable slot and `InstKind::ArrayValueAt` hands it a
/// borrow, the same split `InstKind::ArrayGet` already has.
#[test]
fn a_refcounted_foreach_value_binding_is_retained_for_its_iteration() {
    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
  function m(array<string> $a): void {
    foreach ($a as string $v) {
      echo $v;
    }
  }
}
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `foreach` over an `Iterator<T>` — `rule:iteration/foreach-subjects`'s `Iterator` row. Every
/// member call is a `call.virtual`, never a static `call`: the interface
/// declares both without a body, so there is no compiled function to
/// name. The cursor is retained before each one, since a receiver is
/// parameter 0 and Novis transfers an argument's reference to the callee.
#[test]
fn a_foreach_over_a_cursor_drives_advance_then_current_virtually() {
    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
  function m(Iterator<int> $c): void {
    foreach ($c as int $v) {
      echo $v;
    }
  }
}
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `foreach` over an `Iterable<T>` — the same loop with one `iterate()`
/// ahead of it. The subject's own reference is *transferred* into that
/// call rather than retained for it, which is why the subject never
/// enters the `Env` and the loop's after-block releases the cursor
/// instead.
#[test]
fn a_foreach_over_an_iterable_calls_iterate_once_before_the_loop() {
    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
  function m(Iterable<int> $it): void {
    foreach ($it as int $v) {
      echo $v;
    }
  }
}
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `rule:iteration/generators`'s state-machine transform, end to end: the factory that
/// runs no user code, the entry switch, one resumption arm per `yield`,
/// and the spill/reload pair that lets a value cross a suspension that
/// SSA cannot carry it across. `$limit` gets a loop-header phi despite
/// never being reassigned — see `seed_generator_loop_carried`.
#[test]
fn a_generator_lowers_to_a_factory_a_state_class_and_a_resumption_switch() {
    let (p, map, file) = lower_program(
        "<?nvs
class G {
  static function upTo(int $limit): Iterator<int> {
    var $i = 1;
    while ($i <= $limit) {
      yield $i;
      $i = $i + 1;
    }
  }
}
",
    );
    assert_snapshot!(print_program(&p, map.file(file)));
}

/// An abandoned generator runs the `finally` it is suspended inside — the
/// resume-to-unwind entry point plus the arm each suspension point grows
/// for it, which `lower_generator` § *An abandoned generator runs its
/// `finally`* owns. The suspension inside the `try` branches on
/// `gen#unwind` and lowers a copy of the `finally` body on the arm that
/// takes it; the one after the region has nothing owed and grows no
/// branch at all.
#[test]
fn an_abandoned_generator_resumes_into_the_finally_it_is_suspended_inside() {
    let (p, map, file) = lower_program(
        "<?nvs
class G {
  static function two(): Iterator<int> {
    try {
      yield 1;
    } finally {
      echo \"closing\";
    }
    yield 2;
  }
}
",
    );
    assert_snapshot!(print_program(&p, map.file(file)));
}

/// `rule:types/anonymous-function`'s anonymous function, lowered: the site that writes it allocates the
/// captured-environment object and stores a *retained* snapshot of each
/// capture into it, and the body becomes that class's one `invoke`, which
/// reads every capture back out of parameter 0. See `lower_anon_fn`,
/// which owns the representation.
#[test]
fn an_anon_fn_lowers_to_a_captured_environment_object_and_an_invoke_method() {
    let (p, map, file) = lower_program(
        "<?nvs
string $tag = \"t\";
int $bump = 1;
var $f = fn(int $n): string => $tag;
echo $bump;
",
    );
    assert_snapshot!(print_program(&p, map.file(file)));
}

/// An anonymous function capturing an enclosing `inout $x` parameter, which is the one
/// capture whose `Env` entry is an address rather than a value: `rule:types/implicit-capture` captures by value, so the field takes a `ref.load` snapshot of the
/// cell at the literal, at the declared pointee type, and then the same
/// retain every refcounted capture already takes. Both halves are visible
/// here on purpose — the load alone would leave the environment object
/// sharing the caller's one reference, and the field's `str` type is what
/// says `invoke` reads a value rather than the caller's address, which is
/// what makes the callable safe to outlive the call that staged the cell.
#[test]
fn an_anon_fn_capturing_a_by_reference_parameter_snapshots_the_cell() {
    let (p, map, file) = lower_program(
        "<?nvs
class T {
  static function make(inout string $s): callable {
    return fn (): string => $s;
  }
}
",
    );
    assert_snapshot!(print_program(&p, map.file(file)));
}

/// A refcounted element and a refcounted local both survive a
/// suspension: the field takes its own reference on the way in and the
/// resume block takes one on the way back out, so the two never share.
#[test]
fn a_generator_parks_a_refcounted_local_and_element_in_its_state_object() {
    let (p, map, file) = lower_program(
        "<?nvs
class G {
  static function two(): Iterator<string> {
    var $tag = \"t\";
    yield $tag;
    yield $tag;
  }
}
",
    );
    assert_snapshot!(print_program(&p, map.file(file)));
}

/// `unset($a[$k]);` — `InstKind::ArrayUnset` written back through the same
/// holder a write uses, with the literal key released afterwards because
/// the removal only borrows it.
#[test]
fn unsetting_an_element_writes_the_separated_array_back() {
    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
  function m(array<int> $a): void {
    unset($a[\"k\"]);
  }
}
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `foreach (… as inout $v)` walks its own reference to the array it began
/// with and writes to the subject variable
/// (`rule:iteration/an-inout-loop-walks-the-array-it-began-with`). The snapshot
/// shows the retain of the subject on the way in and the release after the
/// loop, one fallible `array_inout_write` helper call at the write, and the
/// subject's own binding carrying a header phi over what the last iteration
/// wrote.
#[test]
fn a_foreach_by_reference_writes_through_to_its_array() {
    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
  function m(array<int> $a): void {
    foreach ($a as inout int $v) {
      $v = $v + 1;
    }
  }
}
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A Tier 0 `Core` member call: `core.call` naming the symbol
/// `nvs_stdlib::registry` registered, with **no retain** on the array
/// argument even though it is a refcounted aliasing read — a `Core` member
/// borrows what it is handed. Contrast the ordinary static call in
/// `a_self_static_call_with_a_scalar_return`'s snapshot, which retains.
#[test]
fn a_core_member_call_lowers_to_a_symbol_and_borrows_its_argument() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\n",
        "class T {\n",
        "  function m(array<int> $a): uint {\n",
        "    return Core\\Arr::count($a);\n",
        "  }\n",
        "}\n",
    ));
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A refcounted local declared *inside* a loop body is released on the
/// back edge, not carried out of the loop: the next iteration restarts
/// from the header environment, so nothing after this point could ever
/// reach it. Without the release it leaks one reference per iteration,
/// which only a valgrind leg over a fixture declaring such a local — the
/// `examples/report.nvs` shape — sees. See [`Lowering::end_iteration`].
#[test]
fn a_local_declared_in_a_loop_body_is_released_on_the_back_edge() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\n",
        "class T {\n",
        "  function m(array<string> $words): void {\n",
        "    foreach ($words as string $w) {\n",
        "      var $key = Core\\Str::lower($w);\n",
        "      echo $key;\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// The same obligation one level down: a local declared in one `if` branch
/// is bound on that edge only, so the merge drops it — and the edge that
/// bound it is the last place its reference is reachable. See
/// [`Lowering::release_merged_away`].
#[test]
fn a_local_declared_in_one_if_branch_is_released_where_the_branches_merge() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\n",
        "class T {\n",
        "  function m(bool $c): void {\n",
        "    if ($c) {\n",
        "      var $s = Core\\Str::lower(\"AA\");\n",
        "      echo $s;\n",
        "    } else {\n",
        "      echo \"no\";\n",
        "    }\n",
        "  }\n",
        "}\n",
    ));
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `rule:core-api/shape-rules` R2's options bag, flattened: `Core\Arr::range` takes two
/// positional arguments and one bag declaring one option, and both calls
/// below emit a `core.call` with **three** arguments — the written
/// `{ step: 3 }` in the first, the materialized default `1` in the second.
/// The bag itself never appears in the IR at all, which is the property
/// that keeps `nvs-codegen` and the `rule:errors/propagation` helper convention from
/// learning that options exist.
#[test]
fn an_options_bag_flattens_into_one_argument_per_option() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\n",
        "class T {\n",
        "  function m(): void {\n",
        "    echo Core\\Arr::count(Core\\Arr::range(1, 10, {step: 3}));\n",
        "    echo Core\\Arr::count(Core\\Arr::range(1, 10));\n",
        "  }\n",
        "}\n",
    ));
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `name:` argument lands at the ABI position of the parameter its name
/// reached, not at its own place in the list — `nvs_types` records that
/// mapping as `ResolvedCall::arg_slots` and
/// [`Lowering::lower_call_args`] is what reads it.
///
/// The call below writes its two parameters backwards and omits the third,
/// so all three facts are in the one snapshot: the `"z"` reaches argument
/// 0 and the `1` argument 1 despite being written the other way round, and
/// `$mark`'s own default is materialized into argument 2 by
/// [`Lowering::lower_default_arg`] rather than left absent. Evaluation
/// stays in *written* order above the call — the `1` is emitted first —
/// which is the half a positional list cannot tell apart.
#[test]
fn a_named_argument_lowers_to_its_declared_position() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\n",
        "class T {\n",
        "  function m(): void {\n",
        "    T::label(times: 1, who: \"z\");\n",
        "  }\n",
        "  static function label(string $who, int $times, string $mark = \"!\"): void { }\n",
        "}\n",
    ));
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A `...` argument fills the variadic tail's positions, which are not one
/// ABI argument each: the tail is one array, so the spread is an
/// `array_spread` of the subject into the array the written-out prefix
/// built — how many entries arrive being the subject's own run-time length.
///
/// Both call sites are here because they differ by one instruction and
/// nothing else: the second's prefix is empty, so its `array_new` starts
/// with no entries at all.
#[test]
fn a_spread_argument_lowers_to_the_positions_it_fills() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\n",
        "class T {\n",
        "  function m(array<int> $extra): void {\n",
        "    T::total(1, 2, ...$extra);\n",
        "    T::total(1, ...$extra);\n",
        "  }\n",
        "  static function total(int $base, int ...$rest): void { }\n",
        "}\n",
    ));
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// The same `...` through a `callable`, where there is no signature and so
/// no tail to fill: the *whole* argument list becomes the array instead,
/// and the call goes through `Helper::CallCallableArray` rather than the
/// `call_callable` beside it, whose argument count `nvs-codegen` writes as a
/// literal. The second call is the unspread one, which still emits the
/// counted helper — one snapshot holding both rows.
#[test]
fn a_spread_argument_through_a_callable_becomes_one_array() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\n",
        "class T {\n",
        "  function m(callable $f, array<int> $extra): void {\n",
        "    echo $f(1, ...$extra) as string;\n",
        "    echo $f(1) as string;\n",
        "  }\n",
        "}\n",
    ));
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// The whole path in one fixture: a `rule:types/anonymous-function` anonymous function bound to a
/// local, then *called* through the variable holding it — which is what
/// `examples/callable.nvs`'s `direct` line runs.
///
/// The call is the runtime's (`Helper::CallCallable` into
/// `nvs_runtime::call_callable`) rather than a lowered `Call` to a label,
/// because a `callable` names no compiled function; the environment object
/// the literal built is the receiver. The neighbouring fixture asks the
/// same question of a `callable` *parameter*, where there is no literal in
/// the frame at all — the pair is what separates "the anonymous function lowers" from
/// "the variable holding one is callable".
#[test]
fn a_callable_is_called_through_the_variable_holding_it() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\n",
        "class T {\n",
        "  function m(): void {\n",
        "    callable $f = fn (int $a, int $b): int => $a + $b;\n",
        "    echo $f(6, 7) as string;\n",
        "  }\n",
        "}\n",
    ));
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `$x is Name` — one class test naming the *resolved* class label the
/// checker recorded, with no retain of the receiver.
///
/// The subject is the **base** and the target a subclass, so neither fold
/// fires and the instruction survives to be printed: a test whose answer the
/// declaration settles carries a constant instead
/// (`rule:types/type-test`), which would assert nothing about the label.
#[test]
fn a_class_test_names_the_class_the_checker_resolved() {
    let (f, map, file) = lower_first_method(
        "<?nvs
class Animal {
}
class Dog extends Animal {
}
class T {
  function m(Animal $a): bool {
    return $a is Dog;
  }
}
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// A value on the right of `is` that is not a `class<T>` names no class, so
/// the checker reports `E0496` and this crate never sees the program: the
/// fixture does not get past the diagnostics gate these tests run first.
#[test]
#[should_panic(expected = "E0496")]
fn a_value_naming_no_class_records_nothing_to_lower() {
    lower_first_method(
        "<?nvs
class Animal {
}
class T {
  function m(Animal $a, string $n): bool {
    return $a is $n;
  }
}
",
    );
}

/// `$x is $cls` — the value arm, whose right-hand side is a `class<T>` rather
/// than a written name, so the class the walk compares against arrives as a
/// [`crate::ir::TestedClass::Descriptor`] in a register instead of as a label.
/// One instruction either way, which is what `rule:types/type-test`'s value
/// arm promises.
#[test]
fn a_class_reference_test_lowers_to_a_descriptor_valued_class_test() {
    let (f, _, _) = lower_first_method(
        "<?nvs
class Animal {
}
class T {
  function m(Animal $a, class<Animal> $cls): bool {
    return $a is $cls;
  }
}
",
    );
    let tested: Vec<&TestedClass> = f
        .blocks
        .iter()
        .flat_map(|b| &b.insts)
        .filter_map(|i| match &i.kind {
            InstKind::ClassTest { class, .. } => Some(class),
            _ => None,
        })
        .collect();
    assert_eq!(tested.len(), 1, "{:?}", f.blocks);
    assert!(
        matches!(tested[0], TestedClass::Descriptor(_)),
        "{:?}",
        tested[0]
    );
}

/// A `mixed` subject keeps its [`crate::ir::Ty::Tagged`] representation all
/// the way into the instruction: `nvs-codegen` calls
/// `nvs_value_is_class` for it, which reads the tag rather than
/// dereferencing an unchecked payload. A subject whose *declared* type can
/// hold no object settles at the checker instead, so no third representation
/// reaches here.
#[test]
fn a_class_test_over_a_mixed_subject_keeps_its_tag() {
    let (f, _, _) = lower_first_method(
        "<?nvs
class Animal {
}
class T {
  function m(mixed $a): bool {
    return $a is Animal;
  }
}
",
    );
    // Slot 0 is the receiver every lowered method carries; the declared
    // `mixed` parameter behind it is what the test walks, and lowering it
    // at all is the assertion — nothing on this path demands a
    // `Ty::Object` subject.
    assert_eq!(f.params.get(1), Some(&Ty::Tagged), "{:?}", f.params);
    assert!(
        f.blocks
            .iter()
            .flat_map(|b| &b.insts)
            .any(|i| matches!(i.kind, InstKind::ClassTest { .. })),
        "the fixture lowers one class test"
    );
}

/// A read through a union of three classes that all declare `$name` is two
/// class tests and three fixed-offset reads, one per class
/// (`rule:types/a-member-every-class-of-a-union-declares`). The last class is
/// the fall-through, and nothing is looked up by name.
#[test]
fn a_property_every_class_of_a_union_declares_lowers_to_a_class_test_per_class_but_the_last() {
    let (f, _, _) = lower_first_method(
        "<?nvs
class Box { public string $name = \"box\"; }
class Circle { public string $name = \"circle\"; }
class Label { public string $name = \"label\"; }
class T {
  function m(Box|Circle|Label $s): string {
    return $s->name;
  }
}
",
    );
    let insts = || f.blocks.iter().flat_map(|b| &b.insts);
    let tested: Vec<&TestedClass> = insts()
        .filter_map(|i| match &i.kind {
            InstKind::ClassTest { class, .. } => Some(class),
            _ => None,
        })
        .collect();
    assert!(
        matches!(&tested[..], [TestedClass::Named(a), TestedClass::Named(b)] if a == "Box" && b == "Circle"),
        "{tested:?}"
    );
    let read: Vec<&str> = insts()
        .filter_map(|i| match &i.kind {
            InstKind::FieldGet { class, .. } => Some(class.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(read, ["Box", "Circle", "Label"], "{:?}", f.blocks);
    assert!(
        !insts().any(|i| matches!(i.kind, InstKind::SlotGet { .. })),
        "a union read looks nothing up by name: {:?}",
        f.blocks
    );
}

/// A zero divisor throws whatever the operand types are (`rule:types/arithmetic`), and
/// `nvs-codegen` raises that inline rather than through a helper, so the
/// frame's cleanup path has to exist at the operator itself: integer `%` and
/// **float** `/` both carry an
/// [`Inst::on_error`](crate::ir::Inst::on_error) edge.
///
/// The half worth holding is the float `*` beside them, because the edge is
/// not free — one on every `BinOp` would be a landing block per arithmetic
/// expression — and it is `/` alone that earns one on the float row. `%`
/// cannot stand in for that half: `E0717` refuses a `float` operand where
/// it is written.
#[test]
fn a_zero_divisor_carries_an_error_edge_on_the_float_row_too() {
    let (f, _, _) = lower_script_src("<?nvs\nint $a = 7;\nint $b = 2;\nint $q = $a % $b;\n");
    let modulo = f
        .blocks
        .iter()
        .flat_map(|b| &b.insts)
        .find(|i| matches!(i.kind, InstKind::BinOp { op: BinOp::Mod, .. }))
        .expect("the fixture lowers one `%`");
    assert!(modulo.on_error.is_some(), "{modulo:?}");

    let (f, _, _) =
        lower_script_src("<?nvs\nfloat $a = 7.0;\nfloat $b = 2.0;\nfloat $q = $a / $b;\n");
    let division = f
        .blocks
        .iter()
        .flat_map(|b| &b.insts)
        .find(|i| matches!(i.kind, InstKind::BinOp { op: BinOp::Div, .. }))
        .expect("the fixture lowers one `/`");
    assert!(division.on_error.is_some(), "{division:?}");

    let (f, _, _) =
        lower_script_src("<?nvs\nfloat $a = 7.0;\nfloat $b = 2.0;\nfloat $p = $a * $b;\n");
    let product = f
        .blocks
        .iter()
        .flat_map(|b| &b.insts)
        .find(|i| matches!(i.kind, InstKind::BinOp { op: BinOp::Mul, .. }))
        .expect("the fixture lowers one `*`");
    assert!(product.on_error.is_none(), "{product:?}");
}

/// A direct `Core\Math::quotient` call lowers to one [`BinOp::IntDiv`] with an
/// error edge and no `CoreCall`, so `nvs-codegen` emits the division and its
/// two throws inline.
#[test]
fn a_direct_int_div_call_is_an_inline_division_with_an_error_edge() {
    let (f, _, _) = lower_script_src(
        "<?nvs\nint $a = 7;\nint $b = 2;\nint $q = Core\\Math::quotient($a, $b);\n",
    );
    let insts = || f.blocks.iter().flat_map(|b| &b.insts);
    let division = insts()
        .find(|i| {
            matches!(
                i.kind,
                InstKind::BinOp {
                    op: BinOp::IntDiv,
                    ..
                }
            )
        })
        .expect("the fixture lowers one inline `quotient`");
    assert!(division.on_error.is_some(), "{division:?}");
    assert_eq!(division.ty, Some(Ty::Int), "{division:?}");
    assert!(
        !insts().any(|i| matches!(&i.kind, InstKind::CoreCall { symbol, .. }
            if *symbol == nvs_types::CORE_MATH_INT_DIV)),
        "a direct call reaches no helper: {:?}",
        f.blocks
    );
}

/// Every instruction that *returns a status* carries a landing block,
/// whether or not a program can recover from what that status says —
/// [`Inst::on_error`](crate::ir::Inst::on_error) is that rule's one home and
/// `nvs_codegen::emit`'s own `emit_status_check` refuses an arrival without
/// one.
///
/// Asserted as a sweep over the whole function rather than off a named line,
/// because the shapes an exemption is easiest to grant are exactly the ones
/// no `catch` can act on — a conversion helper, the truthy table, a `??` read.
/// A case naming one of them would go green while the rest slipped back, and
/// an exemption costs a leak of every local the frame held.
/// `BinOp`/`UnOp` are left out here on purpose: only `rule:types/arithmetic`'s checked
/// integer rows return a status at all, and the neighbouring
/// `an_integer_modulo_carries_an_error_edge_and_a_float_division_does_not`
/// pins both halves of that split.
#[test]
fn every_status_returning_instruction_carries_a_landing_block() {
    let (f, map, file) = lower_script_src(concat!(
        "<?nvs\n",
        "string $s = \"x\";\n",
        "int $n = 7;\n",
        "array<string> $a = [\"k\" => \"v\"];\n",
        "mixed $m = $a;\n",
        "if ($s) { echo $s, $n, \"\\n\"; }\n",
        "string $got = $a[\"k\"];\n",
        "mixed $opt = $m[\"nope\"] ?? \"d\";\n",
        "echo $got, $opt as string, \"\\n\";\n",
    ));
    let mut checked = 0;
    for inst in f.blocks.iter().flat_map(|b| &b.insts) {
        let returns_status = matches!(
            inst.kind,
            InstKind::HelperCall { .. }
                | InstKind::CoreCall { .. }
                | InstKind::Call { .. }
                | InstKind::CallVirtual { .. }
                | InstKind::New { .. }
                | InstKind::NewDynamic { .. }
                | InstKind::SlotGet { .. }
                | InstKind::SlotSet { .. }
                | InstKind::ArrayGet { .. }
                | InstKind::ArrayAppend { .. }
                | InstKind::ArraySpread { .. }
        );
        if returns_status {
            checked += 1;
            assert!(
                inst.on_error.is_some(),
                "a status-returning instruction with no landing block: {inst:?}"
            );
        }
    }
    // The fixture is only evidence if it actually reaches those shapes, so
    // the count is asserted rather than assumed.
    assert!(checked >= 8, "{}", print_function(&f, map.file(file)));
}

/// [`Terminator::Catch`]'s second exit, and the other half of the rule
/// above: a landing block inside a `try` releases no local on its way to the
/// handler, so a status the handler never sees has to leave by a block that
/// performs the same sweep a `Propagate` at an unprotected site does.
/// Without it a `FATAL` or an `EXITED` raised inside any `try` leaks the
/// whole frame.
#[test]
fn an_uncatchable_status_leaves_a_try_through_a_block_that_releases_the_locals() {
    let (f, _, _) = lower_script_src(concat!(
        "<?nvs\n",
        "string $s = \"x\";\n",
        "try {\n",
        "  echo $s, \"\\n\";\n",
        "} catch (Throwable $e) {\n",
        "  echo \"caught\\n\";\n",
        "}\n",
    ));
    let onward = f
        .blocks
        .iter()
        .find_map(|b| match b.term {
            Terminator::Catch { onward, .. } => Some(onward),
            _ => None,
        })
        .expect("the fixture lowers one protected call");
    let block = f
        .blocks
        .iter()
        .find(|b| b.id.index() == onward.index())
        .expect("the onward block exists");
    assert!(
        block
            .insts
            .iter()
            .any(|i| matches!(i.kind, InstKind::Release { .. })),
        "the uncatchable exit released nothing: {block:?}"
    );
    assert!(
        matches!(block.term, Terminator::Propagate { .. }),
        "{:?}",
        block.term
    );
}

/// A file declaring no class still lowers, and still carries every roster no
/// source declares — `nvs_hir::errors`' exception tree, with the
/// synthesized constructor of every class in it that declares state of its
/// own, `nvs_hir::interfaces`' global interfaces, and
/// `rule:types/callable-values`'s callable marker — the `hello.nvs`
/// shape. Nothing in the file references any of them and they are emitted
/// anyway: a descriptor has to exist before `$x is Stringable` or
/// `$x is callable` has anything to test against, and a class implementing
/// one only keeps the edge if the label it names is in this list
/// (`nvs_types::layout::build_class_layouts`).
#[test]
fn a_file_with_no_class_still_carries_every_compiler_declared_class() {
    let program = lower_whole_file("<?nvs\necho \"hi\";\n");
    let labels: Vec<&str> = program
        .classes
        .iter()
        .map(|class| class.label.as_str())
        .collect();
    assert_eq!(
        labels,
        [
            // The callable marker, ahead of every declared label because `$`
            // cannot start one, and here in a file with no callable for the
            // reason `crate::lower::CALLABLE_MARKER` gives.
            "$callable",
            "ArithmeticError",
            "Comparable",
            // `rule:tooling/a-prompt-is-a-core-member`'s refusal to block, `rule:core-classes/db-error`'s driver failure
            // and `rule:core-classes/db-transactions`' deliberate rollback, and
            // `rule:testing/failure-ledger`'s assertion
            // failure — the exception tree's namespaced entries, classes in
            // it for the reason `nvs_hir::errors::TREE` gives.
            "Core\\Cli\\NotInteractive",
            "Core\\Db\\DbError",
            "Core\\Db\\RolledBack",
            "Core\\DeprecatedError",
            "Core\\Ldap\\LdapError",
            // The table's other root, which is not an exception: a descriptor
            // for it has to exist for the same reason as the rest — a `catch`
            // tests against descriptors, and this is the one that answers no.
            "Core\\Script\\Finished",
            "Core\\Test\\Failure",
            "ExtensionError",
            "IOError",
            "Iterable",
            "Iterator",
            "KeyedIterable",
            "KeyedIterator",
            "LateInit",
            "LogicError",
            "ParseError",
            "Parses",
            "PropertyGuard",
            "PropertyObserver",
            "RecursionError",
            "Revocable",
            "RevokedObjectError",
            "RuntimeError",
            "Stringable",
            "Throwable",
            "TimeoutError",
        ]
    );
    let functions: Vec<&str> = program.functions.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        functions,
        [
            "<script>",
            "Throwable::constructor",
            "ParseError::constructor",
            "Core\\Db\\DbError::constructor",
            "Core\\Db\\RolledBack::constructor",
            "Core\\Ldap\\LdapError::constructor"
        ]
    );
}

/// The root's four slots are the ones `nvs_runtime::throwable` reaches by
/// index, and every other exception class inherits them at the same
/// indices — the property that lets the runtime append a backtrace frame
/// to a value it knows nothing else about.
#[test]
fn every_exception_class_carries_the_root_s_four_slots_at_the_same_indices() {
    let program = lower_whole_file("<?nvs\nclass MyError extends IOError {}\n");
    for label in ["Throwable", "IOError", "MyError"] {
        let class = program
            .classes
            .iter()
            .find(|c| c.label == label)
            .unwrap_or_else(|| panic!("{label} should be in the class table"));
        assert_eq!(
            class.fields,
            ["message", "previous", "backtrace", "location"],
            "{label}"
        );
    }
}

/// `rule:enums/no-class-machinery`: a case is an integer constant inlined at its use site —
/// `Rank::Gold` is a `ConstInt 2` and nothing else, with no storage, no
/// descriptor and no allocation. `rule:types/conversion`'s first row then makes
/// `as int` a free `Reinterpret`.
#[test]
fn an_enum_case_lowers_to_a_constant_and_as_int_is_free() {
    let (f, map, file) = lower_script_src(
        "<?nvs
enum Rank { Bronze, Silver, Gold }
int $g = Rank::Gold as int;
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `rule:enums/one-backing-type`'s `: uint` backing reaches the IR: the case constant is a
/// `ConstUint` at `enum:uint`, and `as uint` is the free row again.
#[test]
fn a_uint_backed_enum_case_lowers_to_a_uint_constant() {
    let (f, map, file) = lower_script_src(
        "<?nvs
enum P: uint { Read = 0b001, Write = 0b010 }
uint $w = P::Write as uint;
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// An enum-typed *binding* is an integer binding, not an object one —
/// `rule:enums/representation`. The parameter's representation is what proves it: a
/// `Ty::Object` here would mean a refcounted receiver slot, a retain and a
/// release, none of which an enum has.
#[test]
fn an_enum_typed_parameter_is_an_integer_parameter() {
    let (f, _map, _file) = lower_first_method(
        "<?nvs
enum Rank { Bronze, Gold }
class T { public function f(Rank $r): int { return $r as int; } }
",
    );
    assert_eq!(f.params, [Ty::Object, Ty::Enum(EnumRepr::Int)]);
    assert_eq!(f.ret, Ty::Int);
}

/// `rule:enums/truthiness`: an enum case is *always* truthy, never judged by its
/// backing value. `Rank::Bronze` is backed by `0`, so a representation
/// that erased it to `Ty::Int` would emit `Helper::IntTruthy` here and
/// come back `false`.
#[test]
fn an_enum_condition_folds_to_true_rather_than_testing_its_backing_value() {
    let (f, map, file) = lower_script_src(
        "<?nvs
enum Rank { Bronze, Gold }
if (Rank::Bronze) { echo \"y\"; }
",
    );
    let text = print_function(&f, map.file(file));
    assert!(text.contains("const.bool true"), "{text}");
    assert!(!text.contains("helper.int_truthy"), "{text}");
}

/// `rule:types/conversion`'s total rows, reached through `as` rather than through
/// `.`: a scalar to `string` reuses the same `Helper` conversion, and a
/// value to `bool` reuses `rule:expressions/truthy-positions`'s truthy table.
#[test]
fn as_string_and_as_bool_reuse_the_conversions_that_already_exist() {
    let (f, map, file) = lower_script_src(
        "<?nvs
string $s = 7 as string;
bool $b = 0 as bool;
",
    );
    let text = print_function(&f, map.file(file));
    assert!(text.contains("helper.int_to_string"), "{text}");
    assert!(text.contains("helper.int_truthy"), "{text}");
}

/// `rule:types/conversion`'s checked rows go through a fallible helper — the same
/// call shape a method call has, error edge included, because either one
/// can throw. The error edge is what this asserts: a checked conversion
/// that skipped it would drop the throw on the floor.
#[test]
fn a_checked_conversion_row_carries_the_call_error_edge() {
    let (f, map, file) = lower_script_src(
        "<?nvs
uint $u = 1;
int $n = $u as int;
",
    );
    let text = print_function(&f, map.file(file));
    // `! bbN` is how `crate::print` renders `Inst::on_error`.
    assert!(
        text.lines()
            .any(|l| l.contains("helper.uint_to_int") && l.contains(" ! bb")),
        "{text}"
    );
}

/// The one line of a printed function naming `needle`.
///
/// This and the helpers beside it are the read half of every fault-edge
/// guard below, which asks what a *named* block releases rather than
/// counting releases in a body.
fn line<'t>(text: &'t str, needle: &str) -> &'t str {
    text.lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no line names `{needle}`:\n{text}"))
}
/// The `vN` that line defines.
fn produced(text: &str, needle: &str) -> String {
    line(text, needle)
        .split_whitespace()
        .next()
        .expect("an instruction line starts with the value it defines")
        .to_owned()
}
/// The `bbN` its `! bb` fault edge names.
fn fault_edge(text: &str, needle: &str) -> String {
    line(text, needle)
        .split("! ")
        .nth(1)
        .unwrap_or_else(|| panic!("`{needle}` takes no fault edge:\n{text}"))
        .split_whitespace()
        .next()
        .expect("a fault edge names a block")
        .to_owned()
}
/// How many of `block`'s own instructions release `value`. Matched on the
/// whole line, since `release v1` is a prefix of `release v10`.
fn release_count(text: &str, block: &str, value: &str) -> usize {
    text.lines()
        .skip_while(|l| !l.trim_start().starts_with(&format!("{block}:")))
        .skip(1)
        .take_while(|l| !l.trim_end().ends_with(':'))
        .filter(|l| l.trim() == format!("release {value}"))
        .count()
}
/// Whether `block`'s own instructions release `value` at all.
fn releases(text: &str, block: &str, value: &str) -> bool {
    release_count(text, block, value) > 0
}

/// Every value in flight inside an expression is released on the throw
/// path, including the ones whose *normal* path releases them by hand.
///
/// `Lowering::owned_temporaries` is the whole mechanism and
/// `Lowering::landing_block` the whole error edge, so the claim is one
/// question asked of the two producers that used to answer it only on the
/// normal edge: a `match` subject, in flight for a label chain whose every
/// comparison can throw, and an `unset` target's rendered key, in flight for
/// a descent that throws on an absent row. Asserted as "the block the fault
/// edge names releases the producer" rather than by counting releases — a
/// function that released it twice on the normal path would satisfy a count
/// while leaking here.
#[test]
fn an_inline_producer_releases_its_value_on_the_throw_path() {
    // The subject is a fresh `mixed`, so every label goes through the
    // fallible `Helper::Identical` — the throw that abandons the `match`
    // with its subject still in flight.
    let (f, map, file) = lower_script_src(
        "<?nvs
class T { public static function pick(): mixed { return \"a\"; } }
string $r = match (T::pick()) { \"b\" => \"hit\", default => \"miss\" };
echo $r;
",
    );
    let text = print_function(&f, map.file(file));
    let subject = produced(&text, "T::pick");
    // The label is the other producer at that edge: it is built for the
    // comparison and released after it, so the comparison's own throw is the
    // window both of them sit in.
    let label = produced(&text, "const.str \"b\"");
    let fault = fault_edge(&text, "helper.identical");
    assert!(releases(&text, &fault, &subject), "{text}");
    assert!(releases(&text, &fault, &label), "{text}");

    // And the `unset` key: `$a` is nested, so the level read below the
    // rendered key is an `AbsentKey::Throws` that can leave with it in hand.
    let (f, map, file) = lower_script_src(
        "<?nvs
array<array<string>> $a = [];
unset($a[\"outer\"][1]);
",
    );
    let text = print_function(&f, map.file(file));
    let key = produced(&text, "helper.int_to_string");
    let fault = fault_edge(&text, "array.get");
    assert!(releases(&text, &fault, &key), "{text}");
}

/// A transferred argument is this frame's until the call it was staged for
/// exists, and the callee's from that instruction onward — so the two edges
/// of that one instruction disagree, and this asks about both.
///
/// `Lowering::owned_temporaries` holds it under `TemporaryKind::Transferred`
/// for the window in between, which is what a *later* argument's own throw
/// abandons it in — `T::take($s, T::boom())`, where `$s` was retained for the
/// transfer and `boom` throws before `take` is ever reached. The other half is
/// why `Lowering::forget_transferred_since` runs *before* the call rather than
/// after it: a callee releases its parameters on its own throwing edge too, so
/// the call's own fault edge releasing them as well would be the double drop
/// this guard would otherwise invite.
///
/// Asserted as an agreement over every transferring site — a static call,
/// an instance call whose *receiver* is a transferred argument like any other,
/// and a `new` — because each answers plausibly on its own line while the
/// mechanism behind them is one stack.
#[test]
fn a_transferred_argument_is_released_when_a_later_one_throws() {
    // `T::boom` is the later argument in each body below. Its body cannot
    // throw, and does not need to: every compiled call takes `rule:errors/propagation`'s fault
    // edge whatever its body does, and that edge is the whole question here.
    // `Pair` is what the `new` builds.
    const PRELUDE: &str = "<?nvs
class T {
  public static function boom(): int { return 1; }
  public static function take(string $a, int $n): int { return $n; }
  public function keep(string $a, int $n): int { return $n; }
}
class Pair { public function constructor(string $a, int $n) {} }
";

    // A static call: `$s` is an aliasing read, so it is retained for the
    // transfer, and `boom`'s fault edge is where that reference is dropped.
    let (f, map, file) = lower_script_src(&format!(
        "{PRELUDE}string $s = \"hi\";
int $r = T::take($s, T::boom());
echo $r;
"
    ));
    let text = print_function(&f, map.file(file));
    let staged = produced(&text, "const.str \"hi\"");
    // Twice on `boom`'s edge — the reference the transfer retained, and the
    // one `$s`'s own slot still holds, which `Self::release_all_locals` drops
    // at the same exit. Counted rather than looked for: one release there is
    // the leak this guard exists for, and it is indistinguishable from two by
    // presence alone.
    assert_eq!(
        release_count(&text, &fault_edge(&text, "T::boom"), &staged),
        2,
        "{text}"
    );
    // And once on `take`'s own edge: from that instruction the callee owns the
    // transferred reference and releases it however it leaves, so all that is
    // left here is the slot's.
    assert_eq!(
        release_count(&text, &fault_edge(&text, "T::take"), &staged),
        1,
        "{text}"
    );

    // An instance call, where the receiver is the transferred argument in
    // slot 0 — staged at the same site, on the same edge.
    let (f, map, file) = lower_script_src(&format!(
        "{PRELUDE}T $t = new T();
string $s = \"hi\";
int $r = $t->keep($s, T::boom());
echo $r;
"
    ));
    let text = print_function(&f, map.file(file));
    let fault = fault_edge(&text, "T::boom");
    let staged = produced(&text, "const.str \"hi\"");
    let receiver = produced(&text, "new T");
    assert_eq!(release_count(&text, &fault, &staged), 2, "{text}");
    assert_eq!(release_count(&text, &fault, &receiver), 2, "{text}");
    let fault = fault_edge(&text, "T::keep");
    assert_eq!(release_count(&text, &fault, &staged), 1, "{text}");
    assert_eq!(release_count(&text, &fault, &receiver), 1, "{text}");

    // And `new`, whose constructor takes the same transferred arguments.
    let (f, map, file) = lower_script_src(&format!(
        "{PRELUDE}string $s = \"hi\";
Pair $p = new Pair($s, T::boom());
echo \"ok\";
"
    ));
    let text = print_function(&f, map.file(file));
    let staged = produced(&text, "const.str \"hi\"");
    assert_eq!(
        release_count(&text, &fault_edge(&text, "T::boom"), &staged),
        2,
        "{text}"
    );
    assert_eq!(
        release_count(&text, &fault_edge(&text, "new Pair"), &staged),
        1,
        "{text}"
    );
}

/// `rule:types/conversion`'s `array<T> as array<U>` row, which is the one row of that
/// grid whose decision the pair of representations cannot carry: both sides
/// erase to `Ty::Array`, so what the walk checks travels beside the value as
/// [`array_element_tags`]' word instead — one [`param_tag_nibble`] per level
/// of `U`, outermost first.
///
/// Asserted as an agreement over every spelling the row has, in one
/// body, because each of them answers plausibly on its own line: the
/// checked walk, `rule:expressions/nullable-conversion`'s `?` twin through the helper that answers
/// `null`, the nesting that makes the word two nibbles rather than one, and
/// the `array<mixed>` target that is `Lowering::convert`'s free widening
/// and must walk *nothing*. The count is what holds the last one down — a
/// snapshot of any single conversion cannot see the free row grow a walk.
#[test]
fn an_array_conversion_walks_its_elements() {
    let (f, map, file) = lower_script_src(
        "<?nvs
array<mixed> $m = [\"a\", \"b\"];
array<tainted string> $s = $m as array<string>;
?array<tainted string> $o = $m as ?array<string>;
array<mixed> $n = [$m];
array<array<tainted string>> $d = $n as array<array<string>>;
array<mixed> $free = $s as array<mixed>;
",
    );
    let text = print_function(&f, map.file(file));
    let walks = |name: &str| {
        text.lines()
            .filter(|l| l.contains(&format!("helper.{name} ")))
            .count()
    };
    // Two checked walks — the one-level one and the nested one — and the
    // free row is the third `as` over an array that emitted neither.
    assert_eq!(walks("to_array_of"), 2, "{text}");
    assert_eq!(walks("to_array_of_or_null"), 1, "{text}");
    assert!(
        !text.contains(&format!("const.uint {FN_PARAM_TAG_ANY}")),
        "{text}"
    );
    // `string` is nibble 5, so `array<string>` is the word `5` and both the
    // checked and the `?` spelling carry it; `array<array<string>>` is the
    // array nibble outermost, `5 << 4 | 6`.
    assert_eq!(
        text.lines()
            .filter(|l| l.contains(&format!(
                "const.uint {}",
                u64::from(param_tag_nibble(Ty::Str))
            )))
            .count(),
        2,
        "{text}"
    );
    let nested =
        (u64::from(param_tag_nibble(Ty::Str)) << 4) | u64::from(param_tag_nibble(Ty::Array));
    assert!(text.contains(&format!("const.uint {nested}")), "{text}");
}

/// `rule:classes/ordering-lowers-to-compare-to`: ordering two objects *is* a `Comparable::compareTo` call
/// followed by a comparison of its `int` against zero — never a comparison
/// of the two values, which for objects would be two heap pointers.
#[test]
fn ordering_two_objects_calls_compare_to_and_tests_its_result_against_zero() {
    let (f, map, file) = lower_script_src(
        "<?nvs
class P implements Comparable {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
    public function compareTo(self $other): int { return $this->n - $other->n; }
}
var $a = new P(1);
var $b = new P(2);
if ($a < $b) { echo \"lt\"; }
",
    );
    let text = print_function(&f, map.file(file));
    assert!(text.contains("::compareTo"), "{text}");
    assert!(text.contains("const.int 0"), "{text}");
    assert!(
        text.lines()
            .any(|l| l.contains("lt v") && l.contains("bool")),
        "{text}"
    );
}

/// `<=>` is the call's own result: `compareTo` already returns exactly
/// what the spaceship operator means, so there is no second comparison.
#[test]
fn the_spaceship_operator_is_the_compare_to_result_itself() {
    let (f, map, file) = lower_script_src(
        "<?nvs
class P implements Comparable {
    public int $n;
    public function constructor(int $n) { $this->n = $n; }
    public function compareTo(self $other): int { return $this->n - $other->n; }
}
var $a = new P(1);
int $c = $a <=> $a;
",
    );
    let text = print_function(&f, map.file(file));
    assert!(text.contains("::compareTo"), "{text}");
    assert!(!text.contains("const.int 0"), "{text}");
}

/// `rule:classes/clone-is-shallow`: one copy instruction, a fresh object with one owner, and
/// no hook — `__clone` is one of the magic methods Novis does not have. The one helper in
/// front of it is `rule:iteration/generator-stays-in-one-isolate`'s generator guard, which
/// throws and runs no program code.
#[test]
fn clone_lowers_to_one_instruction_with_no_hook_call() {
    let (f, map, file) = lower_script_src(
        "<?nvs
class P { public int $n; public function constructor(int $n) { $this->n = $n; } }
var $a = new P(1);
var $b = clone $a;
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// `rule:types/conversion`'s integer *into* an enum: the conversion is free, and
/// what it costs is the check in front of it — one comparison per case of
/// the declaration, throwing with every case named. The accepted set is
/// built from the declaration rather than from the site, which is the
/// whole difference between this and `rule:types/enum-case-type`'s named subset.
#[test]
fn converting_into_an_enum_tests_every_case_of_the_declaration() {
    let (f, map, file) = lower_script_src(
        "<?nvs
enum Rank { Bronze, Gold }
int $n = 1;
Rank $r = $n as Rank;
",
    );
    let text = print_function(&f, map.file(file));
    assert!(text.contains("reinterpret"), "{text}");
    assert!(
        text.contains("`Rank::Bronze`, `Rank::Gold`"),
        "the throw names every case, in the declaration's own order: {text}"
    );
}

// ------------------------------------------------------------------
// By-reference parameters -- `Ty::Ref` owns the representation these
// pin down, and its refcounting section owns the retain/release pairing
// the refcounted-pointee case exists to make visible.
// ------------------------------------------------------------------

/// The caller's half: the holder is read, staged into a one-cell slot
/// whose address is the argument, and copied back out of that slot once
/// the call returns -- rebinding the local, so everything after the call
/// reads the written-back value. `int` is not refcounted, so the whole
/// thing costs a store, a load and no refcount traffic at all.
#[test]
fn a_by_reference_argument_is_staged_and_copied_back() {
    let (f, map, file) = lower_script_src(
        "<?nvs
class Adder {
               public static function bump(inout int $slot): void { $slot = $slot + 5; }
}
             int $n = 1;
Adder::bump(inout $n);
echo $n;
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// The callee's half: the parameter is a `ref`, every read of it is a
/// `ref.load` and every write a `ref.store`. Nothing is released at the
/// frame's exit -- a `Ty::Ref` is not `Ty::is_refcounted`, so
/// `release_all_locals` skips it, which is what keeps the caller's
/// staged reference the caller's.
#[test]
fn a_by_reference_parameter_reads_and_writes_through_its_slot() {
    let (f, map, file) = lower_first_method(
        "<?nvs
class T {
               public static function bump(inout int $slot): void { $slot = $slot + 5; }
}
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// Whether each statement call in `program` carries the plain binds bit, in
/// block order. The thunk's zero word is left out, since it names no call.
fn plain_binds_bits(program: &crate::ir::Program) -> Vec<bool> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .filter_map(|inst| match inst.kind {
            InstKind::QuerySiteConst {
                site: Some(_),
                plain_binds,
            } => Some(plain_binds),
            _ => None,
        })
        .collect()
}

/// `rule:core-classes/db-error`: a statement call whose bound values hold no
/// `secret` sets the bit, a statement with no values included.
#[test]
fn a_statement_call_with_no_secret_bind_sets_the_plain_binds_bit() {
    let (program, _, _) = lower_program(
        "<?nvs
var $db = Core\\Db::open({ driver: Core\\Db\\Driver::Sqlite, path: ':memory:' });
$db->execute('create table t (e text)', []);
string $email = 'a@example.com';
$db->execute('insert into t (e) values (?)', [$email]);
$db->executeMany('insert into t (e) values (?)', [[$email], ['b@example.com']]);
var $rows = $db->query('select e from t where e = ?', [$email . '!']);
",
    );
    assert_eq!(plain_binds_bits(&program), [true, true, true, true]);
}

/// `rule:core-classes/db-error`: a `secret` anywhere in what a statement call
/// binds clears the bit. Each call writes it a different way.
#[test]
fn a_secret_anywhere_in_the_bound_values_clears_the_plain_binds_bit() {
    let (program, _, _) = lower_program(
        "<?nvs
class Account {
    public secret string $token;
    public function constructor(secret string $token) { $this->token = $token; }
    public function store(Core\\Db\\Connection $db, secret string $key, array<secret string> $keys): void {
        $db->execute('insert into t (e) values (?)', [$key]);
        $db->execute('insert into t (e) values (?)', [$this->token]);
        $db->execute('insert into t (e) values (?)', ['Bearer ' . $key]);
        $db->executeMany('insert into t (e) values (?)', [['a'], [$key]]);
        $db->execute('insert into t (e) values (?)', $keys);
    }
}
",
    );
    assert_eq!(plain_binds_bits(&program), [false; 5]);
}

/// A refcounted pointee, where the policy is actually visible: one
/// staging `retain` before `ref.slot`, and one `release` of the holder's
/// previous value at the copy-back. The pair balances, which is what
/// makes the slot's own reference transfer into the holder rather than
/// leak or double-free. On the call's throw edge the landing block loads the
/// slot and releases it instead, so a callee that throws leaks nothing.
#[test]
fn a_refcounted_by_reference_argument_balances_its_staging_retain() {
    let (f, map, file) = lower_script_src(
        "<?nvs
class Shout {
               public static function upper(inout string $s): void { $s = $s . \"!\"; }
}
             string $msg = \"hi\";
Shout::upper(inout $msg);
echo $msg;
",
    );
    assert_snapshot!(print_function(&f, map.file(file)));
}

/// The copy-back lands **where the call is**, not at the enclosing
/// statement — so a call with an `inout $x` argument lowers in an operand
/// position like any other expression, and a read of the holder to the
/// right of it inside the *same* statement sees the written-back value.
///
/// Asserted structurally rather than left to the snapshot, because the
/// snapshot alone would still look plausible if the copy-back had drifted
/// back to the statement boundary: a `ref.load` is emitted once per
/// staged argument at its own call, so what pins the rule is that the
/// *second* one sits before the last `concat` rather than after every one
/// of them. See `Lowering::pending_refs` for the rule, and for the one
/// thing it does not buy — PHP's own operand order.
#[test]
fn a_reference_argument_lowers_in_any_expression_position() {
    let (f, map, file) = lower_script_src(
        "<?nvs
class Adder {
               public static function bump(inout int $slot): int { $slot = $slot + 5; return $slot; }
}
             int $n = 1;
echo \"a=\" . Adder::bump(inout $n) . \" then \" . $n . \" and \" . Adder::bump(inout $n) . \" then \" . $n;
",
    );
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("ref.slot").count(), 2, "{text}");
    let loads: Vec<usize> = text.match_indices("ref.load").map(|(i, _)| i).collect();
    let concats: Vec<usize> = text.match_indices("concat").map(|(i, _)| i).collect();
    assert_eq!(loads.len(), 2, "{text}");
    assert!(
        loads[1] < concats[concats.len() - 1],
        "the second copy-back must precede the last concat, not follow the whole \
             statement: {text}"
    );
    assert_snapshot!(text);
}

/// The same rule at its nesting edge: `Adder::sum(Adder::bump(inout $n), $n)`
/// stages `$n` for the *outer* call before the inner one's argument list
/// is lowered at all, so two stagings are live at once and each is
/// written back at its own call.
///
/// This is what `Lowering::pending_refs_mark` exists for. A flush that
/// drained the whole list would write the outer call's staged slot back
/// when the *inner* call returned — before the outer call had run, so its
/// own write would then be lost. The assertion is that the copy-back sits
/// between the two calls, which no such lowering can satisfy.
#[test]
fn a_nested_reference_argument_is_written_back_at_its_own_call() {
    let (f, map, file) = lower_script_src(
        "<?nvs
class Adder {
               public static function bump(inout int $slot): int { $slot = $slot + 5; return $slot; }
               public static function sum(int $a, int $b): int { return $a + $b; }
}
             int $n = 1;
echo Adder::sum(Adder::bump(inout $n), $n);
",
    );
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("ref.slot").count(), 1, "{text}");
    let load = text.find("ref.load").expect("the copy-back is emitted");
    let inner = text
        .find("call Adder::bump")
        .expect("the inner call is emitted");
    let outer = text
        .find("call Adder::sum")
        .expect("the outer call is emitted");
    assert!(inner < load && load < outer, "{text}");
    assert_snapshot!(text);
}

/// `rule:security/secret-comparison-is-constant-time`: `==` over two `secret` operands is the constant-time
/// helper, and an unqualified pair of the same representation is still
/// the ordinary `BinOp::Eq` that `nvs-codegen` turns into `nvs_str_eq`.
///
/// Both halves are asserted in one fixture on purpose. The qualifier
/// spends no representation (`erase_checked_ty`), so the *only* thing
/// separating these two comparisons in the IR is which helper the arm
/// picked — a version of this test that pinned the `secret` pair alone
/// would still pass if the lowering had started sending every `string`
/// comparison through the constant-time row, which is a real regression:
/// it would spend `rule:security/secret-comparison-is-constant-time`'s cost on
/// every string `==` in the language.
#[test]
fn a_secret_equality_lowers_to_the_constant_time_helper() {
    let (f, map, file) = lower_script_src(
        "<?nvs
secret string $token = \"a\";
secret string $given = \"b\";
bool $secretly = $token == $given;
string $plain = \"a\";
string $other = \"b\";
bool $openly = $plain == $other;
",
    );
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("helper.secret_eq").count(), 1, "{text}");
    // `= eq `, not `eq `: `helper.secret_eq v0, v1` ends in the shorter
    // one, so the loose spelling counts the constant-time call twice and
    // the assertion below can never fail.
    assert_eq!(text.matches("= eq ").count(), 1, "{text}");
}

/// The `bytes` base of the same rule, and `!=` — `rule:security/secret-qualifier` puts the
/// qualifier on both bases, and `Helper::SecretEq` answers `!=` under a
/// `UnOp::Not` rather than through a second helper, the arrangement
/// `Helper::NumericEq` already uses. A `!=` that had grown its own
/// short-circuiting row would be the same timing oracle
/// `rule:security/secret-comparison-is-constant-time` closes.
#[test]
fn a_secret_bytes_inequality_is_the_same_helper_under_a_not() {
    let (f, map, file) = lower_script_src(
        "<?nvs
secret bytes $mac = \"a\" as bytes;
secret bytes $sent = \"b\" as bytes;
bool $differ = $mac != $sent;
",
    );
    let text = print_function(&f, map.file(file));
    assert!(text.contains("helper.secret_eq"), "{text}");
    assert!(text.contains("not "), "{text}");
}

/// The pair whose other side is a `mixed` — a decoded request field, a header,
/// a cache read — which `rule:security/secret-comparison-is-constant-time`
/// covers because `rule:security/secret-propagation` has already poisoned
/// whatever a credential was compared against. `Helper::Identical` is what it
/// used to take, and that row short-circuits on the first differing byte,
/// which is the timing oracle the rule exists to close. The unqualified pair
/// below still takes `Identical`, so the assertion reads the *choice* rather
/// than the presence of a call.
#[test]
fn a_secret_compared_against_a_mixed_string_takes_the_constant_time_helper() {
    let (f, map, file) = lower_script_src(
        "<?nvs
secret string $token = \"a\";
mixed $given = \"b\";
bool $secretly = $token == $given;
mixed $plain = \"a\";
mixed $other = \"b\";
bool $openly = $plain == $other;
",
    );
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("helper.secret_eq").count(), 1, "{text}");
    assert_eq!(text.matches("helper.identical").count(), 1, "{text}");
}

/// A `secret` that is one member of the operand's type — a `?secret string`, a
/// `secret string|int` — is still a credential when it is there, so
/// `rule:security/secret-comparison-is-constant-time` applies to the comparison
/// exactly as it does to a bare `secret string`. Each pair below is compared
/// against a plain `string`, and the unqualified nullable pair at the end
/// still takes the ordinary row, so the count reads the checker's choice.
#[test]
fn a_nullable_or_union_secret_takes_the_constant_time_helper() {
    let (f, map, file) = lower_script_src(
        "<?nvs
?secret string $token = \"a\";
secret string|int $either = \"a\";
string $given = \"b\";
bool $nullable = $token == $given;
bool $union = $given != $either;
?string $plain = \"a\";
bool $openly = $plain == $given;
",
    );
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("helper.secret_eq").count(), 2, "{text}");
}

/// `rule:types/erased-member-access`'s deferral, one storage kind along from a member access: a
/// `mixed` base defers *whether there is an array here* as well as which
/// one, so a subscript through it reaches the helper pair that asks the
/// operand's tag rather than `InstKind::ArrayGet`, whose own base is an
/// `array<T>` by declaration.
///
/// Read off the rendering rather than snapshotted, because what is pinned
/// is *which* entry point each of the two reads takes — a snapshot would go
/// red for any unrelated renumbering and say nothing about that.
///
/// Both carry an error edge, and that is not what tells them apart: every
/// instruction returning a status does, whether or not a *program* can fail
/// in it, so that an uncatchable one leaves the frame swept
/// ([`Inst::on_error`](crate::ir::Inst::on_error)). What the guarded read
/// answers for an absent key or a non-array tag is `null`, and that is a
/// value rather than an edge.
#[test]
fn a_subscript_through_a_tagged_base_lowers() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function read(mixed $m): mixed {\n",
        "    mixed $plain = $m[0];\n",
        "    mixed $guarded = $m[\"k\"] ?? \"d\";\n",
        "    return $guarded;\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    assert!(
        !text.contains("array_required_get") && !text.contains("array_optional_get"),
        "a tagged base reached the statically typed read, whose non-array row \
         is an internal inconsistency rather than a catchable throw: {text}"
    );
    for line in text.lines() {
        if line.contains("value_index_get") || line.contains("value_index_optional_get") {
            assert!(
                line.contains(" ! bb"),
                "a status-returning read with no landing block: {line}"
            );
        }
    }
    assert_eq!(text.matches("value_index_get").count(), 1, "{text}");
    assert_eq!(
        text.matches("value_index_optional_get").count(),
        1,
        "{text}"
    );
}

/// `??=` reads its target the way `??` reads its left operand, at every
/// level, and writes it the way the plain `=` does
/// (`rule:expressions/defaulting-assignment`). The inner level takes the
/// `null`-answering `array.get.ornull`; the outer level's base may then be
/// `null`, so it asks the tag. The write is one `array_write_through`, which
/// builds an absent row, so `$g["k"]["j"] ??= 5` with no `"k"` creates it.
/// `+=` on the same target keeps the reads that throw.
#[test]
fn coalesce_assign_lowers_a_guarded_read_of_its_target() {
    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function fill(): void {\n",
        "    array<array<int>> $g = [];\n",
        "    $g[\"k\"][\"j\"] ??= 5;\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("array.get.ornull").count(), 1, "{text}");
    assert_eq!(
        text.matches("value_index_optional_get").count(),
        1,
        "{text}"
    );
    assert!(!text.contains("array.get v"), "a throwing read: {text}");
    assert_eq!(text.matches("array_write_through").count(), 1, "{text}");

    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function fill(): void {\n",
        "    array<array<int>> $g = [];\n",
        "    $g[\"k\"][\"j\"] += 5;\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    assert!(!text.contains("ornull"), "{text}");
    assert!(!text.contains("value_index_optional_get"), "{text}");
    assert_eq!(text.matches("array.get v").count(), 2, "{text}");
}

/// `??=` on a hooked property calls the `get` hook once, for the read, and
/// the `set` hook once, for the write. The value of the expression is the
/// value written, never a second read of the property.
#[test]
fn coalesce_assign_on_a_hooked_property_reads_once_and_writes_once() {
    let (program, map, file) = lower_whole_file_with_src(concat!(
        "<?nvs\n",
        "class Post {\n",
        "  public ?string $title {\n",
        "    get => $this->title;\n",
        "    set(?string $v) { $this->title = $v; }\n",
        "  }\n",
        "  function constructor() { $this->title = null; }\n",
        "  public function title(): string { return $this->title ??= \"Untitled\"; }\n",
        "}\n",
    ));
    let title = program
        .functions
        .iter()
        .find(|f| f.name == "Post::title")
        .expect("`title` should have been lowered");
    let text = print_function(title, map.file(file));
    assert_eq!(text.matches("Post::$title::get").count(), 1, "{text}");
    assert_eq!(text.matches("Post::$title::set").count(), 1, "{text}");
}

/// `??+=` evaluates its target once (`rule:expressions/defaulting-assignment`):
/// a hooked property's `get` and `set` hooks run once each, and an element
/// target reads each level guarded, once, and builds the row on the write,
/// exactly as `??=` does.
#[test]
fn a_defaulting_assignment_stages_its_target_once() {
    let (program, map, file) = lower_whole_file_with_src(concat!(
        "<?nvs\n",
        "class Post {\n",
        "  public ?int $views {\n",
        "    get => $this->views;\n",
        "    set(?int $v) { $this->views = $v; }\n",
        "  }\n",
        "  function constructor() { $this->views = null; }\n",
        "  public function seen(): int { return $this->views ??+= 1; }\n",
        "}\n",
    ));
    let seen = program
        .functions
        .iter()
        .find(|f| f.name == "Post::seen")
        .expect("`seen` should have been lowered");
    let text = print_function(seen, map.file(file));
    assert_eq!(text.matches("Post::$views::get").count(), 1, "{text}");
    assert_eq!(text.matches("Post::$views::set").count(), 1, "{text}");

    let (f, map, file) = lower_first_method(concat!(
        "<?nvs\nclass T {\n",
        "  function fill(): void {\n",
        "    array<array<string>> $g = [];\n",
        "    $g[\"k\"][\"j\"] ??.= \"x\";\n",
        "  }\n}\n",
    ));
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("array.get.ornull").count(), 1, "{text}");
    assert_eq!(
        text.matches("value_index_optional_get").count(),
        1,
        "{text}"
    );
    assert!(!text.contains("array.get v"), "a throwing read: {text}");
    assert_eq!(text.matches("array_write_through").count(), 1, "{text}");
}

/// The default a defaulting assignment starts from is a constant of the
/// target's own representation: `0` as a `uint`, a `float`, a `decimal` and
/// an `int`, and `""` under `.`. A `mixed` target starts from the `int` `0`.
#[test]
fn a_defaulting_assignment_starts_from_a_constant_of_the_target_representation() {
    for (decl, op, value, zero) in [
        ("?uint $t = null;", "??+=", "1", "const.uint 0"),
        ("?float $t = null;", "??-=", "1.5", "const.float 0"),
        ("?decimal $t = null;", "??+=", "1.5", "const.decimal 0e-0"),
        ("?int $t = null;", "??-=", "1", "const.int 0"),
        ("?string $t = null;", "??.=", "\"x\"", "const.str \"\""),
        ("mixed $t = null;", "??+=", "1", "const.int 0"),
    ] {
        let (f, map, file) = lower_first_method(&format!(
            "<?nvs\nclass T {{\n  function m(): void {{\n    {decl}\n    $t {op} {value};\n  }}\n}}\n"
        ));
        let text = print_function(&f, map.file(file));
        assert!(text.contains(zero), "{decl} {op}: {text}");
    }
}

/// A resolved `Core\Router::url` releases the `$params` array it was handed —
/// [`Lowering::lower_route_link`]'s own accounting, and the one thing about
/// that arm which no snapshot of a *passing* program shows.
///
/// This arm builds its arguments by hand instead of through
/// [`Lowering::lower_call_args`], so it is the only `Core` call site where
/// [`Lowering::account_for_arg`] can be forgotten — and forgetting it leaks
/// one array header per `Core\Router::url(T::show, ["id" => 7])`, which only the
/// `examples/` valgrind sweep sees, because a leak is invisible to the
/// program that causes it. Asserted over the array's own `ValueId` rather than
/// by counting releases, so a release of the *template* beside it cannot stand
/// in for this one.
#[test]
fn a_resolved_route_link_releases_its_params_array() {
    let (f, map, file) = lower_script_src(concat!(
        "<?nvs\nclass T {\n",
        r#"  #[Core\Route(path:"/users/{id}", method: Core\Http\Method::Get)]"#,
        "\n",
        r#"  #[Core\Access(allow: Core\Audience::Public)]"#,
        "\n",
        "  public function show(uint $id): string { return \"u\"; }\n",
        "}\n",
        r#"echo Core\Router::url(T::show, ["id" => 7]);"#,
        "\n",
    ));
    let insts = || f.blocks.iter().flat_map(|b| b.insts.iter());
    let call = insts()
        .find_map(|i| match &i.kind {
            InstKind::CoreCall { symbol, args, .. } if *symbol == nvs_types::CORE_ROUTE_LINK => {
                Some(args.clone())
            }
            _ => None,
        })
        .expect("the link resolved: a `CoreCall` to the prepared-path helper");
    let params = *call.last().expect("the link takes two arguments");
    // The literal threads a fresh `ValueId` per entry written into it, so what
    // reaches the call is the last `array.set` rather than the `array.new` —
    // which is exactly why this asserts over the argument the call names.
    assert!(
        insts().any(|i| i.result == Some(params)
            && matches!(
                i.kind,
                InstKind::ArrayNew { .. } | InstKind::ArraySet { .. }
            )),
        "the link's second argument is the written array literal: {}",
        print_function(&f, map.file(file))
    );
    assert!(
        insts().any(|i| matches!(i.kind, InstKind::Release { operand } if operand == params)),
        "nothing released the `$params` array: {}",
        print_function(&f, map.file(file))
    );
}

/// A file that runs out of statements seals with `1` when a `require` site
/// entered it and with `null` when it is the program's entry frame —
/// `rule:security/isolate-failure-is-a-value`'s "not `require`'s `1`", against
/// `rule:statements/a-require-expression-is-mixed`.
///
/// Both sides in one test, over the *same* source, because they are one
/// decision with two halves: a lowering that moved both would still read
/// plausibly against either half alone. The entry frame is what a
/// `spawn script` child runs, so this line is what the child's `value` is.
#[test]
fn a_fall_through_seals_with_one_for_a_require_and_null_for_the_entry_frame() {
    let src = "<?nvs\necho \"no return\";\n";
    let (entry, entry_map, entry_file) = lower_script_src_as(src, ScriptRole::Entry);
    let (required, req_map, req_file) = lower_script_src_as(src, ScriptRole::Required);
    let entry = print_function(&entry, entry_map.file(entry_file));
    let required = print_function(&required, req_map.file(req_file));

    assert!(entry.contains("const.null"), "the entry frame: {entry}");
    assert!(!entry.contains("const.int 1"), "the entry frame: {entry}");
    assert!(
        required.contains("const.int 1"),
        "a required file: {required}"
    );
    assert!(
        !required.contains("const.null"),
        "a required file: {required}"
    );
}

/// `spawn script` lowers to **one fallible `CoreCall` in the frame that
/// wrote it** — `rule:security/isolate-shares-nothing`, as `Lowering::lower_spawn_script`
/// records it.
///
/// That is what "a task on the current core" is, at this level: the child
/// is named by a path *value* handed to a native symbol, so no second
/// frame is lowered for it, nothing here forks a thread or a process, and
/// the instruction sits in the same block as the statements around it.
/// The whole of the child's identity is the arguments, in the fixed order
/// — the path, the `arguments:` value, the `output:` spelling, the placement
/// and the two narrowings — with every option the program omitted
/// materialized here rather than defaulted in the helper.
///
/// The refcount asymmetry is the transfer, and it is the reason this
/// asserts over the arguments rather than snapshotting them: the path
/// literal is borrowed and is released by this frame, while the `arguments:`
/// array crosses into the child's ownership root and is released by
/// nothing on this side.
#[test]
fn a_spawn_lowers_to_a_task_on_the_current_core() {
    let (f, map, file) =
        lower_script_src("<?nvs\nvar $job = spawn script 'child.nvs' with(arguments: [1, 2]);\n");
    let insts = || f.blocks.iter().flat_map(|b| b.insts.iter());
    let spawns: Vec<&Inst> = insts()
        .filter(|i| {
            matches!(&i.kind, InstKind::CoreCall { symbol, .. } if *symbol == nvs_types::CORE_SCRIPT_SPAWN)
        })
        .collect();
    assert_eq!(
        spawns.len(),
        1,
        "one spawn is one call: {}",
        print_function(&f, map.file(file))
    );
    let spawn = spawns[0];
    assert!(
        spawn.on_error.is_some(),
        "a spawn can throw and carries `rule:errors/propagation`'s error edge: {}",
        print_function(&f, map.file(file))
    );
    let InstKind::CoreCall { args, .. } = &spawn.kind else {
        unreachable!("filtered above")
    };
    assert_eq!(
        args.len(),
        6,
        "the path, the `arguments:` value, the `output:` spelling, the placement and \
         the `limits:` and `grants:` narrowings: {}",
        print_function(&f, map.file(file))
    );
    let (path, payload, output, on) = (args[0], args[1], args[2], args[3]);
    let (limits, grants) = (args[4], args[5]);
    let const_str = |v: ValueId| {
        insts().find_map(|i| match &i.kind {
            InstKind::ConstStr(s) if i.result == Some(v) => Some(s.clone()),
            _ => None,
        })
    };
    assert_eq!(
        const_str(path).as_deref(),
        Some("child.nvs"),
        "the first argument is the written path: {}",
        print_function(&f, map.file(file))
    );
    // The program wrote no `output:`, so the default is visible in the IR
    // a reader dumps rather than living in the helper.
    assert_eq!(
        const_str(output).as_deref(),
        Some("capture"),
        "the third argument is `rule:security/isolate-shares-nothing`'s captured-by-default sink: {}",
        print_function(&f, map.file(file))
    );
    // The same for the placement the program did not write:
    // `rule:concurrency/on-worker-runs-the-child-on-another-core`'s default is
    // the parent's own core, and it is a constant in the dump rather than a
    // `None` the helper reads a default out of.
    assert_eq!(
        const_str(on).as_deref(),
        Some("here"),
        "the fourth argument is the placement, and no `on:` is this core: {}",
        print_function(&f, map.file(file))
    );
    // And the two narrowings, which this program wrote neither of. `null` and
    // not an empty shape or an empty array, because those say something else:
    // `grants: []` is a child that may ask for nothing at all, where no
    // `grants:` leaves the parent's own set standing
    // (`rule:security/isolate-shares-nothing`).
    // Through the `tag`: an option is carried at `tagged`, so what the call
    // takes is the widened value and the constant is one instruction behind it.
    let is_null = |v: ValueId| {
        let produced = |v: ValueId| insts().find(|i| i.result == Some(v)).map(|i| &i.kind);
        let Some(InstKind::Tag { operand }) = produced(v) else {
            return false;
        };
        matches!(produced(*operand), Some(InstKind::ConstNull))
    };
    assert!(
        is_null(limits) && is_null(grants),
        "a spawn that narrowed nothing carries a `null` for each option: {}",
        print_function(&f, map.file(file))
    );
    let released_in = |b: &crate::ir::BasicBlock, v: ValueId| {
        b.insts
            .iter()
            .any(|i| matches!(i.kind, InstKind::Release { operand } if operand == v))
    };
    let normal = f
        .blocks
        .iter()
        .find(|b| b.insts.iter().any(|i| std::ptr::eq(i, spawn)))
        .expect("the spawn is in a block");
    let landing = &f.blocks[spawn.on_error.expect("asserted above").index() as usize];
    assert!(
        released_in(normal, path),
        "the borrowed path string literal is this frame's to release: {}",
        print_function(&f, map.file(file))
    );
    assert!(
        !released_in(normal, payload),
        "the `arguments:` value crossed the boundary and may not be released here: {}",
        print_function(&f, map.file(file))
    );
    // The other side of the same bound: a spawn that *threw* never handed
    // the value over — an argument that cannot cross is the parent's error
    // at the spawn — so the landing block releases what the normal edge
    // must not (`Lowering::release_temporaries_since`).
    assert!(
        released_in(landing, payload),
        "a throwing spawn transferred nothing and owes the release: {}",
        print_function(&f, map.file(file))
    );
}

/// `await` lowers to a second fallible `CoreCall`, and what it does
/// **not** do is the claim: the frame is not split at the await.
///
/// A generator's `yield` cuts its frame in two and spills every local
/// live across the cut (`lower::generator`), because the frame has to be
/// abandoned and rebuilt. An `await` does none of that — the task
/// suspends on a stack of its own (`nvs_host`'s coroutines, `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`)
/// and resumes with the completion in hand — so the handle is read, the
/// call is made and the result is used in one straight run of
/// instructions, with the statements either side of it in the same block.
///
/// The handle is borrowed, exactly as a `Core` member's receiver is: what
/// the helper consumes is the request's entry for the started isolate,
/// not the object `$job` holds.
#[test]
fn an_await_suspends_until_its_task_completes() {
    let (f, map, file) = lower_script_src(concat!(
        "<?nvs\n",
        "var $job = spawn script 'child.nvs';\n",
        "var $done = await $job;\n",
    ));
    let dump = print_function(&f, map.file(file));
    let insts = || f.blocks.iter().flat_map(|b| b.insts.iter());
    let named = |symbol: &'static str| {
        insts().find(|i| matches!(&i.kind, InstKind::CoreCall { symbol: s, .. } if *s == symbol))
    };
    let spawn =
        named(nvs_types::CORE_SCRIPT_SPAWN).unwrap_or_else(|| panic!("the spawn lowered: {dump}"));
    let await_ =
        named(nvs_types::CORE_SCRIPT_AWAIT).unwrap_or_else(|| panic!("the await lowered: {dump}"));
    assert!(await_.on_error.is_some(), "an await can throw: {dump}");
    let InstKind::CoreCall { args, .. } = &await_.kind else {
        unreachable!("filtered above")
    };
    assert_eq!(
        args,
        &vec![spawn.result.expect("a spawn answers a handle")],
        "the await takes the spawn's own handle and nothing else: {dump}"
    );
    // Borrowed: nothing retains the handle for the call.
    let handle = args[0];
    assert!(
        !insts().any(|i| matches!(i.kind, InstKind::Retain { operand } if operand == handle)),
        "the handle is borrowed, not retained, for the await: {dump}"
    );
    // One block for both calls: no state machine, no phi, no spill.
    let block_of = |i: &Inst| {
        f.blocks
            .iter()
            .position(|b| b.insts.iter().any(|x| std::ptr::eq(x, i)))
            .expect("every instruction is in a block")
    };
    assert_eq!(
        block_of(spawn),
        block_of(await_),
        "an await does not cut its frame the way a `yield` does: {dump}"
    );
}

/// A call site writing an inline shape as its type argument carries **two**
/// constants for it, because one class answers for every field type: the
/// descriptor of the class its field names synthesize, and the wire contract
/// that class cannot hold. `rule:types/shape-type` is the type, and the crate
/// docs' *A shape's wire contract* is the carrier.
///
/// Asserted over two sites at once, since what would look right on either line
/// alone is one class *and* one contract: `{ n: int }` and `{ n: string }` are the
/// pair that shares a label and shares no contract at all.
#[test]
fn two_shapes_of_one_field_name_share_a_class_and_carry_a_contract_each() {
    let (program, map, file) = lower_whole_file_with_src(concat!(
        "<?nvs\n",
        "var $ints = Core\\Json::decodeAs<{n: int}>(\"{\\\"n\\\":1}\");\n",
        "var $text = Core\\Json::decodeAs<{n: string}>(\"{\\\"n\\\":\\\"a\\\"}\");\n",
    ));
    // The class is registered by the call site alone — this file writes no
    // `{ n: 1 }` literal, and without it the descriptor constant below would name
    // a class the unit never declares.
    let shapes: Vec<&str> = program
        .classes
        .iter()
        .map(|class| class.label.as_str())
        .filter(|label| label.starts_with("$shape"))
        .collect();
    assert_eq!(shapes, ["$shape{n}"]);
    let keys: Vec<&str> = program
        .shape_codecs
        .iter()
        .map(|codec| codec.key.as_str())
        .collect();
    assert_eq!(keys, ["$codec{n:Int}", "$codec{n:Str}"]);
    let script = program
        .functions
        .iter()
        .find(|f| f.name == "<script>")
        .expect("the script frame should have been lowered");
    let text = print_function(script, map.file(file));
    assert!(text.contains("class.desc $shape{n}"), "{text}");
    assert!(text.contains("shape.codec $codec{n:Int}"), "{text}");
    assert!(text.contains("shape.codec $codec{n:Str}"), "{text}");
}

/// The same slot on a call whose type argument named a **class**: the contract
/// is the class's own, so what the call carries is
/// `crate::ir::InstKind::ShapeCodecConst`'s `None` rather than a second ABI.
#[test]
fn a_written_class_carries_no_contract_beside_its_descriptor() {
    let (program, map, file) = lower_whole_file_with_src(concat!(
        "<?nvs\n",
        // The attribute is what makes the call legal at all: a class that
        // declared no codec is `E0821` at the site that names it, and this
        // fixture is about the slot beside the descriptor rather than about
        // that refusal.
        "#[Core\\Json\\Derive]\n",
        "class Note {\n",
        "  public string $name;\n",
        "  function constructor(string $name) { $this->name = $name; }\n",
        "}\n",
        "var $note = Core\\Json::decodeAs<Note>(\"{\\\"name\\\":\\\"ada\\\"}\");\n",
    ));
    assert!(program.shape_codecs.is_empty());
    let script = program
        .functions
        .iter()
        .find(|f| f.name == "<script>")
        .expect("the script frame should have been lowered");
    let text = print_function(script, map.file(file));
    assert!(text.contains("class.desc Note"), "{text}");
    assert!(text.contains("shape.codec none"), "{text}");
}

/// One frame's [`Lowering::source`] and the [`Lowering::frame_label`] rendered
/// beside it, without running a body through: the fixture's statement span is
/// set by hand to the third line, which is what both are read against.
fn frame_at(label: &str, member: Option<&str>) -> (Source, String) {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", "<?nvs\n$a = 1;\n$b = 2;\n");
    let exprs = ExprTypeTable::new();
    let checked_types = TypeInterner::new();
    let enums = EnumTable::default();
    let mut low = Lowering::new(
        label,
        member,
        map.file(file),
        Ty::Void,
        &exprs,
        &checked_types,
        &enums,
    );
    low.cur_stmt_span = Span::at(file, 14);
    (low.source(), low.frame_label())
}

#[test]
fn a_method_frame_names_its_member_and_a_script_frame_names_none() {
    let (method, _) = frame_at("T::m", Some("T::m"));
    assert_eq!(method.file, "t.nvs");
    assert_eq!(method.line, 3);
    assert_eq!(method.member.as_deref(), Some("T::m"));

    for label in ["script", "file#0$script"] {
        let (script, _) = frame_at(label, None);
        assert_eq!(script.file, "t.nvs");
        assert_eq!(script.line, 3);
        assert_eq!(script.member, None, "{label} is a frame name, not a member");
    }
}

#[test]
fn a_backtrace_frame_renders_from_the_source_a_record_would_carry() {
    for (label, member) in [
        ("T::m", Some("T::m")),
        ("script", None),
        ("file#0$script", None),
    ] {
        let (source, frame) = frame_at(label, member);
        assert_eq!(
            frame,
            format!("{label}() at {}:{}", source.file, source.line)
        );
    }
}

/// `rule:types/grammar`'s atom set is closed, and [`lower_decl_type`] answers
/// all of it: one row below per `nvs_syntax::ast::TypeAtom` variant and per
/// `TypeKind` variant, with the row count as the assertion. That count is the
/// guard rather than a formality — both enums are `#[non_exhaustive]`, so the
/// compiler never asks for the arm, and a spelling added to the grammar
/// without one would otherwise reach a panic in a release nobody ran this
/// against.
///
/// The tables are lowered against an **empty** [`ExprTypeTable`], which is what
/// the match itself answers for: an annotation the checker visited takes the
/// `declared_ty` shortcut instead and never reaches it.
///
/// `TypeAtom::Member` and `TypeAtom::MemberCase` are the rows with no answer
/// off the AST — whether `Rank::Silver` is an `int`, a `string` or an enum's
/// tag is what resolution decided — so their rows are `None` and the assertion at the end is the shortcut
/// that owns it: a checked program's enum-case parameter comes out
/// [`Ty::Enum`], which no arm of that match can produce.
#[test]
fn lower_decl_type_answers_every_type_atom_the_grammar_has() {
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", "");
    let at = nvs_diagnostics::Span::new(file, 0, 1);
    let node = |kind| nvs_syntax::ast::Type { kind, span: at };
    let name = nvs_syntax::ast::Name { span: at };
    let int = || node(TypeKind::Atom(TypeAtom::Int));
    let named = || node(TypeKind::Atom(TypeAtom::Name(name, Vec::new())));
    let exprs = ExprTypeTable::new();
    let checked_types = TypeInterner::new();

    let atoms: Vec<(TypeAtom, Option<Ty>)> = vec![
        (TypeAtom::Null, Some(Ty::Null)),
        (TypeAtom::Bool, Some(Ty::Bool)),
        (TypeAtom::Int, Some(Ty::Int)),
        (TypeAtom::Uint, Some(Ty::Uint)),
        (TypeAtom::Float, Some(Ty::Float)),
        (TypeAtom::Decimal, Some(Ty::Decimal)),
        (TypeAtom::String, Some(Ty::Str)),
        (TypeAtom::Bytes, Some(Ty::Bytes)),
        (TypeAtom::TaintedString, Some(Ty::Str)),
        (TypeAtom::TaintedBytes, Some(Ty::Bytes)),
        (TypeAtom::SecretString, Some(Ty::Str)),
        (TypeAtom::SecretBytes, Some(Ty::Bytes)),
        (TypeAtom::SecretTaintedString, Some(Ty::Str)),
        (TypeAtom::SecretTaintedBytes, Some(Ty::Bytes)),
        (
            TypeAtom::Array(Some(Box::new(int())), None),
            Some(Ty::Array),
        ),
        (TypeAtom::ClassRef(Box::new(named())), Some(Ty::ClassDesc)),
        (TypeAtom::PropertyKey(Box::new(named())), Some(Ty::Str)),
        (TypeAtom::Object, Some(Ty::Object)),
        (
            TypeAtom::Shape(vec![nvs_syntax::ast::ShapeField {
                name: at,
                ty: int(),
                required: true,
                span: at,
                doc: None,
            }]),
            Some(Ty::Object),
        ),
        (TypeAtom::Mixed, Some(Ty::Tagged)),
        (TypeAtom::SecretMixed, Some(Ty::Tagged)),
        (TypeAtom::Void, Some(Ty::Void)),
        (TypeAtom::Never, Some(Ty::Void)),
        (TypeAtom::True, Some(Ty::Bool)),
        (TypeAtom::False, Some(Ty::Bool)),
        (TypeAtom::SingleValueString(at), Some(Ty::Str)),
        (TypeAtom::SingleValueInt(at), Some(Ty::Int)),
        (TypeAtom::Member(name, at), None),
        (TypeAtom::MemberCase(name, at, at), None),
        (TypeAtom::Iterable, Some(Ty::Tagged)),
        (TypeAtom::Callable, Some(Ty::Object)),
        (
            TypeAtom::CallableSig {
                params: vec![int()],
                ret: Box::new(node(TypeKind::Atom(TypeAtom::String))),
            },
            Some(Ty::Object),
        ),
        (TypeAtom::SelfTy, Some(Ty::Object)),
        (TypeAtom::StaticTy, Some(Ty::Object)),
        (TypeAtom::Parent, Some(Ty::Object)),
        (TypeAtom::Name(name, Vec::new()), Some(Ty::Object)),
    ];
    assert_eq!(
        atoms.len(),
        36,
        "one row per `nvs_syntax::ast::TypeAtom` variant — write the row in the slice that \
         writes the arm"
    );
    for (atom, want) in atoms {
        let Some(want) = want else {
            continue;
        };
        let got = lower_decl_type(&node(TypeKind::Atom(atom.clone())), &exprs, &checked_types);
        assert_eq!(got, want, "`{atom:?}` lowered as `{got:?}`");
    }

    let kinds: Vec<(TypeKind, Ty)> = vec![
        (TypeKind::Paren(Box::new(int())), Ty::Int),
        (TypeKind::Nullable(Box::new(int())), Ty::Tagged),
        (
            TypeKind::Union(vec![int(), node(TypeKind::Atom(TypeAtom::String))]),
            Ty::Tagged,
        ),
        (TypeKind::Intersection(vec![named(), named()]), Ty::Tagged),
    ];
    assert_eq!(
        kinds.len(),
        4,
        "one row per `nvs_syntax::ast::TypeKind` variant other than `Atom`, which the table \
         above covers"
    );
    for (kind, want) in kinds {
        let got = lower_decl_type(&node(kind.clone()), &exprs, &checked_types);
        assert_eq!(got, want, "`{kind:?}` lowered as `{got:?}`");
    }

    // The shortcut the `Member` row stands on, end to end. `rule:enums/closed-integer-type`
    // makes the case an integer with the enum's own tag, and nothing in the
    // AST says so.
    let (function, ..) = lower_first_method(
        "<?nvs\nenum Rank { Bronze, Silver, Gold }\n\n\
         class T {\n    public function m(Rank::Silver $r): void {}\n}\n",
    );
    assert_eq!(
        function.params.get(1),
        Some(&Ty::Enum(EnumRepr::Int)),
        "an enum-case parameter is answered by `ExprTypeTable::declared_ty`, not by the AST"
    );
}

/// [`erase_checked_ty`] answers every checked type a value can have, which is
/// why nothing below this boundary asks whether a representation exists — the
/// fold in [`shared_erasure`] is the one site that used to.
///
/// One row per `nvs_types::ty::Ty` variant, the length is the assertion for the
/// same `#[non_exhaustive]` reason the atom probe above gives, and
/// `CheckedTy::CoreShape` is the row that is not a value at all: a `Core`
/// options bag is one argument per merged slot, so what its row asserts is that
/// the arm is *spelled*, rather than a shape quietly reaching the trailing one.
#[test]
fn erase_checked_ty_answers_every_checked_type_a_value_can_have() {
    let mut types = TypeInterner::new();
    let int = types.intern(CheckedTy::Int);
    let string = types.intern(CheckedTy::String);
    let one = nvs_hir::QName::from_segments(vec!["A".to_owned()]);
    let two = nvs_hir::QName::from_segments(vec!["B".to_owned()]);
    let class = types.intern(CheckedTy::Class(one.clone(), Vec::new()));
    let other = types.intern(CheckedTy::Class(two, Vec::new()));

    let rows: Vec<(CheckedTy, Ty)> = vec![
        (CheckedTy::Null, Ty::Null),
        (CheckedTy::Bool, Ty::Bool),
        (CheckedTy::Int, Ty::Int),
        (CheckedTy::Uint, Ty::Uint),
        (CheckedTy::Float, Ty::Float),
        (CheckedTy::Decimal, Ty::Decimal),
        (CheckedTy::Void, Ty::Void),
        (CheckedTy::String, Ty::Str),
        (CheckedTy::Bytes, Ty::Bytes),
        (CheckedTy::TaintedString, Ty::Str),
        (CheckedTy::TaintedBytes, Ty::Bytes),
        (CheckedTy::SecretString, Ty::Str),
        (CheckedTy::SecretBytes, Ty::Bytes),
        (CheckedTy::SecretTaintedString, Ty::Str),
        (CheckedTy::SecretTaintedBytes, Ty::Bytes),
        (CheckedTy::Array(int, None), Ty::Array),
        (CheckedTy::ClassRef(class), Ty::ClassDesc),
        (CheckedTy::PropertyKey(class), Ty::Str),
        (CheckedTy::Object, Ty::Object),
        (CheckedTy::Mixed, Ty::Tagged),
        (CheckedTy::SecretMixed, Ty::Tagged),
        (CheckedTy::Never, Ty::Void),
        (CheckedTy::True, Ty::Bool),
        (CheckedTy::False, Ty::Bool),
        (CheckedTy::SingleValueString("a".to_owned()), Ty::Str),
        (CheckedTy::SingleValueInt(1), Ty::Int),
        (
            CheckedTy::EnumCase(
                one.clone(),
                nvs_types::EnumBacking::Uint,
                "Silver".to_owned(),
            ),
            Ty::Enum(EnumRepr::Uint),
        ),
        (CheckedTy::Iterable, Ty::Tagged),
        (CheckedTy::Callable, Ty::Object),
        (
            CheckedTy::CallableSig {
                params: vec![int],
                ret: string,
            },
            Ty::Object,
        ),
        (CheckedTy::ShapeOfCallables("S".to_owned()), Ty::Object),
        (CheckedTy::Class(one.clone(), Vec::new()), Ty::Object),
        (
            CheckedTy::Enum(one, nvs_types::EnumBacking::Int),
            Ty::Enum(EnumRepr::Int),
        ),
        (
            CheckedTy::Shape(vec![nvs_types::ty::ShapeField {
                name: "a".to_owned(),
                ty: int,
                required: true,
            }]),
            Ty::Object,
        ),
        (CheckedTy::Union(vec![int, string]), Ty::Tagged),
        (CheckedTy::Intersection(vec![class, other]), Ty::Object),
        (CheckedTy::TypeVar("T".to_owned()), Ty::Tagged),
    ];
    assert_eq!(
        rows.len(),
        37,
        "one row per `nvs_types::ty::Ty` variant a value can have — every variant but \
         `CoreShape`, which the assertion below owns"
    );
    for (ty, want) in rows {
        let id = types.intern(ty.clone());
        assert_eq!(erase_checked_ty(id, &types), want, "`{ty:?}` erased wrong");
    }

    let bag = types.intern(CheckedTy::CoreShape(nvs_types::ty::CoreShape {
        fields: Vec::new(),
        arms: vec![Vec::new()],
    }));
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let erased = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        erase_checked_ty(bag, &types)
    }));
    std::panic::set_hook(previous);
    assert!(
        erased.is_err(),
        "a `Core` options bag is not one value, so its arm asserts rather than erasing — \
         `Lowering::lower_fixed_arg` flattens it a slot at a time before this is reached"
    );
}

/// `rule:packaging/extension-calls-are-statically-typed`: a static call into a loaded extension
/// is one `InstKind::ExtensionCall` naming the export, with its arguments lowered against the
/// manifest's types, and never an `InstKind::Call` of a label no compiled function has. The set
/// is the conformance fixture `Shop\Ledger`'s, the one the `.nvst` cases load.
#[test]
fn an_extension_static_call_lowers_to_a_direct_call_of_its_export_trampoline() {
    let path = nvs_repo::path("tests/conformance/ext/fixtures/ledger/manifest.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    let set = [std::sync::Arc::new(
        nvs_ext::manifest::Manifest::parse(&bytes).expect("the fixture's manifest reads"),
    )];
    let src = "<?nvs\n\
               var $n = Shop\\Ledger::echoInt(40 + 2);\n\
               echo Shop\\Ledger::echoString(\"x\"), \"\\n\";\n\
               var $maybe = Shop\\Ledger::echoOptional();\n";
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    let classes = nvs_types::ext_lib::hir_classes(&set);
    let module =
        nvs_hir::resolve_file_with_extensions(&stmts, map.file(file), &classes, &mut diags);
    let mut checked_types = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    let files = [nvs_types::ProgramFile {
        src: map.file(file),
        stmts: &stmts,
    }];
    let enums = nvs_types::check::check_program_loaded(
        &files,
        &module,
        None,
        nvs_types::Backtracking::Allow,
        &nvs_types::Grants::default(),
        None,
        &set,
        &mut checked_types,
        &mut exprs,
        &mut diags,
    );
    assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
    let f = lower_script(
        "<script>",
        &stmts,
        map.file(file),
        &exprs,
        &checked_types,
        &enums,
        ScriptRole::Entry,
    )
    .function;

    let insts: Vec<&crate::ir::Inst> = f.blocks.iter().flat_map(|b| &b.insts).collect();
    let calls: Vec<(&str, &str, usize, Option<Ty>)> = insts
        .iter()
        .filter_map(|inst| match &inst.kind {
            InstKind::ExtensionCall {
                class,
                method,
                args,
            } => Some((class.as_str(), method.as_str(), args.len(), inst.ty)),
            _ => None,
        })
        .collect();
    assert_eq!(
        calls,
        vec![
            ("Shop\\Ledger", "echoInt", 1, Some(Ty::Int)),
            ("Shop\\Ledger", "echoString", 1, Some(Ty::Str)),
            // An omitted parameter is filled from the manifest's default, as for any call.
            ("Shop\\Ledger", "echoOptional", 1, Some(Ty::Tagged)),
        ],
        "one trampoline call per extension call, in source order"
    );
    assert!(
        insts
            .iter()
            .all(|inst| !matches!(inst.kind, InstKind::Call { .. })),
        "no extension call is an ordinary call of a compiled function"
    );
    assert!(
        insts
            .iter()
            .all(|inst| !matches!(inst.kind, InstKind::ExtensionCall { .. })
                || inst.on_error.is_some()),
        "a trampoline call can throw, so it has an error edge"
    );
}

/// `rule:types/property-key`'s written-out operand under `as ?property<T>`.
/// The checker decided the name where it is written, so the lowering emits
/// the free `Ty::Str` row with the `?T` tag put on: no membership chain and
/// no null branch. A computed name beside it still gets the chain, which is
/// what the two counts compare.
#[test]
fn a_written_name_into_a_nullable_property_key_is_tagged_and_tests_nothing() {
    let (written, map, file) = lower_script_src(
        "<?nvs
class Profile { public string $city = \"Vienna\"; }
?property<Profile> $key = \"city\" as ?property<Profile>;
",
    );
    let written = print_function(&written, map.file(file));
    let (computed, map, file) = lower_script_src(
        "<?nvs
class Profile { public string $city = \"Vienna\"; }
string $name = \"ci\" . \"ty\";
?property<Profile> $key = $name as ?property<Profile>;
",
    );
    let computed = print_function(&computed, map.file(file));
    let phis = |text: &str| text.lines().filter(|l| l.contains(" phi ")).count();
    assert_eq!(phis(&written), 0, "{written}");
    assert!(phis(&computed) > 0, "{computed}");
}

/// `rule:testing/tests-never-reach-a-build`: the program `nvs run` lowers has no function and no
/// method-table row for a `#[Test]`, a `#[Fixture]` or a class hook, and keeps every other method
/// of the class that declares them. The program `nvs test` lowers has all of them.
#[test]
fn a_run_lowers_no_test_code_and_a_test_run_lowers_all_of_it() {
    let src = "<?nvs
use Core\\Test;
use Core\\Test\\Fixture;
use Core\\Test\\AfterEachTest;

final class Cart {
    public function total(): int { return 0; }
}

final class CartTest {
    #[Fixture]
    public static function cart(): Cart { return new Cart(); }

    #[Test]
    public function anEmptyCartCostsNothing(Cart $cart): void {
        Test::assertSame(0, $this->expected());
    }

    #[AfterEachTest]
    public function clean(): void {}

    private function expected(): int { return 0; }
}
";
    let mut map = SourceMap::new();
    let file = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(file), &mut diags);
    let module = nvs_hir::resolve_file(&stmts, map.file(file), &mut diags);
    let mut checked_types = TypeInterner::new();
    let mut exprs = ExprTypeTable::new();
    let files = [nvs_types::ProgramFile {
        src: map.file(file),
        stmts: &stmts,
    }];
    let enums =
        nvs_types::check_program(&files, &module, &mut checked_types, &mut exprs, &mut diags);
    assert!(!diags.has_errors(), "fixture failed to check: {diags:?}");
    let layouts = nvs_types::layout::build_class_layouts(&files, &module.graph);
    let lower = |tests| {
        lower_program_as(
            "<script>",
            &files,
            &exprs,
            &checked_types,
            &enums,
            &layouts,
            tests,
        )
    };
    let test_code = [
        "CartTest::cart",
        "CartTest::anEmptyCartCostsNothing",
        "CartTest::clean",
    ];
    let kept = ["Cart::total", "CartTest::expected"];
    let names = |program: &crate::ir::Program| -> Vec<String> {
        program.functions.iter().map(|f| f.name.clone()).collect()
    };
    let rows = |program: &crate::ir::Program| -> Vec<String> {
        let class = program
            .classes
            .iter()
            .find(|c| c.label == "CartTest")
            .expect("the test class has a layout");
        class
            .methods
            .iter()
            .map(|(name, owner, ..)| format!("{owner}::{name}"))
            .collect()
    };

    let run = lower(Tests::Omit);
    for label in test_code {
        assert!(
            !names(&run).iter().any(|n| n == label),
            "{label} was lowered"
        );
        assert!(!rows(&run).iter().any(|n| n == label), "{label} has a row");
    }
    for label in kept {
        assert!(names(&run).iter().any(|n| n == label), "{label} is missing");
    }

    let test = lower(Tests::Lower);
    for label in test_code.iter().chain(&kept) {
        assert!(
            names(&test).iter().any(|n| n == label),
            "{label} is missing"
        );
    }
    for label in test_code {
        assert!(rows(&test).iter().any(|n| n == label), "{label} has no row");
    }
}

/// A bare `%d` over an `int` joins as an integer part, so the join writes its
/// digits and no string of digits is made, also when it is the whole template.
#[test]
fn a_bare_integer_placeholder_joins_as_an_integer_part() {
    let (f, map, file) = lower_script_src(
        "<?nvs
int $i = 7;
echo Core\\Str::format('Teapot %d & lid', $i);
echo Core\\Str::format('%d', $i);
",
    );
    let text = print_function(&f, map.file(file));
    assert!(!text.contains("helper.int_to_string"), "{text}");
    assert!(text.contains("concat v1, int v0, v2"), "{text}");
    assert!(text.contains("= concat int v0  ;"), "{text}");
}

/// How many `Retain` instructions the first method of class `T` in `src`
/// lowers to.
fn retains_in(src: &str) -> usize {
    let (f, _, _) = lower_first_method(src);
    f.blocks
        .iter()
        .flat_map(|b| &b.insts)
        .filter(|i| matches!(i.kind, InstKind::Retain { .. }))
        .count()
}

/// `rule:expressions/an-operand-keeps-its-value-while-later-operands-run`'s
/// free path: a borrowed read followed only by plain property reads, or by a
/// call that writes no local it reads, takes no reference of its own. The one
/// retain in each is the leading piece's, which `InstKind::Concat` consumes,
/// and it does not grow with the operand count. A later call that can
/// replace the property is the contrast, and it adds exactly one.
#[test]
fn a_read_followed_only_by_literals_and_locals_takes_no_reference() {
    let class = |body: &str| {
        format!(
            "<?nvs\nclass T {{\n  public string $a = \"x\";\n  public string $b = \"y\";\n  \
             function m(string $s, string $x): string {{\n    return {body};\n  }}\n  \
             static function f(string $v): string {{\n    return $v;\n  }}\n}}\n"
        )
    };
    for body in [
        "$this->a . $this->b",
        "$this->a . $this->b . $this->a . \"!\"",
    ] {
        assert_eq!(retains_in(&class(body)), 1, "{body}");
    }
    // A literal-led concatenation retains nothing for its leading piece, so
    // the local adds exactly the one retain `Concat` consumes. The call's own
    // retains are the same on both sides.
    for calls in ["T::f($x)", "T::f($x) . T::f($x)"] {
        let led_by_local = retains_in(&class(&format!("$s . {calls}")));
        let led_by_literal = retains_in(&class(&format!("\"k\" . {calls}")));
        assert_eq!(led_by_local, led_by_literal + 1, "$s . {calls}");
    }
    // Both property reads come before a call that can replace them, so each
    // is held: one retain each, beyond what the call itself costs.
    assert_eq!(
        retains_in(&class("$this->b . $this->a . T::f($x)")),
        retains_in(&class("\"k\" . T::f($x)")) + 2,
    );
}

/// `rule:expressions/an-operand-keeps-its-value-while-later-operands-run`'s
/// one judgment: no lowering module judges a borrow by itself. Each one calls
/// `Lowering::sole_operand_borrowed`, `Lowering::borrowed_operand` or
/// `Lowering::borrowed_arg`, so a consumer that reads operands after later
/// ones has to name what follows them. Privacy already keeps a call out of
/// another module; this test also keeps the method private.
#[test]
fn every_borrow_judgment_passes_the_one_operand_lowering() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lower");
    let mut read = 0;
    let mut outside = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("src/lower is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("a lowering source file is UTF-8");
        read += 1;
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned());
        let file = file.unwrap_or_default();
        if file == "operand.rs" {
            assert!(
                src.contains("\n    fn aliasing_read(&self, e: &Expr) -> bool {"),
                "`aliasing_read` must stay a private method of operand.rs"
            );
            continue;
        }
        // This file names the method in the assertion above, so it is
        // skipped here.
        if file == "tests.rs" {
            continue;
        }
        for (n, line) in src.lines().enumerate() {
            if line.contains(".aliasing_read(") || line.contains("fn aliasing_read(") {
                outside.push(format!("{file}:{}", n + 1));
            }
        }
    }
    assert!(
        read > 10,
        "found only {read} lowering source files under {dir:?}"
    );
    assert!(
        outside.is_empty(),
        "a borrow judged outside the one operand lowering: {outside:?}"
    );
}

/// One axis of [`every_replacement_form_has_a_case`]: a row's slug, as the
/// case's file name spells it.
type Axis = &'static [&'static str];

/// Where the replaced value lives: every storage the grammar lets a statement
/// both read and assign.
const REPLACED_TARGETS: Axis = &[
    "local",
    "parameter",
    "inout",
    "captured",
    "this-property",
    "local-property",
    "static-property",
    "element",
    "property-element",
    "property-chain",
];

/// The targets a callee can reach without being handed them: storage on an
/// object or a class. A callee that reads or writes the slot needs one.
const REACHABLE_TARGETS: Axis = &[
    "this-property",
    "local-property",
    "static-property",
    "property-element",
    "property-chain",
];

/// What makes the new value from the old: an owning `Core` member with and
/// without a callback, a Novis function by what it does around its change,
/// and the three operators that open with their own target.
const REPLACING_VALUES: Axis = &[
    "core",
    "core-callback",
    "fn-safe",
    "fn-fails",
    "fn-reads-before",
    "fn-reads-after",
    "fn-writes",
    "concat",
    "interpolation",
    "spread",
];

/// Where the statement stands: each changes which paths read the target
/// after the statement.
const REPLACING_CONTEXTS: Axis = &["straight", "loop", "try", "finally", "generator"];

/// The two kinds of value a replacement changes in place.
const REPLACED_KINDS: Axis = &["array", "string"];

/// Whether the grammar and the type rules allow the cell, and so whether it
/// is owed a case.
///
/// - `.` and an interpolation make a `string`, and a spread makes an `array`.
/// - A callee reads or writes only a slot it can reach, so the three `fn-`
///   values that touch the slot need a target on an object or a class.
/// - A generator takes no `inout` parameter
///   (`rule:statements/inout-is-the-by-reference-spelling`).
/// - A hole in a string opens with `{$`, so an interpolation cannot open with
///   a static property such as `{Store::$text}`.
fn replacement_is_written(target: &str, value: &str, context: &str, kind: &str) -> bool {
    let kind_fits = match value {
        "concat" | "interpolation" => kind == "string",
        "spread" => kind == "array",
        _ => true,
    };
    let reaches = !matches!(value, "fn-reads-before" | "fn-reads-after" | "fn-writes")
        || REACHABLE_TARGETS.contains(&target);
    let interpolates = !(value == "interpolation" && target == "static-property");
    kind_fits && reaches && interpolates && !(target == "inout" && context == "generator")
}

/// Every form of `target = value(target)` the language allows has a
/// conformance case under `tests/conformance/in-place/`, named
/// `<target>-<value>-<context>-<kind>.nvst`
/// (`rule:types/a-replaced-value-is-changed-in-place`).
///
/// The matrix is the product of the axes above, less the cells
/// [`replacement_is_written`] rules out. Each case prints the target after
/// the statement, and in a `try` after an error too, so the differential
/// run of `bun nv suite --no-in-place-too` compares every form both ways.
/// The test fails naming each missing case, and each file in the directory
/// that names no cell of the matrix.
#[test]
fn every_replacement_form_has_a_case() {
    let dir = nvs_repo::path("tests/conformance/in-place");
    let mut owed = std::collections::BTreeSet::new();
    for target in REPLACED_TARGETS {
        for value in REPLACING_VALUES {
            for context in REPLACING_CONTEXTS {
                for kind in REPLACED_KINDS {
                    if replacement_is_written(target, value, context, kind) {
                        owed.insert(format!("{target}-{value}-{context}-{kind}.nvst"));
                    }
                }
            }
        }
    }
    let written: std::collections::BTreeSet<String> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.ends_with(".nvst"))
                .collect()
        })
        .unwrap_or_default();
    let missing: Vec<&String> = owed.difference(&written).collect();
    // A case the matrix does not name is allowed only when it is not shaped
    // like a cell: the attack case of stage 8 lives here too.
    let stray: Vec<&String> = written
        .difference(&owed)
        .filter(|name| {
            REPLACED_TARGETS
                .iter()
                .any(|target| name.starts_with(&format!("{target}-")))
                && REPLACED_KINDS
                    .iter()
                    .any(|kind| name.ends_with(&format!("-{kind}.nvst")))
        })
        .collect();
    assert!(
        missing.is_empty() && stray.is_empty(),
        "{} of {} replacement forms have no case under {}, and {} case(s) name no form.\n\
         missing:\n  {}\nnaming no form:\n  {}",
        missing.len(),
        owed.len(),
        dir.display(),
        stray.len(),
        missing
            .iter()
            .map(|name| name.as_str())
            .collect::<Vec<_>>()
            .join("\n  "),
        stray
            .iter()
            .map(|name| name.as_str())
            .collect::<Vec<_>>()
            .join("\n  "),
    );
}

/// Every row of the table of cheaper forms has a conformance case for each
/// outcome it can have, under `tests/conformance/forms/<folder>/`, named
/// `<row id>-<outcome>.nvst`
/// (`rule:expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named`).
///
/// [`crate::forms::Row::cases`] says what each row owes. Each case prints
/// what the form returns, so the differential run of
/// `bun nv suite tests/conformance/ --no-cheaper-forms-too` compares every row
/// both ways. The test fails naming each missing case, and each case in a
/// folder that starts with a row's id and names none of its outcomes.
#[test]
fn every_form_row_has_its_cases() {
    let root = nvs_repo::path(crate::forms::CASES_DIR);
    let mut missing = Vec::new();
    let mut stray = Vec::new();
    let mut owed_count = 0;
    for folder in [
        crate::forms::Folder::Calls,
        crate::forms::Folder::Loops,
        crate::forms::Folder::Callbacks,
    ] {
        let dir = root.join(folder.name());
        let written: std::collections::BTreeSet<String> = std::fs::read_dir(&dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .filter(|name| name.ends_with(".nvst"))
                    .collect()
            })
            .unwrap_or_default();
        let rows: Vec<&crate::forms::Row> = crate::forms::ROWS
            .iter()
            .filter(|row| row.folder == folder)
            .collect();
        let owed: std::collections::BTreeSet<String> = rows
            .iter()
            .flat_map(|row| row.cases())
            .map(|case| case.file)
            .collect();
        owed_count += owed.len();
        missing.extend(
            owed.difference(&written)
                .map(|file| format!("{}/{file}", folder.name())),
        );
        // A case no row owes is allowed only when it starts with no row's id.
        stray.extend(
            written
                .difference(&owed)
                .filter(|name| {
                    crate::forms::ROWS
                        .iter()
                        .any(|row| name.starts_with(&format!("{}-", row.id)))
                })
                .map(|file| format!("{}/{file}", folder.name())),
        );
    }
    assert!(
        missing.is_empty() && stray.is_empty(),
        "{} of {owed_count} cases the table of cheaper forms owes are not under {}, and {} case(s) \
         name no outcome of their row.\nmissing:\n  {}\nnaming no outcome:\n  {}",
        missing.len(),
        root.display(),
        stray.len(),
        missing.join("\n  "),
        stray.join("\n  "),
    );
}

/// The symbol of every `Core` call `src` lowers to, with the table of cheaper
/// forms on and then off, in the order the functions and their blocks list them.
fn core_calls_both_ways(src: &str) -> (Vec<&'static str>, Vec<&'static str>) {
    fn symbols(program: &crate::ir::Program) -> Vec<&'static str> {
        program
            .functions
            .iter()
            .flat_map(|f| f.blocks.iter())
            .flat_map(|block| block.insts.iter())
            .filter_map(|inst| match inst.kind {
                InstKind::CoreCall { symbol, .. } => Some(symbol),
                _ => None,
            })
            .collect()
    }
    let on = symbols(&lower_whole_file_with_src(src).0);
    let off = crate::lower::without_cheaper_forms(|| symbols(&lower_whole_file_with_src(src).0));
    (on, off)
}

/// How many times `symbol` is among `calls`.
fn count_of(calls: &[&str], symbol: &str) -> usize {
    calls.iter().filter(|call| **call == symbol).count()
}

/// Row `match-null-test`: `== null`, `!= null` and `is null` of a `match`
/// call each compile as `matches`, and the call with a `from` option stays a
/// `match`. With the table off, all four are `match`.
#[test]
fn a_null_test_of_a_match_compiles_as_matches() {
    let src = "<?nvs\nfinal class T {\n  public static function m(string $s, string $p): void {\n    \
               echo Core\\Regex::match($s, $p) == null ? 1 : 0;\n    \
               echo Core\\Regex::match($s, $p) != null ? 1 : 0;\n    \
               echo Core\\Regex::match($s, $p) is null ? 1 : 0;\n    \
               echo Core\\Regex::match($s, $p, {from: 2}) == null ? 1 : 0;\n  }\n}\n";
    let (on, off) = core_calls_both_ways(src);
    assert_eq!(count_of(&on, "nvs_core_regex_matches"), 3, "{on:?}");
    assert_eq!(count_of(&on, "nvs_core_regex_match"), 1, "{on:?}");
    assert_eq!(count_of(&off, "nvs_core_regex_matches"), 0, "{off:?}");
    assert_eq!(count_of(&off, "nvs_core_regex_match"), 4, "{off:?}");
}

/// Rows `count-of-keys-or-values` and `first-of-keys-or-values`: no `keys` or
/// `values` array is built. `count` reads the array itself, `first` of the keys
/// is `firstKey`, and `first` of the values is `first`.
#[test]
fn a_count_or_first_of_keys_or_values_reads_the_array_itself() {
    let src = "<?nvs\nfinal class T {\n  public static function m(array<int> $a): void {\n    \
               echo Core\\Arr::count(Core\\Arr::keys($a));\n    \
               echo Core\\Arr::count(Core\\Arr::values($a));\n    \
               echo Core\\Arr::first(Core\\Arr::keys($a)) ?? \"\";\n    \
               echo Core\\Arr::first(Core\\Arr::values($a)) ?? 0;\n  }\n}\n";
    let (on, off) = core_calls_both_ways(src);
    assert_eq!(
        on,
        [
            "nvs_core_arr_count",
            "nvs_core_arr_count",
            "nvs_core_arr_first_key",
            "nvs_core_arr_first",
        ],
    );
    assert_eq!(count_of(&off, "nvs_core_arr_keys"), 2, "{off:?}");
    // A `values` whose subject is sole is its owning variant.
    let values = off
        .iter()
        .filter(|call| call.starts_with("nvs_core_arr_values"))
        .count();
    assert_eq!(values, 2, "{off:?}");
    assert_eq!(count_of(&off, "nvs_core_arr_first_key"), 0, "{off:?}");
}

/// Rows `caught-conversion` and `caught-parse`: a `try` that converts a
/// variable and assigns `null` in its one `catch` arm takes no thrown value at
/// all. `as int` is the conversion that returns `null`, and `Core\Uuid::parse`
/// is `tryParse`. A `try` with a `finally` keeps its region.
#[test]
fn a_caught_conversion_that_assigns_null_compiles_as_the_nullable_conversion() {
    fn taken(program: &crate::ir::Program) -> usize {
        program
            .functions
            .iter()
            .flat_map(|f| f.blocks.iter())
            .flat_map(|block| block.insts.iter())
            .filter(|inst| matches!(inst.kind, InstKind::TakeThrown))
            .count()
    }
    let src = "<?nvs\nfinal class T {\n  public static function m(mixed $x, string $s): void {\n    \
               ?int $v = 0;\n    \
               try {\n      $v = $x as int;\n    } catch (RuntimeError) {\n      $v = null;\n    }\n    \
               ?Core\\Uuid $u = null;\n    \
               try {\n      $u = Core\\Uuid::parse($s);\n    } catch (Throwable) {\n      $u = null;\n    }\n    \
               echo $v ?? 0, $u?->toString() ?? \"\";\n  }\n}\n";
    let (on, map, file) = lower_whole_file_with_src(src);
    let off = crate::lower::without_cheaper_forms(|| lower_whole_file_with_src(src).0);
    assert_eq!(taken(&on), 0, "{}", print_program(&on, map.file(file)));
    assert_eq!(taken(&off), 2);
    let (on_calls, off_calls) = core_calls_both_ways(src);
    assert_eq!(
        count_of(&on_calls, "nvs_core_uuid_try_parse"),
        1,
        "{on_calls:?}"
    );
    assert_eq!(
        count_of(&on_calls, "nvs_core_uuid_parse"),
        0,
        "{on_calls:?}"
    );
    assert_eq!(
        count_of(&off_calls, "nvs_core_uuid_parse"),
        1,
        "{off_calls:?}"
    );

    let finally = "<?nvs\nfinal class T {\n  public static function m(mixed $x): void {\n    \
                   ?int $v = 0;\n    \
                   try {\n      $v = $x as int;\n    } catch (RuntimeError) {\n      $v = null;\n    \
                   } finally {\n      echo 1;\n    }\n    echo $v ?? 0;\n  }\n}\n";
    assert!(taken(&lower_whole_file_with_src(finally).0) > 0);
}

/// Row `filter-contains`: a `filter` whose callback tests `contains` against a
/// captured array is `difference` where the test is negated and `intersection` where it
/// is not, in an arrow body and in a block of one `return`. A callback over
/// array elements, and one that tests a literal array, stay a `filter`. With
/// the table off, all five are `filter`.
#[test]
fn a_difference_written_as_filter_and_contains_compiles_as_diff() {
    let src = "<?nvs\nfinal class T {\n  \
               public static function m(array<int|float> $a, array<int|float> $b, \
               array<array<int>> $c, array<array<int>> $d): void {\n    \
               echo Core\\Arr::count(Core\\Arr::filter($a, fn(int|float $x): bool => \
               !Core\\Arr::contains($b, $x)));\n    \
               echo Core\\Arr::count(Core\\Arr::filter($a, fn(int|float $x): bool => { \
               return Core\\Arr::contains($b, $x); }));\n    \
               echo Core\\Arr::count(Core\\Arr::filter($c, fn(array<int> $x): bool => \
               !Core\\Arr::contains($d, $x)));\n    \
               echo Core\\Arr::count(Core\\Arr::filter($a, fn(int|float $x): bool => \
               !Core\\Arr::contains([1, 2], $x)));\n    \
               echo Core\\Arr::count(Core\\Arr::filter($a, fn(int|float $x): bool => \
               $x > 1));\n  }\n}\n";
    // A `filter` whose subject is sole is its owning variant.
    fn filters(calls: &[&str]) -> usize {
        calls
            .iter()
            .filter(|call| call.starts_with("nvs_core_arr_filter"))
            .count()
    }
    let (on, off) = core_calls_both_ways(src);
    assert_eq!(count_of(&on, "nvs_core_arr_difference"), 1, "{on:?}");
    assert_eq!(count_of(&on, "nvs_core_arr_intersection"), 1, "{on:?}");
    assert_eq!(filters(&on), 3, "{on:?}");
    assert_eq!(filters(&off), 5, "{off:?}");
    assert_eq!(count_of(&off, "nvs_core_arr_difference"), 0, "{off:?}");
    assert_eq!(count_of(&off, "nvs_core_arr_intersection"), 0, "{off:?}");
}

/// Rows `first-of-filter`, `first-key-of-filter` and `any-of-filter`: a
/// callback that writes nothing and cannot fail makes `first`, `firstKey`,
/// `isEmpty` and a count compared with `0` of a filter the fast member, and no
/// filter is left. With the table off, all four are filters.
#[test]
fn a_proven_callback_form_compiles_as_its_fast_member() {
    let src = "<?nvs\nfinal class T {\n  public static function m(array<int> $a): void {\n    \
               echo Core\\Arr::first(Core\\Arr::filter($a, fn(int $x): bool => $x > 2)) ?? 0;\n    \
               echo Core\\Arr::firstKey(Core\\Arr::filter($a, fn(int $x): bool => $x > 2)) ?? \"\";\n    \
               echo Core\\Arr::isEmpty(Core\\Arr::filter($a, fn(int $x): bool => $x > 2)) ? 1 : 0;\n    \
               echo Core\\Arr::count(Core\\Arr::filter($a, fn(int $x): bool => $x > 2)) > 0 ? 1 : 0;\n    \
               echo Core\\Arr::count(Core\\Arr::filter($a, fn(int $x): bool => $x > 2)) == 0 ? 1 : 0;\n  }\n}\n";
    let (on, off) = core_calls_both_ways(src);
    assert_eq!(filters_in(&on), 0, "{on:?}");
    assert_eq!(count_of(&on, "nvs_core_arr_find"), 1, "{on:?}");
    assert_eq!(count_of(&on, "nvs_core_arr_find_key"), 1, "{on:?}");
    assert_eq!(count_of(&on, "nvs_core_arr_any"), 3, "{on:?}");
    assert_eq!(filters_in(&off), 5, "{off:?}");
    assert_eq!(count_of(&off, "nvs_core_arr_any"), 0, "{off:?}");
    assert!(lower_whole_file_with_src(src).0.forms.is_empty());
}

/// A callback that writes nothing but can fail keeps its form, because the
/// fast member would call it for fewer entries. The form is recorded with its
/// fast form as the program would write it.
#[test]
fn a_callback_that_can_fail_keeps_the_form_and_is_recorded() {
    let src = "<?nvs\nfinal class T {\n  public static function m(array<int> $a): void {\n    \
               echo Core\\Arr::first(Core\\Arr::filter($a, fn(int $x): bool => $x + 1 > 2)) ?? 0;\n    \
               echo Core\\Arr::isEmpty(Core\\Arr::filter($a, fn(int $x): bool => $x + 1 > 2)) ? 1 : 0;\n  }\n}\n";
    let (on, _) = core_calls_both_ways(src);
    assert_eq!(filters_in(&on), 2, "{on:?}");
    assert_eq!(count_of(&on, "nvs_core_arr_find"), 0, "{on:?}");
    let forms = lower_whole_file_with_src(src).0.forms;
    let rows: Vec<(&str, &str)> = forms.iter().map(|f| (f.row, f.fast.as_str())).collect();
    assert_eq!(
        rows,
        [
            (
                "first-of-filter",
                "Core\\Arr::find($a, fn(int $x): bool => $x + 1 > 2)"
            ),
            (
                "any-of-filter",
                "!Core\\Arr::any($a, fn(int $x): bool => $x + 1 > 2)"
            ),
        ]
    );
}

/// A second read of the same value is a `W1035` on the second call, and only
/// where the first call runs whenever its statement does and nothing between
/// them can change the answer.
#[test]
fn a_second_read_of_the_same_value_warns_only_in_straight_code() {
    let src = "<?nvs
final class Shop {
  private static function other(): void {}
  public static function cache(Core\\Cache\\Store $s): void {
    var $a = $s->get(\"k\");
    echo \"between\";
    var $b = $s->get(\"k\");
    var $c = $s->get(\"k\");
  }
  public static function query(Core\\Uri $u): void {
    var $a = $u->queryParameter(\"a\");
    var $b = $u->queryParameter(\"b\");
  }
  public static function file(string $p): void {
    var $a = Core\\IO::isFile($p);
    var $b = Core\\IO::size($p);
  }
  public static function branch(Core\\Cache\\Store $s, bool $f): void {
    if ($f) {
      $s->get(\"k\");
    }
    $s->get(\"k\");
  }
  public static function and(string $p, bool $f): void {
    var $a = $f && Core\\IO::isFile($p);
    var $b = Core\\IO::isFile($p);
  }
  public static function assigned(string $q): void {
    string $p = $q;
    var $a = Core\\IO::isDirectory($p);
    $p = \"other\";
    var $b = Core\\IO::isDirectory($p);
  }
  public static function novis(Core\\Uri $u): void {
    var $a = $u->queryParameter(\"a\");
    self::other();
    var $b = $u->queryParameter(\"b\");
  }
  public static function callable(Core\\Cache\\Store $s, callable $f): void {
    var $a = $s->get(\"k\");
    $f();
    var $b = $s->get(\"k\");
  }
  public static function keys(Core\\Cache\\Store $s): void {
    var $a = $s->get(\"k\");
    var $b = $s->get(\"j\");
  }
  public static function callback(Core\\Cache\\Store $s, array<int> $l, callable(int): int $f): void {
    var $a = $s->get(\"k\");
    var $m = Core\\Arr::map($l, $f);
    var $b = $s->get(\"k\");
  }
  public static function handle(string $p, Core\\IO\\File $h): void {
    var $a = Core\\IO::size($p);
    $h->truncate(8);
    var $b = Core\\IO::size($p);
  }
  public static function path(string $p): void {
    var $a = Core\\IO::size($p);
    var $j = Core\\Path::join($p, \"x\");
    var $b = Core\\IO::size($p);
  }
}
";
    let program = lower_whole_file_with_src(src).0;
    let warnings = crate::rereads::warnings(&program.rereads);
    let line_of = |span: nvs_diagnostics::Span| src[..span.start as usize].lines().count();
    let found: Vec<_> = warnings
        .iter()
        .map(|w| (w.code, line_of(w.primary_span().expect("a primary label"))))
        .collect();
    let w = Some(nvs_diagnostics::code::W_REPEATED_READ);
    assert_eq!(
        found,
        vec![(w, 7), (w, 8), (w, 12), (w, 16), (w, 61)],
        "{warnings:#?}"
    );
    assert!(
        warnings[0].message.contains("cache key `k`"),
        "{warnings:#?}"
    );
    assert!(warnings[2].message.contains("query"), "{warnings:#?}");
    assert!(warnings[3].message.contains("`$p`"), "{warnings:#?}");
}

/// The line and message of each `W1036` of `src`.
fn hand_cuts_of(src: &str) -> Vec<(usize, String)> {
    let program = lower_whole_file_with_src(src).0;
    crate::hand_cuts::warnings(&program.hand_cuts)
        .iter()
        .map(|w| {
            assert_eq!(w.code, Some(nvs_diagnostics::code::W_HAND_CUT));
            let span = w.primary_span().expect("a primary label");
            (
                src[..span.start as usize].lines().count(),
                w.message.clone(),
            )
        })
        .collect()
}

/// A cut written by hand is a `W1036` on the cut in each shape the pass
/// knows: inside the test's `if`, after an early exit, a suffix cut with
/// `-N` and with `length(S) - N`, and a literal `N`.
#[test]
fn hand_cuts_warn_in_each_shape() {
    let src = "<?nvs
final class Shop {
  public static function then(string $s): string {
    if (Core\\Str::startsWith($s, 'ab')) {
      echo 'x';
      return Core\\Str::slice($s, Core\\Str::length('ab') as int);
    }
    return $s;
  }
  public static function early(string $s, string $p): string {
    if (!Core\\Str::startsWith($s, $p)) {
      return '';
    }
    return Core\\Str::slice($s, Core\\Str::length($p) as int);
  }
  public static function negative(array<string> $l): void {
    foreach ($l as string $s) {
      if (!Core\\Str::endsWith($s, '.nvs') || $s == '') {
        continue;
      }
      echo Core\\Str::slice($s, 0, -4);
    }
  }
  public static function minus(string $s, string $p): string {
    if (Core\\Str::endsWith($s, $p) && $p != '') {
      return Core\\Str::slice($s, 0, (Core\\Str::length($s) as int) - (Core\\Str::length($p) as int));
    }
    return $s;
  }
  public static function literal(string $s): string {
    if (Core\\Str::startsWith($s, 'max=')) {
      return Core\\Str::slice($s, 4);
    }
    return $s;
  }
}
";
    let found = hand_cuts_of(src);
    let lines: Vec<usize> = found.iter().map(|(line, _)| *line).collect();
    assert_eq!(lines, vec![6, 14, 21, 26, 32], "{found:#?}");
    assert_eq!(
        found[0].1,
        "`Core\\Str::afterPrefix($s, 'ab')` does this test and this cut in one call"
    );
    assert!(
        found[2].1.contains("beforeSuffix($s, '.nvs')"),
        "{found:#?}"
    );
    assert!(found[3].1.contains("beforeSuffix($s, $p)"), "{found:#?}");
}

/// The pass says nothing where it is unsure: the subject written between
/// the test and the cut, a length that is not the prefix's, an early exit
/// with an `else`, and a prefix that is a call.
#[test]
fn hand_cuts_are_silent_where_unsure() {
    let src = "<?nvs
final class Shop {
  public static function assigned(string $s): string {
    string $t = $s;
    if (Core\\Str::startsWith($t, 'ab')) {
      $t = 'other';
      return Core\\Str::slice($t, 2);
    }
    return $t;
  }
  public static function wrong(string $s): string {
    if (Core\\Str::startsWith($s, 'ab')) {
      return Core\\Str::slice($s, 3);
    }
    return $s;
  }
  public static function otherwise(string $s): string {
    if (!Core\\Str::startsWith($s, 'ab')) {
      echo 'no';
    } else {
      echo 'yes';
    }
    return Core\\Str::slice($s, 2);
  }
  public static function called(string $s): string {
    if (Core\\Str::startsWith($s, Core\\Str::lower('AB'))) {
      return Core\\Str::slice($s, 2);
    }
    return $s;
  }
}
";
    assert_eq!(hand_cuts_of(src), Vec::new());
}

/// The exact shape gets one fix, an alternative: the declaration moves
/// before the `if` as one `afterPrefix` call, and the condition tests it
/// for `null`. The fixed program compiles and has no `W1036`. Any other
/// shape warns with no fix.
#[test]
fn hand_cuts_fix_only_the_exact_shape() {
    let src = "<?nvs
final class Shop {
  public static function m(string $s): string {
    if (Core\\Str::startsWith($s, 'ab')) {
      string $rest = Core\\Str::slice($s, 2);
      return $rest;
    }
    return $s;
  }
  public static function n(string $s): string {
    if (Core\\Str::startsWith($s, 'ab')) {
      echo 'x';
      return Core\\Str::slice($s, 2);
    }
    return $s;
  }
}
";
    let program = lower_whole_file_with_src(src).0;
    let warnings = crate::hand_cuts::warnings(&program.hand_cuts);
    assert_eq!(warnings.len(), 2, "{warnings:#?}");
    assert!(warnings[1].suggestions.is_empty(), "{warnings:#?}");
    let [fix] = warnings[0].suggestions.as_slice() else {
        panic!("one fix: {warnings:#?}");
    };
    let fixed = format!(
        "{}{}{}",
        &src[..fix.span.start as usize],
        fix.replacement,
        &src[fix.span.end as usize..]
    );
    assert!(
        fixed.contains(
            "    ?string $rest = Core\\Str::afterPrefix($s, 'ab');\n    if ($rest != null) {\n      return $rest;\n    }\n"
        ),
        "{fixed}"
    );
    let refixed = lower_whole_file_with_src(&fixed).0;
    assert_eq!(refixed.hand_cuts.len(), 1, "only `n` is left: {fixed}");
}

/// A recorded form is a `W1034` over the written form. Its message names the
/// member to use, and its one fix is the fast form's text, marked an
/// alternative so no batch applies it.
#[test]
fn a_form_warning_carries_its_fast_form_as_an_alternative() {
    let src = "<?nvs\nfinal class T {\n  public static function m(array<int> $a): void {\n    \
               echo Core\\Arr::first(Core\\Arr::filter($a, fn(int $x): bool => $x + 1 > 2)) ?? 0;\n    \
               echo Core\\Arr::count(Core\\Arr::filter($a, fn(int $x): bool => $x + 1 > 2)) == 0 ? 1 : 0;\n  }\n}\n";
    let program = lower_whole_file_with_src(src).0;
    let warnings = crate::forms::warnings(&program.forms);
    assert_eq!(warnings.len(), 2, "{warnings:#?}");
    let first = &warnings[0];
    assert_eq!(first.code, Some(nvs_diagnostics::code::W_CHEAPER_FORM));
    assert!(first.message.contains("`Core\\Arr::find`"), "{first:#?}");
    let span = first.primary_span().expect("a primary label");
    assert_eq!(
        &src[span.start as usize..span.end as usize],
        "Core\\Arr::first(Core\\Arr::filter($a, fn(int $x): bool => $x + 1 > 2))"
    );
    let [fix] = first.suggestions.as_slice() else {
        panic!("one fix: {first:#?}");
    };
    assert!(fix.alternative && !fix.safe, "{fix:#?}");
    assert_eq!(fix.span, span);
    assert_eq!(
        fix.replacement,
        "Core\\Arr::find($a, fn(int $x): bool => $x + 1 > 2)"
    );
    let none = &warnings[1];
    assert!(none.message.contains("`Core\\Arr::any`"), "{none:#?}");
    let span = none.primary_span().expect("a primary label");
    assert_eq!(
        &src[span.start as usize..span.end as usize],
        "Core\\Arr::count(Core\\Arr::filter($a, fn(int $x): bool => $x + 1 > 2)) == 0"
    );
    assert_eq!(
        none.suggestions[0].replacement,
        "!Core\\Arr::any($a, fn(int $x): bool => $x + 1 > 2)"
    );
}

/// A callback that prints or writes a property may be called for every entry
/// on purpose: the form stays as written, and nothing is recorded.
#[test]
fn a_callback_with_an_effect_keeps_the_form_and_is_silent() {
    let src = "<?nvs\nfinal class T {\n  public static int $n = 0;\n  \
               public static function m(array<int> $a): void {\n    \
               echo Core\\Arr::first(Core\\Arr::filter($a, fn(int $x): bool => { echo $x; return $x > 2; })) ?? 0;\n    \
               echo Core\\Arr::firstKey(Core\\Arr::filter($a, fn(int $x): bool => { T::$n = $x; return $x > 2; })) ?? \"\";\n  }\n}\n";
    let (on, _) = core_calls_both_ways(src);
    assert_eq!(filters_in(&on), 2, "{on:?}");
    assert_eq!(count_of(&on, "nvs_core_arr_find"), 0, "{on:?}");
    assert_eq!(count_of(&on, "nvs_core_arr_find_key"), 0, "{on:?}");
    assert!(lower_whole_file_with_src(src).0.forms.is_empty());
}

/// Rows `comparator-by` and `comparator-by-desc`: a comparator that compares
/// one proven expression of each side is passed as `by`, so the call makes one
/// object of the `by` function's class. A comparator that can fail stays, and
/// is recorded with the `by` form. With the table off, no call takes `by`.
#[test]
fn a_comparator_of_one_expression_sorts_by_it() {
    let src = "<?nvs\nfinal class T {\n  \
               public static function m(array<string> $a, array<string> $b): void {\n    \
               echo Core\\Str::join(Core\\Arr::sort($a, {comparator: fn(string $x, string $y): int => Core\\Str::length($x) <=> Core\\Str::length($y)}), \",\");\n    \
               echo Core\\Str::join(Core\\Arr::difference($a, $b, {comparator: fn(string $x, string $y): int => Core\\Str::length($x) <=> Core\\Str::length($y)}), \",\");\n    \
               echo Core\\Str::join(Core\\Arr::sort($a, {comparator: fn(string $x, string $y): int => Core\\Str::length($y) <=> Core\\Str::length($x)}), \",\");\n    \
               echo Core\\Str::join(Core\\Arr::sort($a, {comparator: fn(string $x, string $y): int => ($x as int) <=> ($y as int)}), \",\");\n  }\n}\n";
    let by_objects = |program: &crate::ir::Program| {
        insts_where(
            program,
            |kind| matches!(kind, InstKind::New { class, .. } if class.ends_with("$by")),
        )
    };
    let on = lower_whole_file_with_src(src).0;
    let off = crate::lower::without_cheaper_forms(|| lower_whole_file_with_src(src).0);
    assert_eq!(by_objects(&on), 3);
    assert_eq!(by_objects(&off), 0);
    let rows: Vec<(&str, &str)> = on.forms.iter().map(|f| (f.row, f.fast.as_str())).collect();
    assert_eq!(
        rows,
        [(
            "comparator-by",
            "Core\\Arr::sort($a, {by: fn(string $x) => ($x as int)})"
        )]
    );
}

/// How many of `calls` are a `Core\Arr::filter`, its owning variant included.
fn filters_in(calls: &[&str]) -> usize {
    calls
        .iter()
        .filter(|call| call.starts_with("nvs_core_arr_filter"))
        .count()
}

/// How many instructions of `program` `pick` accepts, across every function.
fn insts_where(program: &crate::ir::Program, pick: impl Fn(&InstKind) -> bool) -> usize {
    program
        .functions
        .iter()
        .flat_map(|f| f.blocks.iter())
        .flat_map(|block| block.insts.iter())
        .filter(|inst| pick(&inst.kind))
        .count()
}

/// The `proven` mask of each `Core` call in `src` whose symbol starts with
/// `member`, in order.
fn proven_masks(src: &str, member: &str) -> Vec<u32> {
    lower_whole_file_with_src(src)
        .0
        .functions
        .iter()
        .flat_map(|f| f.blocks.iter())
        .flat_map(|block| block.insts.iter())
        .filter_map(|inst| match inst.kind {
            InstKind::CoreCall { symbol, proven, .. } if symbol.starts_with(member) => Some(proven),
            _ => None,
        })
        .collect()
}

/// An anonymous function whose parameters take what `Core\Arr::map` passes,
/// exactly as it passes it, is marked in its argument slot, so the member
/// calls it without checking each argument. One that needs `int` widened to
/// `float`, and a callable held in a variable, keep the check. An anonymous
/// function `Core\Arr::fill` stores is a value, not a callback, so it is never
/// marked.
#[test]
fn a_proven_callback_runs_without_the_check_of_each_call() {
    let src = "<?nvs\nfinal class T {\n  public static function m(array<int> $a, array<string> $s): void {\n    \
               var $b = Core\\Arr::map($a, fn(int $x): int => $x + 1);\n    \
               var $c = Core\\Arr::map($s, fn(string $v, string $k): string => $v . $k);\n    \
               var $d = Core\\Arr::map($a, fn(float $x): float => $x);\n    \
               var $f = fn(int $x): int => $x;\n    \
               var $e = Core\\Arr::map($a, $f);\n    \
               var $g = Core\\Arr::fill(2, fn(int $x): int => $x);\n  }\n}\n";
    assert_eq!(proven_masks(src, "nvs_core_arr_map"), [0b10, 0b10, 0, 0]);
    assert_eq!(proven_masks(src, "nvs_core_arr_fill"), [0]);
}

/// A subject whose element type is `mixed` or a union says nothing about the
/// tag each entry has, so the callback keeps the check of each call.
#[test]
fn a_callback_over_a_mixed_subject_keeps_the_check() {
    let src = "<?nvs\nfinal class T {\n  public static function m(array<mixed> $a, array<int|string> $u): void {\n    \
               var $b = Core\\Arr::map($a, fn(mixed $x): string => \"x\");\n    \
               var $c = Core\\Arr::map($u, fn(int|string $x): string => \"x\");\n  }\n}\n";
    assert_eq!(proven_masks(src, "nvs_core_arr_map"), [0, 0]);
}

/// Row `foreach-keys-or-values`: a loop over `keys(m)` or `values(m)` builds
/// neither list. The loop over the keys reads each key of `m` itself.
#[test]
fn a_loop_over_keys_or_values_walks_the_array_itself() {
    let src = "<?nvs\nfinal class T {\n  public static function m(array<int> $m): void {\n    \
               foreach (Core\\Arr::keys($m) as string $k) { echo $k; }\n    \
               foreach (Core\\Arr::values($m) as int $v) { echo $v; }\n  }\n}\n";
    let (on, off) = core_calls_both_ways(src);
    assert!(on.is_empty(), "{on:?}");
    assert_eq!(count_of(&off, "nvs_core_arr_keys"), 1, "{off:?}");
    let values = off
        .iter()
        .filter(|call| call.starts_with("nvs_core_arr_values"))
        .count();
    assert_eq!(values, 1, "{off:?}");
    let program = lower_whole_file_with_src(src).0;
    let keys = insts_where(&program, |kind| matches!(kind, InstKind::ArrayKeyAt { .. }));
    assert_eq!(keys, 1);
}

/// Row `foreach-range`: a loop over `range` builds no list. One helper call
/// checks the step and gives the last value, before the first step.
#[test]
fn a_loop_over_a_range_counts_without_an_array() {
    let src = "<?nvs\nfinal class T {\n  public static function m(int $a, int $b, int $s): void {\n    \
               foreach (Core\\Arr::range($a, $b) as int $i) { echo $i; }\n    \
               foreach (Core\\Arr::range($a, $b, options: {step: $s}) as int $i) { echo $i; }\n  }\n}\n";
    let (on_calls, off_calls) = core_calls_both_ways(src);
    assert!(on_calls.is_empty(), "{on_calls:?}");
    assert_eq!(
        count_of(&off_calls, "nvs_core_arr_range"),
        2,
        "{off_calls:?}"
    );
    let last = |kind: &InstKind| {
        matches!(
            kind,
            InstKind::HelperCall {
                helper: Helper::RangeLast,
                ..
            }
        )
    };
    let walks = |kind: &InstKind| matches!(kind, InstKind::ArrayNextSlot { .. });
    let on = lower_whole_file_with_src(src).0;
    let off = crate::lower::without_cheaper_forms(|| lower_whole_file_with_src(src).0);
    assert_eq!(insts_where(&on, last), 2);
    assert_eq!(insts_where(&on, walks), 0);
    assert_eq!(insts_where(&off, last), 0);
    assert_eq!(insts_where(&off, walks), 2);
}

/// Row `unread-foreach-key`: a key the body never names is not made, and a
/// key an arrow function captures is.
#[test]
fn a_key_nobody_reads_is_not_made() {
    let src = "<?nvs\nfinal class T {\n  public static function m(array<int> $m): void {\n    \
               foreach ($m as string $k => int $v) { echo $v; }\n    \
               foreach ($m as string $key => int $v) { var $f = fn() => $key; echo $v, $f(); }\n  \
               }\n}\n";
    let keys = |kind: &InstKind| matches!(kind, InstKind::ArrayKeyAt { .. });
    let on = lower_whole_file_with_src(src).0;
    let off = crate::lower::without_cheaper_forms(|| lower_whole_file_with_src(src).0);
    assert_eq!(insts_where(&on, keys), 1);
    assert_eq!(insts_where(&off, keys), 2);
}

/// Row `constant-key-literal`: a literal whose keys are all different literals
/// is one `ArrayNew` with its entries, and an integer key is its decimal. A
/// literal with a computed key or two equal keys writes one entry at a time.
#[test]
fn a_literal_with_constant_keys_is_built_in_one_step() {
    let src = "<?nvs\nfinal class T {\n  public static function m(string $k): void {\n    \
               array<int> $a = ['x' => 1, 0x10 => 2];\n    \
               array<int> $b = [$k => 1, 'y' => 2];\n    \
               array<int> $c = ['z' => 1, 'z' => 2];\n    \
               echo Core\\Arr::count($a), Core\\Arr::count($b), Core\\Arr::count($c);\n  }\n}\n";
    let sets = |kind: &InstKind| matches!(kind, InstKind::ArraySet { .. });
    let built = |kind: &InstKind| {
        matches!(kind, InstKind::ArrayNew { entries }
            if entries.iter().map(|(key, _)| key.as_str()).eq(["x", "16"]))
    };
    let on = lower_whole_file_with_src(src).0;
    let off = crate::lower::without_cheaper_forms(|| lower_whole_file_with_src(src).0);
    assert_eq!(insts_where(&on, built), 1);
    assert_eq!(insts_where(&on, sets), 4);
    assert_eq!(insts_where(&off, built), 0);
    assert_eq!(insts_where(&off, sets), 6);
}

/// Row `static-member`: `static::m()` is a direct call and `static::NAME` the
/// declaring class's constant where no class in the program overrides `m` or
/// declares `NAME` again, and each stays late-bound where one does.
#[test]
fn a_static_call_nothing_overrides_is_direct() {
    let src = "<?nvs\nclass Base {\n  public const string KEPT = \"kept-value\";\n  \
               public const string SWAPPED = \"b\";\n  \
               public static function kept(): string { return \"k\"; }\n  \
               public static function swapped(): string { return \"b\"; }\n  \
               public static function run(): string { return static::kept() . static::swapped(); }\n  \
               public static function names(): string { return static::KEPT . static::SWAPPED; }\n}\n\
               class Child extends Base {\n  public const string SWAPPED = \"c\";\n  \
               public static function swapped(): string { return \"c\"; }\n}\n";
    let direct =
        |kind: &InstKind| matches!(kind, InstKind::Call { target, .. } if target == "Base::kept");
    let late = |kind: &InstKind| matches!(kind, InstKind::CallVirtual { .. });
    let inlined = |kind: &InstKind| matches!(kind, InstKind::ConstStr(s) if s == "kept-value");
    let table_read = |kind: &InstKind| {
        matches!(
            kind,
            InstKind::HelperCall {
                helper: Helper::ClassConstant,
                ..
            }
        )
    };
    let on = lower_whole_file_with_src(src).0;
    let off = crate::lower::without_cheaper_forms(|| lower_whole_file_with_src(src).0);
    assert_eq!(insts_where(&on, direct), 1);
    assert_eq!(insts_where(&on, late), 1);
    assert_eq!(insts_where(&off, direct), 0);
    assert_eq!(insts_where(&off, late), 2);
    assert_eq!(insts_where(&on, inlined), 1);
    assert_eq!(insts_where(&on, table_read), 1);
    assert_eq!(insts_where(&off, inlined), 0);
    assert_eq!(insts_where(&off, table_read), 2);
}

/// Rows `named-method-call` of `crate::forms`: an override in a sibling class
/// leaves a call through `Quiet` direct, and an interface method with one
/// body in the program calls that body by its label.
#[test]
fn a_method_nothing_below_the_receiver_overrides_is_direct() {
    let src = "<?nvs\nclass Base {\n  public function speak(int $n): string { return \"b\" . $n; }\n}\n\
               class Loud extends Base {\n  public function speak(int $n): string { return \"L\" . $n; }\n}\n\
               class Quiet extends Base {}\n\
               interface Named {\n  public function label(int $n): string;\n}\n\
               class Only implements Named {\n  public function label(int $n): string { return \"o\" . $n; }\n}\n\
               final class Run {\n  \
               public static function quiet(Quiet $q): string { return $q->speak(1); }\n  \
               public static function named(Named $n): string { return $n->label(2); }\n}\n";
    let speak =
        |kind: &InstKind| matches!(kind, InstKind::Call { target, .. } if target == "Base::speak");
    let label =
        |kind: &InstKind| matches!(kind, InstKind::Call { target, .. } if target == "Only::label");
    let late = |kind: &InstKind| matches!(kind, InstKind::CallVirtual { .. });
    let on = lower_whole_file_with_src(src).0;
    let off = crate::lower::without_cheaper_forms(|| lower_whole_file_with_src(src).0);
    assert_eq!(insts_where(&on, speak), 1);
    assert_eq!(insts_where(&on, label), 1);
    assert_eq!(insts_where(&on, late), 0);
    assert_eq!(insts_where(&off, speak), 0);
    assert_eq!(insts_where(&off, label), 0);
    assert_eq!(insts_where(&off, late), 2);
}

/// Row `named-method-call`'s failing condition: `Loud` declares `speak`
/// again below `Base`, so a call through `Base` has two bodies to choose
/// from, and an interface with two implementors has two as well.
#[test]
fn a_method_a_class_below_the_receiver_overrides_stays_virtual() {
    let src = "<?nvs\nclass Base {\n  public function speak(int $n): string { return \"b\" . $n; }\n}\n\
               class Loud extends Base {\n  public function speak(int $n): string { return \"L\" . $n; }\n}\n\
               interface Named {\n  public function label(int $n): string;\n}\n\
               class One implements Named {\n  public function label(int $n): string { return \"1\"; }\n}\n\
               class Two implements Named {\n  public function label(int $n): string { return \"2\"; }\n}\n\
               final class Run {\n  \
               public static function base(Base $b): string { return $b->speak(1); }\n  \
               public static function named(Named $n): string { return $n->label(2); }\n}\n";
    let direct = |kind: &InstKind| {
        matches!(kind, InstKind::Call { target, .. }
            if target == "Base::speak" || target.ends_with("::label"))
    };
    let late = |kind: &InstKind| matches!(kind, InstKind::CallVirtual { .. });
    let on = lower_whole_file_with_src(src).0;
    assert_eq!(insts_where(&on, direct), 0);
    assert_eq!(insts_where(&on, late), 2);
}

/// Row `property-getter` of `crate::forms`: a direct call of a method whose
/// whole body is `return $this->name;` is the read of `name`, and a method
/// whose property has a hook stays a call.
#[test]
fn a_method_that_returns_one_property_is_the_property_read() {
    let src = "<?nvs\nfinal class User {\n  private string $name = \"a\";\n  \
               public string $shown {\n    get { return \"[\" . $this->shown . \"]\"; }\n  }\n  \
               public function constructor() { $this->shown = \"s\"; }\n  \
               public function getName(): string { return $this->name; }\n  \
               public function getShown(): string { return $this->shown; }\n}\n\
               final class Run {\n  \
               public static function name(User $u): string { return $u->getName(); }\n  \
               public static function shown(User $u): string { return $u->getShown(); }\n}\n";
    let getter_call = |kind: &InstKind| matches!(kind, InstKind::Call { target, .. } if target == "User::getName");
    let hooked_call = |kind: &InstKind| matches!(kind, InstKind::Call { target, .. } if target == "User::getShown");
    let name_read = |kind: &InstKind| matches!(kind, InstKind::FieldGet { class, field, .. } if class == "User" && field == "name");
    let on = lower_whole_file_with_src(src).0;
    let off = crate::lower::without_cheaper_forms(|| lower_whole_file_with_src(src).0);
    assert_eq!(insts_where(&on, getter_call), 0);
    assert_eq!(insts_where(&on, hooked_call), 1);
    // One read in `getName`'s own body and one at the call.
    assert_eq!(insts_where(&on, name_read), 2);
    assert_eq!(insts_where(&off, getter_call), 1);
    assert_eq!(insts_where(&off, name_read), 1);
}

/// `rule:expressions/equality-semantics`' value-class row: two written
/// durations compare through `Helper::Identical`, which reads their value
/// slots, so `30s == 30s` is `true` although each literal builds its own
/// object. A `match` arm over a `Date` takes the same helper.
#[test]
fn time_value_written_durations_are_equal() {
    let (f, map, file) = lower_script_src(
        "<?nvs
Core\\Time\\Duration $a = 30s;
bool $same = $a == 30s;
bool $other = $a != 1m;
Core\\Time\\Date $d = Core\\Time\\Date::at(2026, 1, 5);
string $day = match ($d) { Core\\Time\\Date::at(2026, 1, 5) => \"that day\", default => \"another\" };
",
    );
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("helper.identical").count(), 3, "{text}");
}

/// Two instances of a program's own class still compare by identity: the pair
/// keeps the inline address compare and calls no helper.
#[test]
fn time_value_a_user_class_still_compares_by_identity() {
    let (f, map, file) = lower_script_src(
        "<?nvs
final class Blog { public function constructor(public int $id) {} }
Blog $a = new Blog(1);
Blog $b = new Blog(1);
bool $same = $a == $b;
",
    );
    let text = print_function(&f, map.file(file));
    assert_eq!(text.matches("helper.identical").count(), 0, "{text}");
    assert!(text.contains("= eq "), "{text}");
}

/// `rule:classes/an-extend-block-adds-members-to-a-type`: an added method is
/// compiled under its block's label, and every call of it, instance, static
/// or through `self::`, is one direct call of that function. A static one
/// takes a `0` in slot 0, never a class descriptor.
#[test]
fn added_method_lowers_to_a_direct_call() {
    let src = "<?nvs\nnamespace Shop;\nuse Core\\Time\\Date;\nuse Shop\\Tools;\n\
               extend Date as Tools {\n  \
               public function isFirst(): bool { return $this->day() == 1; }\n  \
               public static function make(): Date { return Date::at(2026, 1, 1); }\n  \
               public static function check(): bool { return self::make()->isFirst(); }\n}\n\
               echo Date::make()->isFirst() ? \"y\" : \"n\";\n";
    let program = lower_whole_file_with_src(src).0;
    for label in [
        "Shop\\Tools::isFirst",
        "Shop\\Tools::make",
        "Shop\\Tools::check",
    ] {
        assert!(
            program.functions.iter().any(|f| f.name == label),
            "no function `{label}`"
        );
    }
    let is_first = |kind: &InstKind| matches!(kind, InstKind::Call { target, .. } if target == "Shop\\Tools::isFirst");
    let make = |kind: &InstKind| matches!(kind, InstKind::Call { target, .. } if target == "Shop\\Tools::make");
    let late = |kind: &InstKind| matches!(kind, InstKind::CallVirtual { .. });
    let descriptor = |kind: &InstKind| {
        matches!(
            kind,
            InstKind::ClassDescConst { .. } | InstKind::ClassDescOf { .. }
        )
    };
    assert_eq!(insts_where(&program, is_first), 2);
    assert_eq!(insts_where(&program, make), 2);
    assert_eq!(insts_where(&program, late), 0);
    assert_eq!(insts_where(&program, descriptor), 0);
}
