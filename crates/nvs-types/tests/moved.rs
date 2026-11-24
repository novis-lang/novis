//! `rule:core-api/a-moved-name-says-where-it-went`: the pass that adds the
//! help line and the fix to an error about a moved `Core` name.
//!
//! `nvs_stdlib::moved::MOVES` is empty while Novis is Pre-Alpha, so these
//! tests run the pass over made-up rows through
//! [`nvs_types::moved::note_moves_in`]. Each old name is one `Core` never had,
//! and each new name is one it has, so a fixed program compiles.

use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_hir::{CoreRoster, resolve_program};
use nvs_stdlib::moved::Move;
use nvs_syntax::parse_file;
use nvs_types::check::check_program;
use nvs_types::expr_table::ExprTypeTable;
use nvs_types::moved::note_moves_in;
use nvs_types::ty::TypeInterner;

/// Made-up moves: a type, a renamed member, a member whose arguments changed
/// and a member removed with no one replacement.
const MOVES: &[Move] = &[
    Move::renamed(r"Core\Sorting", r"Core\Arr::Order"),
    Move::renamed(r"Core\Str::size", r"Core\Str::length"),
    Move {
        old: r"Core\Str::measure",
        new: r"Core\Str::length",
        since: Some("0.9.0"),
        call: Some(r"Core\Str::length($text)"),
        help: None,
    },
    Move::removed(
        r"Core\Str::count",
        r"Core\Str::length",
        "`Core\\Str::count` was removed. Write `Core\\Str::length` or `Core\\Str::countOf`",
    ),
];

/// The diagnostics of `src`, resolved against `Core`'s roster as `nvs check`
/// resolves it, with the pass run over [`MOVES`].
fn check(src: &str) -> Diagnostics {
    let mut map = SourceMap::new();
    let entry = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(entry), &mut diags);
    let core = nvs_stdlib::registry::link_targets();
    let closed = nvs_stdlib::registry::closed_interfaces();
    let sealed = nvs_stdlib::registry::sealed_classes();
    let imports = nvs_types::core_lib::importable();
    let (module, loaded, _autoload) = resolve_program(
        entry,
        stmts,
        &mut map,
        CoreRoster::Names {
            links: &core,
            closed: &closed,
            sealed: &sealed,
            imports: &imports,
        },
        &mut diags,
    );
    let program: Vec<nvs_types::ProgramFile<'_>> = loaded
        .iter()
        .map(|file| nvs_types::ProgramFile {
            src: map.file(file.id),
            stmts: &file.stmts,
        })
        .collect();
    check_program(
        &program,
        &module,
        &mut TypeInterner::new(),
        &mut ExprTypeTable::new(),
        &mut diags,
    );
    drop(program);
    note_moves_in(MOVES, &mut diags, &map);
    diags
}

/// Every help line the pass wrote.
fn helps(diags: &Diagnostics) -> Vec<String> {
    diags
        .iter()
        .flat_map(|d| &d.notes)
        .filter(|note| note.contains("moved to") || note.contains("was removed"))
        .cloned()
        .collect()
}

/// `src` with every edit the pass offers applied, from the end backwards.
fn fixed(src: &str, diags: &Diagnostics) -> String {
    let mut edits: Vec<(u32, u32, String)> = diags
        .iter()
        .flat_map(|d| &d.suggestions)
        .filter(|s| {
            s.safe
                && (s.message == "write the new name"
                    || s.message.starts_with("delete the `use` line"))
        })
        .map(|s| (s.span.start, s.span.end, s.replacement.clone()))
        .collect();
    edits.sort();
    edits.dedup();
    let mut out = src.to_owned();
    for (start, end, text) in edits.into_iter().rev() {
        out.replace_range(start as usize..end as usize, &text);
    }
    out
}

const TYPE: &str = r#"<?nvs
use Core\Sorting;

class Pick {
    public static function rank(Sorting $order): int { return $order == Sorting::Desc ? 1 : 0; }
}

Core\Sorting $order = Core\Sorting::Desc;
echo Pick::rank($order) . "\n";
"#;

const TYPE_FIXED: &str = r#"<?nvs

class Pick {
    public static function rank(Core\Arr::Order $order): int { return $order == Core\Arr::Order::Desc ? 1 : 0; }
}

Core\Arr::Order $order = Core\Arr::Order::Desc;
echo Pick::rank($order) . "\n";
"#;

/// A moved type in a `use` line, a type and an expression each get the help
/// line, and the fix deletes the `use` line and writes the new name at every
/// reference.
#[test]
fn a_moved_type_says_where_it_went_and_its_fix_compiles() {
    let diags = check(TYPE);
    let helps = helps(&diags);
    assert!(
        helps.iter().any(|h| h.contains(
            "`Core\\Sorting` moved to `Core\\Arr::Order`. A `use` line cannot import it."
        )),
        "{diags:?}"
    );
    assert!(
        helps.iter().any(|h| h.contains(
            "`Core\\Sorting` moved to `Core\\Arr::Order`. Write `Core\\Arr::Order` here"
        )),
        "{diags:?}"
    );
    let out = fixed(TYPE, &diags);
    assert_eq!(out, TYPE_FIXED);
    assert!(!check(&out).has_errors(), "{:?}", check(&out));
}

/// A renamed member's fix writes the new member, a member whose arguments
/// changed gets the call to write, and a removed member gets its own help.
#[test]
fn a_moved_member_says_where_it_went() {
    let src = "<?nvs\nstring $text = 'abc';\necho Core\\Str::size($text), Core\\Str::measure($text, 1), Core\\Str::count($text);\n";
    let diags = check(src);
    let helps = helps(&diags);
    assert!(
        helps.iter().any(|h| h.contains(
            "`Core\\Str::size` moved to `Core\\Str::length`. Write `Core\\Str::length` here"
        )),
        "{diags:?}"
    );
    assert!(
        helps.iter().any(|h| h.contains(
            "`Core\\Str::measure` moved to `Core\\Str::length` in 0.9.0, which takes other arguments. Write `Core\\Str::length($text)` here"
        )),
        "{diags:?}"
    );
    assert!(
        helps
            .iter()
            .any(|h| h.contains("`Core\\Str::count` was removed. Write `Core\\Str::length`")),
        "{diags:?}"
    );
    assert_eq!(
        fixed(src, &diags),
        "<?nvs\nstring $text = 'abc';\necho Core\\Str::length($text), Core\\Str::measure($text, 1), Core\\Str::count($text);\n",
        "only the rename has a fix"
    );
}

/// With the real table, which is empty while Novis is Pre-Alpha, an unknown
/// name gets no help line from the pass.
#[test]
fn the_real_table_adds_nothing_while_pre_alpha() {
    assert!(nvs_stdlib::moved::MOVES.is_empty());
    let mut diags = check(TYPE);
    let before = diags.iter().map(|d| d.notes.len()).sum::<usize>();
    let map = SourceMap::new();
    nvs_types::moved::note_moves(&mut diags, &map);
    assert_eq!(diags.iter().map(|d| d.notes.len()).sum::<usize>(), before);
}
