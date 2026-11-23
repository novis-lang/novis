//! The replacement fuzzer: small programs that replace a value with one made
//! from it, each run twice, with the ownership pass and without it
//! (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! The pass decides moves and lends, and must never change what a program
//! prints. So [`in_place`] draws a program from its input, compiles it both
//! ways (`nvs_ir::lower::without_in_place`), runs each, and fails on any
//! difference in output or status.
//!
//! The run without the pass, which builds a new array, is the reference. After
//! any member, the next appended key is the one that run gives, whatever
//! storage the in-place run ends with. The member, cut and append step tests
//! exactly that.
//!
//! A program is a run of steps over the axes of the matrix under
//! `tests/conformance/in-place/`: the kind (`array<int>` or `string`), the
//! target (a local, an element, a static property, a property or an element
//! of a property), the replacing form (an owning `Core` member, one with a
//! callback, a Novis function that is safe, can fail, reads the slot before or
//! after its change, or writes it, a concatenation, an interpolation, a spread,
//! an append, and a member that gives new keys followed by a cut and an
//! append), and the context (straight, a loop, a `try` with a `catch`,
//! a `finally` and a generator). A step may first share its target with a
//! second variable, which it prints after the change, and a function that can
//! fail fails only inside a `try`, whose `catch` prints the target.
//!
//! Every program the generator writes type-checks, so a program the checker
//! refuses is a bug in the generator, and fails the check too.
//!
//! **Memory:** both compiled units of one program at a time, and the output
//! of each run.

use std::fmt::Write as _;

/// What one run of a program gave: its status and what it printed.
#[derive(Debug, PartialEq, Eq)]
struct Ran {
    status: i32,
    output: String,
}

/// The check: draws a program from `data`, runs it with and without the
/// ownership pass, and panics when the two runs differ.
pub fn in_place(data: &[u8]) {
    let text = program(data);
    let with = run(&text, false);
    let without = run(&text, true);
    // A function that can fail fails only inside a `try`, so every program
    // ends normally, and a run that stopped early is a bug of its own.
    assert_eq!(
        with.status,
        nvs_runtime::OK,
        "this program did not finish:\n{text}\n{}",
        with.output
    );
    assert_eq!(
        with, without,
        "this program runs differently with in-place changes switched off:\n{text}"
    );
}

/// Compiles `text`, with the ownership pass unless `off`, runs its script on a
/// buffered context, and gives the status and the output.
fn run(text: &str, off: bool) -> Ran {
    let lowered = if off {
        nvs_ir::lower::without_in_place(|| crate::compile::lower_source(text))
    } else {
        crate::compile::lower_source(text)
    };
    let program = lowered.unwrap_or_else(|first| {
        panic!("the generator wrote a program that does not check: {first}\n{text}")
    });
    let unit = nvs_codegen::compile(&program).unwrap_or_else(|error| {
        panic!("a generated program failed code generation: {error}\n{text}")
    });
    let mut ctx = nvs_runtime::Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    let status = match entry.call(&mut ctx) {
        Ok(_) => nvs_runtime::OK,
        Err(status) => status,
    };
    let output = ctx.take_buffered_output().unwrap_or_default();
    drop(ctx);
    drop(unit);
    Ran {
        status,
        output: String::from_utf8_lossy(&output).into_owned(),
    }
}

/// Reads choices from the input, one byte each. An input that is used up
/// gives 0 for every later choice, so every input is a program.
struct Draw<'a> {
    data: &'a [u8],
    at: usize,
}

impl Draw<'_> {
    /// A number below `n`.
    fn pick(&mut self, n: usize) -> usize {
        let byte = self.data.get(self.at).copied().unwrap_or(0);
        self.at += 1;
        usize::from(byte) % n
    }

    /// One of `options`.
    fn one<'s>(&mut self, options: &[&'s str]) -> &'s str {
        options[self.pick(options.len())]
    }
}

/// The value kind a step replaces.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Array,
    Str,
}

