//! `secret` and `tainted` survive every narrowing, widening and untyped call.
//!
//! Each test tries one position that changes a value's type, and checks that
//! the qualifier is still there after it: a sink written after the position
//! refuses the value, and the same program with a qualified sink compiles.
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;
use nvs_types::expr::quals::ErasingPosition;

/// The subject types a type test is tried over: the declared parameter type,
/// the type it is tested against, the plain sink that must refuse what the
/// test narrows to, the qualified sink that must take it, and a fallback value
/// for the other arm of a ternary or a `match`.
const SUBJECTS: [(&str, &str, &str, &str, &str); 6] = [
    ("secret string", "string", "string", "secret string", "\"\""),
    (
        "tainted string",
        "string",
        "string",
        "tainted string",
        "\"\"",
    ),
    (
        "?tainted string",
        "string",
        "string",
        "tainted string",
        "\"\"",
    ),
    (
        "string|tainted string",
        "string",
        "string",
        "tainted string",
        "\"\"",
    ),
    (
        "tainted bytes",
        "bytes",
        "bytes",
        "tainted bytes",
        "\"\" as bytes",
    ),
    (
        "array<tainted string>",
        "array<string>",
        "array<string>",
        "array<tainted string>",
        "$x",
    ),
];

/// One program per form of test, each with the sink written once.
fn type_test_forms(tested: &str, sink: &str, fallback: &str) -> [String; 3] {
    [
        format!("if ($x is {tested}) {{ {sink} $out = $x; }}"),
        format!("{sink} $out = $x is {tested} ? $x : {fallback};"),
        format!("{sink} $out = match (true) {{ $x is {tested} => $x, default => {fallback} }};"),
    ]
}

fn in_function(param: &str, body: &str) -> String {
    format!("<?nvs\nclass T {{\n  function m({param} $x): void {{\n    {body}\n  }}\n}}\n")
}

#[test]
fn a_type_test_narrows_a_qualified_subject_to_a_type_that_keeps_the_qualifier() {
    for (param, tested, plain, qualified, fallback) in SUBJECTS {
        for body in type_test_forms(tested, plain, fallback) {
            let src = in_function(param, &body);
            let diags = check_src(&src);
            assert!(
                diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
                "the plain sink took a narrowed `{param}`:\n{src}\n{diags:?}"
            );
        }
        for body in type_test_forms(tested, qualified, fallback) {
            let src = in_function(param, &body);
            let diags = check_src(&src);
            assert!(
                !diags.has_errors(),
                "the qualified sink refused a narrowed `{param}`:\n{src}\n{diags:?}"
            );
        }
    }
    // A `secret` value narrowed by a test is still refused at `echo`.
    let src = in_function("?secret string", "if ($x is string) { echo $x; }");
    let diags = check_src(&src);
    assert!(diags.has_errors(), "`echo` took a narrowed secret:\n{src}");
}

#[test]
fn a_secret_value_converts_only_to_a_type_that_can_carry_secret() {
    for (param, line, refused) in [
        ("secret string", "int $n = $x as int;", true),
        ("secret string", "uint $n = $x as uint;", true),
        ("secret string", "bool $b = $x as bool;", true),
        ("secret tainted string", "float $f = $x as float;", true),
        ("?secret string", "int $n = $x as int;", true),
        ("secret string", "secret string $s = $x as string;", false),
        ("secret string", "secret bytes $b = $x as bytes;", false),
        ("?secret string", "secret string $s = $x as string;", false),
        (
            "array<secret string>",
            "array<secret string> $a = $x as array<string>;",
            false,
        ),
        (
            "secret string",
            "int $n = Core\\Secret::reveal($x, \"a port number\") as int;",
            false,
        ),
        // `tainted` alone is unchanged: a conversion to a type with no text
        // proves the value's shape, so it removes `tainted`.
        ("tainted string", "int $n = $x as int;", false),
    ] {
        let src = in_function(param, line);
        let diags = check_src(&src);
        let found = diags
            .iter()
            .any(|d| d.code == Some(code::E_SECRET_CONVERSION_LOSES_SECRET));
        assert_eq!(found, refused, "`{param}`: {line}\n{diags:?}");
        if !refused {
            assert!(!diags.has_errors(), "`{param}`: {line}\n{diags:?}");
        }
    }
}

