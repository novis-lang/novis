//! `secret` and `tainted` survive every narrowing, widening and untyped call.
//!
//! Each test tries one position that changes a value's type, and checks that
//! the qualifier is still there after it: a sink written after the position
//! refuses the value, and the same program with a qualified sink compiles.
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

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
