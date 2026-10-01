//! What a cursor is offered where the token after the one it triggered on is
//! not the name the grammar wanted.
//!
//! Here rather than beside `nvs_lsp::completion`'s unit tests because the
//! answer is what a whole analysis resolved: the receiver's class comes from
//! the checker's own table, so a fixture would be asserting that a type this
//! file wrote survives being copied.
//!
//! The claim is one the `.lspt` corpus cannot make in a single case. Each case
//! under `tests/lsp/completion/` freezes one rendering at one cursor
//! (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`); what is asserted here
//! is that every one of those renderings is the *same* one — a receiver at the
//! end of a block, at the end of the file, before a `)` and before a `]`
//! answers exactly what the identical receiver above another statement answers.
//! The parser stops the access at the arrow in all four, so the closing
//! delimiter is the only thing that varies and none of it may reach the answer.
//!
//! The namespace arm is here for the second reason as well as the first: what
//! `Core\` offers is every class a roster holds, and a case freezing that list
//! would be a copy of `nvs_stdlib::registry` that goes stale the day a class
//! is added. What is asserted is the shape of a row and the absence of a
//! keyword, which is the claim
//! `rule:ide/completion-offers-only-what-the-compiler-derived` makes.
//!
//! The last two tests ask nothing of an analysis at all: they read
//! `nvs_lsp::completion`'s own source and enumerate where each offered value
//! comes from. That is the form the same rule asks for — a test, not review —
//! and it lives here because what it enumerates is what the cases above
//! freeze.

use std::fs;
use std::path::Path;

use lsp_types::PositionEncodingKind;
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::{
    Analysed, CheckScope, Client, Documents, PhpNames, Response, SymbolIndex, analyse, completion,
    uri_of,
};
use nvs_stdlib::{php_names, registry};

/// The class every document below resolves its receiver to.
const CLASS: &str =
    r#"class User { public string $name; public function greet(): string { return "hi"; } }"#;

/// The receiver whose member half is empty in every document.
const RECEIVER: &str = "$b->";

/// `source` analysed as its own entry point, out of an open buffer alone —
/// nothing here `require`s anything, so no directory is needed
/// (`nvs_diagnostics::SourceMap::load`).
fn analysed(source: &str) -> (Documents, Analysed) {
    analysed_at(
        &std::env::temp_dir().join("nvs-completion-case.nvs"),
        source,
    )
}