#[test]
fn a_shape_conversion_over_a_class_refuses_a_secret_property() {
    let program = |line: &str| {
        format!(
            "<?nvs\n\
             class Account {{\n\
             \x20 public string $name = \"a\";\n\
             \x20 public secret string $token = \"b\";\n\
             }}\n\
             class T {{\n\
             \x20 function m(Account $a): void {{\n\
             \x20   {line}\n\
             \x20 }}\n\
             }}\n"
        )
    };
    for line in [
        "$a as {token: string};",
        "$a as {name: string, token: string};",
        "$a as ?{token: string};",
    ] {
        let diags = check_src(&program(line));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_SECRET_CONVERSION_LOSES_SECRET)),
            "{line}\n{diags:?}"
        );
    }
    let diags = check_src(&program("$a as {name: string};"));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A class with one method over a `secret string $x` and an
/// `array<secret string> $xs`, plus a `{M}` property, a `{M}` parameter and a
/// `{M}` return. `{M}` is replaced by the type under test.
fn placement_program(mixed: &str, body: &str) -> String {
    format!(
        "<?nvs\n\
         class T {{\n\
         \x20 public {mixed} $p;\n\
         \x20 function constructor(): void {{ $this->p = 0; }}\n\
         \x20 function take({mixed} $v): void {{}}\n\
         \x20 function give(secret string $x): {mixed} {{ return $x; }}\n\
         \x20 function m(secret string $x, array<secret string> $xs): void {{\n\
         \x20   {body}\n\
         \x20 }}\n\
         }}\n"
    )
    .replace("{M}", mixed)
}

