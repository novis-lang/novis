//! The CSS of an HTML mail, copied onto the elements it styles, and `Core\Mail\Preview`, which
//! shows the result without sending it.
//!
//! Most mail programs drop a `<style>` element, or keep only part of it, and none of them loads a
//! stylesheet. So every HTML body `send` and `preview` take is inlined, with no option: [`inline`]
//! copies each rule of each `<style>` element into the `style` attribute of every element it
//! matches, by the cascade. A more specific selector wins, a later rule breaks a tie, `!important`
//! wins over a normal declaration, and a `style` attribute the author wrote wins over a normal
//! rule. An inlined rule is then removed from its `<style>` element. An option to turn this off
//! is declined until a real case needs one: it would be one more thing to forget, and inlining
//! HTML that is already inlined changes nothing.
//!
//! What stays in the `<style>` element it was written in: at-rules such as `@media` and
//! `@font-face`, rules with a pseudo-class or a pseudo-element such as `:hover`, and **every rule
//! that matched nothing**. A rule aimed at an attribute a mail program adds when it shows the
//! message, such as Outlook's `[data-ogsc]` or Gmail's `u + .body`, matches nothing when the
//! message is sent and must reach the reader. A `<style>` element left with no rule is removed.
//!
//! A `<link rel="stylesheet">` throws a `RuntimeError` that names the fix. Nothing is fetched or
//! read. That error is where a developer who tries the obvious thing learns that inlining exists.
//!
//! # The engine
//!
//! `css-inline`, with every default feature off, so it reads nothing from the network or the disk
//! and keeps nothing between calls. It parses with `html5ever` and matches with Servo's
//! `cssparser` and `selectors`. A matcher of our own is declined: a specificity mistake would be
//! silent.
//!
//! It is handed the HTML [`crate::html::source`] writes from the parse [`super::parts::read_html`]
//! already made, never the program's text. That parse caps the stack of open elements, so the
//! document `css-inline` parses again is as deep as that tree and no deeper. Its serializer
//! recurses once per level, so a tree deeper than [`DEPTH`] throws before it is handed over. A
//! body with no `<style>` element is not handed over at all, and is sent as written.
//!
//! **What it spends** (`rule:programs/memory-priority`): for an HTML body with a `<style>`
//! element, one more parse and one more copy of the HTML, held for the length of the send. A body
//! without one costs nothing more than stage 4's parse. `preview` parses the inlined HTML once
//! more to list the rules left in `<style>`.

use nvs_runtime::{Fault, NvsArray, Value};

use super::parts::Html;
use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc, TextOrigin};
use crate::xml::Kind;

/// The deepest element nesting an HTML body with a `<style>` element may have. `css-inline`
/// writes its result with one native frame per level, and no mail a person writes is half this
/// deep.
pub(super) const DEPTH: usize = 256;

/// The sentence a stylesheet `<link>` throws, after the member's name.
pub(super) const LINK_FIX: &str = "A mail cannot load a stylesheet. Put the CSS in a `<style>` \
                                   element. `Core\\Mail` copies each rule into the elements it \
                                   matches.";

/// The HTML a `Core\Html\Markup` body is sent as: `html` itself when it has no `<style>`
/// element, and otherwise the HTML with each rule copied onto the elements it matches.
///
/// # Errors
///
/// A catchable `RuntimeError` for a stylesheet `<link>`, for a tree deeper than [`DEPTH`], and
/// for CSS the engine cannot read.
pub(super) fn inline(html: &str, read: &Html, member: &str) -> Result<String, Fault> {
    if read.linked {
        return Err(Fault::thrown(format!("{member}: {LINK_FIX}")));
    }
    if !read.styled {
        return Ok(html.to_owned());
    }
    if read.depth > DEPTH {
        return Err(Fault::thrown(format!(
            "{member}: the HTML body is {} elements deep. A body with a `<style>` element can be \
             at most {DEPTH} elements deep.",
            read.depth
        )));
    }
    let written = crate::html::source(&read.tree);
    css_inline::CSSInliner::options()
        .load_remote_stylesheets(false)
        .keep_link_tags(true)
        .remove_inlined_selectors(true)
        .build()
        .inline(&written)
        .map_err(|error| {
            Fault::thrown(format!(
                "{member}: the CSS in the HTML body cannot be read: {error}"
            ))
        })
}

