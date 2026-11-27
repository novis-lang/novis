//! Which fixes `textDocument/codeAction` offers, and which diagnostic each one
//! came from.
//!
//! Here rather than in a unit test for `tests/redactions.rs`' reason: the
//! answer is what a whole analysis reported, so a fixture would be asserting
//! that a `Suggestion` this file wrote survives being copied. Every expectation
//! below is `nvs_lsp::render`'s spelling
//! (`rule:ide/the-rendering-has-one-home`), so a `.lspt` case under
//! `tests/lsp/actions/` and a test here freeze the same text.
//!
//! The boundary these tests pin is
//! `rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`: two
//! fixes ship because their replacement text is already sitting on a
//! diagnostic, and a fix the checker would have to compute is offered by
//! nothing at all.

use nvs_diagnostics::PositionEncoding;
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::{Analysed, Documents, Response, actions, analyse, uri_of};

/// `source` analysed as its own entry point, out of an open buffer alone —
/// nothing here `require`s anything, so no directory is needed
/// (`nvs_diagnostics::SourceMap::load`).
fn analysed(source: &str) -> Analysed {
    let dir = nvs_repo::scratch("lsp-actions");
    let uri = uri_of(&dir.join("case.nvs")).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    analyse(&documents, &uri).expect("an open document analyses")
}

/// Where `needle` starts in `source`, as the cursor a client would send.
fn cursor(source: &str, needle: &str) -> u32 {
    let at = source
        .find(needle)
        .expect("the cursor's anchor is in the document");
    u32::try_from(at).expect("a test document is short")
}

/// What would be offered at `needle`, rendered as a case freezes it.
///
/// UTF-8 columns, which is what a `.lspt` case is read in: the encoding
/// negotiation is `rule:ide/positions-have-one-home`'s business and has nothing
/// to say about which fixes are offered.
fn offered(source: &str, needle: &str) -> String {
    let at = cursor(source, needle);
    let analysed = analysed(source);
    Response::CodeAction(actions::at(
        &analysed,
        &Documents::new(),
        None,
        &CompletionFiles::default(),
        at,
        at,
        actions::Kind::QuickFix,
        PositionEncoding::Utf8,
    ))
    .render()
}

/// Every suggestion the analysis of `source` reported, as `(title, text)`.
fn suggested(source: &str) -> Vec<(String, String)> {
    analysed(source)
        .diags
        .iter()
        .flat_map(|diagnostic| diagnostic.suggestions.iter())
        .map(|suggestion| (suggestion.message.clone(), suggestion.replacement.clone()))
        .collect()
}

/// The provenance claim itself: an action's title and its replacement are the
/// `Suggestion`'s own, not text this crate composed. Nothing here computes a
/// fix, which is why the boundary is enforceable at all — a fix with no
/// suggestion behind it has nothing to be translated from.
#[test]
fn a_code_action_comes_from_a_suggestion_the_diagnostic_already_carried() {
    let source = "<?nvs\nclass user_account {}\n";
    let carried = suggested(source);
    assert_eq!(
        carried,
        vec![(
            "rename to `UserAccount`".to_owned(),
            "UserAccount".to_owned()
        )]
    );

    let at = cursor(source, "user_account");
    let offered: Vec<(String, String)> = actions::at(
        &analysed(source),
        &Documents::new(),
        None,
        &CompletionFiles::default(),
        at,
        at,
        actions::Kind::QuickFix,
        PositionEncoding::Utf8,
    )
    .into_iter()
    .map(|action| (action.title, action.replacement))
    .collect();
    assert_eq!(offered, carried);
}

/// The first of the two that ship (`rule:core-api/identifier-casing`), frozen
/// whole — the range it edits is the name alone, so applying it renames and
/// touches nothing else.
///
/// A casing error is `E0110`, the parser's band, so it is published whatever
/// else the file has wrong with it and the fix is reachable on the file that
/// needs it (`rule:ide/diagnostics-are-phase-gated`).
#[test]
fn the_casing_diagnostic_carries_its_replacement() {
    assert_eq!(
        offered("<?nvs\nclass user_account {}\n", "user_account"),
        "2:7-2:19 quickfix rename to `UserAccount` -> \"UserAccount\"\n"
    );
}

