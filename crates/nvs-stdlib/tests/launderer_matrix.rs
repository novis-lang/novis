//! `rule:security/launderers-are-sink-named`'s promise, checked against every launderer: the
//! answer a launderer gives is safe at the one sink it names.
//!
//! Each launderer on the roster (`rule:security/launderer-answers-a-carrier`'s registry guard
//! reads the same rows) is called through the symbol compiled code calls, with the inputs that
//! change a structure somewhere: `.`, `..`, `/`, `\`, a NUL byte, CR LF and an empty string. Each
//! row names its sink and a check of that sink, and a cell passes when the member throws or its
//! answer passes the check.
//!
//! A launderer that answers a plain `string` can reach a sink it was not written for, because a
//! `string` carries no sink. Where that answer is a dot segment or a separator, a path sink takes
//! it and reads it as a step to another folder. Each such launderer is listed in [`CROSS_SINK`] with
//! what its answer keeps, and `rule:security/launderer-answers-a-carrier`'s Declined list records
//! each as a documented limit: a path sink's launderer is `Core\IO::within`.

use std::collections::BTreeSet;
use std::path::{Component, Path};

use nvs_config::resolve::Disk;
use nvs_config::tree::{CapFs, Capabilities, Setting};
use nvs_runtime::{Ctx, Value};
use nvs_stdlib::registry::{CLASSES, CoreTy, Qual};

/// The inputs, each with the words a failure prints for it.
const INPUTS: &[(&str, &str)] = &[
    ("a dot", "."),
    ("two dots", ".."),
    ("a slash", "/"),
    ("a backslash", "\\"),
    ("a NUL byte", "\0"),
    ("CR LF", "\r\n"),
    ("an empty string", ""),
];

/// What a launderer did with one input.
#[derive(Debug)]
enum Answer {
    /// It threw a catchable error, which is always safe.
    Threw,
    /// It answered this text, or this text is what the sink received.
    Text(String),
}

/// One launderer: its sink, how it is called, and the check its sink makes of the answer.
struct Row {
    member: &'static str,
    sink: &'static str,
    run: fn(&mut Ctx, &str) -> Answer,
    safe: fn(&str, &str) -> Result<(), String>,
}

/// The launderers no input of this matrix reaches, each with the reason.
const EXEMPT: &[(&str, &str)] = &[
    (
        r"Core\Taint::assertTrusted",
        "the identity, and the one launderer that names no sink (`rule:security/assert-trusted`)",
    ),
    (
        r"Core\Taint::assertTrustedBytes",
        "the identity, and the one launderer that names no sink (`rule:security/assert-trusted`)",
    ),
    (
        r"Core\Markdown\Live::push",
        "it answers a `Core\\Markdown\\Update`, which no sink takes; its `done` and `open` are \
         rendered by the renderer `Core\\Markdown::toHtml`'s row checks",
    ),
];

/// The launderers whose plain `string` answer is a dot segment or a separator for some input, so a
/// path sink reads it as a step, each with what its answer keeps.
/// `rule:security/launderer-answers-a-carrier` declines a carrier for them.
const CROSS_SINK: &[(&str, &str)] = &[
    (
        r"Core\Cli::escape",
        "a terminal shows `.`, `..`, `/` and `\\` as text, so the escape keeps them",
    ),
    (
        r"Core\Regex::quote",
        "a pattern matches `/` as itself, and `\\.` matches only a dot",
    ),
    (
        r"Core\Uri::encodeFormValue",
        "a dot is unreserved in a form value, and a query value of `..` is legitimate",
    ),
];

/// The `Core` member compiled code calls by `symbol`.
fn member(symbol: &str) -> nvs_runtime::NvsFn {
    let (_, code) = nvs_stdlib::symbols()
        .into_iter()
        .find(|(name, _)| *name == symbol)
        .unwrap_or_else(|| panic!("no `Core` member is linked as {symbol}"));
    #[expect(
        unsafe_code,
        reason = "every `Core` member is linked with the one helper signature"
    )]
    // SAFETY: as above.
    unsafe {
        std::mem::transmute::<*const u8, nvs_runtime::NvsFn>(code)
    }
}

/// Frees a value this test owns.
fn free(value: Value) {
    #[expect(
        unsafe_code,
        reason = "this frame owns the reference and drops it once"
    )]
    // SAFETY: as above.
    unsafe {
        value.release();
    }
}