/// [`analysed`], with the buffer opened under `path`. Nothing is written
/// there: the path only says which directory the document sits in.
fn analysed_at(path: &Path, source: &str) -> (Documents, Analysed) {
    let uri = uri_of(path).expect("a test path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysis = analyse(&documents, &uri).expect("an open document analyses");
    (documents, analysis)
}

/// What is offered at `at` in `source`, rendered as a `.lspt` case freezes it.
///
/// The index is built the way the server builds one — `SymbolIndex::build` is
/// the crate's single construction site — over the same open buffer, so what
/// the namespace arm reaches here is what a session's would reach.
fn rendered(source: &str, at: u32) -> String {
    let (documents, analysis) = analysed(source);
    rendered_with(&documents, &analysis, at)
}

/// [`rendered`], over an analysis already made.
fn rendered_with(documents: &Documents, analysis: &Analysed, at: u32) -> String {
    let index = SymbolIndex::build(documents, CheckScope::Open, None);
    Response::Completion(completion::at(
        analysis,
        &index,
        at,
        PhpNames::All,
        Client::default(),
        PositionEncoding::Utf8,
    ))
    .render()
}

/// The offset just past the first `written` in `source`.
fn after(source: &str, written: &str) -> u32 {
    let at = source.find(written).expect("the document writes it") + written.len();
    u32::try_from(at).expect("a test document is short")
}

/// What is offered immediately after the one trailing arrow in `source`,
/// rendered as a `.lspt` case freezes it.
fn offered(source: &str) -> String {
    rendered(source, after(source, RECEIVER))
}

/// The kind column of each offered row, which is what says which arm answered.
fn kinds(answer: &str) -> Vec<&str> {
    answer
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .collect()
}

/// A document whose receiver is followed by `tail`.
fn document(tail: &str) -> String {
    format!("<?nvs\n{CLASS}\nvar $b = new User();\n{tail}")
}

/// The instance half of `User`, which is the whole answer in every shape below.
///
/// Its spelling is `nvs_lsp::render`'s and not this file's
/// (`rule:ide/the-rendering-has-one-home`), so the corpus and this test freeze
/// one text.
const MEMBERS: &str = "greet() method    string\nname    property  string\n";

/// A member list is what a receiver offers whatever closes the construct it
/// was written in.
///
/// The first row is the shape that has always worked, and it is in the list as
/// the control: `$b->` followed by another statement parses as a method call
/// whose member name is `if`, so the access runs past the cursor and the
/// cursor is inside it. In the other four nothing follows the arrow that the
/// grammar can take for a member name, so the access ends exactly at the
/// cursor — and a cursor at a node's end is outside it, which is
/// `nvs_syntax::SyntaxIndex::at`'s half-open containment and stays that way
/// because `selectionRange` is frozen on it.
#[test]
fn a_member_list_survives_a_receiver_at_the_end_of_a_block() {
    let shapes = [
        ("another statement", "$b->\nif (true) {\n"),
        ("the end of a block", "if (true) {\n    $b->\n}\n"),
        ("the end of the file", "$b->\n"),
        ("a closing parenthesis", "echo ($b->);\nif (true) {\n"),
        ("a closing bracket", "var $a = [$b->];\nif (true) {\n"),
    ];
    for (delimiter, tail) in shapes {
        assert_eq!(
            offered(&document(tail)),
            MEMBERS,
            "a receiver before {delimiter} is offered something other than its class's members"
        );
    }
}

/// Two types in a namespace of the document's own, so `App\` has an index side
/// to answer from.
const NAMESPACED: &str = "<?nvs\nnamespace App;\ninterface Greets {}\nclass User {}\n";

/// A namespace holds the types declared under it, from both rosters that
/// declare one, and holds no word at all.
///
/// The two halves are one claim asked twice. `App\` can only be answered from
/// the workspace index, because the document that declares those two types is
/// the workspace; `Core\` can only be answered from `nvs_stdlib::registry`,
/// which is the table `Core\Str::` already reads a member list off. Neither
/// answer may carry a keyword: `\` is a trigger character, so the position
/// list reaching this cursor is the defect this arm closes
/// (`rule:ide/completion-offers-only-what-the-compiler-derived`).
#[test]
fn a_namespace_segment_offers_the_registry_and_the_index_and_no_keywords() {
    let workspace = format!("{NAMESPACED}App\\\n");
    assert_eq!(
        rendered(&workspace, after(&workspace, "App\\")),
        "Greets  interface App\\Greets\nUser    class     App\\User\n",
        "a namespace segment is not offered the types the index holds under it"
    );

    let core = "<?nvs\nCore\\\n";
    let offered = rendered(core, after(core, "Core\\"));
    assert!(
        offered.contains("Arr     class     Core\\Arr\n"),
        "`Core\\` is not offered a registry class: {offered}"
    );
    // The label is the *rest* of the name and not its last segment, which is
    // what makes a nested one insertable: replacing the word being completed
    // with `Regex\Match` writes a name that resolves.
    assert!(
        offered.contains("Regex\\Match class     Core\\Regex\\Match\n"),
        "`Core\\` is not offered a class nested a namespace deeper: {offered}"
    );
    let words: Vec<&str> = kinds(&offered)
        .into_iter()
        .filter(|kind| !matches!(*kind, "class" | "enum"))
        .collect();
    assert!(
        words.is_empty(),
        "`Core\\` is offered something that is not a type: {words:?}"
    );
}

/// A body owning a `type` alias beside a constant, with the alias named from a
/// parameter's type — the one place `Owner::` is written that the parser builds
/// no access node for.
const OWNER: &str = "<?nvs\nclass Order {\n  type Meta = {total: int};\n  public const int MAX = \
                     10;\n  public function of(Order::Meta $m): int { return 1; }\n}\n";

/// `Owner::` in type position offers what may stand in a type — the `type`
/// aliases the owner declares and its class constants — and nothing that
/// cannot, so the method beside them is absent (`rule:types/type-alias`).
///
/// Asked twice, of a document that parses and of the same one being typed into,
/// because the two are one question here: a written type is not a node, so the
/// answer is read off the source in both and neither depends on what the parser
/// made of the rest of the line.
#[test]
fn completion_after_owner_double_colon_in_type_position_offers_the_alias() {
    assert_eq!(
        rendered(OWNER, after(OWNER, "of(Order::")),
        "MAX = 10 constant  int\nMeta    typeParameter {total: int}\n"
    );
    let typing =
        "<?nvs\nclass Order {\n  type Meta = {total: int};\n  public function of(Order::\n}\n";
    assert_eq!(
        rendered(typing, after(typing, "of(Order::")),
        "Meta    typeParameter {total: int}\n"
    );
}

/// An import, a class and an interface, each reachable at a bare cursor by a
/// different one of the three spellings.
const IN_REACH: &str =
    "<?nvs\nnamespace App;\nuse Core\\Str;\ninterface Greets {}\nclass User {}\n";

/// A bare name reaches the imports in force and the declarations the index
/// holds, and still reaches the words a statement may open with.
///
/// The three labels are the three spellings, and the rule behind them is that
/// each one resolves where it was offered: `Str` because a `use` put it there,
/// `User` and `Greets` because the cursor is inside the namespace that
/// declares them. The keyword row is the other half of the claim — this arm is
/// added to the position list and does not replace it
/// (`rule:ide/completion-offers-only-what-the-compiler-derived`).
#[test]
fn a_bare_name_offers_the_imports_in_force_and_the_declarations_in_the_index() {
    let source = format!("{IN_REACH}Nam\n");
    let offered = rendered(&source, after(&source, "Nam"));
    for row in [
        "Str [Core] class\n",
        "User [App] class\n",
        "Greets [App] interface\n",
        "return  keyword\n",
    ] {
        assert!(
            offered.contains(row),
            "a bare name is not offered `{}`: {offered}",
            row.trim_end()
        );
    }
}

/// A document whose last line is `written`, at a statement position with a
/// local already declared above it.
fn writing(written: &str) -> String {
    format!("<?nvs\nvar $total = 1;\n{written}")
}

/// What is offered at the end of `source`, as the items themselves.
///
/// The rendering is not the question here: what a PHP-name item may put in a
/// buffer is `insert_text`, and `nvs_lsp::render` freezes a label, a kind and a
/// detail — three fields a client shows and none it types.
fn ending_items(source: &str, php: PhpNames) -> Vec<lsp_types::CompletionItem> {
    let (documents, analysis) = analysed(source);
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);
    let at = u32::try_from(source.len()).expect("a test document is short");
    completion::at(
        &analysis,
        &index,
        at,
        php,
        Client::default(),
        PositionEncoding::Utf8,
    )
}

