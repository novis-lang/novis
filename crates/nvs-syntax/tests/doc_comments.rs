//! Attachment: which declaration a `///` run documents, and what happens to one
//! that documents nothing.
//!
//! `rule:tooling/doc-comment-attaches-to-the-next-declaration` is the whole
//! subject. The lexer's half — which comment is a doc comment at all — is
//! `trivia.rs`, and these cases take it as settled.

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap, Span, code};
use nvs_syntax::ast::{ClassMemberKind, DocComment, DocTagKind, Stmt, StmtKind};
use nvs_syntax::parse_file;

/// Parses `src` as a whole file, the way every compile path does — `parse_file`
/// and not `parse`, because attachment is the grammar's own and must not need
/// the trivia layer to be switched on.
fn parse(src: &str) -> (SourceMap, Vec<Stmt>, Diagnostics) {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src);
    let mut diags = Diagnostics::new();
    let stmts = parse_file(map.file(id), &mut diags);
    (map, stmts, diags)
}

fn text(map: &SourceMap, span: Span) -> &str {
    map.file(span.file)
        .span_text(span)
        .expect("a span from this parse is inside the file it came from")
}

/// A doc comment's lines as text, marker included, in source order.
fn lines(map: &SourceMap, doc: &DocComment) -> Vec<String> {
    doc.lines
        .iter()
        .map(|span| text(map, *span).to_string())
        .collect()
}

/// Every span this parse reported [`code::E_DOC_COMMENT_UNATTACHED`] at.
fn unattached(diags: &Diagnostics) -> Vec<Span> {
    diags
        .iter()
        .filter(|d| d.code == Some(code::E_DOC_COMMENT_UNATTACHED))
        .filter_map(Diagnostic::primary_span)
        .collect()
}

/// The declaration under the run is the one documented, consecutive `///` lines
/// are one comment however many there are, and the run is still the
/// declaration's across the attribute group written between them.
#[test]
fn a_doc_comment_run_attaches_to_the_declaration_that_follows() {
    let src = concat!(
        "<?nvs\n",
        "/// The order a sort was asked for.\n",
        "/// One line is prose; two are still one comment.\n",
        "enum Order: int {\n",
        "/// Smallest first.\n",
        "Asc = 1,\n",
        "Desc = 2,\n",
        "}\n",
        "/// What a page is.\n",
        "#[Route]\n",
        "class Page {\n",
        "/// Both properties, because both are this declaration.\n",
        "public int $width, $height;\n",
        "}\n",
    );
    let (map, stmts, diags) = parse(src);
    assert!(!diags.has_errors(), "unexpected diagnostics: {diags:?}");

    let StmtKind::EnumDecl(order) = &stmts[0].kind else {
        panic!("expected an enum, got {:?}", stmts[0].kind);
    };
    let doc = order.doc.as_ref().expect("the enum's run attached");
    assert_eq!(
        lines(&map, doc),
        [
            "/// The order a sort was asked for.",
            "/// One line is prose; two are still one comment.",
        ]
    );
    assert_eq!(
        text(&map, doc.span),
        "/// The order a sort was asked for.\n/// One line is prose; two are still one comment.",
        "the run's own span covers both its lines and nothing else"
    );
    assert_eq!(
        lines(&map, order.cases[0].doc.as_ref().expect("the case's run")),
        ["/// Smallest first."],
        "a case is a declaration and takes its own run"
    );
    assert!(
        order.cases[1].doc.is_none(),
        "a case with nothing above it takes nothing"
    );

    let StmtKind::ClassDecl(page) = &stmts[1].kind else {
        panic!("expected a class, got {:?}", stmts[1].kind);
    };
    assert_eq!(
        lines(&map, page.doc.as_ref().expect("the class's run attached")),
        ["/// What a page is."],
        "the run is written above the attribute group, and is still the class's"
    );
    assert_eq!(page.members.len(), 2, "`$width, $height` is two members");
    for member in &page.members {
        assert!(
            matches!(member.kind, ClassMemberKind::Property(_)),
            "expected properties, got {:?}",
            member.kind
        );
        assert_eq!(
            lines(&map, member.doc.as_ref().expect("the property's run")),
            ["/// Both properties, because both are this declaration."],
            "one declaration, so one comment over every member it made"
        );
    }
}

/// A blank line between the run and the declaration detaches it — and then the
/// run documents nothing, which is the same refusal as an orphan anywhere else.
#[test]
fn a_blank_line_breaks_attachment() {
    let (map, stmts, diags) = parse(concat!(
        "<?nvs\n",
        "/// Detached.\n",
        "/// Still detached.\n",
        "\n",
        "/// Attached, because the blank line is above this one.\n",
        "class Page {}\n",
    ));
    let StmtKind::ClassDecl(page) = &stmts[0].kind else {
        panic!("expected a class, got {:?}", stmts[0].kind);
    };
    assert_eq!(
        lines(&map, page.doc.as_ref().expect("the nearer run attached")),
        ["/// Attached, because the blank line is above this one."],
        "a blank line ends a run; the one below it is a whole comment of its own"
    );
    let refused = unattached(&diags);
    assert_eq!(refused.len(), 1, "one run above the blank line: {diags:?}");
    assert_eq!(
        text(&map, refused[0]),
        "/// Detached.\n/// Still detached.",
        "the two lines above the blank line are one run, reported once"
    );
}