#[test]
fn a_secret_value_enters_mixed_only_where_secret_mixed_is_written() {
    let bodies = [
        "{M} $m = $x;",
        "{M} $m = 1; $m = $x;",
        "$this->p = $x;",
        "$this->take($x);",
        "foreach ($xs as {M} $v) { $this->take($v); }",
        "array<{M}> $a = $xs;",
        "array<{M}> $a = [$x];",
        "?{M} $m = $x;",
        "{M} $m = {token: $x};",
    ];
    // The return is in the class itself, so a plain `mixed` program has its
    // one refusal there even with an empty body.
    let diags = check_src(&placement_program("mixed", ""));
    let names_secret_mixed =
        |d: &&nvs_diagnostics::Diagnostic| d.notes.iter().any(|note| note.contains("secret mixed"));
    assert!(
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_BAD_RETURN_TYPE))
            .any(|d| names_secret_mixed(&d)),
        "the return took a secret:\n{diags:?}"
    );
    let at_return = diags
        .iter()
        .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
        .count();
    for body in bodies {
        let src = placement_program("mixed", body);
        let diags = check_src(&src);
        let mismatches: Vec<_> = diags
            .iter()
            .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
            .collect();
        assert!(
            mismatches.len() > at_return,
            "plain `mixed` took a secret:\n{src}\n{diags:?}"
        );
        assert!(
            mismatches.iter().all(names_secret_mixed),
            "the refusal does not name `secret mixed`:\n{src}\n{diags:?}"
        );
        let src = placement_program("secret mixed", body);
        let diags = check_src(&src);
        assert!(
            !diags.has_errors(),
            "`secret mixed` refused a secret:\n{src}\n{diags:?}"
        );
    }
    // A value with no secret in it still fits plain `mixed`.
    let diags = check_src(
        &placement_program("mixed", "mixed $m = \"a\"; $this->take(1);")
            .replace("return $x;", "return 1;"),
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn text_out_of_secret_mixed_is_secret_and_tainted() {
    for (line, refused) in [
        ("string $s = $m as string;", true),
        ("secret string $s = $m as string;", true),
        ("tainted string $s = $m as string;", true),
        ("secret tainted string $s = $m as string;", false),
        ("?secret tainted string $s = $m as ?string;", false),
        ("if ($m is string) { string $s = $m; }", true),
        (
            "if ($m is string) { secret tainted string $s = $m; }",
            false,
        ),
        ("mixed $n = $m[\"k\"];", true),
        ("secret mixed $n = $m[\"k\"];", false),
        ("mixed $n = $m;", true),
        ("int $n = $m as int;", true),
        ("secret mixed $n = $m as mixed;", false),
    ] {
        let src = in_function("secret mixed", &line.replace("$m", "$x"));
        let diags = check_src(&src);
        assert_eq!(diags.has_errors(), refused, "{line}\n{diags:?}");
    }
}

#[test]
fn every_secret_sink_refuses_a_secret_mixed() {
    for line in [
        "echo $m;",
        "print $m;",
        "echo \"a{$m}\";",
        "string $s = Core\\Json::encode($m);",
        "bytes $b = Core\\Serialize::encode($m);",
        "Core\\Session::set(\"k\", $m);",
        "throw new LogicException($m as string);",
    ] {
        let src = in_function("secret mixed", &line.replace("$m", "$x"));
        let diags = check_src(&src);
        assert!(
            diags
                .iter()
                .all(|d| d.code != Some(code::E_UNDEFINED_VARIABLE)),
            "{line}\n{diags:?}"
        );
        assert!(diags.has_errors(), "a sink took a `secret mixed`: {line}");
    }
}

/// The qualified types a call or a write with no parameter type is tried
/// with. `mixed` counts, because text out of it is `tainted`.
const QUALIFIED: [&str; 3] = ["tainted string", "secret string", "mixed"];

/// A program over a value `$x` of type `{Q}` and an `array<{Q}> $xs`, with a
/// bare `callable $f`, a `mixed $r`, an `object $o`, a
/// `callable(string): void $g` and a `callable({Q}): void $h`. `{Q}` is
/// replaced by the type under test.
fn untyped_program(qualified: &str, body: &str) -> String {
    format!(
        "<?nvs\n\
         class Box {{\n\
         \x20 public string $p = \"\";\n\
         \x20 function put(string $s): void {{}}\n\
         }}\n\
         class T {{\n\
         \x20 function m({{Q}} $x, array<{{Q}}> $xs, callable $f, mixed $r, object $o, \
         callable(string): void $g, callable({{Q}}): void $h): void {{\n\
         \x20   {body}\n\
         \x20 }}\n\
         }}\n"
    )
    .replace("{Q}", qualified)
}

#[test]
fn a_call_with_no_parameter_list_refuses_a_qualified_argument() {
    let bodies = [
        "$f($x);",
        "$f(1, [$x]);",
        "$r->put($x);",
        "$g(...$xs);",
        "Core\\Reflect::forObject($r)->set($r, \"p\", $x);",
        "Core\\Reflect::forObject($r)->call($r, \"put\", [$x]);",
        "Core\\Reflect::forObject($r)->call($r, \"put\", $xs);",
        "Core\\Reflect::forObject($r)->construct([1, $x]);",
    ];
    for qualified in QUALIFIED {
        for body in bodies {
            let src = untyped_program(qualified, body);
            let diags = check_src(&src);
            assert!(
                diags
                    .iter()
                    .any(|d| d.code == Some(code::E_QUALIFIED_VALUE_WITHOUT_PARAMETER_TYPE)),
                "a call with no parameter list took a `{qualified}`:\n{src}\n{diags:?}"
            );
        }
    }
    // A value with no qualifier still passes, and is checked when it runs.
    for body in [
        "$f(\"a\", 1);",
        "$r->put(\"a\");",
        "Core\\Reflect::forObject($r)->set($r, \"p\", \"a\");",
        "Core\\Reflect::forObject($r)->call($r, \"put\", [\"a\"]);",
        "Core\\Reflect::forObject($r)->construct([]);",
    ] {
        let src = untyped_program("string", body);
        let diags = check_src(&src);
        assert!(!diags.has_errors(), "{src}\n{diags:?}");
    }
}

#[test]
fn a_typed_callable_takes_the_qualified_argument_its_signature_names() {
    for qualified in QUALIFIED {
        for body in ["$h($x);", "$h(...$xs);"] {
            let src = untyped_program(qualified, body);
            let diags = check_src(&src);
            assert!(
                !diags.has_errors(),
                "`callable({qualified}): void` refused its own argument:\n{src}\n{diags:?}"
            );
        }
        // The signature checks the argument: a plain `string` parameter
        // refuses the qualified value.
        let src = untyped_program(qualified, "$g($x);");
        let diags = check_src(&src);
        assert!(diags.has_errors(), "{src}\n{diags:?}");
    }
}

#[test]
fn an_erased_property_write_refuses_a_qualified_value() {
    for qualified in QUALIFIED {
        for body in ["$o->p = $x;", "$r->p = $x;"] {
            let src = untyped_program(qualified, body);
            let diags = check_src(&src);
            // A `secret` value is refused by the `mixed` it is written into,
            // and that one report is the whole of it.
            let code = if qualified.starts_with("secret") {
                code::E_TYPE_MISMATCH
            } else {
                code::E_QUALIFIED_VALUE_WITHOUT_PARAMETER_TYPE
            };
            let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
            assert!(
                codes.contains(&code),
                "an erased write took a `{qualified}`:\n{src}\n{diags:?}"
            );
            assert!(
                !(code == code::E_TYPE_MISMATCH
                    && codes.contains(&code::E_QUALIFIED_VALUE_WITHOUT_PARAMETER_TYPE)),
                "one mistake reported twice:\n{src}\n{diags:?}"
            );
        }
    }
    for body in ["$o->p = \"a\";", "$r->p = \"a\";"] {
        let src = untyped_program("string", body);
        let diags = check_src(&src);
        assert!(!diags.has_errors(), "{src}\n{diags:?}");
    }
}

/// A program over a key `$k` of type `{K}` and an array `$a` of type `{A}`,
/// with a method `take` whose parameter is `{A}` and a method `any` whose
/// parameter is `mixed`.
fn key_program(key: &str, array: &str, body: &str) -> String {
    format!(
        "<?nvs\n\
         class T {{\n\
         \x20 function take({array} $a): void {{}}\n\
         \x20 function any(mixed $m): void {{}}\n\
         \x20 function m({key} $k, {array} $a): void {{\n\
         \x20   {body}\n\
         \x20 }}\n\
         }}\n"
    )
}

/// Every position a value becomes a key of an array the program writes: a
/// subscript write, two compound writes, a literal against a declared type,
/// an inferred literal and a literal in argument position.
const KEY_POSITIONS: [&str; 6] = [
    "$a[$k] = \"v\";",
    "$a[$k] .= \"v\";",
    "$a[$k] ??= \"v\";",
    "{A} $b = [$k => \"v\"];",
    "var $b = [$k => \"v\"]; $this->take($b);",
    "$this->take([$k => \"v\"]);",
];

#[test]
fn a_tainted_key_is_checked_at_every_key_position() {
    for position in KEY_POSITIONS {
        let body = position.replace("{A}", "array<string>");
        let src = key_program("tainted string", "array<string>", &body);
        let diags = check_src(&src);
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "a plain-keyed array took a `tainted` key:\n{src}\n{diags:?}"
        );
        // An element type that carries `tainted` says the keys may come from
        // outside, so the same key is taken. An inferred literal of plain
        // values is `array<string>`, so it has no such form to try.
        if position.starts_with("var") {
            continue;
        }
        let body = position.replace("{A}", "array<tainted string>");
        let src = key_program("tainted string", "array<tainted string>", &body);
        let diags = check_src(&src);
        assert!(!diags.has_errors(), "{src}\n{diags:?}");
    }
}

