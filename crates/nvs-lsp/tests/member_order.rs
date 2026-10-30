//! `nvs_lsp::order`: the member order `rule:tooling/class-members-have-one-order`
//! gives, what moves with a member and what stays, the derived codec's notes,
//! and the kinds the editor's action is offered under.

use nvs_diagnostics::{PositionEncoding, SourceMap};
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::order::{self, NoteKind, Ordered, Refused};
use nvs_lsp::{Documents, actions, analyse, uri_of};

/// What [`order::of`] writes for `source`.
fn ordered(source: &str) -> Result<Ordered, Refused> {
    let mut map = SourceMap::new();
    let id = map.add("case.nvs", source.to_owned());
    order::of(map.file(id))
}

/// The ordered text of `source`, which must move something.
fn text(source: &str) -> String {
    ordered(source)
        .expect("the case parses")
        .text
        .expect("the case is out of order")
}

/// Asserts that each of `needles` appears in `text`, in this order.
fn in_order(text: &str, needles: &[&str]) {
    let at: Vec<usize> = needles
        .iter()
        .map(|needle| {
            text.find(needle)
                .unwrap_or_else(|| panic!("`{needle}` is not in:\n{text}"))
        })
        .collect();
    assert!(
        at.windows(2).all(|pair| pair[0] < pair[1]),
        "not in the order {needles:?}:\n{text}"
    );
}

#[test]
fn order_puts_groups_static_visibility_and_abstract_final_in_order() {
    let source = "<?nvs
abstract class Shop
{
    public function run(): void
    {
    }

    private int $count = 0;
    public const int LIMIT = 3;
    final public function close(): void
    {
    }

    abstract public function open(): void;
    protected static int $made = 0;
    type Id = int;
    private static function helper(): void
    {
    }

    public static function make(): void
    {
    }

    protected int $size = 1;
}
";
    in_order(
        &text(source),
        &[
            "type Id",
            "LIMIT",
            "$made",
            "$size",
            "$count",
            "function make",
            "function helper",
            "function open",
            "function close",
            "function run",
        ],
    );
}

#[test]
fn order_keeps_the_written_order_between_equal_members() {
    let source = "<?nvs
class Suite
{
    public function second(): void
    {
    }

    private int $x = 0;
    public function first(): void
    {
    }

    public function third(): void
    {
    }
}
";
    in_order(
        &text(source),
        &["$x", "function second", "function first", "function third"],
    );
}

#[test]
fn order_puts_the_constructor_first_among_instance_methods() {
    let source = "<?nvs
class Blog
{
    public function title(): string
    {
        return 'a';
    }

    private function constructor()
    {
    }

    public static function create(): Blog
    {
        return new Blog();
    }
}
";
    in_order(
        &text(source),
        &["function create", "function constructor", "function title"],
    );
}

#[test]
fn order_moves_comments_and_attributes_with_their_member() {
    let source = "<?nvs
class Shop
{
    public function run(): void
    {
    } // ends run

    // Explains the count.
    /// The number of items.
    #[Deprecated]
    public int $count = 0;
    // Stays at the end.
}
";
    let written = text(source);
    in_order(
        &written,
        &[
            "// Explains the count.",
            "/// The number of items.",
            "#[Deprecated]",
            "public int $count",
            "function run",
            "} // ends run",
            "// Stays at the end.",
        ],
    );
}

#[test]
fn order_moves_a_joined_declaration_as_one() {
    let source = "<?nvs
class Point
{
    public function sum(): int
    {
        return $this->a + $this->b;
    }

    public int $a = 0, $b = 0;
}
";
    in_order(
        &text(source),
        &["public int $a = 0, $b = 0;", "function sum"],
    );
}

#[test]
fn order_leaves_enum_cases_and_promoted_parameters_where_they_are() {
    let source = "<?nvs
enum Color
{
    Red,
    Blue,
    Green,
}

class Item
{
    public function name(): string
    {
        return $this->label;
    }

    public function constructor(private string $label, public int $weight)
    {
    }
}
";
    let written = text(source);
    in_order(&written, &["Red,", "Blue,", "Green,"]);
    in_order(
        &written,
        &[
            "function constructor(private string $label, public int $weight)",
            "function name",
        ],
    );
}

#[test]
fn order_keeps_the_instance_properties_of_a_keep_key_order_class() {
    let source = "<?nvs
use Core\\Json\\Derive;

#[Derive]
#[Core\\Json\\KeepKeyOrder(reason: 'the client reads the type key first')]
class Event
{
    public function constructor(string $type, int $at, string $note)
    {
        $this->type = $type;
        $this->at = $at;
        $this->note = $note;
    }

    private string $type;
    public int $at;
    protected string $note;
}
";
    let result = ordered(source).expect("the case parses");
    in_order(
        &result
            .text
            .expect("the constructor moves below the properties"),
        &["$type;", "$at;", "$note;", "function constructor"],
    );
    assert_eq!(result.notes, []);
}

