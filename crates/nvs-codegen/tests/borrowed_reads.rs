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
class Holder {
    public string $s = "";
    public array<string> $items = [];
    public static string $shared = "";

    public function replace(): string {
        int $n = 2;
        $this->s = "new-" . $n . "-abcdefghijklmnop";
        $this->items = ["new-" . $n . "-abcdefghijklmnop"];
        Holder::$shared = "new-" . $n . "-abcdefghijklmnop";
        array<string> $fill = [];
        for (int $i = 0; $i < 1024; $i++) {
            $fill[] = "fil-" . $n . "-abcdefghijklmnop";
        }
        return "!";
    }

    public function same(): string {
        // A literal, so the answer takes no block a freed read could share.
        $this->replace();
        return "old-1-abcdefghijklmnop";
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
/// static property.
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