/// The second (`rule:types/no-legacy-cast`): the whole `(int)$n` is replaced by
/// the one conversion spelling Novis has. The edit covers the cast as well as
/// its operand, because a rewrite that dropped `(int)` and left the operand
/// alone would silently change what the expression means.
#[test]
fn the_legacy_cast_diagnostic_carries_its_replacement() {
    assert_eq!(
        offered("<?nvs\nint $n = 3;\nint $m = (int)$n;\n", "(int)"),
        "3:10-3:17 quickfix rewrite as `$n as int` -> \"$n as int\"\n"
    );
}

/// `W1022` carries two groupings (`rule:expressions/misread-grouping-warns`).
/// A light bulb offers both, and a fix-all applies only the one that keeps
/// what the line does, because the other is an alternative a person chooses.
#[test]
fn a_fix_all_applies_the_grouping_that_keeps_the_meaning_and_never_the_alternative() {
    let source = "<?nvs\n?int $n = 5;\necho $n ?? 0 + 10;\n";
    let at = cursor(source, "0 + 10");
    let titles = |kind| -> Vec<String> {
        actions::at(
            &analysed(source),
            &Documents::new(),
            None,
            &CompletionFiles::default(),
            at,
            at,
            kind,
            PositionEncoding::Utf8,
        )
        .into_iter()
        .map(|action| action.title)
        .collect()
    };
    assert_eq!(
        titles(actions::Kind::QuickFix),
        [
            "apply `??` first: `($n ?? 0) + 10`",
            "keep the current meaning: `$n ?? (0 + 10)`",
            "Put on separate lines"
        ]
    );
    assert_eq!(
        titles(actions::Kind::FixAll),
        ["keep the current meaning: `$n ?? (0 + 10)`"]
    );
}

/// The one action with no diagnostic behind it
/// (`rule:ide/a-string-converts-to-an-html-template`): it changes what the line
/// prints, so it is offered beside the quick fixes under its own kind and never
/// in a fix-all, which a save would run.
#[test]
fn the_html_template_refactor_has_its_own_kind_and_is_never_in_a_fix_all() {
    let source = "<?nvs\nvar $name = \"world\";\necho \"<b>\" . $name . \"</b>\";\n";
    let at = cursor(source, "$name . ");
    let offered = |kind| -> Vec<(String, String)> {
        actions::at(
            &analysed(source),
            &Documents::new(),
            None,
            &CompletionFiles::default(),
            at,
            at,
            kind,
            PositionEncoding::Utf8,
        )
        .into_iter()
        .map(|action| (action.kind, action.replacement))
        .collect()
    };
    assert_eq!(
        offered(actions::Kind::QuickFix),
        [
            // *Inline variable* on the read of `$name`, whose edit replaces
            // the declaration's line with nothing first.
            (nvs_lsp::inline::KIND.to_owned(), String::new()),
            (
                nvs_lsp::html_template::KIND.to_owned(),
                "html`<b>{$name}</b>`".to_owned()
            ),
            (
                nvs_lsp::split_join::KIND.to_owned(),
                "\n    . $name\n    . ".to_owned()
            )
        ]
    );
    assert_eq!(offered(actions::Kind::FixAll), []);
}

/// The far side of the boundary: a type error is published, the cursor is on
/// it, and it names no replacement because choosing one would be the checker
/// computing a fix. The string under the cursor is still offered the
/// html-template refactor, which is no fix and is left out of what this test
/// counts.
#[test]
fn a_fix_the_checker_would_have_to_compute_is_offered_by_nothing() {
    let mistyped = "<?nvs\nint $n = \"text\";\n";
    assert!(
        !nvs_lsp::for_document(
            &analysed(mistyped),
            &CompletionFiles::default(),
            nvs_lsp::Phases::Gated,
            PositionEncoding::Utf8
        )
        .is_empty(),
        "the document has a published diagnostic for the cursor to be on"
    );
    let at = cursor(mistyped, "\"text\"");
    let fixes: Vec<String> = actions::at(
        &analysed(mistyped),
        &Documents::new(),
        None,
        &CompletionFiles::default(),
        at,
        at,
        actions::Kind::QuickFix,
        PositionEncoding::Utf8,
    )
    .into_iter()
    .filter(|action| action.kind != nvs_lsp::html_template::KIND)
    .map(|action| action.title)
    .collect();
    assert_eq!(fixes, Vec::<String>::new());
}

