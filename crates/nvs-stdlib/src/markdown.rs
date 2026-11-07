//! `Core\Markdown` — `rule:core-classes/markdown-to-html`'s renderer, a
//! launderer onto the HTML sink beside `Core\Html::escape` and `sanitize`.
//!
//! # Three steps, and the policy is not here
//!
//! `pulldown-cmark` (vendored under `vendor/pulldown-cmark/`, with the
//! patches the rule's engine paragraph lists) reads the document into events.
//! [`Renderer`] turns those events into the [`Parsed`] tree [`crate::html`]
//! serializes, writing only elements and attributes on that module's list.
//! [`crate::html::source`] writes the tree. Nothing here joins an attribute
//! from strings, so a value is escaped by the serializer's own code.
//!
//! A document's raw HTML is the one thing the tree cannot hold as nodes, so
//! it is a text node named [`RAW_SOURCE`], which the serializer writes as it
//! stands. A tree holding one is never the answer: its source is parsed by the
//! WHATWG algorithm and rebuilt through the sanitizer's list, as
//! `Core\Html::sanitize` does. A tree deeper than the parser's
//! [`OPEN_ELEMENTS_CAP`] takes the same path, which is what makes the flatten
//! exactly the parser's. Every other tree is written once, and
//! `sanitizing_the_output_again_changes_nothing` is what makes that skip safe.
//!
//! # Newlines between tags
//!
//! The renderer writes a line feed where cmark's HTML writer does: after a
//! block's end tag, after an opening `<ul>`, `<ol>`, `<blockquote>`,
//! `<table>`, `<thead>`, `<tbody>` and `<tr>`, and before a block that does
//! not start a line. They are text nodes, so the answer reads like every other
//! CommonMark renderer's and the spec's expected HTML compares after only the
//! normalization a reparse does.
//!
//! # What is ours over the engine's events
//!
//! - **The text is prepared first** ([`prepared`]): a leading byte-order mark
//!   is removed, NUL becomes U+FFFD and every line ending becomes a line feed,
//!   which is what the HTML parser would make of each anyway.
//! - **A URL passes [`url_allowed`]** or is not written: a link becomes its
//!   text and an image is not written.
//! - **Footnotes are written at the end**, numbered in the order they are
//!   first referenced, and a definition nothing references is not written.
//! - **A code block's class is `language-` once**, and only when its language
//!   passes the sanitizer's value test.
//! - **GFM's extended autolinks** ([`autolink`]) are found in each run of
//!   text outside a link and a code block.
//! - **Added output has a budget** ([`ADDED_OUTPUT_SLACK`]).
//! - **Four callbacks** ([`callbacks`]) change a link's or an image's URL, set
//!   a heading's `id` and replace a fenced block. A URL a callback returns
//!   passes [`url_allowed`] too, and an `id` passes the sanitizer's test.
//!
//! # One block group at a time
//!
//! `Core\Markdown::toHtmlEach` renders a document of any size in groups
//! (`rule:core-classes/markdown-chunks`). [`segments`] cuts the text into
//! runs of whole top-level blocks, [`groups`] writes each group alone, and
//! [`each`] is the walk a `foreach` reads, filed on the request.
//!
//! **What it spends:** the prepared text when it differs from the input, the
//! output tree, the footnote definitions until the end, and the answer's
//! source. A document with raw HTML also holds its parse and its rebuild.
//! All of it is attributed to the request and freed when the call returns.

use std::collections::HashMap;
use std::ops::Range;

use nvs_runtime::{Fault, Value};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};

use crate::html::{
    MARKUP, MARKUP_NAME, OPEN_ELEMENTS_CAP, RAW_SOURCE, addressable, bounded, bounded_in_page,
    language_class, markdown_id, rebuilt, source,
};
use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};
use crate::xml::{Kind, Parsed};
use callbacks::{Answer, Callables, Callbacks, NoCallbacks, Trusted, given};

mod autolink;
pub(crate) mod callbacks;
mod each;
mod from_html;
mod groups;
mod live;
mod segments;

/// `Core\Markdown`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Converts Markdown to HTML that is safe to put into a page. It reads CommonMark \
            with tables, task lists, strikethrough, autolinks and footnotes. `toHtmlEach` \
            converts a large document one part at a time.",
};

/// The parameter types of the four callbacks, in order: `link`, `image`,
/// `heading` and `code`. Both members take them.
const CALLBACK_PARAMS: [CoreTy; 4] = [
    CoreTy::Nullable(&CoreTy::CallableSig(
        &[CoreTy::Str],
        &CoreTy::Nullable(&CoreTy::Str),
    )),
    CoreTy::Nullable(&CoreTy::CallableSig(
        &[CoreTy::Str],
        &CoreTy::Nullable(&CoreTy::Str),
    )),
    CoreTy::Nullable(&CoreTy::CallableSig(
        &[CoreTy::Int, CoreTy::Str],
        &CoreTy::Nullable(&CoreTy::Str),
    )),
    CoreTy::Nullable(&CoreTy::CallableSig(
        &[CoreTy::Str, CoreTy::Str],
        &CoreTy::Nullable(&CoreTy::Instance(MARKUP_NAME)),
    )),
];