/// The prefix every PHP-name case below writes, chosen because the inventory
/// lists a run of names under it whose migration rows are not all one shape.
const HALF_WRITTEN: &str = "str";

/// A half-written name reaches the PHP inventory, and two characters do not.
///
/// The candidate list is `nvs_stdlib::php_names`'s and not a copy: what is
/// asserted is that every name the table answers for the written prefix is
/// offered, which is
/// `rule:php-migration/every-php-builtin-is-a-completion-candidate`'s claim that
/// the inventory is complete from the first day whatever the migration table's
/// coverage. The second half is the bound the module's own
/// `PHP_PREFIX` states: a name is a candidate always and is *offered* once enough of it is
/// written, so a two-character cursor still gets the program's own words and
/// none of PHP's.
#[test]
fn a_half_written_name_reaches_the_php_inventory_and_a_shorter_one_does_not() {
    let source = writing(HALF_WRITTEN);
    let labels: Vec<String> = ending_items(&source, PhpNames::All)
        .into_iter()
        .map(|item| item.label)
        .collect();
    let candidates = php_names::starting_with(HALF_WRITTEN);
    assert!(
        !candidates.is_empty(),
        "the oracle inventory lists no name under `{HALF_WRITTEN}`"
    );
    for candidate in candidates {
        assert!(
            labels.iter().any(|label| label == candidate.php),
            "`{}` is a candidate under `{HALF_WRITTEN}` and was not offered",
            candidate.php
        );
    }

    let shorter = &HALF_WRITTEN[..2];
    let early: Vec<String> = ending_items(&writing(shorter), PhpNames::All)
        .into_iter()
        .map(|item| item.label)
        .collect();
    for candidate in php_names::starting_with(shorter) {
        assert!(
            !early.iter().any(|label| label == candidate.php),
            "`{}` is offered where only `{shorter}` has been written",
            candidate.php
        );
    }
}