/// Calls `symbol` with `args`, which it frees afterwards. `None` is a throw. A fatal error fails
/// the test, since a launderer must not end a request on its own input.
fn call(ctx: &mut Ctx, symbol: &str, args: Vec<Value>) -> Option<Value> {
    let answer = nvs_runtime::call(member(symbol), ctx, &args);
    for arg in args {
        free(arg);
    }
    match answer {
        Ok(value) => Some(value),
        Err(status) => {
            let message = ctx.take_pending();
            assert_eq!(
                status,
                nvs_runtime::THROWN,
                "{symbol} ended with a fatal error: {message:?}"
            );
            None
        }
    }
}

/// The text of a `string` answer, which this frees.
fn text(value: Value) -> String {
    let out = value
        .as_text()
        .expect("the answer is a `string`")
        .to_owned();
    free(value);
    out
}

/// The text a carrier such as `Core\Html\Markup` has, which this frees.
fn carried(value: Value) -> String {
    let ptr = value.obj_ptr().expect("a carrier is an object");
    #[expect(
        unsafe_code,
        reason = "the carrier is live and the read borrows its one slot"
    )]
    // SAFETY: as above.
    let slot =
        unsafe { nvs_runtime::object::nvs_object_field_get(ptr, nvs_runtime::CARRIER_TEXT_SLOT) };
    let out = String::from_utf8(slot.as_str_bytes().expect("a string").to_vec())
        .expect("a carrier's text is UTF-8");
    free(value);
    out
}

/// A one-argument member that answers a `string`.
fn plain(ctx: &mut Ctx, symbol: &str, input: &str) -> Answer {
    call(ctx, symbol, vec![Value::text(input)]).map_or(Answer::Threw, |v| Answer::Text(text(v)))
}

/// A one-argument member that answers a carrier.
fn carrier(ctx: &mut Ctx, symbol: &str, args: Vec<Value>) -> Answer {
    call(ctx, symbol, args).map_or(Answer::Threw, |v| Answer::Text(carried(v)))
}

/// A `Core\Cli\Text` as the terminal receives it, through its `text` method.
fn terminal_text(ctx: &mut Ctx, made: Option<Value>) -> Answer {
    let Some(made) = made else {
        return Answer::Threw;
    };
    let answer = call(ctx, "nvs_core_cli_text_text", vec![made]);
    Answer::Text(text(answer.expect("`text` does not throw")))
}

/// A `Core\Ldap\Dn` as the DN sink receives it, through `toString`.
fn dn_text(ctx: &mut Ctx, made: Option<Value>) -> Answer {
    let Some(made) = made else {
        return Answer::Threw;
    };
    let answer = call(ctx, "nvs_core_ldap_dn_to_string", vec![made]);
    Answer::Text(text(answer.expect("`toString` does not throw")))
}

/// A document written with one `Core\Xml\Writer` call that takes the input. `before` is the
/// writer's state when the call is made. The document is read back with `Core\Xml::parse`, and a
/// document it cannot read is answered as `Broken` text, which no check passes.
fn xml(ctx: &mut Ctx, symbol: &str, before: Before, input: &str) -> Answer {
    let writer = call(ctx, "nvs_core_xml_writer", vec![Value::text("")]).expect("a writer");
    let step = |ctx: &mut Ctx, symbol: &str, mut args: Vec<Value>| {
        #[expect(
            unsafe_code,
            reason = "the writer is live, and `call` frees the reference this adds"
        )]
        // SAFETY: as above.
        unsafe {
            writer.retain();
        }
        args.insert(0, writer);
        call(ctx, symbol, args).map(free).is_some()
    };
    assert!(step(ctx, "nvs_core_xml_writer_start_document", vec![]));
    if !matches!(before, Before::Prolog) {
        assert!(step(
            ctx,
            "nvs_core_xml_writer_start_element",
            vec![Value::text("r")]
        ));
    }
    let args = match before {
        Before::Attribute => vec![Value::text("a"), Value::text(input)],
        Before::Prolog | Before::Element => vec![Value::text(input)],
    };
    if !step(ctx, symbol, args) {
        free(writer);
        return Answer::Threw;
    }
    if matches!(before, Before::Prolog) {
        assert!(step(
            ctx,
            "nvs_core_xml_writer_start_element",
            vec![Value::text("r")]
        ));
    }
    assert!(step(ctx, "nvs_core_xml_writer_end_element", vec![]));
    let document = call(ctx, "nvs_core_xml_writer_end_document", vec![writer])
        .map(text)
        .expect("a document with one element ends");
    match call(ctx, "nvs_core_xml_parse", vec![Value::text(&document)]) {
        Some(node) => {
            free(node);
            Answer::Text(document)
        }
        None => Answer::Text(format!("Broken: {document:?}")),
    }
}

