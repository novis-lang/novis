//! What compiled code spends on a revocable object: the inline release of an
//! object keeps its one compare, whose bound is the revoked flag, so an object
//! that is not revoked runs the instructions it ran before, and a revoked one
//! is still freed with its last reference. The tests read the Cranelift text,
//! as `refcount_calls.rs` does, or the request heap's live bytes, as
//! `callables.rs` does, and none reads a clock (`rule:testing/bench-counters`).
// covers: lang:classes/revocable-objects

mod common;

use common::*;

/// One function's Cranelift IR out of the text `nvs_codegen::clif` renders,
/// which heads each function with `; <name>`.
fn section<'a>(text: &'a str, name: &str) -> &'a str {
    let head = format!("; {name}\n");
    let start = text
        .find(&head)
        .unwrap_or_else(|| panic!("the unit defines no `{name}`:\n{text}"))
        + head.len();
    let body = &text[start..];
    match body.find("\n; ") {
        Some(end) => &body[..end],
        None => body,
    }
}

/// `drop` releases the object in its typed local, which is not a literal, so
/// the release is inline.
const PLAIN: &str = r#"<?nvs
class Box {
    public int $n = 1;
}
class K {
    public static function drop(): int {
        Box $b = new Box();
        return $b->n;
    }
}
echo K::drop();
"#;

#[test]
fn the_inline_release_of_an_object_that_is_not_revoked_is_unchanged() {
    let program = lower(PLAIN);
    let text = nvs_codegen::clif(&program).expect("the unit compiled");
    let clif = section(&text, "K::drop");
    // The bound is the revoked flag less two: counts from 2 up to one below
    // the flag are decremented inline.
    let bound = format!("{:#x}", nvs_runtime::revoke::REVOKED - 2);
    let compares: Vec<&str> = clif
        .lines()
        .filter(|line| line.contains("icmp ult"))
        .collect();
    assert!(
        compares.iter().any(|line| line.replace('_', "").contains(&bound)),
        "`K::drop`'s release compares against {bound}:\n{clif}"
    );
    // The flag costs no instruction: nothing masks the count inline.
    assert!(
        !clif.contains("band_imm"),
        "`K::drop` masks a count inline:\n{clif}"
    );
}

/// A loop that makes `iterations` revocable objects, revokes each, and lets
/// it go at the end of the iteration.
fn revoking_loop(iterations: u32) -> String {
    format!(
        r#"<?nvs
class Post implements Revocable {{
    public function constructor(public string $title) {{}}
}}
var $i = 0;
var $revoked = 0;
while ($i < {iterations}) {{
    Post $post = new Post("p" . $i);
    $post->revoke();
    if ($post->isRevoked()) {{
        $revoked = $revoked + 1;
    }}
    $i = $i + 1;
}}
echo $revoked;
"#
    )
}

/// What the request heap still holds after one run of `source`, with the
/// context still alive, so nothing the request's end frees is counted.
#[cfg(debug_assertions)]
fn live_bytes_of_run(source: &str) -> isize {
    let unit = compile(source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    let before = nvs_runtime::budget::live_bytes();
    entry.call(&mut ctx).expect("the script ran to completion");
    nvs_runtime::budget::live_bytes() - before
}

#[test]
#[cfg(debug_assertions)]
fn compiled_code_frees_a_revoked_object() {
    // Each object is revoked, so its count word has the flag set, and its last
    // release takes the helper call, which masks the flag and frees it. A
    // release that read the flagged word as a count would keep every object.
    // Measured as a slope: ten times the objects must not hold ten times the
    // bytes.
    assert_eq!(output_of(&revoking_loop(200)), "200");
    let short = live_bytes_of_run(&revoking_loop(200));
    let long = live_bytes_of_run(&revoking_loop(2000));
    assert!(
        long - short < 4096,
        "a revoked object is not freed: 200 objects held {short} bytes, 2000 held {long}"
    );
}

/// A program with one revocable class, one class that is not, and one
/// interface the revocable class implements. Each method of `Uses` makes one
/// kind of access, so each test counts the checks of one method.
const ACCESSES: &str = r#"<?nvs
interface Named
{
    public function name(): string;
}

class Post implements Revocable, Named
{
    public function constructor(public string $title)
    {}

    public function name(): string
    {
        return 'post';
    }
}

class Plain
{
    public function constructor(public string $title)
    {}

    public function name(): string
    {
        return 'plain';
    }
}

class Uses
{
    public static function plain(Plain $plain): string
    {
        $plain->title = 'b';
        return $plain->title . $plain->name();
    }

    public static function post(Post $post): string
    {
        return $post->title;
    }

    public static function named(Named $named): string
    {
        return $named->name();
    }

    public static function end(Post $post): bool
    {
        $post->revoke();
        return $post->isRevoked();
    }
}

echo Uses::plain(new Plain('a')), Uses::post(new Post('a')), Uses::named(new Post('a'));
echo Uses::end(new Post('a')) ? 'yes' : 'no';
"#;

/// How many revoked checks the function `name` of `ACCESSES` compiles.
fn checks_in(name: &str) -> usize {
    let program = lower(ACCESSES);
    let function = program
        .functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("the program defines no `{name}`"));
    function
        .blocks
        .iter()
        .flat_map(|block| &block.insts)
        .filter(|inst| matches!(inst.kind, nvs_ir::ir::InstKind::CheckRevoked { .. }))
        .count()
}

#[test]
fn a_class_that_is_not_revocable_compiles_no_check() {
    // A write, a read and a call through `Plain`, which no revocable class
    // reaches, so none of them is checked.
    assert_eq!(checks_in("Uses::plain"), 0);
}

#[test]
fn a_revocable_class_access_compiles_one_check() {
    assert_eq!(checks_in("Uses::post"), 1);
}

#[test]
fn an_interface_call_compiles_one_check() {
    // `Named` is implemented by the revocable `Post`, so a call through it is
    // checked.
    assert_eq!(checks_in("Uses::named"), 1);
}

#[test]
fn revoke_and_isrevoked_compile_no_check() {
    assert_eq!(checks_in("Uses::end"), 0);
    assert_eq!(output_of(ACCESSES), "bplainapostyes");
}