/// Nothing a PHP-name item types is a name the registry does not hold, and the
/// three shapes that insert nothing leave the buffer as it was.
///
/// `rule:php-migration/an-item-inserts-only-a-registered-member` is the whole
/// of this: an editor that inserts a call which then fails to resolve is worse
/// than one that offers nothing, so the only text these items carry is a
/// `Core` member `nvs_stdlib::registry` actually holds — the same table the
/// checker resolves a call against. The other three shapes carry the characters
/// the developer already typed, which is how "inserts nothing" is asserted
/// against a client that replaces the word being completed with whatever it is
/// given.
#[test]
fn a_php_item_inserts_only_a_member_the_registry_holds() {
    let source = writing(HALF_WRITTEN);
    let offered = ending_items(&source, PhpNames::All);
    let mut seen = 0;
    for candidate in php_names::starting_with(HALF_WRITTEN) {
        let inserted: Vec<String> = offered
            .iter()
            .filter(|item| item.label == candidate.php)
            .map(|item| {
                item.insert_text.clone().unwrap_or_else(|| {
                    panic!("`{}` says nothing about what it types", candidate.php)
                })
            })
            .collect();
        let owed: Vec<String> = candidate
            .items()
            .iter()
            .map(|shape| shape.insertion().unwrap_or_else(|| HALF_WRITTEN.to_owned()))
            .collect();
        assert_eq!(
            inserted, owed,
            "`{}` is offered items that type something its row does not",
            candidate.php
        );
        seen += inserted.len();
        for text in inserted.iter().filter(|text| *text != HALF_WRITTEN) {
            let (class, member) = text
                .rsplit_once("::")
                .unwrap_or_else(|| panic!("`{text}` is not a `Core` member spelling"));
            let core = registry::class(class)
                .unwrap_or_else(|| panic!("`{text}` names a class the registry does not hold"));
            assert!(
                core.members().any(|row| row.name == member) || core.constant(member).is_some(),
                "`{text}` names a member the registry does not hold"
            );
        }
    }
    assert!(seen > 0, "no PHP name was offered under `{HALF_WRITTEN}`");
}