/// Parses `doc` — the `///` lines, with no trailing newline — above a class,
/// and returns the run that attached to it. Every tag case is written this way:
/// what a tag line does is the subject, and the declaration under it is only
/// there so the run has something to attach to.
fn doc_above_a_class(doc: &str) -> (SourceMap, DocComment, Diagnostics) {
    let (map, stmts, diags) = parse(&format!("<?nvs\n{doc}\nclass Page {{}}\n"));
    let StmtKind::ClassDecl(page) = &stmts[0].kind else {
        panic!("expected a class, got {:?}", stmts[0].kind);
    };
    let doc = page.doc.clone().expect("the run attached to the class");
    (map, doc, diags)
}

/// The one [`code::E_DOC_COMMENT_UNKNOWN_TAG`] this parse reported: its
/// headline, and its help line with the `help: ` prefix off.
fn only_refused_tag(diags: &Diagnostics) -> (String, String) {
    let mut refused = diags
        .iter()
        .filter(|d| d.code == Some(code::E_DOC_COMMENT_UNKNOWN_TAG));
    let one = refused.next().expect("a tag was refused");
    assert!(refused.next().is_none(), "one refusal per tag: {diags:?}");
    let help = one
        .notes
        .iter()
        .find_map(|note| note.strip_prefix("help: "))
        .expect("a refused tag says what to write instead");
    (one.message.clone(), help.to_string())
}