#[test]
fn a_secret_value_is_never_an_array_key() {
    for array in ["array<string>", "array<secret string>", "array<mixed>"] {
        for position in KEY_POSITIONS {
            let body = position.replace("{A}", array);
            let src = key_program("secret string", array, &body);
            let diags = check_src(&src);
            assert!(
                diags
                    .iter()
                    .any(|d| d.code == Some(code::E_SECRET_ARRAY_KEY)),
                "`{array}` took a `secret` key:\n{src}\n{diags:?}"
            );
        }
    }
    // A literal handed to a `mixed` parameter, and a `?secret string` key.
    for (key, body) in [
        ("secret string", "$this->any([$k => 1]);"),
        ("?secret string", "$a[$k ?? \"\"] = \"v\";"),
    ] {
        let src = key_program(key, "array<string>", body);
        let diags = check_src(&src);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_SECRET_ARRAY_KEY)),
            "a `{key}` became a key:\n{src}\n{diags:?}"
        );
    }
    // The value of an element may be secret where the element type says so.
    let src = key_program("secret string", "array<secret string>", "$a[\"id\"] = $k;");
    let diags = check_src(&src);
    assert!(!diags.has_errors(), "{src}\n{diags:?}");
}

/// A program over a key `$k` of type `{Q}`, a list `$s` of type `array<{Q}>`
/// and rows `$rows` of type `array<array<{Q}>>`.
fn key_member_program(qualified: &str, body: &str) -> String {
    format!(
        "<?nvs\n\
         class T {{\n\
         \x20 function m({qualified} $k, array<{qualified}> $s, array<array<{qualified}>> $rows): void {{\n\
         \x20   {body}\n\
         \x20 }}\n\
         }}\n"
    )
}