/// The narrowing action's replacement at the first `array<mixed>` of
/// `source`, asked under `kind`, or `None` where it is not offered.
fn narrowed(source: &str, kind: actions::Kind) -> Option<String> {
    let at = cursor(source, "array<mixed>") + 1;
    actions::at(
        &analysed(source),
        &Documents::new(),
        None,
        &CompletionFiles::default(),
        at,
        at,
        kind,
        PositionEncoding::Utf8,
    )
    .into_iter()
    .find(|action| action.title.starts_with("Narrow the type"))
    .map(|action| action.replacement)
}

/// Each diagnostic of `source`, as its code and its message.
fn reported(source: &str) -> Vec<String> {
    analysed(source)
        .diags
        .iter()
        .map(|diagnostic| format!("{:?} {}", diagnostic.code, diagnostic.message))
        .collect()
}

/// `rule:ide/narrow-an-annotation-to-its-literal`'s round trip: the edit
/// applied gives a document with the diagnostics it had before, and the
/// action is not offered on the annotation it wrote. It is never part of a
/// fix-all (`rule:ide/narrowing-is-a-diff-never-a-save-time-fix`).
#[test]
fn a_narrowed_annotation_checks_as_before_and_is_not_offered_again() {
    let sources = [
        (
            "<?nvs\narray<mixed> $grid = [[[1.5]], [[2.5, 3.5]]];\necho $grid[0][0][0];\n",
            Some("array<array<array<float>>>"),
        ),
        (
            "<?nvs\narray<mixed> $limits = ['retry' => 1, 'depth' => 3];\n",
            Some("array<int>"),
        ),
        (
            "<?nvs\nenum Rank { case Ace; case King; }\nclass Hand {\n    public function cards(): void {\n        array<mixed> $cards = [Rank::Ace, Rank::King];\n    }\n}\n",
            Some("array<Rank>"),
        ),
        ("<?nvs\narray<mixed> $row = [1, \"a\", null];\n", None),
    ];
    for (source, expected) in sources {
        let replacement = narrowed(source, actions::Kind::QuickFix)
            .unwrap_or_else(|| panic!("narrowing is offered in {source}"));
        if let Some(expected) = expected {
            assert_eq!(replacement, expected, "{source}");
        }
        assert_eq!(narrowed(source, actions::Kind::FixAll), None, "{source}");
        let applied = source.replacen("array<mixed>", &replacement, 1);
        assert_eq!(reported(&applied), reported(source), "{applied}");
        assert!(
            !applied.contains("array<mixed>"),
            "the edit replaced the one annotation: {applied}"
        );
        let at = cursor(&applied, &replacement) + 1;
        let again: Vec<String> = actions::at(
            &analysed(&applied),
            &Documents::new(),
            None,
            &CompletionFiles::default(),
            at,
            at,
            actions::Kind::QuickFix,
            PositionEncoding::Utf8,
        )
        .into_iter()
        .map(|action| action.title)
        .filter(|title| title.starts_with("Narrow the type"))
        .collect();
        assert_eq!(again, Vec::<String>::new(), "{applied}");
    }
}

/// `rule:ide/the-action-answers-from-the-literal-or-not-at-all`: a value from
/// outside, an empty literal and a `mixed` element get nothing, and so does a
/// literal whose narrowed type a later write would not fit.
#[test]
fn narrowing_is_not_offered_without_a_literal_that_states_the_type() {
    for source in [
        "<?nvs\narray<mixed> $body = Core\\Json::decode(\"[1]\") as array<mixed>;\n",
        "<?nvs\narray<mixed> $none = [];\n",
        "<?nvs\narray<mixed> $nested = [[1], []];\n",
        "<?nvs\nmixed $any = 1;\narray<mixed> $rows = [$any, 2];\n",
        "<?nvs\narray<mixed> $rows = [1, 2];\n$rows[] = \"x\";\n",
    ] {
        assert_eq!(narrowed(source, actions::Kind::QuickFix), None, "{source}");
    }
}