/// `rule:core-classes/markdown-to-html`'s class: the renderer, and the members
/// later stages of the `markdown` goal add beside it.
// rank: 1045
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Markdown",
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "toHtml",
            names: &["markdown", "link", "image", "heading", "code"],
            params: &[
                CoreTy::Text(Qual::Launder),
                CALLBACK_PARAMS[0],
                CALLBACK_PARAMS[1],
                CALLBACK_PARAMS[2],
                CALLBACK_PARAMS[3],
            ],
            defaults: &[Const::Null, Const::Null, Const::Null, Const::Null],
            return_ty: CoreTy::Instance(MARKUP_NAME),
            symbol: "nvs_core_markdown_to_html",
            doc: Some(&TO_HTML_DOC),
        },
        CoreMethod {
            name: "toHtmlEach",
            names: &["markdown", "link", "image", "heading", "code"],
            // The forward source `Core\Json::decodeEach` and `Core\Csv::rows`
            // take. The bytes arm is `tainted bytes`, so a plain `bytes`
            // widens to it. Each group is a `Markup`, as `toHtml`'s answer is.
            params: &[
                CoreTy::Union(&[
                    CoreTy::Alias(
                        r"Core\Bytes::Source",
                        &CoreTy::Union(&[
                            CoreTy::TaintedBytes,
                            CoreTy::Instance(crate::io::FILE_NAME),
                            CoreTy::Instance(crate::request::PART_NAME),
                            CoreTy::Instance(crate::request::BODY_STREAM_NAME),
                        ]),
                    ),
                    CoreTy::Alias(
                        r"Core\Bytes::Chunks",
                        &CoreTy::Iterated(&CoreTy::TaintedBytes),
                    ),
                ]),
                CALLBACK_PARAMS[0],
                CALLBACK_PARAMS[1],
                CALLBACK_PARAMS[2],
                CALLBACK_PARAMS[3],
            ],
            defaults: &[Const::Null, Const::Null, Const::Null, Const::Null],
            return_ty: CoreTy::Instance(GROUPS_NAME),
            symbol: "nvs_core_markdown_to_html_each",
            doc: Some(&TO_HTML_EACH_DOC),
        },
        CoreMethod {
            name: "live",
            names: &["link", "image", "heading", "code"],
            params: &[
                CALLBACK_PARAMS[0],
                CALLBACK_PARAMS[1],
                CALLBACK_PARAMS[2],
                CALLBACK_PARAMS[3],
            ],
            defaults: &[Const::Null, Const::Null, Const::Null, Const::Null],
            return_ty: CoreTy::Instance(LIVE_NAME),
            symbol: "nvs_core_markdown_live",
            doc: Some(&LIVE_DOC),
        },
        CoreMethod {
            name: "fromHtml",
            names: &["html"],
            // The answer is `tainted` whatever the argument is. A node's text
            // is tainted, and a `string` answer from it would launder that
            // text for every sink (`rule:core-classes/markdown-from-html`).
            params: &[CoreTy::Union(&[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Instance(MARKUP_NAME),
                CoreTy::Instance(crate::xml::NODE_NAME),
            ])],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_markdown_from_html",
            doc: Some(&FROM_HTML_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Markdown\Live`'s fully-qualified name.
pub(crate) const LIVE_NAME: &str = r"Core\Markdown\Live";

/// `Core\Markdown\Update`'s fully-qualified name.
pub(crate) const UPDATE_NAME: &str = r"Core\Markdown\Update";

/// What `Core\Markdown::live` returns: a document that arrives in pieces
/// (`rule:core-classes/markdown-chunks`). It is an object because it has a
/// lifetime (`rule:core-api/a-lifetime-is-an-object`).
///
/// Its slots are the key its state is filed under, `0` once it has ended,
/// and the four callbacks. [`live`] owns the state.
// rank: 1047
pub(crate) const LIVE: CoreClass = CoreClass {
    name: LIVE_NAME,
    doc: Some(&LIVE_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "push",
            names: &["piece"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Instance(UPDATE_NAME),
            symbol: "nvs_core_markdown_live_push",
            doc: Some(&LIVE_PUSH_DOC),
        },
        CoreMethod {
            name: "finish",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(MARKUP_NAME),
            symbol: "nvs_core_markdown_live_finish",
            doc: Some(&LIVE_FINISH_DOC),
        },
    ],
    slots: &["reader", "link", "image", "heading", "code"],
    constants: &[],
};

/// What `Core\Markdown\Live::push` returns: the HTML the piece finished, and
/// the HTML of the rest. Two members and not properties, because a `Core`
/// instance has no property a program can read ([`CoreTy::Instance`]).
// rank: 1048
pub(crate) const UPDATE: CoreClass = CoreClass {
    name: UPDATE_NAME,
    doc: Some(&UPDATE_CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "done",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(MARKUP_NAME),
            symbol: "nvs_core_markdown_update_done",
            doc: Some(&UPDATE_DONE_DOC),
        },
        CoreMethod {
            name: "open",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Instance(MARKUP_NAME),
            symbol: "nvs_core_markdown_update_open",
            doc: Some(&UPDATE_OPEN_DOC),
        },
    ],
    slots: &["done", "open"],
    constants: &[],
};

/// `Core\Markdown\Live`'s class card — `rule:core-api/reference-card`.
const LIVE_CARD: ClassDoc = ClassDoc {
    short: "A Markdown document that arrives in pieces, such as the answer of a language \
            model. Each piece gives the HTML of the blocks it finished and the HTML of the \
            last block, which is not finished yet.",
};

/// `Core\Markdown::live`'s reference card — `rule:core-api/reference-card`.
const LIVE_DOC: MethodDoc = MethodDoc {
    short: "Creates a `Core\\Markdown\\Live` for a Markdown document that arrives in pieces. \
            Call `push` with each piece and `finish` at the end.",
    params: &[
        ParamDoc {
            name: "link",
            desc: "The same `link` function as in `Core\\Markdown::toHtml`.",
            shape: &[],
        },
        ParamDoc {
            name: "image",
            desc: "The same `image` function as in `Core\\Markdown::toHtml`.",
            shape: &[],
        },
        ParamDoc {
            name: "heading",
            desc: "The same `heading` function as in `Core\\Markdown::toHtml`.",
            shape: &[],
        },
        ParamDoc {
            name: "code",
            desc: "The same `code` function as in `Core\\Markdown::toHtml`. A block that is \
                   not finished is converted again at each `push`, so a function can run \
                   more than once for the same block.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Markdown\\Live` with no text yet.",
    errors: &[],
};

/// `Core\Markdown\Live::push`'s reference card — `rule:core-api/reference-card`.
const LIVE_PUSH_DOC: MethodDoc = MethodDoc {
    short: "Adds the next piece of the document. A piece can end anywhere, also in the \
            middle of a line or a word.",
    params: &[ParamDoc {
        name: "piece",
        desc: "The next text of the document.",
        shape: &[],
    }],
    ret: "A `Core\\Markdown\\Update`. Its `done()` is the HTML of the blocks this piece \
          finished. Add it after the HTML you already have. Its `open()` is the HTML of the \
          rest. Show it after that, and replace it at the next `push`. A block is finished \
          when the next block starts, and its HTML does not change after that.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`finish` was already called, or a callback threw an error.",
    }],
};

/// `Core\Markdown\Live::finish`'s reference card — `rule:core-api/reference-card`.
const LIVE_FINISH_DOC: MethodDoc = MethodDoc {
    short: "Ends the document. After it, `push` and `finish` throw an error.",
    params: &[],
    ret: "A `Core\\Html\\Markup` with the HTML of the last blocks and the footnotes. Add it \
          after the `done()` parts, and remove the last `open()` part. A link that uses a \
          definition from a later piece is written as text.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`finish` was already called, or a callback threw an error.",
    }],
};

/// `Core\Markdown\Update`'s class card — `rule:core-api/reference-card`.
const UPDATE_CARD: ClassDoc = ClassDoc {
    short: "What `Core\\Markdown\\Live::push` returns: the HTML of the finished blocks, and \
            the HTML of the rest of the text.",
};

/// `Core\Markdown\Update::done`'s reference card — `rule:core-api/reference-card`.
const UPDATE_DONE_DOC: MethodDoc = MethodDoc {
    short: "The HTML of the blocks the piece finished. It is empty when the piece finished \
            no block.",
    params: &[],
    ret: "A `Core\\Html\\Markup`. Add it after the HTML you already have.",
    errors: &[],
};