/// Each `Core` member that turns values into keys, called so that those
/// values are the `{Q}` ones.
const KEY_MEMBERS: [&str; 8] = [
    "var $r = Core\\Arr::flip($s);",
    "var $r = Core\\Arr::fillKeys($s, 1);",
    "var $r = Core\\Arr::fromKeysAndValues($s, [1]);",
    "var $r = Core\\Arr::countBy($s);",
    "var $r = Core\\Arr::countBy([1, 2], { by: fn(int $x): {Q} => $k });",
    "var $r = Core\\Arr::mapKeys([1, 2], fn(int $x, string $i): {Q} => $k);",
    "var $r = Core\\Arr::groupBy([1, 2], fn(int $x, string $i): {Q} => $k);",
    "var $r = Core\\Arr::column($rows, 0, { indexBy: 1 });",
];

#[test]
fn a_member_that_makes_keys_from_values_refuses_qualified_values() {
    for (index, call) in KEY_MEMBERS.iter().enumerate() {
        let body = call.replace("{Q}", "secret string");
        let src = key_member_program("secret string", &body);
        let diags = check_src(&src);
        let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
        assert!(
            codes.contains(&code::E_SECRET_ARRAY_KEY) || codes.contains(&code::E_TYPE_MISMATCH),
            "a `secret` value became a key:\n{src}\n{diags:?}"
        );
        // Where the parameter's type takes the value, only the key mark
        // refuses it: `countBy`'s subject, its `by` result and `column`'s rows.
        if [3, 4, 7].contains(&index) {
            assert!(
                codes.contains(&code::E_SECRET_ARRAY_KEY),
                "the key mark did not refuse it:\n{src}\n{diags:?}"
            );
        }
        assert!(
            !(codes.contains(&code::E_SECRET_ARRAY_KEY) && codes.contains(&code::E_TYPE_MISMATCH)),
            "one mistake reported twice:\n{src}\n{diags:?}"
        );
    }
    // A `tainted` value keys only an array whose type says its keys come
    // from outside, so the members with a plain key parameter refuse it.
    for call in &KEY_MEMBERS[..3] {
        let src = key_member_program("tainted string", call);
        assert!(
            check_src(&src).has_errors(),
            "a `tainted` value became a key:\n{src}"
        );
    }
    // Where the parameter takes it, the key mark refuses a `tainted` value
    // whose answer reads its keys back plain: `countBy` gives `array<uint>`,
    // and `mapKeys` and `groupBy` over `[1, 2]` give arrays of `int`.
    for call in &KEY_MEMBERS[3..7] {
        let body = call.replace("{Q}", "tainted string");
        let src = key_member_program("tainted string", &body);
        let diags = check_src(&src);
        let refused: Vec<_> = diags
            .iter()
            .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
            .collect();
        assert_eq!(
            refused.len(),
            1,
            "a `tainted` key entered a plain-keyed answer:\n{src}\n{diags:?}"
        );
    }
    // `column` without `indexBy` keys by position, so its cells may be secret,
    // and a member over plain values compiles. A `tainted` key enters an
    // answer whose elements are tainted, since its keys read back as outside
    // text: `column` over tainted rows. `mapKeys` over tainted text compiles
    // with a plain key. Its callback type refuses a `tainted` key.
    for (qualified, body) in [
        ("secret string", "var $r = Core\\Arr::column($rows, 0);"),
        (
            "tainted string",
            "var $r = Core\\Arr::mapKeys($s, fn(tainted string $x, tainted string $i): int => 1);",
        ),
        (
            "tainted string",
            "var $r = Core\\Arr::column($rows, 0, { indexBy: 1 });",
        ),
        ("string", "var $r = Core\\Arr::countBy($s);"),
        ("string", "var $r = Core\\Arr::flip($s);"),
        (
            "string",
            "var $r = Core\\Arr::mapKeys([1, 2], fn(int $x, string $i): string => $k);",
        ),
        (
            "string",
            "var $r = Core\\Arr::column($rows, 0, { indexBy: 1 });",
        ),
    ] {
        let src = key_member_program(qualified, body);
        let diags = check_src(&src);
        assert!(!diags.has_errors(), "{src}\n{diags:?}");
    }
}