/// The context a step's statement runs in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    Straight,
    Loop,
    Try,
    Finally,
    Generator,
}

/// The classes every program starts with: the slots a step can replace, and
/// the Novis functions a step can call.
const PRELUDE: &str = r#"<?nvs
final class Store {
    public static array<int> $a = [];
    public static string $s = "";
}

final class Box {
    public array<int> $a = [];
    public string $s;
    public array<array<int>> $g = [];

    public function constructor() {
        $this->a = [1, 2];
        $this->s = "ab";
        $this->g["k"] = [5, 6];
    }
}

final class Lib {
    public static function show(array<int> $items): string {
        var $out = "";
        foreach ($items as string $k => int $v) {
            $out = $out . $k . "=" . ($v as string) . ",";
        }
        return $out;
    }

    public static function addSafe(array<int> $items, int $x): array<int> {
        $items[] = $x;
        return $items;
    }

    public static function addFails(array<int> $items, int $x): array<int> {
        $items[] = $x;
        if ($x < 0) {
            throw new LogicError("negative");
        }
        return $items;
    }

    public static function addReadsBefore(array<int> $items, int $x): array<int> {
        echo "before: ", Lib::show(Store::$a), "\n";
        $items[] = $x;
        return $items;
    }

    public static function addReadsAfter(array<int> $items, int $x): array<int> {
        $items[] = $x;
        echo "after: ", Lib::show(Store::$a), "\n";
        return $items;
    }

    public static function addWrites(array<int> $items, int $x): array<int> {
        Store::$a = [7];
        $items[] = $x;
        return $items;
    }

    public static function catSafe(string $text, int $x): string {
        $text = $text . ($x as string);
        return $text;
    }

    public static function catFails(string $text, int $x): string {
        $text = $text . "f";
        if ($x < 0) {
            throw new LogicError("negative");
        }
        return $text;
    }

    public static function catReadsBefore(string $text, int $x): string {
        echo "before: ", Store::$s, "\n";
        $text = $text . ($x as string);
        return $text;
    }

    public static function catReadsAfter(string $text, int $x): string {
        $text = $text . ($x as string);
        echo "after: ", Store::$s, "\n";
        return $text;
    }

    public static function catWrites(string $text, int $x): string {
        Store::$s = "w";
        $text = $text . ($x as string);
        return $text;
    }
}
"#;

/// The setup the script runs before its steps.
const SETUP: &str = r#"
var $box = new Box();
Store::$a = [3, 4];
Store::$s = "cd";
array<int> $a = [8, 9];
string $s = "ef";
array<array<int>> $rows = [];
$rows["k"] = [1, 2, 3];
array<string> $texts = [];
$texts["k"] = "gh";
"#;

/// The array targets a script step can replace.
const ARRAY_TARGETS: [&str; 5] = [
    "$a",
    "$rows[\"k\"]",
    "Store::$a",
    "$box->a",
    "$box->g[\"k\"]",
];

/// The string targets a script step can replace. The last two cannot open an
/// interpolation, which [`statement`] reads from their position.
const STRING_TARGETS: [&str; 4] = ["$s", "$box->s", "$texts[\"k\"]", "Store::$s"];

/// The program `data` draws: the prelude, one generator class for the steps
/// that run in a generator, and the script.
#[must_use]
fn program(data: &[u8]) -> String {
    let mut draw = Draw { data, at: 0 };
    let steps = 1 + draw.pick(5);
    let mut script = String::from(SETUP);
    let mut generators = String::new();
    for step in 0..steps {
        let kind = if draw.pick(2) == 0 {
            Kind::Array
        } else {
            Kind::Str
        };
        let context = match draw.pick(5) {
            0 => Context::Straight,
            1 => Context::Loop,
            2 => Context::Try,
            3 => Context::Finally,
            _ => Context::Generator,
        };
        if context == Context::Generator {
            generator_step(&mut draw, step, kind, &mut generators, &mut script);
        } else {
            script_step(&mut draw, step, kind, context, &mut script);
        }
    }
    for target in ARRAY_TARGETS {
        let _ = writeln!(script, "echo Lib::show({target}), \"\\n\";");
    }
    for target in STRING_TARGETS {
        let _ = writeln!(script, "echo {target}, \"\\n\";");
    }
    format!("{PRELUDE}\nfinal class Gen {{\n{generators}}}\n{script}")
}