/// `Core\Markdown\Update::open`'s reference card — `rule:core-api/reference-card`.
const UPDATE_OPEN_DOC: MethodDoc = MethodDoc {
    short: "The HTML of the text that is not in a finished block yet, as if the document \
            ended after this piece.",
    params: &[],
    ret: "A `Core\\Html\\Markup`. Replace the `open()` part of the last `push` with it.",
    errors: &[],
};

/// `Core\Markdown\Groups`' fully-qualified name.
pub(crate) const GROUPS_NAME: &str = r"Core\Markdown\Groups";

/// The symbols behind [`GROUPS`]' `iterate()`, `advance()` and `current()`,
/// reached by name through [`crate::instance`]'s dispatch roster, as
/// `Core\Json\Elements`' are.
pub(crate) const GROUPS_ITERATE_SYMBOL: &str = "nvs_core_markdown_groups_iterate";
/// See [`GROUPS_ITERATE_SYMBOL`].
pub(crate) const GROUPS_ADVANCE_SYMBOL: &str = "nvs_core_markdown_groups_advance";
/// See [`GROUPS_ITERATE_SYMBOL`].
pub(crate) const GROUPS_CURRENT_SYMBOL: &str = "nvs_core_markdown_groups_current";

/// What `Core\Markdown::toHtmlEach` returns: the document's HTML, one block
/// group per `advance()`. It is its own iterator, as `Core\Json\Elements` is,
/// because a group does not exist until the walk has read that far.
///
/// Its slots are the key the walk is filed under, `0` once it has ended, the
/// source, the group the last `advance()` wrote, and the four callbacks.
/// [`each`] owns the walk.
// rank: 1046
pub(crate) const GROUPS: CoreClass = CoreClass {
    name: GROUPS_NAME,
    doc: Some(&GROUPS_CARD),
    methods: &[],
    instance: &[],
    slots: &[
        "reader", "source", "group", "link", "image", "heading", "code",
    ],
    constants: &[],
};

/// `Core\Markdown\Groups`' class card — `rule:core-api/reference-card`.
const GROUPS_CARD: ClassDoc = ClassDoc {
    short: "The HTML of a Markdown document, one part at a time, while a `foreach` loop \
            runs. Each part is a `Core\\Html\\Markup` with one or more whole blocks. The \
            parts joined together are the HTML of the whole document.",
};