/// One cell of [`the_form_each_refusal_names_compiles_at_every_erasing_position`]:
/// the parameters, the form the refusal's help names with a `SINK` mark, the
/// sink written at the mark, and the code the sink reports.
struct FormCell {
    params: &'static str,
    form: &'static str,
    sink: &'static str,
    code: nvs_diagnostics::Code,
}

/// The `secret` cell and the `tainted` cell of one position. Each form is what
/// the position's refusal names in its help: the typed form where one exists,
/// and the launderer where the help names it.
fn form_cells(position: ErasingPosition) -> [FormCell; 2] {
    use ErasingPosition as P;
    let out = code::E_SECRET_OUTPUT;
    let plain = code::E_TYPE_MISMATCH;
    match position {
        P::TypeTest => [
            FormCell {
                params: "?secret string $x",
                form: "secret string $y = \"\"; if ($x is string) { $y = $x; } SINK",
                sink: "echo $y;",
                code: out,
            },
            FormCell {
                params: "?tainted string $x",
                form: "tainted string $y = \"\"; if ($x is string) { $y = $x; } SINK",
                sink: "Sink::plain($y);",
                code: plain,
            },
        ],
        P::Conversion => [
            FormCell {
                params: "secret string $x",
                form: "int $n = Core\\Secret::reveal($x, \"reason\") as int; \
                       secret string $y = $x as string; SINK",
                sink: "echo $y;",
                code: out,
            },
            FormCell {
                params: "tainted string $x",
                form: "tainted string $y = $x as string; SINK",
                sink: "Sink::plain($y);",
                code: plain,
            },
        ],
        P::Widening => [
            FormCell {
                params: "secret string $x",
                form: "secret mixed $m = $x; SINK",
                sink: "echo $m;",
                code: out,
            },
            FormCell {
                params: "tainted string $x",
                form: "mixed $m = $x; SINK",
                sink: "Sink::plain($m as string);",
                code: plain,
            },
        ],
        P::UntypedCall => [
            FormCell {
                params: "secret string $x",
                form: "callable(secret string): void $f = \
                       fn(secret string $s): void => { SINK }; $f($x);",
                sink: "echo $s;",
                code: out,
            },
            FormCell {
                params: "tainted string $x",
                form: "callable(tainted string): void $f = \
                       fn(tainted string $s): void => { SINK }; $f($x);",
                sink: "Sink::plain($s);",
                code: plain,
            },
        ],
        P::ErasedWrite => [
            FormCell {
                params: "object $o, secret string $x",
                form: "if ($o is Holder) { $o->token = $x; SINK }",
                sink: "echo $o->token;",
                code: out,
            },
            FormCell {
                params: "object $o, tainted string $x",
                form: "if ($o is Holder) { $o->name = $x; SINK }",
                sink: "Sink::plain($o->name);",
                code: plain,
            },
        ],
        P::ErasedRead => [
            FormCell {
                params: "Holder $h",
                form: "secret string $t = $h->token; SINK",
                sink: "echo $t;",
                code: out,
            },
            FormCell {
                params: "mixed $m",
                form: "if ($m is Holder) { tainted string $n = $m->name; SINK }",
                sink: "Sink::plain($n);",
                code: plain,
            },
        ],
        P::Key => [
            FormCell {
                params: "secret string $x",
                form: "array<secret string> $a = []; $a[\"k\"] = $x; SINK",
                sink: "echo $a[\"k\"];",
                code: out,
            },
            FormCell {
                params: "tainted string $x",
                form: "array<tainted string> $a = []; $a[$x] = \"v\"; SINK",
                sink: "Sink::plain($a[$x]);",
                code: plain,
            },
        ],
        // The help names the launderer, so the sink is the member over the
        // values that were not laundered.
        P::KeyMakingMember => [
            FormCell {
                params: "array<secret string> $xs",
                form: "array<string> $plain = Core\\Arr::map($xs, \
                       fn(secret string $s): string => Core\\Secret::reveal($s, \"reason\")); \
                       array<uint> $c = Core\\Arr::countBy($plain); SINK",
                sink: "array<uint> $d = Core\\Arr::countBy($xs);",
                code: code::E_SECRET_ARRAY_KEY,
            },
            FormCell {
                params: "array<tainted string> $xs",
                form: "array<string> $plain = Core\\Arr::map($xs, \
                       fn(tainted string $s): string => Core\\Taint::assertTrusted($s, \"reason\")); \
                       array<uint> $c = Core\\Arr::countBy($plain); SINK",
                sink: "array<uint> $d = Core\\Arr::countBy($xs);",
                code: plain,
            },
        ],
        P::Reflection => [
            FormCell {
                params: "Holder $h, secret string $x",
                form: "$h->token = $x; SINK",
                sink: "echo $h->token;",
                code: out,
            },
            FormCell {
                params: "tainted string $x",
                form: "string $name = Core\\Taint::assertTrusted($x, \"reason\"); \
                       Core\\Reflect::forClass($name); SINK",
                sink: "Core\\Reflect::forClass($x);",
                code: plain,
            },
        ],
        P::GenericInstantiation => [
            FormCell {
                params: "array<secret string> $xs",
                form: "secret string $f = Core\\Arr::first($xs) ?? \"\"; SINK",
                sink: "echo $f;",
                code: out,
            },
            FormCell {
                params: "array<tainted string> $xs",
                form: "tainted string $f = Core\\Arr::first($xs) ?? \"\"; SINK",
                sink: "Sink::plain($f);",
                code: plain,
            },
        ],
    }
}