/// Where in a document a `Core\Xml\Writer` call is made.
#[derive(Clone, Copy)]
enum Before {
    /// Before the root element, where a doctype goes.
    Prolog,
    /// Inside the open root element, after its start tag.
    Element,
    /// On the open root element's start tag, as one attribute `a`.
    Attribute,
}

/// This crate's folder, which `Core\IO::within` resolves against.
fn base() -> std::path::PathBuf {
    nvs_repo::path("crates/nvs-stdlib")
}

/// A context that may read [`base`] and nothing else.
fn reading() -> Ctx {
    let mut caps = Capabilities {
        fs: Some(CapFs {
            read: Some(Setting::List(vec![base().to_string_lossy().into_owned()])),
            write: Some(Setting::List(Vec::new())),
        }),
        ..Capabilities::default()
    };
    caps.canonicalize(&Disk);
    let mut ctx = Ctx::buffered();
    ctx.set_config(std::sync::Arc::new(nvs_config::Snapshot {
        config: nvs_config::tree::Config {
            capabilities: Some(caps),
            ..nvs_config::tree::Config::default()
        },
        ..nvs_config::Snapshot::default()
    }));
    ctx
}

/// `Core\IO::within(base(), $input)`.
fn within(ctx: &mut Ctx, input: &str) -> Answer {
    let args = vec![Value::text(&base().to_string_lossy()), Value::text(input)];
    call(ctx, "nvs_core_io_within", args).map_or(Answer::Threw, |v| Answer::Text(text(v)))
}

/// The terminal sink: no control byte but `LF` and `TAB`, and no `DEL`.
fn terminal(_: &str, answer: &str) -> Result<(), String> {
    match answer.chars().find(|&c| {
        (c.is_ascii_control() && c != '\n' && c != '\t') || ('\u{80}'..='\u{9f}').contains(&c)
    }) {
        Some(c) => Err(format!("the terminal reads {c:?} as a control")),
        None => Ok(()),
    }
}

/// An HTML text sink: no tag but the ones named, and every `&` starts a character reference.
fn html_text(answer: &str, tags: &[&str]) -> Result<(), String> {
    let mut rest = answer.to_owned();
    for tag in tags {
        rest = rest.replace(tag, "");
    }
    if let Some(at) = rest.find(['<', '>']) {
        return Err(format!("a tag character at byte {at} of {rest:?}"));
    }
    for (at, _) in rest.match_indices('&') {
        let tail = &rest[at + 1..];
        let end = tail.find(';').unwrap_or(0);
        let name = &tail[..end];
        let named =
            !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'#');
        if !named {
            return Err(format!("a bare `&` at byte {at} of {rest:?}"));
        }
    }
    Ok(())
}

/// `Core\Html::escape`'s sink: element text, with no tag at all.
fn html_escaped(_: &str, answer: &str) -> Result<(), String> {
    html_text(answer, &[])
}

/// `sanitize` and `toHtml`'s sink: element text, where a paragraph and a line break are the
/// structure the member writes itself.
fn html_rebuilt(_: &str, answer: &str) -> Result<(), String> {
    html_text(answer, &["<p>", "</p>", "<br>", "<br />"])
}

/// The URI component sink: only unreserved bytes and `%XX` escapes, and never a dot segment.
fn uri_component(_: &str, answer: &str) -> Result<(), String> {
    if answer == "." || answer == ".." {
        return Err(format!(
            "{answer:?} is a dot segment, which a path resolves"
        ));
    }
    escapes_only(answer, b"-_.~")
}

/// The form value sink: only unreserved bytes, `+` and `%XX` escapes.
fn form_value(_: &str, answer: &str) -> Result<(), String> {
    escapes_only(answer, b"-_.+")
}