/// The expression that prints `target` of `kind`.
fn shown(kind: Kind, target: &str) -> String {
    match kind {
        Kind::Array => format!("Lib::show({target})"),
        Kind::Str => target.to_owned(),
    }
}

/// The statement that replaces `target`. `interpolates` says whether the
/// target can open an interpolation, and `may_fail` whether a function that
/// can fail is given a value that makes it fail.
fn statement(
    draw: &mut Draw<'_>,
    kind: Kind,
    target: &str,
    interpolates: bool,
    may_fail: bool,
) -> String {
    let n = if may_fail && draw.pick(2) == 0 {
        "-1"
    } else {
        draw.one(&["3", "4"])
    };
    match kind {
        Kind::Array => match draw.pick(11) {
            0 => format!("{target} = Core\\Arr::append({target}, {n});"),
            1 => format!("{target} = Core\\Arr::reverse({target});"),
            2 => format!("{target} = Core\\Arr::slice({target}, 1);"),
            3 => format!("{target} = Core\\Arr::map({target}, fn(int $v): int => $v * 10);"),
            4 => {
                format!("{target} = Core\\Arr::filter({target}, fn(int $v): bool => $v % 2 == 0);")
            }
            5 => {
                let function = draw.one(&[
                    "addSafe",
                    "addFails",
                    "addReadsBefore",
                    "addReadsAfter",
                    "addWrites",
                ]);
                format!("{target} = Lib::{function}({target}, {n});")
            }
            6 => format!("{target} = [...{target}, {n}];"),
            7 => format!("{target}[] = {n};"),
            8 => {
                // A member that gives new keys, then a cut, then an append:
                // the append lands where it lands on the built array.
                let member = draw.one(&[
                    r"Core\Arr::mapKeys({t}, fn(int $v, string $k): string => $k)",
                    r"Core\Arr::mapKeys({t}, fn(int $v): int => $v)",
                    r"Core\Arr::values({t})",
                ]);
                let cut = draw.one(&[
                    r"Core\Arr::withoutLast({t})",
                    r"Core\Arr::withoutFirst({t})",
                    r"Core\Arr::filter({t}, fn(int $v): bool => $v % 2 == 0)",
                ]);
                let append = draw.one(&["{t}[] = {n};", r"{t} = Core\Arr::append({t}, {n});"]);
                format!(
                    "{target} = {}; {target} = {}; {}",
                    member.replace("{t}", target),
                    cut.replace("{t}", target),
                    append.replace("{t}", target).replace("{n}", n),
                )
            }
            _ => format!("{target} = Lib::addFails({target}, {n});"),
        },
        Kind::Str => match draw.pick(9) {
            0 => format!("{target} = Core\\Str::upper({target});"),
            1 => format!("{target} = Core\\Str::lower({target});"),
            2 => format!(
                "{target} = Core\\Regex::replaceWith({target}, \"[aeiou]\", fn(Core\\Regex\\Match $m): string => \"0\");"
            ),
            3 => {
                let function = draw.one(&[
                    "catSafe",
                    "catFails",
                    "catReadsBefore",
                    "catReadsAfter",
                    "catWrites",
                ]);
                format!("{target} = Lib::{function}({target}, {n});")
            }
            4 => format!("{target} = {target} . \"x\";"),
            5 if interpolates => format!("{target} = \"{{{target}}}y\";"),
            5 => format!("{target} = \"y\" . {target};"),
            6 => format!("{target} .= \"z\";"),
            7 => format!("{target} = {target} . \"-\" . {target};"),
            _ => format!("{target} = Lib::catFails({target}, {n});"),
        },
    }
}

