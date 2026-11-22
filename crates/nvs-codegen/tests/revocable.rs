//! What compiled code spends on a revocable object: the inline release of an
//! object keeps its one compare, whose bound is the revoked flag, so an object
//! that is not revoked runs the instructions it ran before, and a revoked one
//! is still freed with its last reference. The tests read the Cranelift text,
//! as `refcount_calls.rs` does, or the request heap's live bytes, as
//! `callables.rs` does, and none reads a clock (`rule:testing/bench-counters`).
//!
//! `USES` is the table of every use form, one or more rows per syntax kind of
//! `nvs-syntax`'s `ast.rs`, and `NO_OBJECT` lists the kinds that cannot take a
//! revocable object. A kind in neither fails the test by name.
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
        compares
            .iter()
            .any(|line| line.replace('_', "").contains(&bound)),
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

/// One object that stays alive while `iterations` sets each take it as a
/// member and are dropped. Each set is freed before the next is made, and
/// `revoke()` at the end walks the object's list.
fn passing_sets(iterations: u32) -> String {
    format!(
        r#"<?nvs
class Post implements Revocable {{
    public function constructor(public string $title) {{}}
}}
class K {{
    public static function hold(Post $post): int {{
        var $set = new Core\ObjectSet<Post>();
        $set->add($post);
        return $set->has($post) ? 1 : 0;
    }}
}}
var $post = new Post("kept");
var $i = 0;
var $held = 0;
while ($i < {iterations}) {{
    $held = $held + K::hold($post);
    $i = $i + 1;
}}
$post->revoke();
echo $held;
"#
    )
}

#[test]
#[cfg(debug_assertions)]
fn a_freed_collection_clears_its_back_links() {
    // A freed set takes itself off its member's list. If it did not, the
    // list would grow by one entry per set, and `revoke()` would follow a
    // pointer to a freed set. Measured as a slope, as above.
    assert_eq!(output_of(&passing_sets(200)), "200");
    let short = peak_bytes_of_run(&passing_sets(200));
    let long = peak_bytes_of_run(&passing_sets(2000));
    assert!(
        long - short < 4096,
        "a freed set leaves its membership behind: 200 sets held {short} bytes, 2000 held {long}"
    );
}

/// `iterations` cycles of a node and a map: the node holds the map, and the
/// map holds the node as a key and as a value. Each cycle is dropped at the end
/// of its iteration, so only the cycle collector frees it.
fn map_cycles(iterations: u32) -> String {
    format!(
        r#"<?nvs
class Node implements Revocable {{
    public ?Core\ObjectMap<Node, Node> $links = null;
}}
class K {{
    public static function cycle(): int {{
        var $node = new Node();
        var $map = new Core\ObjectMap<Node, Node>();
        $map->set($node, $node);
        $node->links = $map;
        return $map->has($node) ? 1 : 0;
    }}
}}
var $i = 0;
var $made = 0;
while ($i < {iterations}) {{
    $made = $made + K::cycle();
    $i = $i + 1;
}}
echo $made;
"#
    )
}

#[test]
#[cfg(debug_assertions)]
fn a_cycle_of_a_map_and_its_member_is_reclaimed_cleanly() {
    // The collector unlinks the map's members before it frees any object of
    // the cycle. Freeing the node first would leave the map's unlink reading
    // a freed node, which the debug allocator and Miri report. The collector
    // runs only near the request's ceiling, so the run is held to a ceiling
    // far below what its cycles take together: 20,000 cycles of about 960
    // bytes each finish under 512 KiB only if every cycle, its list included,
    // is freed.
    const CEILING: usize = 512 * 1024;
    let unit = compile(&map_cycles(20_000)).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    ctx.rebase_memory();
    ctx.set_memory_limit(CEILING);
    let ran = entry.call(&mut ctx);
    assert!(
        ran.is_ok(),
        "the cycles were not reclaimed under a {CEILING}-byte ceiling: {:?}",
        ctx.take_pending()
    );
    assert_eq!(ctx.take_buffered_output().as_deref(), Some(&b"20000"[..]));
}

/// Revocable objects left in a map, a set and a heap when the script ends.
const LEFT_IN_COLLECTIONS: &str = r#"<?nvs
class Post implements Revocable {
    public function constructor(public string $title) {}
}
var $map = new Core\ObjectMap<string, Post>();
var $set = new Core\ObjectSet<Post>();
var $heap = new Core\Heap<int>();
var $i = 0;
while ($i < 100) {
    var $post = new Post("p" . $i);
    $map->set("p" . $i, $post);
    $set->add($post);
    $i = $i + 1;
}
$heap->push(1);
echo $map->count() + $set->count();
"#;