/// Whether `answer` is letters, digits, `extra` and `%XX` escapes alone.
fn escapes_only(answer: &str, extra: &[u8]) -> Result<(), String> {
    let bytes = answer.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte == b'%' {
            let hex = bytes
                .get(at + 1..at + 3)
                .is_some_and(|pair| pair.iter().all(u8::is_ascii_hexdigit));
            if !hex {
                return Err(format!("a `%` without two hex digits in {answer:?}"));
            }
            at += 3;
        } else if byte.is_ascii_alphanumeric() || extra.contains(&byte) {
            at += 1;
        } else {
            return Err(format!("the byte {byte:#04x} is a delimiter in {answer:?}"));
        }
    }
    Ok(())
}

/// The identifier sink: a delimited bare identifier.
fn identifier(_: &str, answer: &str) -> Result<(), String> {
    let name = answer.trim_matches(['"', '`']);
    let bare = name
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    if bare {
        Ok(())
    } else {
        Err(format!("{answer:?} is not a bare identifier"))
    }
}

/// The regex pattern sink: the answer compiles, and matches the input and nothing else.
fn pattern(input: &str, answer: &str) -> Result<(), String> {
    let anchored = regex::Regex::new(&format!("^(?:{answer})$")).map_err(|e| e.to_string())?;
    if !anchored.is_match(input)
        || anchored.is_match(&format!("{input}x"))
        || anchored.is_match("x")
    {
        return Err(format!("{answer:?} matches more or less than {input:?}"));
    }
    Ok(())
}

/// The DN sink for `Dn::of("cn", $input)`: one level, whose value is the input.
fn dn_one(input: &str, answer: &str) -> Result<(), String> {
    dn_levels(input, answer, 1)
}

/// The DN sink for `Dn::of("dc", "example")->child("cn", $input)`: two levels.
fn dn_two(input: &str, answer: &str) -> Result<(), String> {
    dn_levels(input, answer, 2)
}

/// Whether `answer` reads back as `levels` levels, the first a single pair whose value is `input`.
fn dn_levels(input: &str, answer: &str, levels: usize) -> Result<(), String> {
    let dn =
        nvs_ldap::Dn::parse(answer).map_err(|e| format!("{answer:?} does not parse: {e:?}"))?;
    let rdn = &dn.rdns()[0];
    if dn.rdns().len() != levels || rdn.parts().len() != 1 || rdn.first().value != input {
        return Err(format!(
            "{answer:?} does not read back as {levels} levels with {input:?}"
        ));
    }
    Ok(())
}

/// The path sink `Core\Path::fromCwd` answers for: a path that starts at a root, with every `.`
/// and `..` already resolved. On Windows `\` starts at the root of the current drive.
fn resolved_path(_: &str, answer: &str) -> Result<(), String> {
    let path = Path::new(answer);
    if !path.has_root() {
        return Err(format!("{answer:?} does not start at a root"));
    }
    if path
        .components()
        .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Err(format!("{answer:?} still has a dot segment"));
    }
    Ok(())
}

/// The path sink `Core\IO::within` answers for: a path inside the base.
fn inside_base(_: &str, answer: &str) -> Result<(), String> {
    resolved_path("", answer)?;
    let root = match within(&mut reading(), "") {
        Answer::Text(root) => root,
        Answer::Threw => return Err("the base alone throws".to_owned()),
    };
    if Path::new(answer).starts_with(&root) {
        Ok(())
    } else {
        Err(format!("{answer:?} is outside {root:?}"))
    }
}

/// `Core\Http::allowUrl`'s sink is an outbound address. None of the inputs is one, so it throws.
fn throws_only(_: &str, answer: &str) -> Result<(), String> {
    Err(format!("it answered {answer:?}"))
}

/// An XML document that `Core\Xml::parse` read back.
fn document(_: &str, answer: &str) -> Result<(), String> {
    if answer.starts_with("Broken: ") {
        Err(answer.to_owned())
    } else {
        Ok(())
    }
}

