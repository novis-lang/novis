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
use nvs_lsp::{
    Analysed, CheckScope, Documents, Response, SymbolIndex, analyse, completion, uri_of,
};

/// The class every document below resolves its receiver to.
const CLASS: &str =
    r#"class User { public string $name; public function greet(): string { return "hi"; } }"#;

/// The receiver whose member half is empty in every document.
const RECEIVER: &str = "$b->";

/// `source` analysed as its own entry point, out of an open buffer alone —
/// nothing here `require`s anything, so no directory is needed
/// (`nvs_diagnostics::SourceMap::load`).
fn analysed(source: &str) -> (Documents, Analysed) {
    let uri = uri_of(&std::env::temp_dir().join("nvs-completion-case.nvs"))
        .expect("a temp path is UTF-8");
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
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);
    Response::Completion(completion::at(&analysis, &index, at)).render()
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
const MEMBERS: &str = "greet   method    (): string\nname    property  string\n";

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
        "Str     class     Core\\Str\n",
        "User    class     App\\User\n",
        "Greets  interface App\\Greets\n",
        "return  keyword\n",
    ] {
        assert!(
            offered.contains(row),
            "a bare name is not offered `{}`: {offered}",
            row.trim_end()
        );
    }
}

/// Each trigger character `initialize` declares, and the construct it is one
/// character of.
///
/// `-` and `>` share a construct because they are the two characters of one
/// arrow: a client fires on each keystroke, and what the pair promises is that
/// the arrow it is halfway through reaches a member list.
const TRIGGERED: [(&str, &str); 4] = [
    ("-", "$b->"),
    (">", "$b->"),
    (":", "User::"),
    ("\\", "Core\\"),
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
        .map(|(character, _)| (*character).to_owned())
        .collect();
    assert_eq!(
        characters, named,
        "the declared trigger characters and the constructs they are part of \
         are not the same list"
    );

    for (character, construct) in TRIGGERED {
        let source = format!("{TRIGGER_DOC}{construct}\n");
        let offered = rendered(&source, after(&source, construct));
        assert!(
            !offered.is_empty(),
            "`{character}` is a trigger character and `{construct}` is offered nothing"
        );
        let words: Vec<&str> = kinds(&offered)
            .into_iter()
            .filter(|kind| *kind == "keyword")
            .collect();
        assert!(
            words.is_empty(),
            "`{character}` is a trigger character and `{construct}` is answered \
             the position list: {offered}"
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
/// to it by the module's `every_word_offered_is_one_the_lexer_reserves`. Three
/// rows are dispatches and one is the constructor, and they name what they
/// route to — which is what stops an arm being added with no row here.
const SOURCED: [(&str, &str); 15] = [
    ("at", "asked("),
    ("members_of", "registry::class("),
    ("position", "words("),
    ("under", "registry::CLASSES"),
    ("in_reach", "symbols.declarations_in("),
    ("in_scope", ".bodies_at("),
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