/// The rules an inlined HTML body kept in its `<style>` elements, and those of them that matched
/// nothing.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Report {
    /// Every rule left in a `<style>` element, in document order.
    pub kept: Vec<String>,
    /// The kept style rules with no pseudo-class and no pseudo-element. Each could have matched
    /// an element and matched none.
    pub unused: Vec<String>,
}

/// The [`Report`] of `html`, which [`inline`] has written.
pub(super) fn report(html: &str) -> Report {
    let tree = crate::html::parse(html);
    let mut report = Report::default();
    let mut stack = vec![&tree];
    while let Some(node) = stack.pop() {
        if node.kind == Kind::Element && node.name == "style" {
            let css: String = node
                .children
                .iter()
                .map(|child| child.text.as_str())
                .collect();
            for rule in rules(&css) {
                if unused(&rule) {
                    report.unused.push(rule.clone());
                }
                report.kept.push(rule);
            }
        }
        stack.extend(node.children.iter().rev());
    }
    report
}

/// The top-level rules of a style sheet, each trimmed: a block rule to its closing `}`, and a
/// statement such as `@import` to its `;`. Strings and comments are skipped as a CSS reader does.
fn rules(css: &str) -> Vec<String> {
    let bytes = css.as_bytes();
    let mut found = Vec::new();
    let (mut start, mut depth, mut at) = (0, 0_usize, 0);
    while at < bytes.len() {
        match bytes[at] {
            quote @ (b'"' | b'\'') => {
                at += 1;
                while at < bytes.len() && bytes[at] != quote {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
            }
            b'/' if bytes.get(at + 1) == Some(&b'*') => {
                at = css[at + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |end| at + 2 + end + 1);
            }
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    push_rule(&css[start..=at], &mut found);
                    start = at + 1;
                }
            }
            b';' if depth == 0 => {
                push_rule(&css[start..=at], &mut found);
                start = at + 1;
            }
            _ => {}
        }
        at += 1;
    }
    if let Some(rest) = css.get(start..) {
        push_rule(rest, &mut found);
    }
    found
}

/// `rule`, trimmed, onto `found` when anything is left.
fn push_rule(rule: &str, found: &mut Vec<String>) {
    let rule = rule.trim();
    if !rule.is_empty() {
        found.push(rule.to_owned());
    }
}

/// Whether a kept rule could have matched an element: a style rule whose selectors have no `:`
/// outside brackets and strings, so no pseudo-class and no pseudo-element.
fn unused(rule: &str) -> bool {
    if rule.starts_with('@') {
        return false;
    }
    let selectors = rule.split('{').next().unwrap_or(rule);
    let mut nested = 0_usize;
    let mut quote = None;
    for byte in selectors.bytes() {
        match (quote, byte) {
            (Some(open), _) if byte == open => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b'[' | b'(') => nested += 1,
            (None, b']' | b')') => nested = nested.saturating_sub(1),
            (None, b':') if nested == 0 => return false,
            _ => {}
        }
    }
    true
}

/// The name of the class `preview` returns.
pub(crate) const PREVIEW_NAME: &str = r"Core\Mail\Preview";

/// Slot 0: the HTML as `send` sends it, or `null` for a `string` body.
const HTML_SLOT: usize = 0;
/// Slot 1: the plain text.
const TEXT_SLOT: usize = 1;
/// Slot 2: [`Report::kept`], an `array<string>`.
const KEPT_SLOT: usize = 2;
/// Slot 3: [`Report::unused`], an `array<string>`.
const UNUSED_SLOT: usize = 3;

