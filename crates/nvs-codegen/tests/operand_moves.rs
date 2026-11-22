//! A value an earlier operand read stays alive while a later operand moves
//! the local it was read from
//! (`rule:expressions/an-operand-keeps-its-value-while-later-operands-run`).
//!
//! Each program reads part of a local, then passes that local to a later
//! operand that the ownership pass may move it into: a Novis callee, an owning
//! `Core` member or a spread. The callee drops the value and allocates many
//! strings of the same size, so a read that kept no reference of its own shows
//! a freed string, or crashes. A use after free does not always change the
//! output, so each case also reads the lowered function and finds the `retain`
//! of the earlier read.
//!
//! Every case runs with in-place changes on and off, and both runs print the
//! same text. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_ir::ir::InstKind;

/// The classes every case uses. Each `take` drops what it was given, then
/// allocates many strings of the same size as the value read before it.
const CLASSES: &str = r#"<?nvs
class Post {
    public string $title = "";

    public function constructor(string $title) {
        $this->title = $title;
    }
}

class Shop {
    public static function takeStrings(array<string> $x): string {
        $x = [];
        Shop::fill();
        return "n";
    }

    public static function takePosts(array<Post> $x): string {
        $x = [];
        Shop::fill();
        return "n";
    }

    public static function takePost(Post $x): string {
        $x = new Post("");
        Shop::fill();
        return "n";
    }

    public static function pair(string $a, string $b): string {
        return $a . $b;
    }

    public static function fill(): void {
        array<string> $fill = [];
        for (int $i = 0; $i < 256; $i++) { $fill[] = Core\Str::repeat("y", 64); }
    }
}
"#;

/// The text every read value has: 64 `x`.
fn whole() -> String {
    "x".repeat(64)
}

/// What is read before the move, and the local the move takes.
#[derive(Clone, Copy, Debug)]
enum Read {
    /// `$a["k"]`, a string element of an `array<string>`.
    Element,
    /// `$a[0]->title`, the property of an object in an `array<Post>`.
    PropertyOfElement,
    /// `$p->title`, the string property of a `Post` local.
    Property,
}

impl Read {
    /// The statement that declares the local.
    fn setup(self) -> &'static str {
        match self {
            Self::Element => {
                r#"array<string> $a = ["k" => Core\Str::repeat("x", 64), "j" => "z"];"#
            }
            Self::PropertyOfElement => {
                r#"array<Post> $a = [new Post(Core\Str::repeat("x", 64)), new Post("z")];"#
            }
            Self::Property => r#"Post $p = new Post(Core\Str::repeat("x", 64));"#,
        }
    }

    /// The earlier operand.
    fn expr(self) -> &'static str {
        match self {
            Self::Element => r#"$a["k"]"#,
            Self::PropertyOfElement => "$a[0]->title",
            Self::Property => "$p->title",
        }
    }
}

/// The later operand, which passes the whole local on. Each is a `string`.
#[derive(Clone, Copy, Debug)]
enum Move {
    /// A known Novis callee takes the local.
    Callee,
    /// An owning `Core` member takes the local.
    CoreMember,
    /// A spread copies the local's elements into a new array.
    Spread,
}

impl Move {
    /// The later operand for `read`, or `None` where the move does not apply
    /// to the local's type: an object cannot be spread or passed to an array
    /// member.
    fn expr(self, read: Read) -> Option<&'static str> {
        Some(match (self, read) {
            (Self::Callee, Read::Element) => "Shop::takeStrings($a)",
            (Self::Callee, Read::PropertyOfElement) => "Shop::takePosts($a)",
            (Self::Callee, Read::Property) => "Shop::takePost($p)",
            (Self::CoreMember, Read::Element | Read::PropertyOfElement) => {
                r#"Core\Str::repeat("n", Core\Arr::count(Core\Arr::withoutFirst($a)))"#
            }
            (Self::Spread, Read::Element) => {
                r#"Core\Str::repeat("n", Core\Arr::count([...$a, "w"]))"#
            }
            (Self::Spread, Read::PropertyOfElement) => {
                r#"Core\Str::repeat("n", Core\Arr::count([...$a, new Post("w")]))"#
            }
            (Self::CoreMember | Self::Spread, Read::Property) => return None,
        })
    }
}