/// `nvs.completion.phpNames` selects among the four item shapes and touches
/// nothing else a position is offered.
///
/// The three values are one lever over one arm
/// (`rule:ide/contributions-are-frozen-and-only-ever-added` freezes the
/// spelling): `off` is the developer who never wants PHP in this editor,
/// `resolved` the one who wants only the names that go somewhere, and `all` —
/// the default — the one converting a codebase, for whom the dropped and
/// undecided rows are the audit. What no value may do is take the words that
/// open a statement away with them.
#[test]
fn the_php_names_setting_selects_among_the_shapes_and_nothing_else() {
    let source = writing(HALF_WRITTEN);
    // A candidate under the written prefix, and not any label the inventory
    // happens to spell: a reserved word this position offers can be a PHP
    // built-in's name too, and that row came from the grammar.
    let candidate = |item: &lsp_types::CompletionItem| {
        php_names::starting_with(HALF_WRITTEN)
            .iter()
            .any(|row| row.php == item.label)
    };
    let candidates =
        |items: &[lsp_types::CompletionItem]| items.iter().filter(|i| candidate(i)).count();

    let off = ending_items(&source, PhpNames::Off);
    assert_eq!(
        candidates(&off),
        0,
        "`off` still offers a PHP name: {:?}",
        off.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
    assert!(
        off.iter().any(|item| item.label == "var"),
        "`off` took the words that open a statement with it"
    );

    let resolved = ending_items(&source, PhpNames::Resolved);
    assert!(candidates(&resolved) > 0, "`resolved` offers no PHP name");
    for item in &resolved {
        if candidate(item) {
            let inserted = item.insert_text.as_deref().unwrap_or_default();
            assert_ne!(
                inserted, HALF_WRITTEN,
                "`{}` inserts nothing and is offered under `resolved`",
                item.label
            );
        }
    }
    assert!(
        candidates(&resolved) < candidates(&ending_items(&source, PhpNames::All)),
        "`resolved` and `all` offer the same list, so one of them is not doing its job"
    );
}

/// Each trigger character `initialize` declares, and the construct it is one
/// character of.
///
/// Each is the last character of its construct, because a client fires on the
/// keystroke and `nvs_lsp::completion::continues_a_trigger` answers only where
/// the whole spelling was written. The open tag's construct leaves code first:
/// `<?` is half-written only in a run of markup.
///
/// The third column is what follows the cursor. A path character's construct
/// is a `require` literal, closed after the cursor the way an editor closes a
/// quote, in a document placed in this crate's own directory: a quote lists
/// that directory, and `/` lists the one above it.
const TRIGGERED: [(&str, &str, &str); 8] = [
    (">", "$b->", ""),
    (":", "User::", ""),
    ("\\", "Core\\", ""),
    ("$", "echo $", ""),
    ("?", "?>\n<?", ""),
    ("'", "require '", "';"),
    ("\"", "require \"", "\";"),
    ("/", "require '../", "';"),
];

/// A class with both halves declared, so every construct below has something
/// of its own to answer with.
const TRIGGER_DOC: &str = "<?nvs\nclass User {\n    public static int $count = 0;\n    \
     public string $name = \"\";\n    public function greet(): string { return \"hi\"; }\n}\n\
     var $b = new User();\n";

/// A declared trigger character is a promise, and an unanswered one is worse
/// than no trigger at all.
///
/// A client asks on the keystroke, so a character `initialize` names and no
/// arm answers gets the position list — the words that open a statement, at
/// the one place the grammar takes none of them. The table is checked against
/// the declaration in both directions: a character added to
/// `crate::capabilities` without an arm fails here, and so does one dropped
/// from it while this table still claims it.
#[test]
fn every_trigger_character_reaches_an_arm_that_is_not_the_position_list() {
    let declared = nvs_lsp::declared_capabilities(PositionEncodingKind::UTF8);
    let characters: Vec<String> = declared["completionProvider"]["triggerCharacters"]
        .as_array()
        .expect("`initialize` declares the completion trigger characters")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("a trigger character is a string")
                .to_owned()
        })
        .collect();
    let named: Vec<String> = TRIGGERED
        .iter()
        .map(|(character, _, _)| (*character).to_owned())
        .collect();
    assert_eq!(
        characters, named,
        "the declared trigger characters and the constructs they are part of \
         are not the same list"
    );

    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("nvs-trigger-case.nvs");
    for (character, construct, tail) in TRIGGERED {
        let source = format!("{TRIGGER_DOC}{construct}{tail}\n");
        let at = after(&source, construct);
        let (documents, analysis) = analysed_at(&here, &source);
        assert!(
            completion::continues_a_trigger(&analysis, at),
            "`{character}` ends `{construct}` and the request it raises is not answered"
        );
        let offered = rendered_with(&documents, &analysis, at);
        assert!(
            !offered.is_empty(),
            "`{character}` is a trigger character and `{construct}` is offered nothing"
        );
        // An open tag is rendered as a keyword and is no word of the position
        // list, which is what this refuses.
        let words: Vec<&str> = offered
            .lines()
            .filter(|line| !line.starts_with("<?"))
            .filter(|line| line.split_whitespace().nth(1) == Some("keyword"))
            .collect();
        assert!(
            words.is_empty(),
            "`{character}` is a trigger character and `{construct}` is answered \
             the position list: {offered}"
        );
    }
}