/// The texts of a preview are the program's own: its `Core\Html\Markup`, CSS it wrote, and a
/// `string` body or `text` option the row only takes untainted.
const PREVIEW_TEXT: TextOrigin =
    TextOrigin::Clean("The text is made from the program's own body, which is never `tainted`.");

/// One reader with no parameter.
const fn reader(
    name: &'static str,
    return_ty: CoreTy,
    symbol: &'static str,
    doc: &'static MethodDoc,
) -> CoreMethod {
    CoreMethod {
        name,
        names: &[],
        params: &[],
        defaults: &[],
        return_ty,
        text: PREVIEW_TEXT.on(&return_ty),
        symbol,
        doc: Some(doc),
    }
}

/// `Core\Mail\Preview` — the body of a message as `send` would send it.
///
/// A value with four readers and no constructor a program can call: `Core\Mail::preview` builds
/// it through the same function `send` reads its body with.
// rank: 883
pub(crate) const PREVIEW: CoreClass = CoreClass {
    name: PREVIEW_NAME,
    doc: Some(&PREVIEW_CARD),
    methods: &[],
    instance: &[
        reader(
            "html",
            CoreTy::Nullable(&CoreTy::Instance(crate::html::MARKUP_NAME)),
            "nvs_core_mail_preview_html",
            &HTML_DOC,
        ),
        reader("text", CoreTy::Str, "nvs_core_mail_preview_text", &TEXT_DOC),
        reader(
            "keptCss",
            CoreTy::Array(&CoreTy::Str),
            "nvs_core_mail_preview_kept_css",
            &KEPT_DOC,
        ),
        reader(
            "unusedCss",
            CoreTy::Array(&CoreTy::Str),
            "nvs_core_mail_preview_unused_css",
            &UNUSED_DOC,
        ),
    ],
    slots: &["html", "text", "keptCss", "unusedCss"],
    constants: &[],
};

/// The class card — `rule:core-api/reference-card`.
const PREVIEW_CARD: ClassDoc = ClassDoc {
    short: "The body of a message, exactly as `Core\\Mail::send` would send it. \
            `Core\\Mail::preview` returns it.",
};

/// `html()`'s card.
const HTML_DOC: MethodDoc = MethodDoc {
    short: "Returns the HTML that would be sent. Each CSS rule of a `<style>` element is copied \
            into the `style` attribute of the elements it matches.",
    params: &[],
    ret: "The HTML, or `null` when the body is a `string`.",
    errors: &[],
};

/// `text()`'s card.
const TEXT_DOC: MethodDoc = MethodDoc {
    short: "Returns the plain text that would be sent.",
    params: &[],
    ret: "A `string` body as it is, the `text` option, or the plain text made from the HTML.",
    errors: &[],
};

/// `keptCss()`'s card.
const KEPT_DOC: MethodDoc = MethodDoc {
    short: "Returns the CSS rules that stay in a `<style>` element. These are rules such as \
            `@media` and `a:hover`, and rules that match no element.",
    params: &[],
    ret: "One string for each rule, in the order of the HTML. The list is empty when every rule \
          was copied into the elements.",
    errors: &[],
};

/// `unusedCss()`'s card.
const UNUSED_DOC: MethodDoc = MethodDoc {
    short: "Returns the CSS rules that match no element of the HTML. A rule for an attribute that \
            a mail program adds later is in this list, and it still works there.",
    params: &[],
    ret: "One string for each rule, in the order of the HTML. Each is also in `keptCss()`.",
    errors: &[],
};

/// A `Core\Mail\Preview` of `html`, `text` and `html`'s [`Report`].
pub(super) fn preview(html: Option<&str>, text: &str) -> Value {
    let report = html.map(report).unwrap_or_default();
    let list = |rules: &[String]| {
        let mut out = NvsArray::new();
        for rule in rules {
            out.append(Value::text(rule));
        }
        Value::array(out)
    };
    crate::instance::build(
        &PREVIEW,
        [
            html.map_or_else(Value::null, |html| {
                crate::instance::build(&crate::html::MARKUP, [Value::text(html)])
            }),
            Value::text(text),
            list(&report.kept),
            list(&report.unused),
        ],
    )
}