/// The operand site that consumes both operands.
#[derive(Clone, Copy, Debug)]
enum Site {
    /// `"v=" . read . move`.
    Concat,
    /// `read == move`, a binary operator.
    Binary,
    /// `Shop::pair(read, move)`, the arguments of a Novis call.
    CallArg,
    /// `Core\Str::startsWith(read, move)`, the arguments of a `Core` call.
    CoreArg,
}

impl Site {
    /// The statement that prints what the site gives.
    fn statement(self, read: &str, moved: &str) -> String {
        match self {
            Self::Concat => format!(r#"echo "v=" . {read} . {moved}, "\n";"#),
            Self::Binary => format!(r#"echo {read} == {moved} ? "eq" : "ne", "\n";"#),
            Self::CallArg => format!(r#"echo Shop::pair({read}, {moved}), "\n";"#),
            Self::CoreArg => {
                format!(r#"echo Core\Str::startsWith({read}, {moved}) ? "t" : "f", "\n";"#)
            }
        }
    }

    /// Whether the site prints the whole value it read.
    fn prints_the_read(self) -> bool {
        matches!(self, Self::Concat | Self::CallArg)
    }
}

/// The program for one case. `Cart::run` holds only the case, so the retain
/// the test looks for is the one the case owes.
fn program(site: Site, read: Read, moved: &str) -> String {
    format!(
        "{CLASSES}
class Cart {{
    public static function run(): void {{
        {setup}
        {statement}
    }}
}}
Cart::run();
",
        setup = read.setup(),
        statement = site.statement(read.expr(), moved),
    )
}

/// Whether `Cart::run` retains the result of an element or property read.
fn retains_a_read(program: &nvs_ir::Program) -> bool {
    let run = program
        .functions
        .iter()
        .find(|function| function.name.ends_with("Cart::run"))
        .expect("Cart::run was lowered");
    let insts = || run.blocks.iter().flat_map(|block| &block.insts);
    let reads: Vec<_> = insts()
        .filter(|inst| {
            matches!(
                inst.kind,
                InstKind::ArrayGet { .. } | InstKind::FieldGet { .. }
            )
        })
        .filter_map(|inst| inst.result)
        .collect();
    insts()
        .any(|inst| matches!(inst.kind, InstKind::Retain { operand } if reads.contains(&operand)))
}

#[test]
fn an_operand_read_before_a_moved_local_is_read_whole() {
    let sites = [Site::Concat, Site::Binary, Site::CallArg, Site::CoreArg];
    let reads = [Read::Element, Read::PropertyOfElement, Read::Property];
    let moves = [Move::Callee, Move::CoreMember, Move::Spread];
    let mut cases = 0;
    for site in sites {
        for read in reads {
            for mv in moves {
                let Some(moved) = mv.expr(read) else {
                    continue;
                };
                cases += 1;
                let source = program(site, read, moved);
                let case = format!("{site:?} reading {read:?} before a {mv:?} move");
                let in_place = common::lower(&source);
                let copying = nvs_ir::lower::without_in_place(|| common::lower(&source));
                for (way, lowered) in [("in place", &in_place), ("not in place", &copying)] {
                    assert!(
                        retains_a_read(lowered),
                        "{case}, {way}: the earlier read has no retain of its own"
                    );
                }
                let printed = output_of_program(&in_place);
                assert_eq!(
                    printed,
                    output_of_program(&copying),
                    "{case}: the two ways print different text"
                );
                if site.prints_the_read() {
                    assert!(
                        printed.contains(&whole()),
                        "{case}: the read value is not whole in {printed:?}"
                    );
                }
            }
        }
    }
    assert_eq!(cases, 28, "every site times every move that applies");
}

/// The first program of the review that found the hole: an element read, then
/// a call that is moved the array.
#[test]
fn the_reported_concatenation_prints_the_element_it_read() {
    let source = r#"<?nvs
class Shop {
    public static function len(array<string> $x): string {
        $x = [];
        array<string> $fill = [];
        for (int $i = 0; $i < 64; $i++) { $fill[] = Core\Str::repeat("y", 64); }
        return "n";
    }
    public static function main(): void {
        array<string> $a = ["k" => Core\Str::repeat("x", 64)];
        echo "v=" . $a["k"] . Shop::len($a), "\n";
    }
}
Shop::main();
"#;
    let expected = format!("v={}n\n", whole());
    assert_eq!(output_of(source), expected);
    let copying = nvs_ir::lower::without_in_place(|| common::lower(source));
    assert_eq!(output_of_program(&copying), expected);
}
