//! A store keeps its target alive while its value runs code that replaces
//! that target (`rule:expressions/an-operand-keeps-its-value-while-later-operands-run`).
//!
//! Each program reads part of a store's target, then runs a value that
//! replaces the storage the target was read from and allocates many values of
//! the same size. A store that did not keep its own reference writes into a
//! freed block, which a later read shows, or crashes.
//!
//! See `tests/common/mod.rs` for the shared fixtures, and `borrowed_reads.rs`
//! for the read sites.

mod common;

use common::*;
use nvs_ir::lower::OperandSite;

/// The class every program below uses. `replace()` overwrites every slot a
/// store form reads, then allocates many strings and boxes of the same size.
/// Some heaps reuse a freed block only after many others, so the fill is
/// large. The boxes stay alive in `$boxes`, so a store into a freed box shows
/// in one of them.
const HOLDER: &str = r#"<?nvs
class Box {
    public string $c = "";

    public function constructor(string $c) {
        $this->c = $c;
    }
}

class Holder {
    public string $s = "";
    public array<string> $items = [];
    public Box $box;
    public array<Box> $boxes = [];

    public function constructor() {
        int $n = 1;
        $this->box = new Box("old-" . $n . "-abcdefghijklmnop");
    }

    public function replace(): string {
        int $n = 2;
        $this->s = "new-" . $n . "-abcdefghijklmnop";
        $this->items = ["new-" . $n . "-abcdefghijklmnop"];
        $this->box = new Box("new-" . $n . "-abcdefghijklmnop");
        array<string> $fill = [];
        for (int $i = 0; $i < 1024; $i++) {
            $fill[] = "fil-" . $n . "-abcdefghijklmnop";
            $this->boxes[] = new Box("fil-" . $n . "-abcdefghijklmnop");
        }
        return "!";
    }
}

int $n = 1;
Holder $h = new Holder();
$h->s = "old-" . $n . "-abcdefghijklmnop";
$h->items = ["old-" . $n . "-abcdefghijklmnop"];
"#;

/// Prints `ok` when no box holds a value a store wrote into a freed block.
const BOXES_INTACT: &str = r#"
string $bad = "";
if (!Core\Str::startsWith($h->box->c, "new-")) { $bad = "the new box"; }
foreach ($h->boxes as Box $b) {
    if (!Core\Str::startsWith($b->c, "fil-")) { $bad = "a fill box"; }
}
echo $bad == "" ? "ok" : $bad;
"#;

/// Each store form, the statements that run it and read the storage back,
/// and the text they print. The array forms write into the array that is
/// there once the value has run. The other forms write into the object their
/// receiver named before it. `$o->s .= e` reads the slot only after `e` runs,
/// so it appends to the string `replace()` stored.
fn forms() -> Vec<(&'static str, String, &'static str)> {
    vec![
        (
            "$o->s .= e",
            "$h->s .= $h->replace();\necho $h->s;\n".to_owned(),
            "new-2-abcdefghijklmnop!",
        ),
        (
            "$o->arr[] = e",
            "$h->items[] = $h->replace();\necho $h->items[0] . \"|\" . $h->items[1];\n".to_owned(),
            "new-2-abcdefghijklmnop|!",
        ),
        (
            "$o->arr[k] = e",
            "$h->items[1] = $h->replace();\necho $h->items[0] . \"|\" . $h->items[1];\n".to_owned(),
            "new-2-abcdefghijklmnop|!",
        ),
        (
            "$m[$o->key] = e",
            "array<string> $m = [];\n$m[$h->s] = $h->replace();\n\
             echo $m[\"old-1-abcdefghijklmnop\"];\n"
                .to_owned(),
            "!",
        ),
        (
            "$a->b->c = e",
            format!("$h->box->c = $h->replace();\n{BOXES_INTACT}"),
            "ok",
        ),
    ]
}

#[test]
fn every_store_form_keeps_its_target_alive_while_the_value_runs_code() {
    // Every store site is one of the forms above.
    assert!(
        OperandSite::ALL.iter().any(|site| site.is_store()),
        "OperandSite names no store site"
    );
    let mut failures = Vec::new();
    for (form, statements, want) in forms() {
        let source = format!("{HOLDER}{statements}");
        let mut ctx = Ctx::buffered();
        let status = run_with(&mut ctx, &source);
        let output = ctx
            .take_buffered_output()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        match status {
            Ok(_) if output == want => {}
            Ok(_) => failures.push(format!("{form}: printed {output:?}, want {want:?}")),
            Err(code) => failures.push(format!("{form}: stopped with status {code}")),
        }
    }
    assert!(
        failures.is_empty(),
        "a store lost its target:\n{}",
        failures.join("\n")
    );
}