nvs_runtime::nvs_helper! {
    /// `$preview->html(): ?Core\Html\Markup`.
    fn nvs_core_mail_preview_html(_ctx, args: [1]) {
        crate::instance::read_slot(args, &PREVIEW, HTML_SLOT, "html")
    }
}

nvs_runtime::nvs_helper! {
    /// `$preview->text(): string`.
    fn nvs_core_mail_preview_text(_ctx, args: [1]) {
        crate::instance::read_slot(args, &PREVIEW, TEXT_SLOT, "text")
    }
}

nvs_runtime::nvs_helper! {
    /// `$preview->keptCss(): array<string>`.
    fn nvs_core_mail_preview_kept_css(_ctx, args: [1]) {
        crate::instance::read_slot(args, &PREVIEW, KEPT_SLOT, "keptCss")
    }
}

nvs_runtime::nvs_helper! {
    /// `$preview->unusedCss(): array<string>`.
    fn nvs_core_mail_preview_unused_css(_ctx, args: [1]) {
        crate::instance::read_slot(args, &PREVIEW, UNUSED_SLOT, "unusedCss")
    }
}

/// Where each reader's helper lives.
pub(super) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    Some(nvs_runtime::Entry::borrowed(match symbol {
        "nvs_core_mail_preview_html" => (nvs_core_mail_preview_html as *const ()).cast(),
        "nvs_core_mail_preview_text" => (nvs_core_mail_preview_text as *const ()).cast(),
        "nvs_core_mail_preview_kept_css" => (nvs_core_mail_preview_kept_css as *const ()).cast(),
        "nvs_core_mail_preview_unused_css" => {
            (nvs_core_mail_preview_unused_css as *const ()).cast()
        }
        _ => return None,
    }))
}