/// A path character outside a path literal raises no list: `/` divides, and a
/// quote opens every other string.
#[test]
fn a_path_character_outside_a_path_literal_is_not_answered() {
    for (construct, tail) in [
        ("var $half = 4 /", " 2;"),
        ("echo '", "';"),
        ("echo \"", "\";"),
        ("echo 'lib/", "';"),
    ] {
        let source = format!("{TRIGGER_DOC}{construct}{tail}\n");
        let (_documents, analysis) = analysed(&source);
        assert!(
            !completion::continues_a_trigger(&analysis, after(&source, construct)),
            "`{construct}` is no path literal and its last character raised a list"
        );
    }
}

/// One function of `nvs_lsp::completion`, with the signature that says whether
/// it produces an offered item and the body that says where the value in it
/// came from.
struct Source {
    name: String,
    signature: String,
    body: String,
}

/// The module's own source, function by function.
///
/// A `fn` at column zero only, so the unit tests inside `mod tests` are not
/// read as sources — they are indented, and the ones that matter are not.
fn sources() -> Vec<Source> {
    let text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("completion.rs"),
    )
    .expect("the module's own source");
    let mut found: Vec<Source> = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line
            .strip_prefix("pub fn ")
            .or_else(|| line.strip_prefix("fn "))
        {
            found.push(Source {
                name: rest.split(['(', '<']).next().unwrap_or_default().to_owned(),
                signature: String::new(),
                body: String::new(),
            });
        }
        if let Some(source) = found.last_mut() {
            if !source.signature.trim_end().ends_with('{') {
                source.signature.push_str(line);
            }
            source.body.push_str(line);
            source.body.push('\n');
        }
    }
    assert!(found.len() > 10, "the module's functions were not found");
    found
}

/// Every function that produces an offered item, and the needle in its own
/// body that says where the value came from.
///
/// `rule:ide/completion-offers-only-what-the-compiler-derived` is held by a
/// test rather than by review, and this is the test. Every needle is a table
/// the compiler builds for another reason: `nvs_stdlib::registry`, which is
/// what a `Core\Str::` call resolves against; `declared_type`, which is the
/// tree the front end already parsed; the workspace index, which is
/// `textDocument/codeLens`' own query; the body scopes `nvs_types` recorded;
/// and the reserved words, which are the grammar's own dispatch and are held
/// to it by the module's `every_word_offered_is_one_the_lexer_reserves`. The
/// rows that are dispatches, and the one that is the constructor, name what
/// they route to — which is what stops an arm being added with no row here.
/// The rows that shape an item another row produced name the item they were
/// handed, and the one that filters a list names the index it asks.
///
/// The PHP-name arm is the one row whose table is not built from the program
/// under the cursor, and it is admitted by the same rule rather than beside it:
/// `nvs_stdlib::php_names::CANDIDATES` is a build-time join of the differential
/// oracle's own inventory and `docs/spec/02-php-migration.md`, both audited in
/// this repository, and what the arm may *insert* is bounded by the registry
/// (`rule:php-migration/an-item-inserts-only-a-registered-member`). No
/// directory is walked and no annotation is read for it.
///
/// The path arm reads one directory, the one a `require` or `autoload` path
/// literal's text reaches, through `nvs_hir::autoload::entries_of`: the
/// listing the compiler resolves a `discover` glob with. It walks no tree and
/// looks for no convention, so what it offers is what resolution would find.
const SOURCED: [(&str, &str); 30] = [
    ("named_type", "..item("),
    ("type_row", "..named_type("),
    ("method_row", "..item("),
    ("typed_row", "..item("),
    ("valued_row", "..item("),
    ("at", "asked("),
    ("members_of", "registry::class("),
    ("type_members_of", "registry::class("),
    ("declared_type_members", "declared_type("),
    ("open_tags", "OPEN_TAGS"),
    ("paths", "autoload::entries_of("),
    ("position", "words("),
    ("statement_words", "STATEMENT_WORDS"),
    ("followed", "Classes::of(cursor.symbols)"),
    ("scoped", "..offered"),
    ("called", "..offered"),
    ("under", "registry::CLASSES"),
    ("in_reach", "every_type(symbols)"),
    ("in_scope", ".bodies_at("),
    ("php_builtins", "php_names::starting_with("),
    ("php_item", "php_names::Item"),
    ("words", "CompletionItemKind::KEYWORD"),
    ("declared_members", "declared_type("),
    ("declared_member", "Modifier::Static"),
    ("enum_case", "case.value"),
    ("core_members", "&CoreClass"),
    ("core_member", "&CoreMethod"),
    ("core_constant", "&CoreConst"),
    ("core_cases", "&CoreEnum"),
    ("item", "CompletionItem {"),
];