/// A program with the matrix's `Sink` and `Holder` and one method over
/// `params` whose body is `body`.
fn form_program(params: &str, body: &str) -> String {
    format!(
        "<?nvs\n\
         class Sink {{\n\
         \x20 public static function plain(string $s): void {{ echo $s, \"\\n\"; }}\n\
         }}\n\
         class Holder {{\n\
         \x20 public secret string $token = \"\";\n\
         \x20 public tainted string $name = \"\";\n\
         }}\n\
         class Cell {{\n\
         \x20 public static function run({params}): void {{\n\
         \x20   {body}\n\
         \x20 }}\n\
         }}\n"
    )
}

#[test]
fn the_form_each_refusal_names_compiles_at_every_erasing_position() {
    for position in ErasingPosition::ALL {
        for cell in form_cells(position) {
            let name = position.name();
            let src = form_program(cell.params, &cell.form.replace("SINK", ""));
            let diags = check_src(&src);
            assert!(
                !diags.has_errors(),
                "{name}: the form the help names does not compile:\n{src}\n{diags:?}"
            );
            let src = form_program(cell.params, &cell.form.replace("SINK", cell.sink));
            let diags = check_src(&src);
            assert!(
                diags.iter().any(|d| d.code == Some(cell.code)),
                "{name}: the sink after the form took the value, so the form lost the \
                 qualifier:\n{src}\n{diags:?}"
            );
        }
    }
}

/// The help of a call through a bare `callable` names a parameter of the
/// argument's own type, so the form it names takes the value.
#[test]
fn a_bare_callable_refusal_names_a_parameter_of_the_arguments_type() {
    for qualified in ["secret string", "tainted string"] {
        let src = form_program(&format!("callable $f, {qualified} $x"), "$f($x);");
        let diags = check_src(&src);
        let help = format!("`callable({qualified}): void`");
        assert!(
            diags.iter().any(|d| {
                d.code == Some(code::E_QUALIFIED_VALUE_WITHOUT_PARAMETER_TYPE)
                    && d.notes.iter().any(|h| h.contains(&help))
            }),
            "the help does not name {help}:\n{src}\n{diags:?}"
        );
    }
}