/// The four readers, for the tests of [`super`]: a helper is a private `fn`.
#[cfg(test)]
pub(super) const HELPERS: [nvs_runtime::NvsFn; 4] = [
    nvs_core_mail_preview_html,
    nvs_core_mail_preview_text,
    nvs_core_mail_preview_kept_css,
    nvs_core_mail_preview_unused_css,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail::parts::read_html;

    /// `html` as `send` sends it.
    fn inlined(html: &str) -> String {
        inline(html, &read_html(html), "send").expect("CSS the engine reads")
    }

    /// The opening tag of the first element with `marker` in it, such as `id="a"`.
    fn tag_with<'a>(html: &'a str, marker: &str) -> &'a str {
        let at = html.find(marker).expect("the marker is in the HTML");
        let start = html[..at].rfind('<').expect("an opening tag");
        let end = at + html[at..].find('>').expect("the tag ends");
        &html[start..=end]
    }

    /// A rule is copied into the `style` attribute of each element it matches.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`css-inline` builds selectors with `servo_arc`, which breaks the Stacked Borrows rules Miri checks"
    )]
    #[test]
    fn a_style_rule_is_copied_onto_every_element_it_matches() {
        let html = inlined(
            "<style>p { color: red }</style><p id=\"a\">One</p><p id=\"b\">Two</p><div \
             id=\"c\">Three</div>",
        );
        assert!(tag_with(&html, "id=\"a\"").contains("color: red"), "{html}");
        assert!(tag_with(&html, "id=\"b\"").contains("color: red"), "{html}");
        assert!(!tag_with(&html, "id=\"c\"").contains("style="), "{html}");
    }

    /// The more specific selector wins, and of two equal ones the later rule wins.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`css-inline` builds selectors with `servo_arc`, which breaks the Stacked Borrows rules Miri checks"
    )]
    #[test]
    fn the_more_specific_rule_wins_and_a_later_one_breaks_a_tie() {
        let html = inlined(
            "<style>p { color: red } .note { color: blue } p { color: green }</style>\
             <p id=\"a\" class=\"note\">One</p><p id=\"b\">Two</p>",
        );
        let first = tag_with(&html, "id=\"a\"");
        assert!(first.contains("color: blue"), "{html}");
        assert!(!first.contains("green"), "{html}");
        let second = tag_with(&html, "id=\"b\"");
        assert!(second.contains("color: green"), "{html}");
        assert!(!second.contains("red"), "{html}");
    }

    /// An `!important` declaration wins over a normal one with a more specific selector.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`css-inline` builds selectors with `servo_arc`, which breaks the Stacked Borrows rules Miri checks"
    )]
    #[test]
    fn an_important_rule_beats_a_normal_one() {
        let html = inlined(
            "<style>#a { color: blue } p { color: red !important }</style>\
             <p id=\"a\">One</p>",
        );
        let tag = tag_with(&html, "id=\"a\"");
        assert!(tag.contains("color: red !important"), "{html}");
        assert!(!tag.contains("blue"), "{html}");
    }

    /// A `style` attribute the author wrote wins over a normal rule, and the rule's other
    /// declarations are still added.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`css-inline` builds selectors with `servo_arc`, which breaks the Stacked Borrows rules Miri checks"
    )]
    #[test]
    fn a_style_attribute_the_author_wrote_wins() {
        let html = inlined(
            "<style>p { color: red; margin: 0 }</style><p id=\"a\" style=\"color: \
             blue\">One</p>",
        );
        let tag = tag_with(&html, "id=\"a\"");
        assert!(tag.contains("color: blue"), "{html}");
        assert!(tag.contains("margin: 0"), "{html}");
        assert!(!tag.contains("red"), "{html}");
    }

    /// `@media`, `@font-face`, `:hover` and `::before` stay in a `<style>` element, and none of
    /// them is copied into an element.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`css-inline` builds selectors with `servo_arc`, which breaks the Stacked Borrows rules Miri checks"
    )]
    #[test]
    fn a_media_query_and_a_pseudo_class_stay_in_a_style_element() {
        let html = inlined(
            "<style>@media (max-width: 600px) { p { color: red } }\n\
             @font-face { font-family: Brand; src: url(https://example.com/b.woff2) }\n\
             a:hover { color: green }\np::before { content: \"-\" }\n\
             p { margin: 0 }</style><p id=\"a\"><a id=\"b\" href=\"https://example.com\">Go</a></p>",
        );
        let report = report(&html);
        assert_eq!(report.kept.len(), 4, "{:?}", report.kept);
        assert!(report.kept[0].starts_with("@media"), "{:?}", report.kept);
        assert!(
            report.kept[1].starts_with("@font-face"),
            "{:?}",
            report.kept
        );
        assert!(report.kept[2].starts_with("a:hover"), "{:?}", report.kept);
        assert!(report.kept[3].starts_with("p::before"), "{:?}", report.kept);
        assert!(report.unused.is_empty(), "{:?}", report.unused);
        let paragraph = tag_with(&html, "id=\"a\"");
        assert!(paragraph.contains("margin: 0"), "{html}");
        assert!(!paragraph.contains("color"), "{html}");
        assert!(!tag_with(&html, "id=\"b\"").contains("style="), "{html}");
    }

    /// A rule that matches nothing when the message is sent stays, such as one for an attribute
    /// a mail program adds when it shows the message. It is in both lists.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`css-inline` builds selectors with `servo_arc`, which breaks the Stacked Borrows rules Miri checks"
    )]
    #[test]
    fn a_rule_that_matches_nothing_stays_in_a_style_element() {
        let html = inlined(
            "<style>[data-ogsc] p { color: white }\n.missing { color: red }\n\
             p { margin: 0 }</style><p>One</p>",
        );
        let report = report(&html);
        assert_eq!(report.kept.len(), 2, "{:?}", report.kept);
        assert!(
            report.kept[0].starts_with("[data-ogsc] p"),
            "{:?}",
            report.kept
        );
        assert!(report.kept[1].starts_with(".missing"), "{:?}", report.kept);
        assert_eq!(report.unused, report.kept);
    }

    /// A rule copied into every element it matches is removed, and a `<style>` element with no
    /// rule left is removed with it.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`css-inline` builds selectors with `servo_arc`, which breaks the Stacked Borrows rules Miri checks"
    )]
    #[test]
    fn an_inlined_rule_is_removed_from_its_style_element() {
        let html = inlined("<style>p { color: red }</style><p>One</p>");
        assert!(!html.contains("<style"), "{html}");
        assert!(html.contains("color: red"), "{html}");
        assert_eq!(report(&html), Report::default());
    }

    /// A stylesheet `<link>` throws with the fix, and nothing is fetched. A `<link>` that is not
    /// a stylesheet is sent as written.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`css-inline` builds selectors with `servo_arc`, which breaks the Stacked Borrows rules Miri checks"
    )]
    #[test]
    fn a_stylesheet_link_throws_and_names_the_fix() {
        for html in [
            "<link rel=\"stylesheet\" href=\"https://example.com/mail.css\"><p>One</p>",
            "<html><head><LINK REL=\"Alternate StyleSheet\" href=\"a.css\"></head><p>One</p>",
        ] {
            let error = inline(html, &read_html(html), "send").expect_err("a stylesheet link");
            let Fault::Thrown(_, message) = &error else {
                panic!("a catchable error: {error:?}");
            };
            assert_eq!(message.to_string(), format!("send: {LINK_FIX}"));
        }
        let icon = "<link rel=\"icon\" href=\"a.png\"><style>p { color: red }</style><p>One</p>";
        assert!(inlined(icon).contains("rel=\"icon\""));
    }

    /// Inlining HTML that is already inlined gives the same HTML.
    // covers: Core\Mail::send
    #[cfg_attr(
        miri,
        ignore = "`css-inline` builds selectors with `servo_arc`, which breaks the Stacked Borrows rules Miri checks"
    )]
    #[test]
    fn inlining_twice_gives_the_same_html() {
        let once = inlined(
            "<html><head><style>p { color: red } .x { margin: 0 } a:hover { color: blue }\n\
             @media (max-width: 600px) { p { color: green } } .missing { padding: 0 }\
             </style></head><body><p class=\"x\" style=\"font-weight: bold\">One <a \
             href=\"https://example.com\">link</a></p></body></html>",
        );
        assert_eq!(inlined(&once), once);
    }

    /// The rules of a style sheet split at the top level, with strings and comments skipped.
    #[test]
    fn a_style_sheet_splits_into_its_top_level_rules() {
        assert_eq!(
            rules("@import \"a;b.css\"; /* } */ p { content: \"}\" }\n@media x { a { b: c } }"),
            [
                "@import \"a;b.css\";",
                "/* } */ p { content: \"}\" }",
                "@media x { a { b: c } }"
            ]
        );
        assert!(unused("a[href=\"https:x\"] { b: c }"));
        assert!(!unused("a:hover { b: c }"));
        assert!(!unused("@media x { a { b: c } }"));
    }

    /// A body nested deeper than [`DEPTH`] with a `<style>` element throws, and one without a
    /// `<style>` element is sent as written.
    #[cfg_attr(
        miri,
        ignore = "the parse's cap on open elements is 16 under Miri, so the tree never reaches \
                  the bound"
    )]
    #[test]
    fn a_body_deeper_than_the_bound_with_css_throws() {
        let deep = "<div>".repeat(DEPTH + 1);
        let styled = format!("<style>div {{ margin: 0 }}</style>{deep}");
        assert!(inline(&styled, &read_html(&styled), "send").is_err());
        assert_eq!(inlined(&deep), deep);
    }
}