/// The two tags parse, wherever the indentation puts them, and the prose around
/// them is left alone.
#[test]
fn see_and_example_parse_as_tags() {
    let (map, doc, diags) = doc_above_a_class(concat!(
        "/// The price in cents, never a float.\n",
        "///\n",
        "/// @see Core\\Money::fromCents\n",
        "///   @example examples/charge.nvs  "
    ));
    assert!(!diags.has_errors(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        doc.lines.len(),
        4,
        "prose and tags are all lines of the run"
    );
    let tags: Vec<_> = doc
        .tags
        .iter()
        .map(|tag| (tag.kind, text(&map, tag.argument)))
        .collect();
    assert_eq!(
        tags,
        [
            (DocTagKind::See, "Core\\Money::fromCents"),
            (DocTagKind::Example, "examples/charge.nvs"),
        ],
        "an argument is what the tag names, with the whitespace off"
    );
    assert_eq!(
        text(&map, doc.tags[0].span),
        "@see Core\\Money::fromCents",
        "a tag's own span runs from the `@` to the end of its line"
    );
}

/// `@param` is refused, and the help says the parameter goes in a sentence
/// because its type is already in the signature.
#[test]
fn param_is_refused_naming_the_signature() {
    let (_, doc, diags) = doc_above_a_class("/// Charges a card.\n/// @param $cents the amount");
    assert!(doc.tags.is_empty(), "a refused tag is not kept as one");
    let (message, help) = only_refused_tag(&diags);
    assert_eq!(message, "`@param` is not a documentation tag");
    assert!(help.contains("signature"), "help was {help:?}");
}

/// `@returns` — and `@return`, the other spelling — is refused for the same
/// reason: the type is in the signature.
#[test]
fn returns_is_refused_naming_the_signature() {
    for tag in ["returns", "return"] {
        let (_, _, diags) =
            doc_above_a_class(&format!("/// Charges a card.\n/// @{tag} the price"));
        let (message, help) = only_refused_tag(&diags);
        assert_eq!(message, format!("`@{tag}` is not a documentation tag"));
        assert!(help.contains("signature"), "help was {help:?}");
    }
}

/// `@throws` is refused naming the prose — for user code a sentence, and for a
/// `Core` member the registry card that already carries it.
#[test]
fn throws_is_refused_naming_the_prose() {
    let (_, _, diags) = doc_above_a_class("/// Charges a card.\n/// @throws Core\\Error\\Invalid");
    let (message, help) = only_refused_tag(&diags);
    assert_eq!(message, "`@throws` is not a documentation tag");
    assert!(help.contains("sentence"), "help was {help:?}");
}

/// A tag nobody has ever written is refused exactly like the ones PHPDoc
/// taught, and the help is the rule: the set is two.
#[test]
fn an_invented_tag_is_refused() {
    let (map, doc, diags) = doc_above_a_class("/// A page.\n/// @audience internal");
    assert!(doc.tags.is_empty(), "a refused tag is not kept as one");
    let (message, help) = only_refused_tag(&diags);
    assert_eq!(message, "`@audience` is not a documentation tag");
    assert!(help.contains("`@see` and `@example`"), "help was {help:?}");
    assert_eq!(
        text(&map, doc.lines[1]),
        "/// @audience internal",
        "the line is still part of the run it was written in"
    );
}

/// An `@` anywhere but the start of a line is a character, not a tag — which is
/// what lets a doc comment hold an address, a handle or an annotation quoted
/// from another language.
#[test]
fn an_at_sign_inside_prose_is_not_a_tag() {
    let (_, doc, diags) = doc_above_a_class(concat!(
        "/// Mail team@novis.example about a page, and see @see below.\n",
        "/// PHP wrote `@param` on this line, mid-sentence, and it is prose.\n",
        "/// @see Core\\Str"
    ));
    assert!(!diags.has_errors(), "unexpected diagnostics: {diags:?}");
    assert_eq!(
        doc.tags.len(),
        1,
        "only the line that starts with one is a tag"
    );
    assert_eq!(doc.tags[0].kind, DocTagKind::See);
}

/// The three shapes that document nothing: a run at end of file, a run above a
/// statement that declares nothing, and a run held off its declaration by an
/// ordinary comment. Each is one diagnostic for the whole run.
#[test]
fn a_doc_comment_attached_to_nothing_is_refused() {
    for (src, refused) in [
        ("<?nvs\n/// Nothing follows.\n", "/// Nothing follows."),
        (
            "<?nvs\n/// A note, marked wrong.\necho \"hello\";\n",
            "/// A note, marked wrong.",
        ),
        (
            "<?nvs\n/// Documentation.\n// A note between.\nclass Page {}\n",
            "/// Documentation.",
        ),
    ] {
        let (map, _, diags) = parse(src);
        let spans = unattached(&diags);
        assert_eq!(
            spans.len(),
            1,
            "expected one refusal for {src:?}: {diags:?}"
        );
        assert_eq!(text(&map, spans[0]), refused);
    }
}

/// Every span this parse warned [`code::W_DOC_BLOCK_BEFORE_A_DECLARATION`] at.
fn docblocks(diags: &Diagnostics) -> Vec<Span> {
    diags
        .iter()
        .filter(|d| d.code == Some(code::W_DOC_BLOCK_BEFORE_A_DECLARATION))
        .filter_map(Diagnostic::primary_span)
        .collect()
}

/// The PHPDoc habit: a `/** … */` block directly above a class, a constant, a
/// property or a method is an ordinary comment that documents
/// nothing, and `rule:tooling/doc-comment-is-three-slashes` says so with a
/// warning at the block — once per block, on the compile path, which records
/// no other ordinary comment. The program is still accepted.
#[test]
fn a_php_docblock_directly_above_a_declaration_warns() {
    let src = "<?nvs\n\
        /** A price in cents. */\n\
        final class Price {\n\
        \x20   /** The lowest price. */\n\
        \x20   public const int MIN = 0;\n\
        \x20   /**\n\
        \x20    * Doubles a price.\n\
        \x20    */\n\
        \x20   public static function double(int $p): int { return $p * 2; }\n\
        \x20   /** The name. */\n\
        \x20   public string $name = \"\";\n\
        }\n";
    let (map, _, diags) = parse(src);
    assert!(!diags.has_errors(), "{diags:?}");
    let warned: Vec<&str> = docblocks(&diags)
        .into_iter()
        .map(|span| text(&map, span))
        .collect();
    assert_eq!(
        warned,
        [
            "/** A price in cents. */",
            "/** The lowest price. */",
            "/**\n     * Doubles a price.\n     */",
            "/** The name. */",
        ]
    );
    let notes = diags
        .iter()
        .find(|d| d.code == Some(code::W_DOC_BLOCK_BEFORE_A_DECLARATION))
        .map(|d| d.notes.join("\n"))
        .unwrap_or_default();
    assert!(
        notes.contains("write `///`"),
        "the help names the fix: {notes}"
    );
}

/// The warning is about the shape above a declaration and nothing else: a
/// block a blank line away, a `/* … */` block, an empty `/**/`, a block above
/// a statement, and a block above a `///` run that is itself separated from
/// the declaration are all silent.
#[test]
fn a_docblock_that_is_not_directly_above_a_declaration_is_silent() {
    for src in [
        "<?nvs\n/** Separated. */\n\nclass Page {}\n",
        "<?nvs\n/* Plain. */\nclass Page {}\n",
        "<?nvs\n/**/\nclass Page {}\n",
        "<?nvs\n/** Above a statement. */\necho \"hi\";\n",
        "<?nvs\n/** Above a note. */\n// a note\nclass Page {}\n",
    ] {
        let (_, _, diags) = parse(src);
        assert!(
            docblocks(&diags).is_empty(),
            "no warning for {src:?}: {diags:?}"
        );
    }
}

/// A block directly above a `///` run is directly above the declaration the
/// run documents, so the habit is named even where the author has already
/// half-switched — and the run still attaches.
#[test]
fn a_docblock_above_an_attached_run_warns_and_the_run_still_attaches() {
    let src = "<?nvs\n/** Old. */\n/// New.\nclass Page {}\n";
    let (map, _, diags) = parse(src);
    assert!(unattached(&diags).is_empty(), "{diags:?}");
    let warned: Vec<&str> = docblocks(&diags)
        .into_iter()
        .map(|span| text(&map, span))
        .collect();
    assert_eq!(warned, ["/** Old. */"]);
}