#[test]
#[cfg(debug_assertions)]
fn the_request_end_walks_no_back_link() {
    // The request's end frees the heap whole. No collection is dismantled,
    // so no membership list is walked (`rule:classes/no-destructors`).
    let unit = compile(LEFT_IN_COLLECTIONS).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    entry.call(&mut ctx).expect("the script ran to completion");
    let before = nvs_runtime::budget::membership_walks();
    drop(ctx);
    assert_eq!(
        nvs_runtime::budget::membership_walks(),
        before,
        "the request's end walked a membership list"
    );
}

/// `count` objects of a class that is or is not revocable, each added to one
/// set. With `remove`, each is taken out of the set again.
fn members(revocable: bool, remove: bool) -> String {
    let implements = if revocable {
        " implements Revocable"
    } else {
        ""
    };
    let removal = if remove { "$set->remove($post);" } else { "" };
    format!(
        r#"<?nvs
class Post{implements} {{
    public function constructor(public string $title) {{}}
}}
var $set = new Core\ObjectSet<Post>();
array<Post> $all = [];
var $i = 0;
while ($i < 1000) {{
    var $post = new Post("p" . $i);
    $set->add($post);
    {removal}
    $all[] = $post;
    $i = $i + 1;
}}
echo Core\Arr::count($all);
"#
    )
}

#[test]
#[cfg(debug_assertions)]
fn a_membership_is_charged_to_the_request_and_returned() {
    // A revocable object costs its 16-byte hidden slot. While it is in a set,
    // its list and one 32-byte membership are charged to the request too, so
    // the request's peak has them.
    let slot = 1000 * 16;
    let held = peak_bytes_of_run(&members(true, false)) - peak_bytes_of_run(&members(false, false));
    assert!(
        held >= slot + 1000 * 32,
        "1000 memberships were not charged to the request: its peak grew by {held} bytes"
    );
    // Taken out of the set, each list is freed at once, and only the slot is
    // left at the peak.
    let removed =
        peak_bytes_of_run(&members(true, true)) - peak_bytes_of_run(&members(false, true));
    assert!(
        removed <= slot + 4096,
        "1000 removed memberships were not returned: the peak grew by {removed} bytes"
    );
    // Once the script ends, everything it charged is returned.
    let left = live_bytes_of_run(&members(true, false)) - live_bytes_of_run(&members(false, false));
    assert!(
        left < 4096,
        "the script's end left {left} bytes of memberships"
    );
}