#[test]
fn order_leaves_a_keep_key_order_class_whose_promoted_field_comes_first() {
    let source = "<?nvs
#[Core\\Json\\Derive]
#[Core\\Json\\KeepKeyOrder(reason: 'the client reads the id key first')]
class Event
{
    public function constructor(public int $id, string $kind)
    {
        $this->kind = $kind;
    }

    public string $kind;
}
";
    let result = ordered(source).expect("the case parses");
    assert_eq!(result.text, None);
    assert_eq!(result.moved, []);
    assert_eq!(result.notes.len(), 1);
    assert_eq!(result.notes[0].class.name, "Event");
    assert_eq!(result.notes[0].kind, NoteKind::KeptAsWritten);
}

#[test]
fn order_notes_a_derived_class_whose_key_order_changes() {
    let source = "<?nvs
namespace Shop;

use Core\\Json\\Derive;

#[Derive]
class Order
{
    private string $note;
    public int $id;

    public function constructor(int $id, string $note)
    {
        $this->id = $id;
        $this->note = $note;
    }
}

#[Derive]
class Line
{
    public int $count;
    private string $sku;

    public function constructor(int $count, string $sku)
    {
        $this->count = $count;
        $this->sku = $sku;
    }
}
";
    let result = ordered(source).expect("the case parses");
    in_order(
        result.text.as_deref().expect("`Order` moves"),
        &["public int $id;", "private string $note;"],
    );
    assert_eq!(
        result
            .notes
            .iter()
            .map(|note| (note.class.name.as_str(), note.kind))
            .collect::<Vec<_>>(),
        [("Shop\\Order", NoteKind::KeyOrderChanges)]
    );
    assert_eq!(
        result
            .moved
            .iter()
            .map(|class| class.name.as_str())
            .collect::<Vec<_>>(),
        ["Shop\\Order"]
    );
}

#[test]
fn order_of_an_ordered_file_is_no_edit() {
    let source = "<?nvs
class Shop
{
    public const int LIMIT = 3;
    public static int $made = 0;
    public int $size = 1;
    private int $count = 0;

    public function constructor()
    {
    }

    public function run(): void
    {
    }
}
";
    let result = ordered(source).expect("the case parses");
    assert_eq!(result.text, None);
    assert_eq!(result.moved, []);
    assert_eq!(result.notes, []);
}

#[test]
fn order_formats_the_file_it_writes() {
    let source = "<?nvs
class Shop {
    public function run(): void {
    }
    private int $count = 0;
    public function stop(): void {
    }
}

interface Fast {
    public function go(): void;
    public const int SPEED = 1;
}
";
    let written = text(source);
    let mut map = SourceMap::new();
    let id = map.add("case.nvs", written.clone());
    assert_eq!(
        nvs_fmt::format(map.file(id)).expect("the written file formats"),
        written
    );
    in_order(
        &written,
        &[
            "$count",
            "function run",
            "function stop",
            "SPEED",
            "function go",
        ],
    );
}

#[test]
fn order_offers_nothing_for_a_file_with_a_syntax_error() {
    let source = "<?nvs
class Shop
{
    public function run(): void
    {
    }

    private int $count = ;
}
";
    assert_eq!(ordered(source), Err(Refused::DoesNotParse));
}

#[test]
fn order_action_is_offered_under_source_and_never_under_fix_all() {
    let source = "<?nvs
class Shop
{
    public function run(): void
    {
    }

    private int $count = 0;
}
";
    let dir = nvs_repo::scratch("lsp-member-order");
    let uri = uri_of(&dir.join("case.nvs")).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysed = analyse(&documents, &uri).expect("an open document analyses");
    let kinds = |only: &[&str]| -> Vec<String> {
        let only: Vec<lsp_types::CodeActionKind> = only
            .iter()
            .map(|kind| lsp_types::CodeActionKind::from((*kind).to_owned()))
            .collect();
        actions::at(
            &analysed,
            &Documents::new(),
            &CompletionFiles::default(),
            0,
            0,
            actions::Kind::asked_for(Some(&only)),
            PositionEncoding::Utf8,
        )
        .into_iter()
        .map(|action| action.kind)
        .collect()
    };
    assert_eq!(kinds(&[order::KIND]), [order::KIND]);
    assert_eq!(kinds(&["source"]), [order::KIND]);
    assert!(!kinds(&["source.fixAll.nvs"]).contains(&order::KIND.to_owned()));
    assert!(!kinds(&["quickfix"]).contains(&order::KIND.to_owned()));
    assert!(!kinds(&[]).contains(&order::KIND.to_owned()));
}
