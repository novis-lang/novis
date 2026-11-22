//! `rule:security/taint-propagation` over every expression kind: a tainted
//! operand that reaches a result makes the result tainted.
//!
//! [`ROWS`] has one row or more for each name in `ExprKind::NAMES`, and the
//! test fails on a name with none. `nvs_syntax::ast::ExprKind::name` matches
//! every variant with no catch-all arm, so a kind added later must be named
//! there before the parser compiles, and then needs a row here.
//!
//! A [`Row::Text`] row is one program in which `{T}` is the operand's type.
//! With `{T}` as `string` the program compiles clean. With `{T}` as
//! `tainted string` it must report a type mismatch where the result lands in
//! a plain `string`. A [`Row::NoText`] row is a kind whose value carries no
//! outside text, with the reason.

mod common;

use common::check_src_table_allowing_errors;
use nvs_diagnostics::{Diagnostics, Severity};

use nvs_syntax::ast::ExprKind;
use nvs_syntax::walk;

/// One kind's row.
enum Row {
    /// A program in which a `{T}` operand reaches a plain `string` or
    /// `bytes` target through the kind.
    Text(&'static str),
    /// A program whose result comes from outside without any `{T}` operand.
    /// It must report a type mismatch whatever `{T}` is.
    Outside(&'static str),
    /// The kind's value never carries outside text, and why.
    NoText(&'static str),
}

/// Wraps `body` in a class method whose one parameter `$t` has type `{T}`.
macro_rules! in_method {
    ($body:literal) => {
        concat!(
            "<?nvs\nclass Fixture {\n  static function m({T} $t): void {\n",
            $body,
            "\n  }\n}\n"
        )
    };
}

/// A class with a property, a getter and a nullable static property, for
/// the member rows.
macro_rules! with_box {
    ($body:literal) => {
        concat!(
            "<?nvs\nclass Box {\n",
            "  public {T} $v;\n",
            "  function constructor({T} $v) { $this->v = $v; }\n",
            "  function get(): {T} { return $this->v; }\n",
            "  static function fail(): {T} { throw new RuntimeError(\"no\"); }\n",
            "}\n",
            "class Fixture {\n  static function m({T} $t): void {\n",
            $body,
            "\n  }\n}\n"
        )
    };
}

/// Every row, by the `ExprKind` name it covers. A kind may have more than
/// one row.
const ROWS: &[(&str, Row)] = &[
    ("Null", Row::NoText("`null` has no text.")),
    ("Bool", Row::NoText("a boolean has no text.")),
    ("Int", Row::NoText("a number literal has no text.")),
    ("Float", Row::NoText("a number literal has no text.")),
    (
        "Duration",
        Row::NoText("a duration literal is a `Core\\Time\\Duration`."),
    ),
    ("Str", Row::NoText("a literal is text the program wrote.")),
    (
        "Interpolated",
        Row::Text(in_method!("    string $s = \"a {$t} b\";")),
    ),
    (
        "Interpolated",
        Row::Text(in_method!("    string $s = <<<EOT\n    a {$t} b\n    EOT;")),
    ),
    (
        "HtmlTemplate",
        Row::NoText(
            "the result is `Core\\Html\\Markup`, and every hole is escaped as element text \
             (`rule:core-classes/html-template`).",
        ),
    ),
    ("Variable", Row::Text(in_method!("    string $s = $t;"))),
    (
        "ConstFetch",
        Row::NoText("a constant's value is a constant expression the program wrote."),
    ),
    ("SelfExpr", Row::NoText("`self` names a class.")),
    ("StaticExpr", Row::NoText("`static` names a class.")),
    ("ParentExpr", Row::NoText("`parent` names a class.")),
    (
        "ArrayLiteral",
        Row::Text(in_method!("    array<string> $a = [$t, \"b\"];")),
    ),
    (
        "ArrayLiteral",
        Row::Text(in_method!("    array<string> $a = [\"k\" => $t];")),
    ),
    (
        "ArrayLiteral",
        Row::Text(in_method!(
            "    var $x = [$t];\n    array<string> $a = [...$x, \"b\"];"
        )),
    ),
    (
        "Unary",
        Row::NoText("`-`, `+`, `!` and `~` give a number or a boolean."),
    ),
    ("PreIncDec", Row::NoText("`++` and `--` give a number.")),
    ("PostIncDec", Row::NoText("`++` and `--` give a number.")),
    (
        "Binary",
        Row::Text(in_method!("    string $s = $t . \"b\";")),
    ),
    (
        "Binary",
        Row::Text(in_method!("    string $s = \"a\" . $t;")),
    ),
    (
        "Binary",
        Row::Text(in_method!(
            "    ?{T} $n = $t;\n    string $s = $n ?? \"b\";"
        )),
    ),
    (
        "Binary",
        Row::Text(in_method!(
            "    ?string $n = null;\n    string $s = $n ?? $t;"
        )),
    ),
    (
        "Assign",
        Row::Text(in_method!("    string $s = \"a\";\n    $s = $t;")),
    ),
    (
        "Assign",
        Row::Text(in_method!(
            "    {T} $u = \"a\";\n    string $s = ($u = $t);"
        )),
    ),
    (
        "Assign",
        Row::Text(in_method!("    string $s = \"a\";\n    $s .= $t;")),
    ),
    (
        "Assign",
        Row::Text(in_method!("    ?string $s = null;\n    $s ??= $t;")),
    ),
    (
        "Assign",
        Row::Text(in_method!("    ?string $s = null;\n    $s ??.= $t;")),
    ),
    (
        "Ternary",
        Row::Text(in_method!("    string $s = true ? $t : \"b\";")),
    ),
    (
        "Ternary",
        Row::Text(in_method!("    string $s = false ? \"b\" : $t;")),
    ),
    (
        "Ternary",
        Row::Text(in_method!("    string $s = $t ?: \"b\";")),
    ),
    (
        "Conversion",
        Row::Text(in_method!("    string $s = $t as string;")),
    ),
    (
        "Conversion",
        Row::Text(in_method!("    bytes $b = $t as bytes;")),
    ),
    (
        "Conversion",
        Row::Text(in_method!("    string $s = ($t as bytes) as string;")),
    ),
    (
        "Conversion",
        Row::Outside(concat!(
            "<?nvs\nclass Fixture {\n  static function m({T} $t, Core\\Ldap\\Filter $f): void {\n",
            "    string $s = $f as string;\n    echo $t;\n  }\n}\n"
        )),
    ),
    (
        "Conversion",
        Row::Outside(concat!(
            "<?nvs\nclass Fixture {\n  static function m({T} $t, Core\\Ldap\\Dn $d): void {\n",
            "    string $s = $d as string;\n    echo $t;\n  }\n}\n"
        )),
    ),
    (
        "Interpolated",
        Row::Outside(concat!(
            "<?nvs\nclass Fixture {\n  static function m({T} $t, Core\\Ldap\\Filter $f): void {\n",
            "    string $s = \"({$f})\";\n    echo $t;\n  }\n}\n"
        )),
    ),
    (
        "Binary",
        Row::Outside(concat!(
            "<?nvs\nclass Fixture {\n  static function m({T} $t, Core\\Ldap\\Dn $d): void {\n",
            "    string $s = \"dn: \" . $d;\n    echo $t;\n  }\n}\n"
        )),
    ),
    ("TypeTest", Row::NoText("`is` gives a boolean.")),
    (
        "Call",
        Row::Text(in_method!(
            "    var $f = fn({T} $x) => $x . \"!\";\n    string $s = $f($t);"
        )),
    ),
    (
        "MethodCall",
        Row::Text(with_box!("    string $s = (new Box($t))->get();")),
    ),
    (
        "MethodCall",
        Row::Text(with_box!(
            "    ?Box $b = new Box($t);\n    string $s = $b?->get() ?? \"\";"
        )),
    ),
    (
        "StaticCall",
        Row::Text(in_method!("    string $s = Core\\Str::upper($t);")),
    ),
    (
        "StaticCall",
        Row::Text(in_method!("    string $s = Core\\Json::encode([$t]);")),
    ),
    (
        "StaticCall",
        Row::Text(in_method!(
            "    string $s = Core\\Str::format(\"%s!\", $t);"
        )),
    ),
    (
        "PropertyAccess",
        Row::Text(with_box!("    string $s = (new Box($t))->v;")),
    ),
    (
        "PropertyAccess",
        Row::Text(with_box!(
            "    ?Box $b = new Box($t);\n    string $s = $b?->v ?? \"\";"
        )),
    ),
    (
        "PropertyAccess",
        Row::Text(in_method!(
            "    var $o = { v: $t };\n    string $s = $o->v;"
        )),
    ),
    (
        "StaticPropertyAccess",
        Row::NoText(
            "a static property is a constant expression the program wrote, and \
             only its declared type is read.",
        ),
    ),
    (
        "ClassConstAccess",
        Row::NoText("a class constant is a constant expression."),
    ),
    (
        "ClassNameConst",
        Row::NoText("`C::class` is a class name the program wrote."),
    ),
    (
        "Index",
        Row::Text(in_method!("    var $a = [$t];\n    string $s = $a[0];")),
    ),
    (
        "Index",
        Row::Text(in_method!(
            "    var $a = [\"k\" => $t];\n    string $s = $a[\"k\"];"
        )),
    ),
    (
        "New",
        Row::NoText("`new` gives an object, and each property has its declared type."),
    ),
    (
        "Clone",
        Row::Text(in_method!(
            "    var $o = { v: $t };\n    string $s = (clone $o)->v;"
        )),
    ),
    (
        "Fn",
        Row::Text(in_method!(
            "    var $f = fn() => $t;\n    string $s = $f();"
        )),
    ),
    (
        "Fn",
        Row::Text(in_method!(
            "    var $f = fn() => $t . \"!\";\n    string $s = $f();"
        )),
    ),
    (
        "Match",
        Row::Text(in_method!(
            "    int $k = 1;\n    string $s = match ($k) { 1 => $t, default => \"b\" };"
        )),
    ),
    (
        "Catch",
        Row::Text(with_box!(
            "    string $s = Box::fail() catch (RuntimeError $e) => \"b\";\n    echo $t;"
        )),
    ),
    (
        "Catch",
        Row::Text(with_box!(
            "    string $s = Box::fail() catch (RuntimeError $e) => $t;"
        )),
    ),
    (
        "Yield",
        Row::Text(concat!(
            "<?nvs\nclass Fixture {\n",
            "  static function m({T} $t): Iterator<string> {\n    yield $t;\n  }\n}\n"
        )),
    ),
    (
        "YieldFrom",
        Row::NoText("`yield from` does not compile, so it gives no value."),
    ),
    ("Print", Row::NoText("`print` gives `1`.")),
    ("Throw", Row::NoText("`throw` gives no value.")),
    ("Isset", Row::NoText("`isset` gives a boolean.")),
    ("IsInit", Row::NoText("`isinit` gives a boolean.")),
    ("Empty", Row::NoText("`empty` gives a boolean.")),
    ("Exit", Row::NoText("`exit` gives no value.")),
    ("SpawnScript", Row::NoText("`spawn script` gives a handle.")),
    (
        "Await",
        Row::Outside(in_method!(
            "    var $h = spawn script \"c.nvs\";\n    string $s = (await $h)->value as string;\n    echo $t;"
        )),
    ),
    (
        "Require",
        Row::NoText(
            "a required file's result is the program's own code, checked where it is written.",
        ),
    ),
    ("Paren", Row::Text(in_method!("    string $s = ($t);"))),
    (
        "AnonObject",
        Row::Text(in_method!(
            "    var $o = { v: $t, w: 1 };\n    string $s = $o->v;"
        )),
    ),
    (
        "Error",
        Row::NoText("a recovery placeholder: the parser already reported an error."),
    ),
];

/// Statement forms that move text into a written type without an
/// expression of their own.
const STATEMENTS: &[(&str, &str)] = &[
    (
        "destructuring",
        in_method!(
            "    array<{T}> $p = [$t, \"b\"];\n    [string $a, string $b] = $p;\n    echo $a . $b;"
        ),
    ),
    (
        "keyed destructuring",
        in_method!(
            "    array<{T}> $p = [\"k\" => $t];\n    [\"k\" => string $a] = $p;\n    echo $a;"
        ),
    ),
    (
        "foreach value",
        in_method!("    array<{T}> $p = [$t];\n    foreach ($p as string $v) { echo $v; }"),
    ),
    (
        "foreach key",
        in_method!(
            "    var $p = [$t => 1];\n    foreach ($p as string $k => int $v) { echo $k . $v; }"
        ),
    ),
    (
        "return",
        concat!(
            "<?nvs\nclass Fixture {\n",
            "  static function m({T} $t): string {\n    return $t;\n  }\n}\n"
        ),
    ),
    (
        "property assignment",
        concat!(
            "<?nvs\nclass Holder { public string $v = \"\"; }\n",
            "class Fixture {\n  static function m({T} $t): void {\n",
            "    Holder $h = new Holder();\n    $h->v = $t;\n  }\n}\n"
        ),
    ),
];

/// `src`'s diagnostics, from resolution and checking both.
fn check_src(src: &str) -> Diagnostics {
    check_src_table_allowing_errors(src).0
}

/// Whether `diags` has an error that names a `tainted` value, in its message
/// or a label.
fn names_taint(diags: &Diagnostics) -> bool {
    diags.iter().any(|d| {
        d.severity == Severity::Error
            && (d.message.contains("tainted")
                || d.labels.iter().any(|l| l.message.contains("tainted")))
    })
}

/// `src` with `{T}` written as `ty`.
fn program(src: &str, ty: &str) -> String {
    src.replace("{T}", ty)
}

/// The failure for one program whose clean form or tainted form is wrong,
/// or `None`.
fn judge(label: &str, src: &str, outside: bool) -> Option<String> {
    let clean = program(src, "string");
    let diags = check_src(&clean);
    if !outside && diags.has_errors() {
        return Some(format!(
            "{label}: the clean form does not compile: {diags:?}\n{clean}"
        ));
    }
    let tainted = program(src, "tainted string");
    let diags = check_src(&tainted);
    if !names_taint(&diags) {
        return Some(format!(
            "{label}: the result is not tainted: {diags:?}\n{tainted}"
        ));
    }
    if outside && !names_taint(&check_src(&clean)) {
        return Some(format!(
            "{label}: outside text reached a plain `string`\n{clean}"
        ));
    }
    None
}

/// `Core\Json::decode` writes a repeated key of the document into its
/// error's message, so every `Throwable::$message` is `tainted string`. The
/// constructor takes one, so a program can wrap another error's message.
#[test]
fn a_throwable_message_is_tainted_and_a_new_error_takes_one() {
    let keep = in_method!(
        "    try { echo $t; } catch (RuntimeError $e) {\n      string $m = $e->message;\n    }"
    );
    assert!(names_taint(&check_src(&program(keep, "string"))));

    let wrap = in_method!(
        "    try { echo $t; } catch (RuntimeError $e) {\n      \
         throw new LogicError('failed: ' . $e->message);\n    }"
    );
    let diags = check_src(&program(wrap, "string"));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn every_expression_kind_has_a_row() {
    let missing: Vec<&str> = ExprKind::NAMES
        .iter()
        .copied()
        .filter(|name| !ROWS.iter().any(|(kind, _)| kind == name))
        .collect();
    assert!(missing.is_empty(), "kinds with no taint row: {missing:?}");
    let unknown: Vec<&str> = ROWS
        .iter()
        .map(|(kind, _)| *kind)
        .filter(|kind| !ExprKind::NAMES.contains(kind))
        .collect();
    assert!(unknown.is_empty(), "rows for no kind: {unknown:?}");
}

#[test]
fn a_no_text_row_says_why() {
    for (kind, row) in ROWS {
        if let Row::NoText(reason) = row {
            assert!(
                !reason.trim().is_empty(),
                "{kind}: a no-text row with no reason"
            );
        }
    }
}

#[test]
fn each_row_contains_its_kind() {
    let mut failures = Vec::new();
    for (kind, row) in ROWS {
        let src = match row {
            Row::Text(src) | Row::Outside(src) => src,
            Row::NoText(_) => continue,
        };
        match walk::of_source("t.nvs", &program(src, "string")) {
            Ok(tree) if tree.descendants().iter().any(|node| node.kind == *kind) => {}
            Ok(_) => failures.push(format!("{kind}: its row has no such expression\n{src}")),
            Err(e) => failures.push(format!("{kind}: its row does not parse: {e}\n{src}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn a_tainted_operand_gives_a_tainted_result_in_every_form() {
    let mut failures = Vec::new();
    for (kind, row) in ROWS {
        let failure = match row {
            Row::Text(src) => judge(kind, src, false),
            Row::Outside(src) => judge(kind, src, true),
            Row::NoText(_) => None,
        };
        failures.extend(failure);
    }
    for (label, src) in STATEMENTS {
        failures.extend(judge(label, src, false));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
