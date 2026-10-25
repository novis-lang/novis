//! `Core\\Str` end to end, and `rule:core-api/shape-rules` R2's options bag: omission, name-matching, and a rejected option.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

#[test]
fn a_core_str_member_runs_and_hands_its_result_back_as_a_string() {
    let source = "<?nvs
echo Core\\Str::upper(\"nvs\"), \"|\", Core\\Str::upperFirst(\"runs\"), \"|\", Core\\Str::repeat(\"ab\", 2);
";
    assert_eq!(output_of(source), "NVS|Runs|abab");
}

/// The `Core` half of the parameter-default mechanism: `join`'s separator and
/// `padStart`'s padding come from `nvs_stdlib::registry::CoreMethod::defaults`
/// through the same call-site materialization a written `= expr` uses, so the
/// helper still receives its declared arity.
#[test]
fn a_core_member_may_have_its_trailing_argument_omitted() {
    let source = "<?nvs
array<string> $parts = [\"a\", \"b\", \"c\"];
echo Core\\Str::join($parts), \"|\", Core\\Str::join($parts, \"-\");
echo \"|\", Core\\Str::padStart(\"7\", 3, \"0\"), \"|\", Core\\Str::padStart(\"7\", 3);
";
    assert_eq!(output_of(source), "abc|a-b-c|007|  7");
}

/// `rule:core-api/shape-rules` R2's options bag end to end. The bag has no runtime
/// representation at all — `nvs_ir::lower::lower_call_args` flattens it into
/// one ordinary argument per declared option — so this is the check that the
/// flattened arity and the helper's own `args: [3]` agree, in both the
/// written and the omitted case.
#[test]
fn a_core_member_takes_an_options_shape_that_may_be_omitted() {
    let source = "<?nvs
echo Core\\Arr::count(Core\\Arr::range(1, 10));
echo \"|\", Core\\Arr::count(Core\\Arr::range(1, 10, {step: 3}));
echo \"|\", Core\\Arr::count(Core\\Arr::range(10, 1, {step: 2}));
";
    assert_eq!(output_of(source), "10|4|5");
}

/// A `Core` parameter declared `int|string` takes either, at the argument's
/// own representation: `nvs-codegen` writes each helper argument's tag from
/// the value it is passing, so the union never needs an IR type of its own.
/// `nvs_ir::lower::ArgSig::helper` owns why that is the helper convention's
/// property and not this member's.
#[test]
fn a_core_union_parameter_arrives_at_the_helper_tagged_as_what_was_passed() {
    let source = "<?nvs
array<int> $a = [\"name\" => 1, 5 => 2];
if (Core\\Arr::hasKey($a, \"name\")) { echo \"y\"; } else { echo \"n\"; }
if (Core\\Arr::hasKey($a, 5)) { echo \"y\"; } else { echo \"n\"; }
if (Core\\Arr::hasKey($a, \"5\")) { echo \"y\"; } else { echo \"n\"; }
if (Core\\Arr::hasKey($a, 6)) { echo \"y\"; } else { echo \"n\"; }
";
    assert_eq!(output_of(source), "yyyn");
}

/// A bag with *two* options, written in every combination: neither, one, the
/// other, both, and the second one written first. Options are named rather
/// than positional, so the written order is not the ABI order — this is what
/// holds `lower_options_arg`'s "walk the declared options, look each one up"
/// against the easier and wrong "walk the written fields".
#[test]
fn an_option_is_matched_by_name_not_by_the_order_it_was_written() {
    let source = "<?nvs
var $s = \"a-b-A-b\";
echo Core\\Str::replace($s, \"-\", \"+\");
echo \"|\", Core\\Str::replace($s, \"-\", \"+\", {limit: 2});
echo \"|\", Core\\Str::replace($s, \"a\", \"z\", {caseInsensitive: true});
echo \"|\", Core\\Str::replace($s, \"a\", \"z\", {limit: 1, caseInsensitive: true});
";
    assert_eq!(output_of(source), "a+b+A+b|a+b+A-b|z-b-z-b|z-b-A-b");
}

/// A `Core` member may throw over an option it was given: `range`'s `step`
/// must be positive, PHP 8.5's own rule. Proves the option reached the helper
/// as a real argument rather than being dropped on the way — an option that
/// never arrived would have taken its default of `1` and succeeded.
///
/// Asserted as a status rather than through a `catch`, which is the narrower
/// claim: what is under test is that the option reached the helper at all, and
/// a status says so without also depending on the promotion `catch` needs. The
/// `nvs_stdlib` unit test beside the member covers the message itself.
#[test]
fn an_option_a_core_member_rejects_throws_through_the_helper_boundary() {
    let source = "<?nvs
echo Core\\Arr::count(Core\\Arr::range(1, 5, {step: 0}));
";
    let mut ctx = Ctx::buffered();
    let status = run_with(&mut ctx, source).expect_err("a step of 0 is refused");
    assert_eq!(status, nvs_runtime::THROWN);
}

/// How many `Core\Str::format` calls the lowered `source` makes.
fn format_calls(source: &str) -> usize {
    lower(source)
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.insts.iter())
        .filter(|inst| {
            matches!(inst.kind, nvs_ir::ir::InstKind::CoreCall { symbol, .. }
                if symbol.ends_with("core_str_format"))
        })
        .count()
}

/// Every literal template lowers to a concatenation: plain placeholders, a
/// width, a flag, an argument number, a float conversion, a conversion that
/// can throw and a width past four digits. A template that is not a constant
/// stays a call.
#[test]
fn a_literal_format_template_lowers_to_a_concatenation() {
    let lowered = "<?nvs
int $n = 3;
string $s = \"x\";
echo Core\\Str::format(\"%d-%s-%s 100%%\", $n, $s, $n);
";
    assert_eq!(format_calls(lowered), 0, "a plain template stayed a call");

    for (kept, why) in [
        ("Core\\Str::format(\"%05d\", $n)", "nothing: a width lowers"),
        (
            "Core\\Str::format('%1$d', $n)",
            "nothing: an argument number lowers",
        ),
        (
            "Core\\Str::format('%2$s %1$s %2$s', $s, $n)",
            "nothing: arguments read out of order lower",
        ),
        ("Core\\Str::format(\"%d\", $u)", "nothing: a `uint` lowers"),
        ("Core\\Str::format(\"%x\", $n)", "nothing: `%x` lowers"),
        (
            "Core\\Str::format(\"%-6.2s|\", $s)",
            "nothing: a precision lowers",
        ),
        (
            "Core\\Str::format(\"%.2f\", $f)",
            "nothing: a float conversion lowers",
        ),
        (
            "Core\\Str::format(\"%d\", $f)",
            "nothing: a `float` under `%d` lowers",
        ),
        (
            "Core\\Str::format(\"%10000d\", $n)",
            "nothing: a width of five digits lowers",
        ),
        ("Core\\Str::format(\"\")", "an empty template"),
        (
            "Core\\Str::format(\"%s %s\", $s, $s)",
            "nothing: this one lowers",
        ),
        (
            "Core\\Str::format($t, $n)",
            "a template that is not a constant",
        ),
    ] {
        let source = format!(
            "<?nvs
int $n = 3;
uint $u = 3;
float $f = 1.5;
string $s = \"x\";
string $t = Core\\Str::repeat(\"%d\", 1);
echo {kept};
"
        );
        let expected = usize::from(!why.starts_with("nothing"));
        assert_eq!(format_calls(&source), expected, "for {why}");
    }
}

/// The concatenation prints exactly what the member prints, for the values
/// where a conversion could go wrong: the smallest and largest `int`, a
/// negative number, and strings holding `%`, a multi-byte character and HTML.
/// The member is reached through a parameter, which is not a constant.
#[test]
fn a_lowered_format_prints_what_the_member_prints() {
    let source = "<?nvs
class Dynamic {
    public static function render(string $template, int $a, string $b, int $c): string {
        return Core\\Str::format($template, $a, $b, $c);
    }
}
array<int> $ints = [0, -1, 7, 9223372036854775807, -9223372036854775807 - 1];
array<string> $texts = [\"\", \"50%\", \"é & <b>\", \"%d\"];
foreach ($ints as int $a) {
    foreach ($texts as string $b) {
        string $lowered = Core\\Str::format(\"a%db%sc%%d%s!\", $a, $b, $a);
        string $called = Dynamic::render(\"a%db%sc%%d%s!\", $a, $b, $a);
        if ($lowered != $called) {
            echo \"differs: \", $lowered, \" | \", $called, \"\\n\";
        }
    }
}
echo Core\\Str::format(\"a%db%sc%%d%s!\", -12, \"x\", 5);
";
    assert_eq!(output_of(source), "a-12bxc%d5!");
}

/// A template that numbers its arguments, converts a float or reads a `mixed`
/// prints what the member prints, for the same values. An argument read twice
/// is evaluated once. The member is reached through a parameter, which is not
/// a constant.
#[test]
fn a_rendered_template_prints_what_the_member_prints() {
    let source = "<?nvs
class Dynamic {
    public static function render(string $template, float $f, mixed $m, string $s): tainted string {
        return Core\\Str::format($template, $f, $m, $s);
    }
}
class Count {
    public static int $calls = 0;
    public static function once(): string {
        Count::$calls = Count::$calls + 1;
        return \"o\";
    }
}
array<float> $floats = [0.0, -1.5, 3.14159, 1e300];
array<mixed> $mixed = [7, \"x\", true, 2.5];
foreach ($floats as float $f) {
    foreach ($mixed as mixed $m) {
        tainted string $lowered = Core\\Str::format('%3$s|%1$.2f|%2$s|%1$e|%3$5s|%%', $f, $m, \"é\");
        tainted string $called = Dynamic::render('%3$s|%1$.2f|%2$s|%1$e|%3$5s|%%', $f, $m, \"é\");
        if ($lowered != $called) {
            echo \"differs\\n\";
        }
    }
}
echo Core\\Str::format('%1$s%1$s', Count::once()), \" \", Count::$calls;
";
    assert_eq!(output_of(source), "oo 1");
}

/// A rendered placeholder that throws does so after every argument was
/// evaluated, with the member's own message, as the member does. The fresh
/// argument it was given is freed, so the balance does not grow with the loop.
#[test]
fn a_rendered_placeholder_throws_after_every_argument_ran() {
    let source = "<?nvs
class Side {
    public static function mark(): string {
        echo \"evaluated \";
        return Core\\Str::repeat(\"s\", 3);
    }
}
mixed $m = \"x\";
try {
    echo Core\\Str::format(\"%d|%s\", $m, Side::mark());
} catch (LogicError $error) {
    echo $error->message;
}
";
    assert_eq!(
        output_of(source),
        "evaluated Core\\Str::format(): `%d` has no reading for this value"
    );

    let balance = |count: usize| {
        let source = format!(
            "<?nvs
mixed $m = \"x\";
for (int $i = 0; $i < {count}; $i++) {{
    try {{
        echo Core\\Str::format('%2$s%1$d', $m, Core\\Str::repeat(\"ab\", 2));
    }} catch (LogicError $error) {{
    }}
}}
"
        );
        let unit = compile(&source).expect("the fixture compiles");
        let mut ctx = Ctx::buffered();
        unit.install_in(&mut ctx);
        let entry = unit.script().expect("the script frame was compiled");
        let before = nvs_runtime::budget::live_bytes();
        entry.call(&mut ctx).expect("the script ran to completion");
        nvs_runtime::budget::live_bytes() - before
    };
    assert_eq!(
        balance(10),
        balance(1000),
        "a thrown render left a piece or an argument behind"
    );
}

/// A placeholder with a flag, a width or a precision prints what the member
/// prints, for the same values. The member is reached through a parameter,
/// which is not a constant.
#[test]
fn a_lowered_placeholder_with_flags_prints_what_the_member_prints() {
    let source = "<?nvs
class Dynamic {
    public static function ints(string $template, int $a, uint $b): string {
        return Core\\Str::format($template, $a, $a, $a, $a, $b, $b);
    }
    public static function text(string $template, string $a, int $b): string {
        return Core\\Str::format($template, $a, $b, $b);
    }
}
array<int> $ints = [0, -1, 7, 255, 9223372036854775807, -9223372036854775807 - 1];
array<uint> $uints = [0, 7, 18446744073709551615];
array<string> $texts = [\"\", \"ab\", \"é & <b>\", \"%d\"];
int $checked = 0;
foreach ($ints as int $a) {
    foreach ($uints as uint $b) {
        string $lowered = Core\\Str::format(\"[%05d|%+d|%o %b|%'*8u|%X]\", $a, $a, $a, $a, $b, $b);
        string $called = Dynamic::ints(\"[%05d|%+d|%o %b|%'*8u|%X]\", $a, $b);
        $checked = $checked + 1;
        if ($lowered != $called) {
            echo \"differs: \", $lowered, \" | \", $called, \"\\n\";
        }
    }
}
foreach ($texts as string $t) {
    foreach ($ints as int $n) {
        string $padded = Core\\Str::format(\"<%-6.2s|%5s|%08d>\", $t, $n, $n);
        string $member = Dynamic::text(\"<%-6.2s|%5s|%08d>\", $t, $n);
        $checked = $checked + 1;
        if ($padded != $member) {
            echo \"differs: \", $padded, \" | \", $member, \"\\n\";
        }
    }
}
echo $checked, \" \", Core\\Str::format(\"%05d|%x|%-3s|\", 42, 255, \"a\");
";
    assert_eq!(output_of(source), "42 00042|ff|a  |");
}

/// A converted placeholder frees a `string` argument it was given fresh. The
/// balance after a run does not grow with the loop's length, so nothing is
/// left behind per call.
#[test]
fn a_lowered_placeholder_frees_a_fresh_argument() {
    let balance = |count: usize| {
        let source = format!(
            "<?nvs
string $kept = \"\";
for (int $i = 0; $i < {count}; $i++) {{
    $kept = Core\\Str::format(\"%5s|%-4s\", Core\\Str::repeat(\"ab\", 2), $kept);
    $kept = Core\\Str::format(\"%.1s\", $kept);
}}
"
        );
        let unit = compile(&source).expect("the fixture compiles");
        let mut ctx = Ctx::buffered();
        unit.install_in(&mut ctx);
        let entry = unit.script().expect("the script frame was compiled");
        let before = nvs_runtime::budget::live_bytes();
        entry.call(&mut ctx).expect("the script ran to completion");
        nvs_runtime::budget::live_bytes() - before
    };
    assert_eq!(
        format_calls(
            "<?nvs\nstring $s = \"a\";\necho Core\\Str::format(\"%5s|%-4s\", Core\\Str::repeat(\"ab\", 2), $s);\n"
        ),
        0
    );
    assert_eq!(balance(10), balance(1000));
}

/// How many allocation requests the second run of `source` makes. The first
/// run pays for whatever the runtime sets up once.
fn allocations_of_run(source: &str) -> usize {
    let unit = compile(source).expect("the fixture compiles");
    let entry = unit.script().expect("the script frame was compiled");
    let mut ctx = Ctx::buffered();
    entry.call(&mut ctx).expect("the script ran to completion");
    let before = nvs_runtime::budget::allocations();
    entry.call(&mut ctx).expect("the script ran to completion");
    nvs_runtime::budget::allocations() - before
}

/// A lowered `%d` call allocates twice: the integer's text and the joined
/// result. The count is the difference between two loop lengths, so the
/// script's own fixed cost cancels out.
#[test]
fn a_lowered_format_allocates_the_integer_text_and_the_result() {
    let looped = |count: usize| {
        format!(
            "<?nvs
string $s = \"\";
int $i = 0;
while ($i < {count}) {{
    $s = Core\\Str::format(\"item %d\", $i);
    $i = $i + 1;
}}
echo $s;
"
        )
    };
    let spent = allocations_of_run(&looped(40)) - allocations_of_run(&looped(4));
    assert_eq!(spent, 36 * 2, "36 more calls made {spent} more allocations");
}