/// `source` with the split or join offered at `needle` applied, and which of
/// the two it was.
fn relaid(source: &str, needle: &str) -> (nvs_fmt::Toggle, String) {
    let mut map = nvs_diagnostics::SourceMap::new();
    let id = map.add("case.nvs", source);
    let edit = nvs_fmt::relayout(map.file(id), cursor(source, needle) as usize)
        .unwrap_or_else(|| panic!("a split or join is offered at {needle:?} in:\n{source}"));
    let applied = format!(
        "{}{}{}",
        &source[..edit.start],
        edit.replacement,
        &source[edit.end..]
    );
    (edit.toggle, applied)
}

/// What `nvs fmt` writes for `source`.
fn formatted(source: &str) -> String {
    let mut map = nvs_diagnostics::SourceMap::new();
    let id = map.add("case.nvs", source);
    nvs_fmt::format(map.file(id)).expect("the case formats")
}

/// `rule:ide/a-list-splits-onto-lines-and-joins-onto-one`: a split of the
/// formatted `joined` at `needle` writes `split`, the join of that writes
/// `joined` again, and formatting either changes nothing more.
fn splits_and_joins(joined: &str, needle: &str, split: &str) {
    assert_eq!(formatted(joined), joined, "the joined case is formatted");
    let (toggle, written) = relaid(joined, needle);
    assert_eq!(toggle, nvs_fmt::Toggle::Split);
    assert_eq!(written, split);
    assert_eq!(
        formatted(&written),
        written,
        "the split is what `nvs fmt` writes"
    );
    let (toggle, written) = relaid(split, needle);
    assert_eq!(toggle, nvs_fmt::Toggle::Join);
    assert_eq!(written, joined);
}

#[test]
fn a_plus_chain_splits_and_joins_as_nvs_fmt_lays_it_out() {
    splits_and_joins(
        "<?nvs\nint $a = 1;\nint $b = 2;\nint $total = $a + $b + 3;\necho $total;\n",
        "+ 3",
        "<?nvs\nint $a = 1;\nint $b = 2;\nint $total = $a\n    + $b\n    + 3;\necho $total;\n",
    );
}

#[test]
fn a_ternary_splits_and_joins_as_nvs_fmt_lays_it_out() {
    splits_and_joins(
        "<?nvs\nint $a = 1;\nstring $label = $a > 0 ? 'more' : 'less';\necho $label;\n",
        "'more'",
        "<?nvs\nint $a = 1;\nstring $label = $a > 0\n    ? 'more'\n    : 'less';\necho $label;\n",
    );
}

#[test]
fn a_match_header_splits_and_joins_as_nvs_fmt_lays_it_out() {
    splits_and_joins(
        "<?nvs\nint $a = 1;\nint $b = 2;\nstring $size = match ($a + $b) {\n    3 => 'small',\n    \
         default => 'large',\n};\necho $size;\n",
        "+ $b",
        "<?nvs\nint $a = 1;\nint $b = 2;\nstring $size = match (\n    $a\n    + $b\n) {\n    \
         3 => 'small',\n    default => 'large',\n};\necho $size;\n",
    );
}

#[test]
fn a_foreach_header_splits_and_joins_as_nvs_fmt_lays_it_out() {
    splits_and_joins(
        "<?nvs\narray<int> $rows = [1, 2];\nforeach ($rows as int $row) {\n    echo $row;\n}\n",
        "as int",
        "<?nvs\narray<int> $rows = [1, 2];\nforeach (\n    $rows\n    as int $row\n) {\n    \
         echo $row;\n}\n",
    );
}

/// A condition broken only inside a nested chain is joined whole, the nested
/// chain with it.
#[test]
fn a_condition_broken_inside_a_nested_chain_joins_whole() {
    let broken = "<?nvs\nint $a = 1;\nint $b = 2;\nif (\n    $a > 0\n    && $b\n        + 1 > 2\n) {\n    \
                  echo $a;\n}\n";
    let (toggle, written) = relaid(broken, "&& $b");
    assert_eq!(toggle, nvs_fmt::Toggle::Join);
    assert_eq!(
        written,
        "<?nvs\nint $a = 1;\nint $b = 2;\nif ($a > 0 && $b + 1 > 2) {\n    echo $a;\n}\n"
    );
}

