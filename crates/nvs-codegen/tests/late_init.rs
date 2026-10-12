//! `rule:classes/lateinit-restrictions`'s `lateinit` on every type, and
//! `rule:classes/isinit-reads-nothing`'s `isinit`: the compiled read tests the slot's tag, so a
//! zero, `false` or `null` written to the slot is a value, and `isinit` is that
//! test alone.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;
use nvs_ir::ir::InstKind;

/// The instructions of the script frame, in block order.
fn script_insts(program: &nvs_ir::Program) -> Vec<&InstKind> {
    program
        .functions
        .iter()
        .filter(|function| function.name == nvs_ir::lower::ENTRY_SCRIPT_LABEL)
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.insts)
        .map(|inst| &inst.kind)
        .collect()
}

/// A `lateinit int` starts with no value, and after its first write it reads
/// as an ordinary `int` — `0` included, since the payload of the unset state is
/// also zero and only the tag tells the two apart.
#[test]
fn a_lateinit_int_reads_its_value_after_the_first_write() {
    let source = "<?nvs
class Counter {
    public lateinit int $n;
}
Counter $c = new Counter();
$c->n = 0;
echo $c->n, \"/\";
$c->n = $c->n + 41;
$c->n++;
echo $c->n;
";
    assert_eq!(output_of(source), "0/42");
}

/// A `null` written to a `lateinit ?T` is a value: the read returns it, and
/// `isinit` is `true`. The unset state and `null` are two different tags.
#[test]
fn a_lateinit_nullable_holds_null_as_a_value() {
    let source = "<?nvs
class Author { public string $name = \"Ada\"; }
class Post {
    public lateinit ?Author $author;
}
Post $p = new Post();
echo isinit($p->author) ? \"y\" : \"n\", \"/\";
$p->author = null;
echo isinit($p->author) ? \"y\" : \"n\", \"/\";
echo $p->author?->name ?? \"none\", \"/\";
$p->author = new Author();
echo $p->author?->name ?? \"none\";
";
    assert_eq!(output_of(source), "n/y/none/Ada");
}

/// The read before the first write throws a `LogicError` on every type a
/// `lateinit` property may declare, and the message names the property.
#[test]
fn a_lateinit_read_before_the_first_write_throws_on_every_type() {
    let source = "<?nvs
class Post {
    public lateinit int $views;
    public lateinit bool $draft;
    public lateinit float $score;
    public lateinit string $title;
    public lateinit array<string> $tags;
    public lateinit ?string $note;
}
Post $p = new Post();
int $throws = 0;
try { echo $p->views; } catch (LogicError $e) { $throws = $throws + 1; echo $e->message, \"/\"; }
try { echo $p->draft ? \"y\" : \"n\"; } catch (LogicError $e) { $throws = $throws + 1; }
try { echo $p->score; } catch (LogicError $e) { $throws = $throws + 1; }
try { echo $p->title; } catch (LogicError $e) { $throws = $throws + 1; }
try { echo Core\\Json::encode($p->tags); } catch (LogicError $e) { $throws = $throws + 1; }
try { echo $p->note ?? \"none\"; } catch (LogicError $e) { $throws = $throws + 1; }
echo $throws;
";
    assert_eq!(
        output_of(source),
        "`Post`'s property `$views` is read before it is written/6"
    );
}

/// `isinit` lowers to one tag test of the slot: no read of the property, and no
/// call, so neither a `get` hook nor `onPropertyGet` can run. The observer
/// prints nothing in the run either.
#[test]
fn isinit_reads_no_property_and_calls_nothing() {
    let source = "<?nvs
final class Post implements PropertyObserver {
    public lateinit int $views;
    public function onPropertyGet(string $name, mixed $value): void { echo \"get \"; }
    public function onPropertySet(string $name, mixed $value): void { echo \"set \"; }
}
Post $p = new Post();
bool $before = isinit($p->views);
$p->views = 3;
bool $after = isinit($p->views);
echo $before ? \"y\" : \"n\", $after ? \"y\" : \"n\";
";
    let program = lower(source);
    let insts = script_insts(&program);
    let tests = insts
        .iter()
        .filter(|kind| matches!(kind, InstKind::FieldIsInit { .. }))
        .count();
    assert_eq!(tests, 2, "{insts:?}");
    assert!(
        !insts
            .iter()
            .any(|kind| matches!(kind, InstKind::FieldGet { .. })),
        "{insts:?}"
    );
    let calls: Vec<_> = insts
        .iter()
        .filter_map(|kind| match kind {
            InstKind::Call { target, .. } => Some(target.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        calls.iter().all(|target| !target.contains("onPropertyGet")),
        "{calls:?}"
    );
    assert_eq!(output_of(source), "set ny");
}