/// One step of the script: an optional share, the statement in its context,
/// and a print of the target and of the share.
fn script_step(draw: &mut Draw<'_>, step: usize, kind: Kind, context: Context, out: &mut String) {
    let (target, interpolates, ty) = match kind {
        Kind::Array => (
            ARRAY_TARGETS[draw.pick(ARRAY_TARGETS.len())],
            false,
            "array<int>",
        ),
        Kind::Str => {
            let at = draw.pick(STRING_TARGETS.len());
            (STRING_TARGETS[at], at < 2, "string")
        }
    };
    let shared = draw.pick(3) == 0;
    if shared {
        let _ = writeln!(out, "{ty} $keep{step} = {target};");
    }
    let show = shown(kind, target);
    let stmt = statement(draw, kind, target, interpolates, context == Context::Try);
    match context {
        Context::Loop => {
            let _ = writeln!(
                out,
                "var $i{step} = 0;\nwhile ($i{step} < 3) {{\n    {stmt}\n    $i{step} = $i{step} + 1;\n}}"
            );
        }
        Context::Try => {
            let _ = writeln!(
                out,
                "try {{\n    {stmt}\n    echo {show}, \"\\n\";\n    throw new LogicError(\"after\");\n}} catch (LogicError $e) {{\n    echo $e->message, \": \", {show}, \"\\n\";\n}}"
            );
        }
        Context::Finally => {
            let _ = writeln!(
                out,
                "try {{\n    {stmt}\n}} finally {{\n    echo \"finally: \", {show}, \"\\n\";\n}}"
            );
        }
        Context::Straight | Context::Generator => {
            let _ = writeln!(out, "{stmt}");
        }
    }
    let _ = writeln!(out, "echo {show}, \"\\n\";");
    if shared {
        let _ = writeln!(
            out,
            "echo {}, \"\\n\";",
            shown(kind, &format!("$keep{step}"))
        );
    }
}

/// One step in a generator: a method of `Gen` that yields its local, replaces
/// it, and yields it again, and a script loop that keeps the first value and
/// prints it after the change.
fn generator_step(
    draw: &mut Draw<'_>,
    step: usize,
    kind: Kind,
    methods: &mut String,
    out: &mut String,
) {
    let (ty, start) = match kind {
        Kind::Array => ("array<int>", "[1, 2]"),
        Kind::Str => ("string", "\"ab\""),
    };
    let stmt = statement(draw, kind, "$t", true, false);
    let _ = writeln!(
        methods,
        "    public static function steps{step}(): Iterator<{ty}> {{\n        {ty} $t = {start};\n        yield $t;\n        {stmt}\n        yield $t;\n    }}\n"
    );
    let _ = writeln!(
        out,
        "{ty} $kept{step} = {start};\nbool $first{step} = true;\nforeach (Gen::steps{step}() as {ty} $v{step}) {{\n    if ($first{step}) {{\n        $kept{step} = $v{step};\n        $first{step} = false;\n    }}\n    echo {}, \"\\n\";\n}}\necho {}, \"\\n\";",
        shown(kind, &format!("$v{step}")),
        shown(kind, &format!("$kept{step}"))
    );
}

#[cfg(test)]
mod tests {
    /// The input bytes of the fixed seed `seed`: a SplitMix64 stream, so every
    /// run of the test draws the same programs.
    fn input(seed: u64) -> Vec<u8> {
        let mut state = seed;
        (0..64)
            .flat_map(|_| {
                state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
                let mut z = state;
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                (z ^ (z >> 31)).to_le_bytes()
            })
            .collect()
    }

    #[test]
    fn replacement_programs_print_the_same_without_in_place() {
        for seed in 0..128 {
            super::in_place(&input(seed));
        }
    }

    #[test]
    fn an_empty_input_is_a_program_too() {
        super::in_place(&[]);
    }
}