/// The document the command tests open: a method `add` whose body reads its
/// parameter, called once.
const CART: &str = "<?nvs\nclass Cart {\n    public function add(int $count): int {\n        \
                    return $count;\n    }\n}\nvar $cart = new Cart();\necho $cart->add(2);\n";

/// `RequestFailed`, the code of a refusal the client shows.
const REQUEST_FAILED: i32 = -32803;

/// Runs `exchange` against a server in this process that has opened `source`
/// as `cart.nvs`, for a client that applies an edit the server sends. Then
/// it shuts the server down and checks that it served without a protocol
/// error.
fn commanded(source: &str, exchange: impl FnOnce(&lsp_server::Connection, &lsp_types::Uri)) {
    use lsp_server::{Connection, Message, Notification};

    let dir = nvs_test::temp::TempDir::new(nvs_repo::scratch("lsp-actions-command"));
    dir.write("cart.nvs", source);
    let uri = uri_of(&dir.at("cart.nvs")).expect("a scratch path is UTF-8");
    let (server, client) = Connection::memory();
    let serving = std::thread::spawn(move || nvs_lsp::serve(&server));
    let notify = |method: &str, params: serde_json::Value| {
        client
            .sender
            .send(Message::Notification(Notification::new(
                method.to_owned(),
                params,
            )))
            .expect("the server is still reading");
    };

    ask(
        &client,
        1,
        "initialize",
        serde_json::json!({ "capabilities": { "workspace": { "applyEdit": true } } }),
    );
    notify("initialized", serde_json::json!({}));
    notify(
        "textDocument/didOpen",
        serde_json::json!({ "textDocument": {
            "uri": uri.as_str(), "languageId": "novis", "version": 1, "text": source,
        } }),
    );

    exchange(&client, &uri);

    ask(&client, 99, "shutdown", serde_json::json!(null));
    notify("exit", serde_json::json!(null));
    serving
        .join()
        .expect("the server thread did not panic")
        .expect("the server served without a protocol error");
}

/// Sends one request and returns the requests the server sent before its
/// response, and the response. A notification is skipped.
fn ask(
    client: &lsp_server::Connection,
    id: i32,
    method: &str,
    params: serde_json::Value,
) -> (Vec<lsp_server::Request>, lsp_server::Response) {
    use lsp_server::{Message, Request, RequestId};

    client
        .sender
        .send(Message::Request(Request::new(
            RequestId::from(id),
            method.to_owned(),
            params,
        )))
        .expect("the server is still reading");
    let mut sent = Vec::new();
    loop {
        match client
            .receiver
            .recv_timeout(std::time::Duration::from_secs(60))
        {
            Ok(Message::Response(response)) if response.id == RequestId::from(id) => {
                return (sent, response);
            }
            Ok(Message::Request(request)) => sent.push(request),
            Ok(_) => {}
            Err(error) => panic!("no response to `{method}`: {error}"),
        }
    }
}

/// The line and character of the start of `needle` in `source`, which is
/// ASCII, so a UTF-16 column is a byte column.
fn position_of(source: &str, needle: &str) -> serde_json::Value {
    let at = source.find(needle).expect("the anchor is in the document");
    let line = source[..at].matches('\n').count();
    let character = at - source[..at].rfind('\n').map_or(0, |newline| newline + 1);
    serde_json::json!({ "line": line, "character": character })
}

/// `source` with the text edits `edit` makes to `uri`.
fn applied(source: &str, edit: &serde_json::Value, uri: &lsp_types::Uri) -> String {
    let offset = |position: &serde_json::Value| -> usize {
        let line = usize::try_from(position["line"].as_u64().expect("a line")).expect("small");
        let character =
            usize::try_from(position["character"].as_u64().expect("a column")).expect("small");
        let start: usize = source.split_inclusive('\n').take(line).map(str::len).sum();
        start + character
    };
    let mut edits: Vec<(usize, usize, String)> = edit["changes"][uri.as_str()]
        .as_array()
        .expect("the edit changes the document")
        .iter()
        .map(|text_edit| {
            (
                offset(&text_edit["range"]["start"]),
                offset(&text_edit["range"]["end"]),
                text_edit["newText"].as_str().expect("new text").to_owned(),
            )
        })
        .collect();
    edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    let mut written = source.to_owned();
    for (start, end, text) in edits {
        written.replace_range(start..end, &text);
    }
    written
}