/// `Core\Markdown::toHtmlEach`'s reference card — `rule:core-api/reference-card`.
const TO_HTML_EACH_DOC: MethodDoc = MethodDoc {
    short: "Converts the Markdown document `$markdown` to HTML one part at a time. Each part \
            has one or more whole blocks, such as a paragraph or a list. Use it for a \
            document that is too large to keep in memory.",
    params: &[
        ParamDoc {
            name: "markdown",
            desc: "The Markdown document: a `bytes` value, an open file, an upload part, a \
                   request body stream, or a list of `bytes` chunks. The bytes must be UTF-8. \
                   A file is read twice, and only one part is in memory at a time. Every \
                   other source is kept in memory until the loop ends.",
            shape: &[],
        },
        ParamDoc {
            name: "link",
            desc: "The same `link` function as in `Core\\Markdown::toHtml`. It runs when the \
                   loop reaches the part with the link.",
            shape: &[],
        },
        ParamDoc {
            name: "image",
            desc: "The same `image` function as in `Core\\Markdown::toHtml`.",
            shape: &[],
        },
        ParamDoc {
            name: "heading",
            desc: "The same `heading` function as in `Core\\Markdown::toHtml`.",
            shape: &[],
        },
        ParamDoc {
            name: "code",
            desc: "The same `code` function as in `Core\\Markdown::toHtml`.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Markdown\\Groups` that a `foreach` loop reads. Each part is a \
          `Core\\Html\\Markup`. The parts joined together are the same HTML that \
          `Core\\Markdown::toHtml` returns for the whole document. Nothing is read until \
          the loop starts.",
    errors: &[ErrorDoc {
        error: "ParseError",
        desc: "The document is not valid UTF-8. The message gives the position of the first \
               wrong byte. The loop throws it when it reaches that byte.",
    }],
};

const TO_HTML_DOC: MethodDoc = MethodDoc {
    short: "Converts the Markdown document `$markdown` to HTML. The result is safe to write \
            into a page, even when the document comes from a user.",
    params: &[
        ParamDoc {
            name: "markdown",
            desc: "The Markdown document. Any text is a valid document.",
            shape: &[],
        },
        ParamDoc {
            name: "link",
            desc: "A function that gets each link's URL and returns the URL to write. If it \
                   returns `null`, the link is removed and its text stays. The URL it returns \
                   is checked like a URL in the document.",
            shape: &[],
        },
        ParamDoc {
            name: "image",
            desc: "A function that gets each image's URL and returns the URL to write. If it \
                   returns `null`, the image is removed. The URL it returns is checked like a \
                   URL in the document.",
            shape: &[],
        },
        ParamDoc {
            name: "heading",
            desc: "A function that gets each heading's level and text and returns its `id`. \
                   The `id` is written after `md-`, so `intro` gives `md-intro`. If it returns \
                   `null`, or an `id` with a space, the heading has no `id`.",
            shape: &[],
        },
        ParamDoc {
            name: "code",
            desc: "A function that gets the language and the text of each fenced code block \
                   that names a language. The `Core\\Html\\Markup` it returns replaces the \
                   block, exactly as it is. If it returns `null`, the block is written as usual. \
                   Use it for syntax highlighting, math or diagrams.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Html\\Markup` with the document's HTML. HTML inside the document is \
          cleaned the same way `Core\\Html::sanitize` cleans it. A link with an unsafe URL, \
          such as `javascript:`, is written as plain text. An image with an unsafe URL is \
          removed. If a callback throws an error, the conversion stops and your code gets \
          that error.",
    errors: &[],
};

const FROM_HTML_DOC: MethodDoc = MethodDoc {
    short: "Converts HTML to Markdown. Text in the HTML stays text, so it never becomes \
            Markdown syntax.",
    params: &[ParamDoc {
        name: "html",
        desc: "The HTML. It can be a string, a `Core\\Html\\Markup`, or a node from \
               `Core\\Html::parse`. Pass a node to convert only that part of a page. \
               Elements are kept or removed as `Core\\Html::sanitize` keeps or removes them.",
        shape: &[],
    }],
    ret: "The Markdown, as a `tainted` string. A table that Markdown cannot write, such as \
          one with `colspan`, is written as HTML. A link with an unsafe URL is written as its \
          text.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_markdown_from_html" => (nvs_core_markdown_from_html as *const ()).cast(),
        "nvs_core_markdown_to_html" => (nvs_core_markdown_to_html as *const ()).cast(),
        "nvs_core_markdown_to_html_each" => {
            (each::nvs_core_markdown_to_html_each as *const ()).cast()
        }
        GROUPS_ITERATE_SYMBOL => (each::nvs_core_markdown_groups_iterate as *const ()).cast(),
        GROUPS_ADVANCE_SYMBOL => (each::nvs_core_markdown_groups_advance as *const ()).cast(),
        GROUPS_CURRENT_SYMBOL => (each::nvs_core_markdown_groups_current as *const ()).cast(),
        "nvs_core_markdown_live" => (live::nvs_core_markdown_live as *const ()).cast(),
        "nvs_core_markdown_live_push" => (live::nvs_core_markdown_live_push as *const ()).cast(),
        "nvs_core_markdown_live_finish" => {
            (live::nvs_core_markdown_live_finish as *const ()).cast()
        }
        "nvs_core_markdown_update_done" => {
            (live::nvs_core_markdown_update_done as *const ()).cast()
        }
        "nvs_core_markdown_update_open" => {
            (live::nvs_core_markdown_update_open as *const ()).cast()
        }
        _ => return None,
    })
}

/// How much output a document may add beyond its own length:
/// `rule:core-classes/markdown-cost`'s 64 KiB.
///
/// Added output is a reference's URL and title written at each use, and an
/// empty cell that pads a short table row. Past the budget a reference is
/// written as its source text and a row is not padded.
pub(crate) const ADDED_OUTPUT_SLACK: usize = 64 << 10;

/// The GFM extensions the engine reads. Its blockquote tags, heading
/// attributes, math and metadata blocks are not on, because
/// `rule:core-classes/markdown-to-html` reads one flavour.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

/// `markdown` as the engine reads it: without a leading byte-order mark, NUL
/// as U+FFFD, and each CR LF or lone CR as LF.
///
/// The text is borrowed when there is nothing to change, which is almost
/// every document.
fn prepared(markdown: &str) -> std::borrow::Cow<'_, str> {
    let markdown = markdown.strip_prefix('\u{FEFF}').unwrap_or(markdown);
    if !markdown.contains(['\0', '\r']) {
        return std::borrow::Cow::Borrowed(markdown);
    }
    let mut out = String::with_capacity(markdown.len());
    let mut chars = markdown.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\0' => out.push('\u{FFFD}'),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('\n');
            }
            _ => out.push(ch),
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Whether `url` may be written into a link or an image.
///
/// `rule:core-classes/markdown-to-html`'s one URL check, for a URL from the
/// document and one a callback returns. The engine has already decoded the
/// entities. This decodes the percent-encoding as well, and the URL passes
/// only when both readings pass the sanitizer's own [`addressable`]: its
/// scheme list, its scheme-relative rule and its removal of tab, line feed
/// and carriage return.
pub(crate) fn url_allowed(url: &str) -> bool {
    addressable(url) && (!url.contains('%') || addressable(&percent_decoded(url)))
}

/// `url` with each `%` and two hex digits replaced by the byte they name,
/// read as UTF-8 with every invalid sequence as U+FFFD.
fn percent_decoded(url: &str) -> String {
    let bytes = url.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = |b: u8| char::from(b).to_digit(16);
        if bytes[at] == b'%'
            && let (Some(&high), Some(&low)) = (bytes.get(at + 1), bytes.get(at + 2))
            && let (Some(high), Some(low)) = (hex(high), hex(low))
        {
            out.push(u8::try_from(high * 16 + low).expect("two hex digits are one byte"));
            at += 3;
            continue;
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `url` percent-encoded the way CommonMark's reference renderer writes an
/// `href`: every byte outside its safe set as `%` and two hex digits.
///
/// `&` and `'` stay, because the serializer escapes `&` and writes every
/// attribute value between double quotes.
fn encoded_href(url: &str) -> String {
    let mut out = String::with_capacity(url.len());
    for &byte in url.as_bytes() {
        let safe = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'!' | b'#'
                    | b'$'
                    | b'%'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b'-'
                    | b'.'
                    | b'/'
                    | b':'
                    | b';'
                    | b'='
                    | b'?'
                    | b'@'
                    | b'_'
                    | b'~'
            );
        if safe {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Whether a link of `link_type` writes a URL and title that its own bytes
/// did not, which is the reference half of added output.
fn is_reference(link_type: LinkType) -> bool {
    matches!(
        link_type,
        LinkType::Reference
            | LinkType::ReferenceUnknown
            | LinkType::Collapsed
            | LinkType::CollapsedUnknown
            | LinkType::Shortcut
            | LinkType::ShortcutUnknown
    )
}

/// The bytes one padded table cell writes: `<td></td>` and a line feed.
const PADDED_CELL: usize = "<td></td>\n".len();

/// An image whose `alt` text is still being read.
struct Image {
    /// Images nested inside its description, plus one for itself.
    depth: usize,
    /// The `src` to write, or `None` when [`url_allowed`] refused it.
    src: Option<String>,
    title: String,
    alt: String,
}

/// The table being written.
struct Table {
    alignments: Vec<Alignment>,
    /// Whether the cells being read are the header's.
    head: bool,
    /// Whether `<tbody>` is open.
    body: bool,
    /// The index of the next cell in its row.
    cell: usize,
}

/// The event walk, building the output tree.
///
/// `stack[0]` is the document. Every start event pushes a node and its end
/// event pops it into its parent. A [`Kind::Document`] node is a link that is
/// written as its text: its children join the parent.
///
/// It borrows the text being read and the callbacks, and everything it builds
/// is its [`State`], which it reaches through `Deref`.
struct Renderer<'a, 'h> {
    text: &'a str,
    hooks: &'h mut dyn Callbacks,
    state: State,
}

impl std::ops::Deref for Renderer<'_, '_> {
    type Target = State;

    fn deref(&self) -> &State {
        &self.state
    }
}

impl std::ops::DerefMut for Renderer<'_, '_> {
    fn deref_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

/// What a [`Renderer`] has built, which borrows nothing.
struct State {
    /// What the first callback that failed raised. No callback runs after it,
    /// and [`tree`] stops reading.
    fault: Option<Fault>,
    /// The `code` results, and the tokens written in their place.
    trusted: Trusted,
    /// The language of the fenced block being read, when the `code`
    /// callback runs for it.
    language: Option<String>,
    stack: Vec<Parsed>,
    /// The deepest the stack has been, to compare with [`OPEN_ELEMENTS_CAP`].
    deepest: usize,
    /// Whether the tree holds a [`RAW_SOURCE`] node.
    raw: bool,
    /// Whether the last thing written ends a line, which is what cmark's
    /// writer tests before it starts a block.
    line_start: bool,
    /// The added output the document may still write.
    budget: usize,
    /// End events to skip: those of a reference written as its source text
    /// and of a padded cell past the budget.
    skipping: usize,
    image: Option<Image>,
    table: Option<Table>,
    /// Each definition's list item by label, the first definition winning.
    definitions: HashMap<String, Parsed>,
    /// The labels of the definitions being read, innermost last.
    defining: Vec<(String, bool)>,
    /// Each referenced label's number, and the labels in that order.
    numbers: HashMap<String, usize>,
    order: Vec<String>,
    /// Text read since the last other event, which [`autolink::spans`]
    /// reads as one run.
    pending: String,
    /// The links open around the text being read.
    links: usize,
    /// Whether the text being read is a code block's.
    code: bool,
    /// How many of the engine's start events are open. A block group can end
    /// only where it is `0`.
    depth: usize,
    /// The raw HTML of the group being read, outside footnote definitions.
    balance: groups::Balance,
    /// Whether a footnote definition holds raw HTML.
    notes_raw: bool,
    /// The deepest the footnote section will be, counted where each
    /// definition is read: one level deeper than there, under its `ol`.
    notes_deepest: usize,
}

impl State {
    /// An empty document that may add `budget` bytes of output.
    fn new(budget: usize) -> Self {
        Self {
            fault: None,
            trusted: Trusted::default(),
            language: None,
            stack: vec![Parsed::new(Kind::Document)],
            deepest: 1,
            raw: false,
            line_start: true,
            budget,
            skipping: 0,
            image: None,
            table: None,
            definitions: HashMap::new(),
            defining: Vec::new(),
            numbers: HashMap::new(),
            order: Vec::new(),
            pending: String::new(),
            links: 0,
            code: false,
            depth: 0,
            balance: groups::Balance::default(),
            notes_raw: false,
            notes_deepest: 0,
        }
    }
}

impl<'a, 'h> Renderer<'a, 'h> {
    /// A renderer over the whole document `text`.
    fn new(text: &'a str, hooks: &'h mut dyn Callbacks) -> Self {
        let budget = text.len().saturating_add(ADDED_OUTPUT_SLACK);
        Self {
            text,
            hooks,
            state: State::new(budget),
        }
    }

    /// What `call` returns from the callbacks, or [`Answer::Default`] once a
    /// callback has failed, keeping that fault.
    fn ask(&mut self, call: impl FnOnce(&mut dyn Callbacks) -> Result<Answer, Fault>) -> Answer {
        if self.fault.is_some() {
            return Answer::Default;
        }
        call(&mut *self.hooks).unwrap_or_else(|fault| {
            self.fault = Some(fault);
            Answer::Default
        })
    }

    /// The URL to write for a link to `url`, after the `link` callback and
    /// [`url_allowed`], or `None` when the link is written as its text.
    fn link_url(&mut self, url: String) -> Option<String> {
        let url = match self.ask(|hooks| hooks.link(&url)) {
            Answer::Default => url,
            Answer::Null => return None,
            Answer::Value(changed) => changed,
        };
        url_allowed(&url).then(|| encoded_href(&url))
    }

    /// The pending text written, with each extended autolink in it as a link.
    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.pending);
        let mut done = 0;
        for span in autolink::spans(&text) {
            self.write(&text[done..span.range.start]);
            let shown = &text[span.range.clone()];
            if let Some(href) = self.link_url(span.url) {
                self.open("a", vec![("href".to_owned(), href)]);
                self.write(shown);
                self.close();
            } else {
                self.write(shown);
            }
            done = span.range.end;
        }
        self.write(&text[done..]);
        self.pending = text;
        self.pending.clear();
    }

    fn top(&mut self) -> &mut Parsed {
        self.stack
            .last_mut()
            .expect("the document node is never popped")
    }

    /// An element named `name` opened with `attributes`.
    fn open(&mut self, name: &str, attributes: Vec<(String, String)>) {
        let mut node = Parsed::new(Kind::Element);
        node.name.push_str(name);
        node.attributes = attributes;
        self.stack.push(node);
        self.deepest = self.deepest.max(self.stack.len());
        if !self.defining.is_empty() {
            self.notes_deepest = self.notes_deepest.max(self.stack.len() + 1);
        }
        self.line_start = false;
    }

    /// A node that writes no tag of its own, whose children join its parent.
    fn open_transparent(&mut self) {
        self.stack.push(Parsed::new(Kind::Document));
    }

    /// The innermost open node closed into its parent.
    fn close(&mut self) {
        let node = self.stack.pop().expect("an end event follows its start");
        if node.kind == Kind::Document {
            self.top().children.extend(node.children);
        } else {
            self.top().children.push(node);
            self.line_start = false;
        }
    }

    /// An element with no content, such as `br`, `hr` or `img`.
    fn void(&mut self, name: &str, attributes: Vec<(String, String)>) {
        self.open(name, attributes);
        self.close();
    }

    /// `text` appended as character data.
    fn write(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.line_start = text.ends_with('\n');
        let top = self.top();
        if let Some(last) = top.children.last_mut()
            && last.kind == Kind::Text
            && last.name.is_empty()
        {
            last.text.push_str(text);
            return;
        }
        let mut node = Parsed::new(Kind::Text);
        node.text.push_str(text);
        top.children.push(node);
    }

    /// `html` appended as source, which forces the rebuild.
    fn write_raw(&mut self, html: &str) {
        if self.defining.is_empty() {
            self.raw = true;
            self.balance.feed(html);
        } else {
            self.notes_raw = true;
        }
        self.line_start = html.ends_with('\n');
        let top = self.top();
        if let Some(last) = top.children.last_mut()
            && last.kind == Kind::Text
            && last.name == RAW_SOURCE
        {
            last.text.push_str(html);
            return;
        }
        let mut node = Parsed::new(Kind::Text);
        node.name.push_str(RAW_SOURCE);
        node.text.push_str(html);
        top.children.push(node);
    }

    /// A line feed, unless the output is empty or already starts a line.
    fn cr(&mut self) {
        if !self.line_start {
            self.write("\n");
        }
    }

    /// Whether `cost` bytes of added output still fit, spending them if so.
    fn spend(&mut self, cost: usize) -> bool {
        if cost > self.budget {
            return false;
        }
        self.budget -= cost;
        true
    }

    fn event(&mut self, event: Event<'_>, range: Range<usize>) {
        if !matches!(event, Event::Text(_)) {
            self.flush();
        }
        match event {
            Event::Start(_) => self.depth += 1,
            Event::End(_) => self.depth -= 1,
            _ => {}
        }
        if !matches!(event, Event::Html(_) | Event::InlineHtml(_)) {
            self.balance.interrupt();
        }
        if self.skipping > 0 {
            match event {
                Event::Start(_) => self.skipping += 1,
                Event::End(_) => self.skipping -= 1,
                _ => {}
            }
            return;
        }
        if self.image.is_some() {
            self.alt_event(event);
            return;
        }
        match event {
            Event::Start(tag) => self.start(tag, range),
            Event::End(tag) => self.end(tag),
            Event::Text(text) if self.code || self.links > 0 => self.write(&text),
            Event::Text(text) => self.pending.push_str(&text),
            Event::Code(text) => {
                self.open("code", Vec::new());
                self.write(&text);
                self.close();
            }
            Event::Html(html) | Event::InlineHtml(html) => self.write_raw(&html),
            Event::InlineMath(text) | Event::DisplayMath(text) => self.write(&text),
            Event::FootnoteReference(label) => self.reference(&label),
            Event::SoftBreak => self.write("\n"),
            Event::HardBreak => {
                self.void("br", Vec::new());
                self.write("\n");
            }
            Event::Rule => {
                self.cr();
                self.void("hr", Vec::new());
                self.write("\n");
            }
            Event::TaskListMarker(checked) => {
                let mut attributes = Vec::new();
                if checked {
                    attributes.push(("checked".to_owned(), String::new()));
                }
                attributes.push(("disabled".to_owned(), String::new()));
                attributes.push(("type".to_owned(), "checkbox".to_owned()));
                self.void("input", attributes);
                self.write(" ");
            }
        }
    }

    /// An event inside an image's description, which is read as its `alt`.
    fn alt_event(&mut self, event: Event<'_>) {
        let image = self.image.as_mut().expect("the caller checked");
        match event {
            Event::Start(Tag::Image { .. }) => image.depth += 1,
            Event::End(TagEnd::Image) => {
                image.depth -= 1;
                if image.depth == 0 {
                    let image = self.image.take().expect("it is open");
                    if let Some(src) = image.src {
                        let mut attributes =
                            vec![("src".to_owned(), src), ("alt".to_owned(), image.alt)];
                        if !image.title.is_empty() {
                            attributes.push(("title".to_owned(), image.title));
                        }
                        self.void("img", attributes);
                    }
                }
            }
            // Raw HTML in a description is its source text in the `alt`, as
            // cmark writes it: the attribute is text, so no tag is made.
            Event::Text(text)
            | Event::Code(text)
            | Event::InlineMath(text)
            | Event::InlineHtml(text) => {
                image.alt.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak => image.alt.push('\n'),
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>, range: Range<usize>) {
        match tag {
            Tag::Paragraph => {
                self.cr();
                self.open("p", Vec::new());
            }
            Tag::Heading { level, .. } => {
                self.cr();
                self.open(&format!("h{}", level as usize), Vec::new());
            }
            Tag::BlockQuote(_) => {
                self.cr();
                self.open("blockquote", Vec::new());
                self.write("\n");
            }
            Tag::CodeBlock(kind) => {
                self.cr();
                self.open("pre", Vec::new());
                let mut attributes = Vec::new();
                if let CodeBlockKind::Fenced(info) = kind {
                    let word = info.split([' ', '\t']).next().unwrap_or("");
                    let language = word.strip_prefix("language-").unwrap_or(word);
                    let class = format!("language-{language}");
                    if language_class(&class) {
                        attributes.push(("class".to_owned(), class));
                    }
                    if !language.is_empty() && self.hooks.wants_code() {
                        self.language = Some(language.to_owned());
                    }
                }
                self.open("code", attributes);
                self.code = true;
            }
            Tag::HtmlBlock => self.cr(),
            Tag::List(start) => {
                self.cr();
                match start {
                    Some(1) => self.open("ol", Vec::new()),
                    Some(n) => self.open("ol", vec![("start".to_owned(), n.to_string())]),
                    None => self.open("ul", Vec::new()),
                }
                self.write("\n");
            }
            Tag::Item => {
                self.cr();
                self.open("li", Vec::new());
            }
            Tag::FootnoteDefinition(label) => {
                let line_start = self.line_start;
                self.defining.push((label.into_string(), line_start));
                self.open("li", Vec::new());
                self.line_start = false;
            }
            Tag::Table(alignments) => {
                self.cr();
                self.open("table", Vec::new());
                self.write("\n");
                self.table = Some(Table {
                    alignments,
                    head: false,
                    body: false,
                    cell: 0,
                });
            }
            Tag::TableHead => {
                self.open("thead", Vec::new());
                self.write("\n");
                self.open("tr", Vec::new());
                self.write("\n");
                if let Some(table) = self.table.as_mut() {
                    table.head = true;
                    table.cell = 0;
                }
            }
            Tag::TableRow => {
                if let Some(table) = self.table.as_mut() {
                    table.head = false;
                    table.cell = 0;
                    if !table.body {
                        table.body = true;
                        self.open("tbody", Vec::new());
                        self.write("\n");
                    }
                }
                self.open("tr", Vec::new());
                self.write("\n");
            }
            Tag::TableCell => self.cell(range),
            Tag::Emphasis => self.open("em", Vec::new()),
            Tag::Strong => self.open("strong", Vec::new()),
            Tag::Strikethrough => self.open("del", Vec::new()),
            Tag::Superscript => self.open("sup", Vec::new()),
            Tag::Subscript => self.open("sub", Vec::new()),
            Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            } => {
                if is_reference(link_type) && !self.spend(dest_url.len() + title.len()) {
                    self.as_source(range);
                    return;
                }
                self.links += 1;
                let url = if link_type == LinkType::Email {
                    format!("mailto:{dest_url}")
                } else {
                    dest_url.into_string()
                };
                if let Some(href) = self.link_url(url) {
                    let mut attributes = vec![("href".to_owned(), href)];
                    if !title.is_empty() {
                        attributes.push(("title".to_owned(), title.into_string()));
                    }
                    self.open("a", attributes);
                } else {
                    self.open_transparent();
                }
            }
            Tag::Image {
                link_type,
                dest_url,
                title,
                ..
            } => {
                if is_reference(link_type) && !self.spend(dest_url.len() + title.len()) {
                    self.as_source(range);
                    return;
                }
                let url = match self.ask(|hooks| hooks.image(&dest_url)) {
                    Answer::Default => Some(dest_url.into_string()),
                    Answer::Null => None,
                    Answer::Value(changed) => Some(changed),
                };
                self.image = Some(Image {
                    depth: 1,
                    src: url
                        .filter(|url| url_allowed(url))
                        .map(|url| encoded_href(&url)),
                    title: title.into_string(),
                    alt: String::new(),
                });
            }
            // Not enabled by [`options`], so the engine never starts one. The
            // node keeps the walk balanced if it ever did.
            Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::MetadataBlock(_) => self.open_transparent(),
        }
    }

    /// The source text of `range` written as text, and the events inside it
    /// skipped up to its end.
    fn as_source(&mut self, range: Range<usize>) {
        let text = self.text.get(range).unwrap_or("");
        self.write(text);
        self.skipping = 1;
    }

    /// A table cell, which is padding when its range is empty.
    fn cell(&mut self, range: Range<usize>) {
        let Some(table) = self.table.as_ref() else {
            self.open_transparent();
            return;
        };
        let name = if table.head { "th" } else { "td" };
        let align = match table.alignments.get(table.cell) {
            Some(Alignment::Left) => Some("left"),
            Some(Alignment::Center) => Some("center"),
            Some(Alignment::Right) => Some("right"),
            _ => None,
        };
        if range.is_empty() && !table.head && !self.spend(PADDED_CELL) {
            self.skipping = 1;
            return;
        }
        let attributes = align
            .map(|align| vec![("align".to_owned(), align.to_owned())])
            .unwrap_or_default();
        self.open(name, attributes);
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(level) => {
                if self.hooks.wants_heading() {
                    let mut text = String::new();
                    text_of(self.top(), &mut text);
                    if let Answer::Value(id) =
                        self.ask(|hooks| hooks.heading(level as usize, &text))
                    {
                        let id = format!("md-{id}");
                        if markdown_id(&id) {
                            self.top().attributes.push(("id".to_owned(), id));
                        }
                    }
                }
                self.close();
                self.write("\n");
            }
            TagEnd::Paragraph | TagEnd::Item | TagEnd::TableRow => {
                self.close();
                self.write("\n");
            }
            TagEnd::BlockQuote(_) | TagEnd::List(_) => {
                self.cr();
                self.close();
                self.write("\n");
            }
            TagEnd::CodeBlock => {
                self.code = false;
                // Every line of a code block ends with a line feed, the last
                // one too when the document ends without one.
                let unended = self.top().children.last().is_some_and(|last| {
                    last.kind == Kind::Text && last.name.is_empty() && !last.text.ends_with('\n')
                });
                if unended {
                    self.write("\n");
                }
                let replaced = match self.language.take() {
                    Some(language) => {
                        let mut code = String::new();
                        text_of(self.top(), &mut code);
                        match self.ask(|hooks| hooks.code(&language, &code)) {
                            Answer::Value(html) => Some(html),
                            Answer::Default | Answer::Null => None,
                        }
                    }
                    None => None,
                };
                if let Some(html) = replaced {
                    self.stack.pop();
                    self.stack.pop();
                    let token = self.trusted.token(html);
                    let mut node = Parsed::new(Kind::Text);
                    node.text.push_str(&token);
                    self.top().children.push(node);
                    self.line_start = false;
                } else {
                    self.close();
                    self.close();
                }
                self.write("\n");
            }
            TagEnd::HtmlBlock => self.cr(),
            TagEnd::FootnoteDefinition => {
                let node = self.stack.pop().expect("an end event follows its start");
                let (label, line_start) = self.defining.pop().expect("a definition is open");
                self.line_start = line_start;
                self.definitions.entry(label).or_insert(node);
            }
            TagEnd::TableHead => {
                self.close();
                self.write("\n");
                self.close();
                self.write("\n");
            }
            TagEnd::TableCell => {
                self.close();
                self.write("\n");
                if let Some(table) = self.table.as_mut() {
                    table.cell += 1;
                }
            }
            TagEnd::Table => {
                if self.table.take().is_some_and(|table| table.body) {
                    self.close();
                    self.write("\n");
                }
                self.close();
                self.write("\n");
            }
            TagEnd::Link => {
                self.links -= 1;
                self.close();
            }
            TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition
            | TagEnd::MetadataBlock(_) => self.close(),
            // An image's end is read by [`Self::alt_event`].
            TagEnd::Image => {}
        }
    }

    /// A footnote reference, numbered the first time its label is seen.
    fn reference(&mut self, label: &str) {
        let (number, first) = match self.numbers.get(label) {
            Some(&number) => (number, false),
            None => {
                self.order.push(label.to_owned());
                let number = self.order.len();
                self.numbers.insert(label.to_owned(), number);
                (number, true)
            }
        };
        self.open("sup", Vec::new());
        let mut attributes = vec![("href".to_owned(), format!("#md-fn-{number}"))];
        if first {
            attributes.push(("id".to_owned(), format!("md-fnref-{number}")));
        }
        self.open("a", attributes);
        self.write(&number.to_string());
        self.close();
        self.close();
    }

    /// Whether the output so far is a block group that can be written alone
    /// (`rule:core-classes/markdown-chunks`): every block it opened is
    /// closed, it is not inside a footnote definition, its raw HTML leaves
    /// nothing open, and it has written something.
    fn group_ends(&self) -> bool {
        self.depth == 0
            && self.stack.len() == 1
            && self.skipping == 0
            && self.image.is_none()
            && self.defining.is_empty()
            && self.balance.closed()
            && !self.stack[0].children.is_empty()
    }

    /// The group [`Self::group_ends`] found, taken out of the tree: its
    /// nodes, whether it must be rebuilt, and the `code` results it names.
    fn take_group(&mut self) -> (Parsed, bool, Trusted) {
        let tree = std::mem::replace(&mut self.stack[0], Parsed::new(Kind::Document));
        let rebuild = self.raw || self.deepest + 2 >= OPEN_ELEMENTS_CAP;
        self.raw = false;
        self.deepest = 1;
        (tree, rebuild, std::mem::take(&mut self.trusted))
    }

    /// The tree with every open block closed and no footnotes, for text that
    /// has not ended yet: whether it must be rebuilt, and the `code` results
    /// its tokens name.
    fn unfinished(mut self) -> (Parsed, bool, Trusted) {
        self.flush();
        while self.stack.len() > 1 {
            self.close();
        }
        let rebuild = self.raw || self.deepest + 2 >= OPEN_ELEMENTS_CAP;
        let tree = self.stack.pop().expect("the document node");
        (tree, rebuild, std::mem::take(&mut self.trusted))
    }

    /// The tree with its footnotes written after it, whether it must be
    /// rebuilt before it is written, and the `code` results its tokens name.
    fn finish(mut self) -> (Parsed, bool, Trusted) {
        self.flush();
        while self.stack.len() > 1 {
            self.close();
        }
        let order = std::mem::take(&mut self.order);
        let mut notes: Vec<(usize, Parsed)> = order
            .iter()
            .enumerate()
            .filter_map(|(at, label)| self.definitions.remove(label).map(|item| (at + 1, item)))
            .collect();
        let has_notes = !notes.is_empty();
        if has_notes {
            self.cr();
            self.void("hr", Vec::new());
            self.write("\n");
            self.open("ol", Vec::new());
            self.write("\n");
            for (number, item) in &mut notes {
                item.attributes
                    .push(("id".to_owned(), format!("md-fn-{number}")));
                let mut back = Parsed::new(Kind::Element);
                back.name.push('a');
                back.attributes
                    .push(("href".to_owned(), format!("#md-fnref-{number}")));
                let mut arrow = Parsed::new(Kind::Text);
                arrow.text.push('\u{21A9}');
                back.children.push(arrow);
                let mut space = Parsed::new(Kind::Text);
                space.text.push(' ');
                let into = match item.children.last_mut() {
                    Some(last) if last.kind == Kind::Element && last.name == "p" => last,
                    _ => item,
                };
                into.children.push(space);
                into.children.push(back);
            }
            for (_, item) in notes {
                self.top().children.push(item);
                self.line_start = false;
                self.write("\n");
            }
            self.close();
            self.write("\n");
            // The list sits two levels below the document, and each item's
            // own content below that.
            self.deepest = self.deepest.max(3).max(self.notes_deepest);
        }
        let deep = self.deepest + 2 >= OPEN_ELEMENTS_CAP;
        let raw = self.raw || (self.notes_raw && has_notes);
        let tree = self.stack.pop().expect("the document node");
        (tree, raw || deep, std::mem::take(&mut self.trusted))
    }
}

/// The character data under `node`, appended to `out`. A [`RAW_SOURCE`]
/// node is markup, so it adds nothing.
fn text_of(node: &Parsed, out: &mut String) {
    for child in &node.children {
        match child.kind {
            Kind::Text if child.name.is_empty() => out.push_str(&child.text),
            Kind::Element | Kind::Document => text_of(child, out),
            _ => {}
        }
    }
}

/// `markdown`'s output tree, and whether it must be rebuilt.
#[cfg(test)]
fn tree(markdown: &str) -> (Parsed, bool) {
    let (tree, rebuild, _) = tree_with(markdown, &mut NoCallbacks).expect("no callbacks");
    (tree, rebuild)
}

/// `markdown`'s output tree under `hooks`, whether it must be rebuilt, and
/// the `code` results it names.
fn tree_with(markdown: &str, hooks: &mut dyn Callbacks) -> Result<(Parsed, bool, Trusted), Fault> {
    let text = prepared(markdown);
    let mut renderer = Renderer::new(&text, hooks);
    for (event, range) in Parser::new_ext(&text, options()).into_offset_iter() {
        renderer.event(event, range);
        if let Some(fault) = renderer.fault.take() {
            return Err(fault);
        }
    }
    Ok(renderer.finish())
}

/// `markdown` rendered to HTML source, `member` naming the caller in the
/// fault a document past the memory limit raises.
///
/// # Errors
///
/// A [`Fault::Fatal`] when a document with raw HTML parses to a tree larger
/// than the request's memory limit allows.
pub(crate) fn rendered(markdown: &str, member: &str) -> Result<String, Fault> {
    rendered_with(markdown, member, &mut NoCallbacks)
}

/// [`rendered`] under `hooks`.
///
/// # Errors
///
/// [`rendered`]'s, and the fault of the first callback that fails.
pub(crate) fn rendered_with(
    markdown: &str,
    member: &str,
    hooks: &mut dyn Callbacks,
) -> Result<String, Fault> {
    let (tree, rebuild, trusted) = tree_with(markdown, hooks)?;
    let out = source(&tree);
    if !rebuild {
        return Ok(trusted.spliced(out));
    }
    drop(tree);
    Ok(trusted.spliced(source(&rebuilt(&bounded_in_page(&out, member)?))))
}

nvs_runtime::nvs_helper! {
    /// `Core\Markdown::toHtml(tainted string $markdown, ?callable $link,
    /// ?callable $image, ?callable $heading, ?callable $code):
    /// Core\Html\Markup` — `rule:core-classes/markdown-to-html`'s renderer
    /// over a whole document.
    ///
    /// The answer is a carrier for `Core\Html::sanitize`'s reason
    /// (`rule:security/launderer-answers-a-carrier`), and the parse never
    /// fails: Markdown has no invalid input. A callback that throws ends the
    /// render with its fault.
    fn nvs_core_markdown_to_html(ctx, args: [5]) {
        // unreachable from source: the parameter is `CoreTy::Text`.
        let Some(markdown) = args[0].as_text() else {
            return Err(Fault::fatal(format!(
                "Core\\Markdown::toHtml expected a `string`, got tag {}",
                args[0].tag_byte()
            )));
        };
        let mut hooks = Callables {
            ctx,
            link: given(args[1]),
            image: given(args[2]),
            heading: given(args[3]),
            code: given(args[4]),
        };
        let out = rendered_with(markdown, r"Core\Markdown::toHtml", &mut hooks)?;
        Ok(crate::instance::build(&MARKUP, [Value::text(&out)]))
    }
}

/// `html`, an HTML document, as `Core\Markdown::fromHtml` writes it.
///
/// # Errors
///
/// The memory limit's `FATAL`.
pub(crate) fn from_html_text(html: &str) -> Result<String, Fault> {
    from_html::markdown(bounded(html, from_html::MEMBER)?)
}

nvs_runtime::nvs_helper! {
    /// `Core\Markdown::fromHtml(string|Core\Html\Markup|Core\Xml\Node $html):
    /// tainted string` — `rule:core-classes/markdown-from-html`'s converter.
    ///
    /// A string or a `Markup` is parsed by the WHATWG algorithm, and a node is
    /// copied out of the tree it is in. [`from_html::markdown`] does the rest.
    fn nvs_core_markdown_from_html(_ctx, args: [1]) {
        let tree = if let Some(html) = args[0].as_text() {
            bounded(html, from_html::MEMBER)?
        } else if crate::instance::is_instance(args[0], &MARKUP) {
            let held = crate::html::markup_slot(args[0], from_html::MEMBER)?;
            let html = held.as_text().ok_or_else(|| {
                Fault::fatal(format!(
                    "{}: a `Core\\Html\\Markup` held tag {}",
                    from_html::MEMBER,
                    held.tag_byte()
                ))
            })?;
            bounded(html, from_html::MEMBER)?
        } else {
            crate::xml::parsed_of(args[0], from_html::MEMBER)?
        };
        let out = from_html::markdown(tree)?;
        Ok(Value::text(&out))
    }
}

/// `markdown` written one block group at a time and read `piece` bytes at a
/// time, as `Core\Markdown::toHtmlEach` writes a file.
///
/// # Errors
///
/// Bytes that are not UTF-8, and the memory limit's `FATAL`.
pub(crate) fn grouped_in(markdown: &[u8], piece: usize) -> Result<Vec<String>, Fault> {
    segments::grouped_in(
        markdown,
        piece,
        r"Core\Markdown::toHtmlEach",
        &mut NoCallbacks,
    )
}

/// `markdown` pushed into a `Core\Markdown\Live` `piece` bytes at a time,
/// each piece moved on to a character boundary: the `done` and `open` parts
/// of each push, and what `finish()` returns.
///
/// # Errors
///
/// The memory limit's `FATAL`.
pub(crate) fn live_in(
    markdown: &str,
    piece: usize,
) -> Result<(Vec<(String, String)>, String), Fault> {
    let mut text = live::Text::new();
    let mut parts = Vec::new();
    let mut at = 0;
    while at < markdown.len() {
        let mut end = at.saturating_add(piece.max(1)).min(markdown.len());
        while !markdown.is_char_boundary(end) {
            end += 1;
        }
        parts.push(text.push(&markdown[at..end], &mut NoCallbacks)?);
        at = end;
    }
    let rest = text.finish(&mut NoCallbacks)?;
    Ok((parts, rest))
}

#[cfg(test)]
mod tests;