const ROWS: &[Row] = &[
    Row {
        member: r"Core\Cli::escape",
        sink: "the terminal",
        run: |ctx, input| plain(ctx, "nvs_core_cli_escape", input),
        safe: terminal,
    },
    Row {
        member: r"Core\Cli\Text::plain",
        sink: "the terminal",
        run: |ctx, input| {
            let made = call(ctx, "nvs_core_cli_text_plain", vec![Value::text(input)]);
            terminal_text(ctx, made)
        },
        safe: terminal,
    },
    Row {
        member: r"Core\Cli\Text::styled",
        sink: "the terminal",
        run: |ctx, input| {
            let style = call(
                ctx,
                "nvs_core_cli_style_of",
                (0..7).map(|_| Value::null()).collect(),
            )
            .expect("a style with no options");
            let made = call(
                ctx,
                "nvs_core_cli_text_styled",
                vec![Value::text(input), style],
            );
            terminal_text(ctx, made)
        },
        safe: terminal,
    },
    Row {
        member: r"Core\Db::quoteIdentifier",
        sink: "an SQL identifier",
        run: |ctx, input| plain(ctx, "nvs_core_db_quote_identifier", input),
        safe: identifier,
    },
    Row {
        member: r"Core\Html::escape",
        sink: "HTML element text",
        run: |ctx, input| carrier(ctx, "nvs_core_html_escape", vec![Value::text(input)]),
        safe: html_escaped,
    },
    Row {
        member: r"Core\Html::sanitize",
        sink: "HTML element text",
        run: |ctx, input| carrier(ctx, "nvs_core_html_sanitize", vec![Value::text(input)]),
        safe: html_rebuilt,
    },
    Row {
        member: r"Core\Http::allowUrl",
        sink: "an outbound address",
        run: |ctx, input| {
            call(ctx, "nvs_core_http_allow_url", vec![Value::text(input)]).map_or(
                Answer::Threw,
                |v| {
                    free(v);
                    Answer::Text("a `Core\\Http\\Target`".to_owned())
                },
            )
        },
        safe: throws_only,
    },
    Row {
        member: r"Core\IO::within",
        sink: "a file path",
        run: |_, input| within(&mut reading(), input),
        safe: inside_base,
    },
    Row {
        member: r"Core\Ldap\Dn::child",
        sink: "a DN",
        run: |ctx, input| {
            let parent = call(
                ctx,
                "nvs_core_ldap_dn_of",
                vec![Value::text("dc"), Value::text("example")],
            )
            .expect("a one-level DN");
            let made = call(
                ctx,
                "nvs_core_ldap_dn_child",
                vec![parent, Value::text("cn"), Value::text(input)],
            );
            dn_text(ctx, made)
        },
        safe: dn_two,
    },
    Row {
        member: r"Core\Ldap\Dn::of",
        sink: "a DN",
        run: |ctx, input| {
            let made = call(
                ctx,
                "nvs_core_ldap_dn_of",
                vec![Value::text("cn"), Value::text(input)],
            );
            dn_text(ctx, made)
        },
        safe: dn_one,
    },
    Row {
        member: r"Core\Markdown::toHtml",
        sink: "HTML element text",
        run: |ctx, input| {
            let mut args = vec![Value::text(input)];
            args.extend((0..4).map(|_| Value::null()));
            carrier(ctx, "nvs_core_markdown_to_html", args)
        },
        safe: html_rebuilt,
    },
    Row {
        member: r"Core\Path::fromCwd",
        sink: "a file path typed on the command line",
        run: |ctx, input| plain(ctx, "nvs_core_path_from_cwd", input),
        safe: resolved_path,
    },
    Row {
        member: r"Core\Regex::quote",
        sink: "a regex pattern",
        run: |ctx, input| plain(ctx, "nvs_core_regex_quote", input),
        safe: pattern,
    },
    Row {
        member: r"Core\Uri::encodeComponent",
        sink: "a URI component",
        run: |ctx, input| plain(ctx, "nvs_core_uri_encode_component", input),
        safe: uri_component,
    },
    Row {
        member: r"Core\Uri::encodeFormValue",
        sink: "a form value",
        run: |ctx, input| plain(ctx, "nvs_core_uri_encode_form_value", input),
        safe: form_value,
    },
    Row {
        member: r"Core\Xml\Writer::attribute",
        sink: "an XML document",
        run: |ctx, input| {
            xml(
                ctx,
                "nvs_core_xml_writer_attribute",
                Before::Attribute,
                input,
            )
        },
        safe: document,
    },
    Row {
        member: r"Core\Xml\Writer::cdata",
        sink: "an XML document",
        run: |ctx, input| xml(ctx, "nvs_core_xml_writer_cdata", Before::Element, input),
        safe: document,
    },
    Row {
        member: r"Core\Xml\Writer::comment",
        sink: "an XML document",
        run: |ctx, input| xml(ctx, "nvs_core_xml_writer_comment", Before::Element, input),
        safe: document,
    },
    Row {
        member: r"Core\Xml\Writer::content",
        sink: "an XML document",
        run: |ctx, input| xml(ctx, "nvs_core_xml_writer_content", Before::Element, input),
        safe: document,
    },
    Row {
        member: r"Core\Xml\Writer::doctype",
        sink: "an XML document",
        run: |ctx, input| xml(ctx, "nvs_core_xml_writer_doctype", Before::Prolog, input),
        safe: document,
    },
    Row {
        member: r"Core\Xml\Writer::startElement",
        sink: "an XML document",
        run: |ctx, input| {
            xml(
                ctx,
                "nvs_core_xml_writer_start_element",
                Before::Element,
                input,
            )
        },
        safe: document,
    },
];

