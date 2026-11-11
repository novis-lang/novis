//! A borrowed read keeps its value while a later operand runs code that
//! replaces the storage it was read from
//! (`rule:expressions/an-operand-keeps-its-value-while-later-operands-run`).
//!
//! Each program stores a heap string, reads it as an operand, and then runs a
//! later operand that replaces the storage and allocates strings of the same
//! size. A read that did not keep its own reference prints the bytes of a
//! string allocated afterwards, or crashes.
//!
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_ir::lower::OperandSite;

/// The class every program below uses. `replace()` overwrites every slot a
/// read form reads from with strings of the same length, then allocates more
/// strings of that length. Some heaps reuse a freed block only after many
/// others, so the fill is large enough that the freed block is reused.
const HOLDER: &str = r#"<?nvs
class Version implements Comparable {
    public string $tag;

    public function constructor(string $tag) {
        $this->tag = $tag;
    }

    public function compareTo(self $other): int {
        return $this->tag == $other->tag ? 0 : 1;
    }
}

class Holder {
    public string $s = "";
    public array<string> $items = [];
    public static string $shared = "";
    public Version $v;
    public Core\Time\Date $d;

    public function constructor() {
        int $n = 1;
        $this->v = new Version("old-" . $n . "-abcdefghijklmnop");
        $this->d = Core\Time\Date::at(1999, 1, 1);
    }

    public function replace(): string {
        int $n = 2;
        $this->s = "new-" . $n . "-abcdefghijklmnop";
        $this->items = ["new-" . $n . "-abcdefghijklmnop"];
        Holder::$shared = "new-" . $n . "-abcdefghijklmnop";
        $this->v = new Version("new-" . $n . "-abcdefghijklmnop");
        $this->d = Core\Time\Date::at(2002, 2, 2);
        array<string> $fill = [];
        array<Core\Time\Date> $dates = [];
        for (int $i = 0; $i < 1024; $i++) {
            $fill[] = "fil-" . $n . "-abcdefghijklmnop";
            $dates[] = Core\Time\Date::at(2003, 3, 3);
        }
        return "!";
    }

    public function yearFormat(): string {
        $this->replace();
        return "yyyy";
    }

    public function tagKey(): property<Version> {
        $this->replace();
        return "tag" as property<Version>;
    }

    public function same(): string {
        // A literal, so the answer takes no block a freed read could share.
        $this->replace();
        return "old-1-abcdefghijklmnop";
    }

    public function zero(): int {
        $this->replace();
        return 0;
    }

    public function sameVersion(): Version {
        $this->replace();
        return new Version("old-1-abcdefghijklmnop");
    }
}

int $n = 1;
Holder $h = new Holder();
$h->s = "old-" . $n . "-abcdefghijklmnop";
$h->items = ["old-" . $n . "-abcdefghijklmnop"];
Holder::$shared = "old-" . $n . "-abcdefghijklmnop";
"#;

/// The text every read form holds before `replace()` runs.
const OLD: &str = "old-1-abcdefghijklmnop";

/// Each way to read a borrowed string out of storage `replace()` overwrites.
const READS: &[&str] = &["$h->s", "$h->items[0]", "Holder::$shared"];