/// The arguments of the *Change parameters…* action at the name `add`.
fn change_parameters_arguments(
    client: &lsp_server::Connection,
    uri: &lsp_types::Uri,
) -> Vec<String> {
    let at = position_of(CART, "add(int");
    let (_, response) = ask(
        client,
        2,
        "textDocument/codeAction",
        serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "range": { "start": at, "end": at },
            "context": { "diagnostics": [] },
        }),
    );
    let actions = response.response_result.expect("code actions are answered");
    let action = actions
        .as_array()
        .expect("a list of actions")
        .iter()
        .find(|action| action["command"]["command"] == "nvs.changeParameters")
        .unwrap_or_else(|| panic!("no Change parameters action in {actions}"));
    assert_eq!(action["title"], "Change parameters of `add`…");
    assert_eq!(action["kind"], nvs_lsp::split_join::KIND);
    action["command"]["arguments"]
        .as_array()
        .expect("arguments")
        .iter()
        .map(|argument| argument.as_str().expect("a string").to_owned())
        .collect()
}

/// The action on a method's name carries the method and its current list.
/// Sent back with a new list, the command answers `null` and sends the edit
/// the engine computed with `workspace/applyEdit`.
#[test]
fn change_parameters_command_applies_the_engines_edit() {
    commanded(CART, |client, uri| {
        let arguments = change_parameters_arguments(client, uri);
        assert_eq!(arguments[1], "int $count", "{arguments:?}");
        let (sent, response) = ask(
            client,
            3,
            "workspace/executeCommand",
            serde_json::json!({
                "command": "nvs.changeParameters",
                "arguments": [arguments[0], "int $count, int $times = 1"],
            }),
        );
        let answered = response
            .response_result
            .unwrap_or_else(|error| panic!("the command was refused: {}", error.message));
        assert_eq!(answered, serde_json::Value::Null);
        let [request] = sent.as_slice() else {
            panic!("expected one applyEdit, got {sent:?}");
        };
        assert_eq!(request.method, "workspace/applyEdit");
        let written = applied(CART, &request.params["edit"], uri);
        assert!(
            written.contains("public function add(int $count, int $times = 1): int {"),
            "{written}"
        );
        assert!(written.contains("echo $cart->add(2);"), "{written}");
    });
}

/// A list the engine refuses is answered with the engine's sentence, and no
/// edit is sent.
#[test]
fn a_refused_change_parameters_command_answers_the_reason() {
    commanded(CART, |client, uri| {
        let arguments = change_parameters_arguments(client, uri);
        let (sent, response) = ask(
            client,
            3,
            "workspace/executeCommand",
            serde_json::json!({ "command": "nvs.changeParameters", "arguments": [arguments[0], ""] }),
        );
        assert!(sent.is_empty(), "{sent:?}");
        let error = response
            .response_result
            .expect_err("the removal is refused");
        assert_eq!(error.code, REQUEST_FAILED);
        assert!(
            error.message.contains("so the parameter cannot be removed"),
            "{}",
            error.message
        );
    });
}

/// A command this server does not declare is refused by name, and no edit is
/// sent.
#[test]
fn an_unknown_command_is_refused() {
    commanded(CART, |client, _| {
        let (sent, response) = ask(
            client,
            3,
            "workspace/executeCommand",
            serde_json::json!({ "command": "nvs.unknown", "arguments": [] }),
        );
        assert!(sent.is_empty(), "{sent:?}");
        let error = response
            .response_result
            .expect_err("an unknown command is refused");
        assert_eq!(error.code, REQUEST_FAILED);
        assert!(error.message.contains("`nvs.unknown`"), "{}", error.message);
    });
}