/// The highest the request heap reached during one run of `source`, measured
/// from the balance after the unit was installed.
#[cfg(debug_assertions)]
fn peak_bytes_of_run(source: &str) -> isize {
    let unit = compile(source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    ctx.rebase_memory();
    entry.call(&mut ctx).expect("the script ran to completion");
    isize::try_from(ctx.memory_peak()).expect("a peak fits in an `isize`")
}

/// What one use form of a revoked object does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Does {
    /// It throws `RevokedObjectError`.
    Throws,
    /// It does not throw, for the reason the sentence of
    /// `rule:classes/revocable-objects` gives.
    Passes(&'static str),
}

/// One row of the table: the syntax kind of `nvs-syntax`'s `ast.rs` the form
/// belongs to, a label, the statements that use the revoked `$p`, and what
/// they do.
struct Use {
    kind: &'static str,
    form: &'static str,
    body: &'static str,
    does: Does,
}

const PASSING: &str = "passing the object, storing it";
const IDENTITY: &str = "`==` (identity)";

/// `rule:classes/revocable-objects`'s promise is "every use": a kind the rule
/// does not name throws. Each row is run against a revoked `$p` and a revoked
/// `$feed`.
const USES: &[Use] = &[
    Use {
        kind: "ExprKind::Interpolated",
        form: "a string with a hole",
        body: r#"var $s = "post {$p}";"#,
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::HtmlTemplate",
        form: "an html template with a hole",
        body: "var $h = html`<p>{$p}</p>`;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::ArrayLiteral",
        form: "an array literal",
        body: "var $a = [$p];",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "ExprKind::Unary",
        form: "a minus on a property",
        body: "var $n = -$p->count;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::PreIncDec",
        form: "`++` on a property",
        body: "++$p->count;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::PostIncDec",
        form: "`--` on a property",
        body: "$p->count--;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Binary",
        form: "a concatenation",
        body: r#"var $s = "post " . $p;"#,
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Binary",
        form: "`<`, which calls `compareTo`",
        body: "var $b = $p < $other;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Binary",
        form: "`<=>`, which calls `compareTo`",
        body: "var $n = $p <=> $other;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Binary",
        form: "`==`",
        body: "var $b = $p == $other;",
        does: Does::Passes(IDENTITY),
    },
    Use {
        kind: "ExprKind::Binary",
        form: "`??`",
        body: "var $q = $p ?? $other;",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "ExprKind::Assign",
        form: "a property write",
        body: r#"$p->title = "x";"#,
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Assign",
        form: "`.=` on a property",
        body: r#"$p->title .= "x";"#,
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Assign",
        form: "`+=` on a property",
        body: "$p->count += 1;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Assign",
        form: "an assignment of the object",
        body: "$other = $p;",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "ExprKind::Ternary",
        form: "a ternary that gives the object",
        body: "var $q = $other->count == 0 ? $p : $other;",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "ExprKind::Conversion",
        form: "`as` to an interface",
        body: "var $n = $p as Named;",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "ExprKind::TypeTest",
        form: "`is`",
        body: "var $b = $p is Post;",
        does: Does::Passes("the type test `is`"),
    },
    Use {
        kind: "ExprKind::Call",
        form: "an argument",
        body: "Probe::take($p);",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "ExprKind::Call",
        form: "an `inout` property argument",
        body: "Probe::bump(inout $p->count);",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Call",
        form: "making a method reference",
        body: "$f = $p->name(...);",
        does: Does::Passes("a method reference checks its object when it is called"),
    },
    Use {
        kind: "ExprKind::Call",
        form: "calling a method reference",
        body: "$f = $p->name(...); $f();",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Call",
        form: "`Core\\Json::encode`",
        body: "var $j = Core\\Json::encode($p);",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::MethodCall",
        form: "a method call",
        body: "$p->name();",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::MethodCall",
        form: "a `?->` method call",
        body: "$p?->name();",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::MethodCall",
        form: "a `static` method called through the object",
        body: "$p->make();",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::MethodCall",
        form: "`toString()`",
        body: "$p->toString();",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::MethodCall",
        form: "a delegated method",
        body: "$holder->name();",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::MethodCall",
        form: "`isRevoked()`",
        body: "$p->isRevoked();",
        does: Does::Passes("`isRevoked()`"),
    },
    Use {
        kind: "ExprKind::MethodCall",
        form: "a second `revoke()`",
        body: "$p->revoke();",
        does: Does::Passes("a second `revoke()`"),
    },
    Use {
        kind: "ExprKind::PropertyAccess",
        form: "a property read",
        body: "var $t = $p->title;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::PropertyAccess",
        form: "a `?->` property read",
        body: "var $t = $p?->title;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::ClassNameConst",
        form: "`::class`",
        body: "var $c = $p::class;",
        does: Does::Passes("`::class`"),
    },
    Use {
        kind: "ExprKind::Index",
        form: "an element that is the object",
        body: "var $q = [$p][0];",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "ExprKind::Clone",
        form: "`clone`",
        body: "var $q = clone $p;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Match",
        form: "a `match` on the object",
        body: "var $m = match ($p) { $other => 1, default => 2 };",
        does: Does::Passes(IDENTITY),
    },
    Use {
        kind: "ExprKind::Catch",
        form: "a `catch` expression",
        body: r#"var $t = $p->title catch (RecursionError $e) => "caught";"#,
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Yield",
        form: "`yield`",
        body: "foreach (Probe::each($p) as var $x) {}",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "ExprKind::Print",
        form: "`print`",
        body: "print $p;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Isset",
        form: "`isset` of a property",
        body: "var $b = isset($p->note);",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::IsInit",
        form: "`isinit` of a property",
        body: "var $b = isinit($p->late);",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Empty",
        form: "`empty` of a property",
        body: "var $b = empty($p->title);",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::Paren",
        form: "a parenthesized receiver",
        body: "var $t = ($p)->title;",
        does: Does::Throws,
    },
    Use {
        kind: "ExprKind::AnonObject",
        form: "an anonymous object field",
        body: "var $o = { post: $p };",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "StmtKind::Expr",
        form: "an expression statement",
        body: "$p->name();",
        does: Does::Throws,
    },
    Use {
        kind: "StmtKind::Return",
        form: "`return`",
        body: "Probe::same($p);",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "StmtKind::If",
        form: "an `if` condition",
        body: "if ($p->count > 0) {}",
        does: Does::Throws,
    },
    Use {
        kind: "StmtKind::While",
        form: "a `while` condition",
        body: "while ($p->count > 5) {}",
        does: Does::Throws,
    },
    Use {
        kind: "StmtKind::DoWhile",
        form: "a `do` `while` condition",
        body: "do {} while ($p->count > 5);",
        does: Does::Throws,
    },
    Use {
        kind: "StmtKind::For",
        form: "a `for` condition",
        body: "for (int $i = 0; $i < $p->count; $i++) {}",
        does: Does::Throws,
    },
    Use {
        kind: "StmtKind::Foreach",
        form: "a `foreach` over a revoked iterator",
        body: "foreach ($feed as var $n) {}",
        does: Does::Throws,
    },
    Use {
        kind: "StmtKind::Foreach",
        form: "a `foreach` over an array that has the object",
        body: "foreach ([$p] as var $x) {}",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "StmtKind::Switch",
        form: "a `switch` on the object",
        body: "switch ($p) { case $other: break; default: break; }",
        does: Does::Passes(IDENTITY),
    },
    Use {
        kind: "StmtKind::Echo",
        form: "`echo`",
        body: "echo $p;",
        does: Does::Throws,
    },
    Use {
        kind: "StmtKind::Unset",
        form: "`unset` of an element that is the object",
        body: "var $list = [$p]; unset($list[0]);",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "StmtKind::LocalDecl",
        form: "a typed variable",
        body: "Post $q = $p;",
        does: Does::Passes(PASSING),
    },
    Use {
        kind: "StmtKind::Destructure",
        form: "a destructuring assignment",
        body: "array<Post> $list = [$p]; [Post $a] = $list;",
        does: Does::Passes(PASSING),
    },
];

/// The kinds of `ast.rs` that cannot have a revocable object as an operand,
/// each with the reason.
const NO_OBJECT: &[(&str, &str)] = &[
    ("ExprKind::Null", "a literal"),
    ("ExprKind::Bool", "a literal"),
    ("ExprKind::Int", "a literal"),
    ("ExprKind::Float", "a literal"),
    ("ExprKind::Duration", "a literal"),
    ("ExprKind::Str", "a literal"),
    (
        "ExprKind::Variable",
        "reading a variable is passing the object",
    ),
    ("ExprKind::ConstFetch", "a constant is a compile-time value"),
    ("ExprKind::SelfExpr", "names a class"),
    ("ExprKind::StaticExpr", "names a class"),
    ("ExprKind::ParentExpr", "names a class"),
    (
        "ExprKind::StaticCall",
        "its left side is a class name or a class reference",
    ),
    (
        "ExprKind::StaticPropertyAccess",
        "its left side is a class name or a class reference",
    ),
    (
        "ExprKind::ClassConstAccess",
        "its left side is a class name or a class reference",
    ),
    (
        "ExprKind::New",
        "its target is a class name or a class reference",
    ),
    (
        "ExprKind::Fn",
        "a closure's body is a function, and its rows are its own",
    ),
    (
        "ExprKind::Throw",
        "a thrown value extends an error class, and only a root class is revocable",
    ),
    ("ExprKind::Exit", "takes an `int` or a `string`"),
    (
        "ExprKind::SpawnScript",
        "a script's options are copied as a graph copy is",
    ),
    (
        "ExprKind::Await",
        "awaits a task handle, which is a `Core` object",
    ),
    ("ExprKind::Require", "takes a path"),
    (
        "ExprKind::YieldFrom",
        "Novis has no `yield from`, and the parser reports it",
    ),
    ("ExprKind::Error", "a parse error"),
    ("StmtKind::Block", "has statements, not a value"),
    ("StmtKind::Empty", "has no value"),
    ("StmtKind::InlineHtml", "has text"),
    ("StmtKind::Break", "takes a level"),
    ("StmtKind::Continue", "takes a level"),
    ("StmtKind::Try", "has statements, not a value"),
    ("StmtKind::Global", "names variables"),
    ("StmtKind::Goto", "names a label"),
    ("StmtKind::StaticLocal", "a default is a compile-time value"),
    ("StmtKind::ClassDecl", "a declaration"),
    ("StmtKind::InterfaceDecl", "a declaration"),
    ("StmtKind::EnumDecl", "a declaration"),
    ("StmtKind::NamespaceDecl", "a declaration"),
    ("StmtKind::UseDecl", "a declaration"),
    ("StmtKind::AutoloadDecl", "a declaration"),
    ("StmtKind::TypeAliasDecl", "a declaration"),
    ("StmtKind::AttributeDecl", "a declaration"),
    ("StmtKind::TopLevelFunction", "a declaration"),
    (
        "StmtKind::TopLevelConst",
        "a constant is a compile-time value",
    ),
    ("StmtKind::Error", "a parse error"),
];

/// Every variant of `pub enum <name>` in `nvs-syntax`'s `ast.rs`, read from the
/// source, so a kind added there fails `every_syntax_kind_has_a_row` by name.
fn variants_of(name: &str) -> Vec<String> {
    let ast = include_str!("../../nvs-syntax/src/ast.rs");
    let head = format!("pub enum {name} {{\n");
    let start = ast
        .find(&head)
        .unwrap_or_else(|| panic!("`ast.rs` declares no `{name}`"))
        + head.len();
    let body = &ast[start..];
    let body = &body[..body.find("\n}\n").expect("the enum is closed")];
    body.lines()
        .filter_map(|line| line.strip_prefix("    "))
        .filter(|line| line.starts_with(|c: char| c.is_ascii_uppercase()))
        .map(|line| {
            let end = line
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(line.len());
            format!("{name}::{}", &line[..end])
        })
        .collect()
}

#[test]
fn every_syntax_kind_has_a_row() {
    let mut missing = Vec::new();
    for kind in variants_of("ExprKind")
        .into_iter()
        .chain(variants_of("StmtKind"))
    {
        let used = USES.iter().any(|row| row.kind == kind);
        let excused = NO_OBJECT.iter().any(|(name, _)| *name == kind);
        assert!(
            !(used && excused),
            "`{kind}` has a row and is listed as taking no object"
        );
        if !used && !excused {
            missing.push(kind);
        }
    }
    assert!(
        missing.is_empty(),
        "these syntax kinds have no row in `USES` or `NO_OBJECT`: {missing:?}"
    );
}

/// The program each row runs: `$p` and `$feed` are revoked, and the row's body
/// runs inside a `try` that reports whether it threw. No method body reads
/// `$this`, so a row throws only where the use itself is checked.
fn use_program(body: &str) -> String {
    format!(
        r#"<?nvs
interface Named
{{
    public function name(): string;
}}

class Post implements Revocable, Named, Stringable, Comparable
{{
    public int $count = 0;
    public ?string $note = null;
    public lateinit string $late;

    public function constructor(public string $title)
    {{}}

    public function name(): string
    {{
        return 'post';
    }}

    public function toString(): string
    {{
        return 'post';
    }}

    public function compareTo(self $other): int
    {{
        return 0;
    }}

    public static function make(): string
    {{
        return 'made';
    }}
}}

class Feed implements Revocable, Iterator<int>
{{
    public function advance(): bool
    {{
        return false;
    }}

    public function current(): int
    {{
        return 1;
    }}
}}

class Holder implements Named by $post
{{
    public function constructor(public Post $post)
    {{}}
}}

class Probe
{{
    public static function bump(inout int $n): void
    {{
        $n = $n + 1;
    }}

    public static function take(Post $p): string
    {{
        return 'taken';
    }}

    public static function same(Post $p): Post
    {{
        return $p;
    }}

    public static function each(Post $p): Iterator<Post>
    {{
        yield $p;
    }}

    public static function run(): string
    {{
        Post $p = new Post('a');
        Post $other = new Post('b');
        Feed $feed = new Feed();
        Holder $holder = new Holder($p);
        callable $f = $other->name(...);
        $p->revoke();
        $feed->revoke();
        try {{
            {body}
        }} catch (RevokedObjectError $e) {{
            return 'throws';
        }}
        return 'passes';
    }}
}}

echo Probe::run();
"#
    )
}

#[test]
fn a_revoked_object_does_what_its_row_says() {
    let mut wrong = Vec::new();
    for row in USES {
        let program = use_program(row.body);
        let ran = std::panic::catch_unwind(|| output_of(&program));
        let want = match row.does {
            Does::Throws => "throws",
            Does::Passes(_) => "passes",
        };
        match ran {
            Ok(out) if out.ends_with(want) => {}
            Ok(out) => wrong.push(format!(
                "{} ({}): wanted `{want}`, got `{out}`",
                row.kind, row.form
            )),
            Err(panic) => {
                let why = panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                    .unwrap_or_default();
                wrong.push(format!("{} ({}): did not run: {why}", row.kind, row.form));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} use forms do not do what their row says:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