/// Every source of an offered value is enumerated, and every one of them names
/// the table it reads.
///
/// The two sets have to match in both directions. A row naming a function the
/// module no longer has is a source that was renamed out from under this test;
/// a function producing an item that no row names is an arm that arrived
/// without one, which is exactly the drift
/// `rule:ide/completion-offers-only-what-the-compiler-derived` closes by being
/// a closed rule rather than a starting point.
#[test]
fn every_completion_source_names_a_compiler_table() {
    let found = sources();
    let producers: Vec<&Source> = found
        .iter()
        .filter(|source| {
            source.signature.contains("CompletionItem>")
                || source.signature.contains("-> CompletionItem {")
        })
        .collect();
    let produced: Vec<&str> = producers
        .iter()
        .map(|source| source.name.as_str())
        .collect();
    let mut named: Vec<&str> = SOURCED.iter().map(|(name, _)| *name).collect();
    named.sort_unstable();
    let mut offered = produced.clone();
    offered.sort_unstable();
    assert_eq!(
        offered, named,
        "the module's item-producing functions and the enumerated sources are \
         not the same set, so a source was renamed or an arm arrived without one"
    );

    for (name, table) in SOURCED {
        let source = producers
            .iter()
            .find(|source| source.name == name)
            .expect("the set above is equal");
        assert!(
            source.body.contains(table),
            "`{name}` is enumerated as reading `{table}` and does not name it"
        );
    }
}

/// What a completion source may not reach, and what reaching it would mean.
///
/// Not a survey of every way to read a file or open a socket — it is the short
/// list of the things a language server in this niche is expected to do and
/// this one refuses: scan a directory layout for a convention, read a second
/// description of the program's shape, or ask a remote index.
const REFUSED: [(&str, &str); 8] = [
    (
        "read_dir",
        "a directory walk is the convention scan this rule refuses",
    ),
    (
        "std::fs",
        "a file this analysis did not read is a second source of truth",
    ),
    (
        "fs::read",
        "a file this analysis did not read is a second source of truth",
    ),
    (
        "WalkDir",
        "a directory walk is the convention scan this rule refuses",
    ),
    ("TcpStream", "the language server makes no network request"),
    ("reqwest", "the language server makes no network request"),
    ("ureq", "the language server makes no network request"),
    (
        "annotation",
        "an annotation dialect is a second description of the program",
    ),
];

/// No source reads a directory layout, a second description of the program, or
/// anything off this machine.
///
/// The other half of the rule, and the half a table of sources cannot state:
/// enumerating what each arm reads says nothing about what the module reads
/// beside them. Comments are not code — the module doc says what this refuses,
/// and saying it is not doing it.
#[test]
fn no_completion_source_reads_a_directory_layout_or_the_network() {
    for source in sources() {
        for line in source.body.lines().map(str::trim_start) {
            if line.starts_with("//") {
                continue;
            }
            for (needle, why) in REFUSED {
                assert!(
                    !line.contains(needle),
                    "`{}` names `{needle}`: {why}",
                    source.name
                );
            }
        }
    }
}