/// The statement that reads `read` at `site` and then calls `replace()`, or
/// `None` where the site cannot contain that read: a string hole reads no
/// static property, an index base reads only an array, an ordering and a
/// keyed read read only the object `$h->v`, and a `Core` receiver reads only
/// the date `$h->d`.
fn statement(site: OperandSite, read: &str) -> Option<String> {
    match site {
        OperandSite::Concat => Some(format!("echo {read} . $h->replace();\n")),
        OperandSite::StringHole if read.contains("::") => None,
        OperandSite::StringHole => Some(format!("echo \"{{{read}}}{{$h->replace()}}\";\n")),
        // `same()` replaces the storage, then returns the old text. The pair is
        // equal only while the read still has its own value.
        OperandSite::Binary => Some(format!(
            "echo ({read} == $h->same()) ? \"{OLD}!\" : \"lost\";\n"
        )),
        OperandSite::MatchSubject => Some(format!(
            "echo match ({read}) {{ $h->same() => \"{OLD}!\", default => \"lost\" }};\n"
        )),
        OperandSite::Argument => Some(format!(
            "echo Core\\Str::startsWith({read}, $h->same()) ? \"{OLD}!\" : \"lost\";\n"
        )),
        OperandSite::CallableArgument => Some(format!(
            "callable $f = fn(string $a, string $b): string => $a . $b;\n\
             echo $f({read}, $h->replace());\n"
        )),
        // The read is the array, and the key replaces it before the element
        // is read. Only the element form reads an array.
        OperandSite::IndexBase => read
            .strip_suffix("[0]")
            .map(|array| format!("echo {array}[$h->zero()] . \"!\";\n")),
        // The read is an object, and `compareTo` reads its string after the
        // right operand replaced it. It runs once, with the first read form.
        OperandSite::Ordering if read == READS[0] => Some(format!(
            "echo ($h->v <=> $h->sameVersion()) == 0 ? \"{OLD}!\" : \"lost\";\n"
        )),
        OperandSite::Ordering => None,
        // `yearFormat()` replaces the date before `format` reads it.
        OperandSite::Receiver if read == READS[0] => Some(format!(
            "echo ($h->d->format($h->yearFormat()) == \"1999\") ? \"{OLD}!\" : \"lost\";\n"
        )),
        OperandSite::Receiver => None,
        // `tagKey()` replaces the object before its property is read.
        OperandSite::KeyedReceiver if read == READS[0] => {
            Some("echo $h->v->{$h->tagKey()} . \"!\";\n".to_owned())
        }
        OperandSite::KeyedReceiver => None,
        // `borrowed_stores.rs` runs the store sites.
        site if site.is_store() => None,
        other => panic!("borrowed_reads has no program for {other:?}; add one"),
    }
}