/// Every member with a `Qual::Launder` parameter, as `Class::member`, and whether it answers a
/// plain `string`.
fn roster() -> Vec<(String, bool)> {
    CLASSES
        .iter()
        .flat_map(|class| class.members().map(move |method| (class.name, method)))
        .filter(|(_, method)| {
            method.params.iter().any(|param| {
                matches!(
                    param,
                    CoreTy::Text(Qual::Launder)
                        | CoreTy::Blob(Qual::Launder)
                        | CoreTy::Path(Qual::Launder)
                )
            })
        })
        .map(|(class, method)| {
            (
                format!("{class}::{}", method.name),
                matches!(method.return_ty, CoreTy::Str),
            )
        })
        .collect()
}

#[test]
fn every_launderer_has_a_row_or_a_reason() {
    let listed: BTreeSet<&str> = ROWS
        .iter()
        .map(|row| row.member)
        .chain(EXEMPT.iter().map(|(member, _)| *member))
        .collect();
    let on_roster: BTreeSet<String> = roster().into_iter().map(|(name, _)| name).collect();
    let on_roster: BTreeSet<&str> = on_roster.iter().map(String::as_str).collect();
    assert_eq!(
        on_roster, listed,
        "every launderer is in `ROWS` with its sink, or in `EXEMPT` with the reason no input reaches it"
    );
}

#[test]
fn every_launderer_answers_what_its_own_sink_takes() {
    let mut ctx = Ctx::buffered();
    let mut unsafe_cells = Vec::new();
    for row in ROWS {
        for (words, input) in INPUTS {
            if let Answer::Text(answer) = (row.run)(&mut ctx, input)
                && let Err(why) = (row.safe)(input, &answer)
            {
                unsafe_cells.push(format!(
                    "{} given {words}, at {}: {why}",
                    row.member, row.sink
                ));
            }
        }
    }
    assert!(
        unsafe_cells.is_empty(),
        "each answer is safe at the launderer's own sink:\n{}",
        unsafe_cells.join("\n")
    );
}

#[test]
fn a_plain_answer_a_path_reads_as_a_step_has_a_gap_record() {
    let mut ctx = Ctx::buffered();
    let plain: BTreeSet<String> = roster()
        .into_iter()
        .filter_map(|(name, plain)| plain.then_some(name))
        .collect();
    let mut found = BTreeSet::new();
    for row in ROWS
        .iter()
        .filter(|row| plain.contains(row.member) && !row.sink.contains("path"))
    {
        for (_, input) in INPUTS {
            if let Answer::Text(answer) = (row.run)(&mut ctx, input) {
                let step = answer == "." || answer == ".." || answer.contains(['/', '\\']);
                if step {
                    found.insert(row.member);
                }
            }
        }
    }
    let recorded: BTreeSet<&str> = CROSS_SINK.iter().map(|(member, _)| *member).collect();
    assert_eq!(
        found, recorded,
        "a launderer whose plain answer a path reads as a step is listed in `CROSS_SINK`, and \
         `rule:security/launderer-answers-a-carrier`'s Declined list names it"
    );
    for (member, keeps) in CROSS_SINK {
        assert!(!keeps.is_empty(), "{member} says what its answer keeps");
    }
}
