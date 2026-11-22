//! The borrow verifier refuses a function that reads a borrowed element after
//! its owner was moved, and passes the functions the lowering and the
//! ownership pass make (`nvs_ir::ownership::verify_borrows`).
//!
//! The refused function is built by hand from a lowered one: the element's
//! own `retain` and `release` are taken out, which is the function the
//! lowering made before it retained an operand a later operand outlives. The
//! verifier runs in test and debug builds alone, so this file does too.
#![cfg(debug_assertions)]

mod common;

use nvs_ir::ir::{Function, InstKind};
use nvs_ir::ownership::verify_borrows;

/// The first program of the review: an element read, then a call that is
/// moved the array the element was read from.
const MOVED_TO_CALLEE: &str = r#"<?nvs
class Shop {
    public static function len(array<string> $x): string {
        $x = [];
        return "n";
    }
    public static function main(): void {
        array<string> $a = ["k" => Core\Str::repeat("x", 64)];
        echo "v=" . $a["k"] . Shop::len($a), "\n";
    }
}
Shop::main();
"#;

/// The same read, then an owning `Core` member that is moved the array.
const MOVED_TO_CORE: &str = r#"<?nvs
class Shop {
    public static function main(): void {
        array<string> $a = [Core\Str::repeat("x", 64)];
        echo "v=" . $a[0] . Core\Arr::count(Core\Arr::withoutFirst($a)), "\n";
    }
}
Shop::main();
"#;

/// The second program of the review: a `toString` run by `as ?string`
/// empties the static property its caller would lend.
const TO_STRING: &str = r#"<?nvs
class Blog {
    public static array<string> $items = [];
    public static function add(array<string> $xs, mixed $m): array<string> {
        $xs[] = Core\Str::length(($m as ?string) ?? "") > 0 ? "t" : "f";
        return $xs;
    }
}
class Post {
    public function toString(): string { Blog::$items = []; return "zz"; }
}
Blog::$items = [Core\Str::repeat("a", 40), Core\Str::repeat("b", 40)];
Blog::$items = Blog::add(Blog::$items, new Post());
echo Core\Json::encode(Blog::$items), "\n";
"#;

/// A call that is given `$cart` and the property it lends from
/// `$cart->item`. The call's own retain of `$cart` comes before the read of
/// `$cart->item`, and the local still owns `$cart` after the call.
const RETAINED_BEFORE_THE_READ: &str = r#"<?nvs
final class Post {
    public string $text;
    public function constructor(string $text) { $this->text = $text; }
}
final class Cart {
    public Post $item;
    public function constructor(string $text) { $this->item = new Post($text); }
}
final class Shop {
    public static function add(Cart $cart, string $text, string $x): string {
        echo "saw: ", $cart->item->text, "\n";
        $text .= $x;
        return $text;
    }
}
Cart $cart = new Cart("ab");
$cart->item->text = Shop::add($cart, $cart->item->text, "c");
echo $cart->item->text, "\n";
"#;

/// The lowered function whose name ends with `name`.
fn function(program: &nvs_ir::Program, name: &str) -> Function {
    program
        .functions
        .iter()
        .find(|f| f.name.ends_with(name))
        .unwrap_or_else(|| panic!("{name} was lowered"))
        .clone()
}

/// `f` with every `retain` and `release` of an element read taken out.
/// Returns the function and the element reads it changed.
fn without_element_retains(mut f: Function) -> (Function, Vec<nvs_ir::ids::ValueId>) {
    let reads: Vec<_> = f
        .blocks
        .iter()
        .flat_map(|block| &block.insts)
        .filter(|inst| matches!(inst.kind, InstKind::ArrayGet { .. }))
        .filter_map(|inst| inst.result)
        .collect();
    let mut changed = Vec::new();
    for block in &mut f.blocks {
        block.insts.retain(|inst| match inst.kind {
            InstKind::Retain { operand } | InstKind::Release { operand }
                if reads.contains(&operand) =>
            {
                changed.push(operand);
                false
            }
            _ => true,
        });
    }
    changed.dedup();
    (f, changed)
}

#[test]
fn a_function_that_reads_an_element_after_its_array_was_moved_is_refused() {
    for (case, source) in [("callee", MOVED_TO_CALLEE), ("Core", MOVED_TO_CORE)] {
        let program = common::lower(source);
        let main = function(&program, "Shop::main");
        assert_eq!(
            verify_borrows(&main),
            Ok(()),
            "{case}: the lowered function"
        );
        let (broken, changed) = without_element_retains(main);
        assert!(
            !changed.is_empty(),
            "{case}: the element read has a retain of its own"
        );
        let refusal = verify_borrows(&broken).expect_err(&format!(
            "{case}: the element is read after its array was moved"
        ));
        assert!(
            changed.contains(&refusal.borrowed),
            "{case}: the refusal names {refusal}"
        );
    }
}

#[test]
fn an_element_read_while_its_array_is_still_owned_is_not_refused() {
    // With in-place changes off, the call is given a retained copy of the
    // array, so the element stays alive without a retain of its own.
    let program = nvs_ir::lower::without_in_place(|| common::lower(MOVED_TO_CALLEE));
    let (unretained, _) = without_element_retains(function(&program, "Shop::main"));
    assert_eq!(verify_borrows(&unretained), Ok(()));
}

#[test]
fn a_retain_of_the_owner_before_the_read_pays_for_the_call_that_takes_it() {
    let program = common::lower(RETAINED_BEFORE_THE_READ);
    let script = function(&program, "<script>");
    assert_eq!(verify_borrows(&script), Ok(()), "the lowered function");
    // Without the call's retain of `$cart`, the call takes the local's own
    // reference, and `$cart->item` is read after it.
    let given: Vec<_> = script
        .blocks
        .iter()
        .flat_map(|block| &block.insts)
        .filter_map(|inst| match &inst.kind {
            InstKind::Call { args, .. } => Some(args.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    let mut broken = script;
    let mut removed = false;
    for block in &mut broken.blocks {
        block.insts.retain(|inst| match inst.kind {
            InstKind::Retain { operand } if given.contains(&operand) && !removed => {
                removed = true;
                false
            }
            _ => true,
        });
    }
    assert!(removed, "the call's argument has a retain");
    verify_borrows(&broken).expect_err("`$cart->item` is read after `$cart` was given up");
}

#[test]
fn the_programs_of_the_review_pass_the_verifier() {
    for source in [MOVED_TO_CALLEE, MOVED_TO_CORE, TO_STRING] {
        for program in [
            common::lower(source),
            nvs_ir::lower::without_in_place(|| common::lower(source)),
        ] {
            for f in &program.functions {
                assert_eq!(verify_borrows(f), Ok(()), "{}", f.name);
            }
        }
    }
}