#[test]
fn every_operand_site_keeps_a_borrowed_read_alive_across_later_code() {
    let mut failures = Vec::new();
    for &site in OperandSite::ALL {
        for read in READS {
            let Some(statement) = statement(site, read) else {
                continue;
            };
            let source = format!("{HOLDER}{statement}");
            let output = output_of(&source);
            if output != "old-1-abcdefghijklmnop!" {
                failures.push(format!("{site:?} with {read}: printed {output:?}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "a borrowed read lost its value:\n{}",
        failures.join("\n")
    );
}

#[test]
fn a_local_read_survives_a_later_operand_that_rebinds_it() {
    let prelude = r#"<?nvs
class Bump {
    public static function swap(inout string $slot): string {
        int $n = 2;
        $slot = "new-" . $n . "-abcdefghijklmnop";
        return "";
    }
    public static function swapIn(inout array<string> $slot): string {
        int $n = 2;
        $slot = ["new-" . $n . "-abcdefghijklmnop"];
        return "";
    }
    public static function fill(): string {
        int $n = 2;
        array<string> $fill = [];
        for (int $i = 0; $i < 1024; $i++) {
            $fill[] = "fil-" . $n . "-abcdefghijklmnop";
        }
        return "!";
    }
}

int $n = 1;
string $s = "old-" . $n . "-abcdefghijklmnop";
array<string> $a = ["old-" . $n . "-abcdefghijklmnop"];
"#;
    // The old value is freed when the write lands, which for an `inout`
    // argument is after the callee returns. `Bump::fill()` runs after that
    // and reuses the freed block.
    let cases = [
        (
            "echo $s . Bump::swap(inout $s) . Bump::fill();\n",
            "local, inout argument",
        ),
        (
            "echo $a[0] . Bump::swapIn(inout $a) . Bump::fill();\n",
            "element, inout argument",
        ),
        (
            "echo $s . ($s = \"new-\" . $n . \"-abcdefghijklmnop\") . Bump::fill();\n",
            "local, assignment",
        ),
        (
            "echo $a[0] . ($a = [\"new-\" . $n . \"-abcdefghijklmnop\"])[0] . Bump::fill();\n",
            "element, assignment",
        ),
    ];
    let mut failures = Vec::new();
    for (statement, label) in cases {
        let output = output_of(&format!("{prelude}{statement}"));
        if !output.starts_with("old-1-abcdefghijklmnop") || !output.ends_with('!') {
            failures.push(format!("{label}: printed {output:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "a local read lost its value:\n{}",
        failures.join("\n")
    );
}

/// A `toString` that a conversion calls replaces the storage of an earlier
/// piece, or frees its own receiver. Each piece and the receiver keep their
/// value while it runs.
#[test]
fn a_to_string_conversion_keeps_its_receiver_alive() {
    let prelude = r#"<?nvs
class Name implements Stringable {
    public Holder $h;
    public string $text;

    public function constructor(Holder $h, string $text) {
        $this->h = $h;
        $this->text = $text;
    }

    public function toString(): string {
        $this->h->replace();
        return $this->text . "!";
    }
}

class Holder {
    public string $s = "";
    public ?Name $name = null;

    public function replace(): void {
        int $n = 2;
        $this->s = "new-" . $n . "-abcdefghijklmnop";
        $this->name = null;
        array<string> $fill = [];
        for (int $i = 0; $i < 1024; $i++) {
            $fill[] = "fil-" . $n . "-abcdefghijklmnop";
        }
    }
}

int $n = 1;
Holder $h = new Holder();
$h->s = "old-" . $n . "-abcdefghijklmnop";
Name $name = new Name($h, "");
mixed $any = $name;
"#;
    let cases = [
        ("echo $h->s . $name;\n", "concat, object"),
        ("echo $h->s . $any;\n", "concat, mixed"),
        ("echo \"{$h->s}{$name}\";\n", "string hole, object"),
        ("echo \"{$h->s}{$any}\";\n", "string hole, mixed"),
        (
            "$h->name = new Name($h, \"old-\" . $n . \"-abcdefghijklmnop\");\necho $h->name;\n",
            "own receiver",
        ),
    ];
    let mut failures = Vec::new();
    for (statement, label) in cases {
        let output = output_of(&format!("{prelude}{statement}"));
        if output != "old-1-abcdefghijklmnop!" {
            failures.push(format!("{label}: printed {output:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "a conversion lost a value its `toString` freed:\n{}",
        failures.join("\n")
    );
}

/// A `PropertyGuard` replaces the property the stored value was read from.
/// The store still writes that value.
#[test]
fn a_property_guard_cannot_free_the_value_being_stored() {
    let prelude = r#"<?nvs
class Box implements PropertyGuard {
    public string $s = "";
    public string $t = "";
    public array<string> $items = [];

    public function beforePropertySet(string $name, mixed $value, array<string> $keys): void {
        if ($name == "s") {
            return;
        }
        int $n = 2;
        $this->s = "new-" . $n . "-abcdefghijklmnop";
        array<string> $fill = [];
        for (int $i = 0; $i < 1024; $i++) {
            $fill[] = "fil-" . $n . "-abcdefghijklmnop";
        }
    }

    public function beforePropertyUnset(string $name, array<string> $keys): void {}

    public static function fill(): void {
        int $n = 3;
        array<string> $fill = [];
        for (int $i = 0; $i < 1024; $i++) {
            $fill[] = "fil-" . $n . "-abcdefghijklmnop";
        }
    }
}

int $n = 1;
Box $b = new Box();
$b->s = "old-" . $n . "-abcdefghijklmnop";
"#;
    // The old value is freed when the guard returns, so the fill that reuses
    // its block runs after the store.
    let cases = [
        ("$b->t = $b->s;\nBox::fill();\necho $b->t;\n", "property"),
        (
            "$b->items[] = $b->s;\nBox::fill();\necho $b->items[0];\n",
            "element",
        ),
    ];
    let mut failures = Vec::new();
    for (statement, label) in cases {
        let output = output_of(&format!("{prelude}{statement}"));
        if output != "old-1-abcdefghijklmnop" {
            failures.push(format!("{label}: printed {output:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "a guarded store lost the value it stores:\n{}",
        failures.join("\n")
    );
}

/// A rendered `Core\Str::format` reads every argument first and renders them
/// after. A later argument replaces the storage an earlier argument was read
/// from. The earlier argument keeps its value.
#[test]
fn a_rendered_format_keeps_each_argument_alive() {
    let prelude = r#"<?nvs
class Holder {
    public string $s = "";

    public function replace(): void {
        int $n = 2;
        $this->s = "new-" . $n . "-abcdefghijklmnop";
        array<string> $fill = [];
        for (int $i = 0; $i < 1024; $i++) {
            $fill[] = "fil-" . $n . "-abcdefghijklmnop";
        }
    }

    public function bang(): string {
        $this->replace();
        return "!";
    }
}

int $n = 1;
Holder $h = new Holder();
$h->s = "old-" . $n . "-abcdefghijklmnop";
"#;
    // An argument number makes the template a rendered one. Rendering an
    // object with `%s` throws today, so no row reaches a `toString` yet; the
    // lowering holds every borrowed argument before such a rendering anyway.
    let statement = "echo Core\\Str::format('%1$s%2$s', $h->s, $h->bang());\n";
    let output = output_of(&format!("{prelude}{statement}"));
    assert_eq!(
        output, "old-1-abcdefghijklmnop!",
        "a rendered format lost an argument a later argument freed"
    );
}

/// A `PropertyGuard` on an element store replaces the array the store read
/// and the property its key was read from. The store writes the key it read
/// into the array the property has once the guard has run.
#[test]
fn a_guarded_element_store_keeps_its_key_and_array_alive() {
    let program = r#"<?nvs
class Box implements PropertyGuard {
    public string $k = "";
    public array<string> $items = [];
    public bool $armed = false;

    public function beforePropertySet(string $name, mixed $value, array<string> $keys): void {
        // The guard's own store to `items` runs the guard again.
        if ($name != "items" || !$this->armed) {
            return;
        }
        $this->armed = false;
        int $n = 2;
        $this->k = "new-" . $n . "-abcdefghijklmnop";
        array<string> $fresh = [];
        $fresh["new-" . $n . "-abcdefghijklmnop"] = "new";
        $this->items = $fresh;
        array<string> $fill = [];
        array<array<string>> $rows = [];
        for (int $i = 0; $i < 1024; $i++) {
            $fill[] = "fil-" . $n . "-abcdefghijklmnop";
            $rows[] = ["fil-" . $n . "-abcdefghijklmnop" => "fil"];
        }
    }

    public function beforePropertyUnset(string $name, array<string> $keys): void {}
}

int $n = 1;
Box $b = new Box();
$b->k = "old-" . $n . "-abcdefghijklmnop";
array<string> $start = [];
$start["old-" . $n . "-abcdefghijklmnop"] = "start";
$b->items = $start;
$start = [];
$b->armed = true;
$b->items[$b->k] = "v";
foreach ($b->items as string $key => string $value) {
    echo $key . "=" . $value . ";";
}
"#;
    let output = output_of(program);
    assert_eq!(
        output, "new-2-abcdefghijklmnop=new;old-1-abcdefghijklmnop=v;",
        "a guarded element store lost its key or its array"
    );
}

/// A memory stop anywhere in a call of a generator factory releases the
/// arguments the factory owns.
///
/// The ceiling that stops at a given step is not known in advance, so the run
/// is swept over every ceiling up to one it fits under. Every stopped run must
/// leave nothing allocated.
///
/// The state object's `new` has no constructor, so it compiles to an
/// allocation with no status. A crossing there is reported at the next poll,
/// and the sweep cannot stop at the `new` itself. Its landing block releases
/// the arguments for the day that allocation can fail.
#[test]
fn a_memory_stop_in_a_generator_factory_releases_its_arguments() {
    /// Each argument's length.
    const FILLER: usize = 4 << 10;
    /// The step between two ceilings, below the state object's size.
    const STEP: usize = 8;

    let filler = "x".repeat(FILLER);
    let source = format!(
        "<?nvs\nclass Gen {{\n    \
         public static function items(string $a, string $b): Iterator<string> {{\n        \
         yield $a;\n        yield $b;\n    }}\n}}\n\
         int $n = 1;\n\
         Gen::items(\"{filler}\" . $n, \"{filler}\" . $n);\n"
    );
    let unit = compile(&source).expect("the fixture compiles");
    let mut leaks = Vec::new();
    let mut last_stop = None;
    let mut completed = false;
    let mut ceiling = STEP;
    while !completed {
        assert!(
            ceiling < 64 * FILLER,
            "no ceiling up to {ceiling} bytes let the run complete"
        );
        let mut ctx = Ctx::buffered();
        unit.install_in(&mut ctx);
        let entry = unit.script().expect("the script frame was compiled");
        nvs_runtime::prime_empty_array();
        let before = nvs_runtime::budget::live_bytes();
        ctx.set_memory_limit(ceiling);
        let stopped = entry.call(&mut ctx).is_err();
        drop(ctx.take_pending());
        drop(ctx.take_buffered_output());
        if nvs_runtime::budget::live_bytes() != before {
            leaks.push(ceiling);
        }
        if stopped {
            last_stop = Some(ceiling);
        } else {
            completed = true;
        }
        ceiling += STEP;
    }
    assert!(
        last_stop.is_some_and(|stop| stop >= 2 * FILLER),
        "no run stopped once both arguments were allocated: {last_stop:?}"
    );
    assert!(
        leaks.is_empty(),
        "a memory stop left bytes allocated at the ceilings {leaks:?}"
    );
}
