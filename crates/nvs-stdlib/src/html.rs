//! `Core\Html` — [ADR 0024](../../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)
//! § 3's narrow, sink-named launderer, over the sink § 5 makes out of HTML
//! text.
//!
//! § 3 writes `Core\Html::escape(tainted string): string` out as *the* worked
//! example of what a launderer is allowed to be: one member, one sink, and a
//! contract naming it. This module is that member. There is deliberately no
//! `sanitize()` here yet and no generic `clean()` ever — a value safe for HTML
//! text is not safe for a shell argument, and § 3's whole argument is that a
//! catch-all buys the false confidence the qualifier exists to prevent.
//!
//! # Known gaps
//!
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) § 3 gives
//! this class three more things than it has: `sanitize`, the `Markup` value
//! type § 5 makes the sink's only raw-write bypass, and
//! [ADR 0122](../../../../docs/adr/0122-html-parsing-is-a-whatwg-entry-on-core-html-over-core-xmls-tree.md)'s
//! WHATWG parser over `Core\Xml`'s tree. The parser waits on that tree
//! existing at all. `Core\Html\Markup` waits on nothing in this module: the
//! checker already refuses a `tainted` or `secret` conversion to it
//! (`nvs_types::expr::quals`) and the runtime already renders it as a sink
//! carrier (`nvs_runtime::is_carrier`), so what it owes is the registered
//! class those two already speak for.
//!
//! # Why the escape set is fixed at five, with no argument
//!
//! `escape` writes `&`, `<`, `>`, `"` and `'` as references, always. PHP
//! spells the same operation as `htmlspecialchars($s, $flags, $encoding,
//! $double)` — four arguments, of which the first is a bitmask whose default
//! left `'` unescaped until PHP 8.1 and produced a decade of attribute-context
//! XSS. [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R6
//! forbids the bitmask outright, and the safe member of every pair the flags
//! chose between is the only one worth having: escaping both quote characters
//! makes the answer safe in an unquoted-attribute position as well as in text,
//! and escaping one fewer character has never been the reason a page was fast.
//! `$encoding` has no analogue because a `string` is UTF-8 by
//! [ADR 0009](../../../../docs/adr/0009-string-and-bytes.md), and `$double`
//! has none because "do not escape what already looks escaped" is exactly the
//! repair [ADR 0095](../../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md)
//! refuses: `&amp;` in the input is text that said `&amp;`, and it comes back
//! as `&amp;amp;`.
//!
//! `'` becomes `&#39;` rather than `&apos;` because the named reference is
//! XML's and HTML 4 does not define it; the numeric one is understood by every
//! parser that has ever existed.
//!
//! # Why the bidi row is here and not in `nvs_render`
//!
//! ADR 0024 § 5's last bullet puts an unterminated bidirectional control on
//! this member: escaping the five characters says nothing about *display
//! order*, so a payload that reverses the rendering of the text after it
//! survives the escape untouched.
//! [ADR 0087](../../../../docs/adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)
//! owns the predicate, and this is its third caller — the lexer refuses a
//! source span, the terminal sink substitutes, and this sink substitutes too.
//! What it is *not* is [`nvs_render::text::substitute`]: that function is
//! ADR 0086 § 1's terminal table, which also turns every C0 byte into a
//! Control Picture. A newline is legitimate HTML text, and rewriting it as `␊`
//! would corrupt every escaped document, so this member calls the bidi
//! predicate directly and leaves the C0 rows to the sink that wants them.
//!
//! A *balanced* control passes through: mixed-direction text is what those
//! code points are for, and ADR 0087's whole position is that banning them
//! breaks Arabic and Hebrew.

use nvs_runtime::{Fault, NvsStr, Tag, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc, Qual};

/// ADR 0024 § 3's launderer for the HTML sink.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Html",
    methods: &[CoreMethod {
        name: "escape",
        names: &["text"],
        params: &[CoreTy::Text(Qual::Launder)],
        defaults: &[],
        return_ty: CoreTy::Str,
        symbol: "nvs_core_html_escape",
        doc: Some(&ESCAPE_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Html::escape`'s reference card — ADR 0117.
const ESCAPE_DOC: MethodDoc = MethodDoc {
    short: "Writes `&`, `<`, `>`, `\"` and `'` in `$text` as character references, and replaces \
            every unterminated bidirectional control with `\u{FFFD}` — the launderer for the HTML \
            sink, so its result is accepted where a `tainted` string is not.",
    params: &[ParamDoc {
        name: "text",
        desc: "The text to write into an HTML document, as text rather than as markup.",
        shape: &[],
    }],
    ret: "The escaped text, safe in element content and in an attribute value quoted either way. \
          Text with none of the five characters and no unterminated control comes back unchanged. \
          The five are escaped unconditionally: there is no flag, and an input that already reads \
          as a reference is escaped again, since `&amp;` in the input is text that said `&amp;`.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_html_escape" => (nvs_core_html_escape as *const ()).cast(),
        _ => return None,
    })
}

/// What each of the five characters is written as.
///
/// A `match` rather than a table because it is the whole of the transformation
/// and the compiler turns it into one: five arms over an ASCII byte.
fn escaped(c: char) -> Option<&'static str> {
    match c {
        // First, and the only one whose escape is not about a delimiter: an
        // unescaped `&` makes every other reference in the output ambiguous.
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '"' => Some("&quot;"),
        // Not `&apos;` — that name is XML's and HTML 4 never defined it.
        '\'' => Some("&#39;"),
        _ => None,
    }
}

/// A `string` argument, or the fault a non-`string` tag produces.
///
/// The tag check is ADR 0009's UTF-8 guarantee itself: `bytes` is its own tag
/// over the same allocation and reaches `None` here, which is what keeps a
/// binary payload out of a text format.
fn text<'a>(value: &'a Value, position: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        // Unreachable from source: the row declares one `CoreTy::Text`
        // parameter, so a `bytes` argument — the only other tag over this
        // allocation — is `E0401` (*expected `string`, found `bytes`*) at the
        // call and never reaches this body. The check stays because the ABI
        // is `*const Value` and nothing in it carries the row's promise.
        Fault::fatal(format!(
            "Core\\Html::escape expected {:?} for {position}, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Html::escape(tainted string $text): string` — ADR 0024 § 3's
    /// launderer for the sink § 5 describes, replacing `htmlspecialchars`.
    ///
    /// Removing the qualifier is the *registry row's* job, not this body's:
    /// `tainted` has no run-time representation at all, so what the checker
    /// reads is [`Qual::Launder`] on the parameter and `CoreTy::Str` on the
    /// answer. What runs here is the transformation that makes that judgement
    /// true.
    ///
    /// # Why the unchanged case answers the argument itself
    ///
    /// ADR 0024 § 5 makes this the sink's *only* behaviour: every non-`Markup`
    /// interpolation into an HTML response passes through here, whether or not
    /// it is tainted. So the input with nothing to escape is not an edge case,
    /// it is most of a page — and answering the argument keeps that path at
    /// one scan and no allocation, which is what makes a rule that cannot be
    /// switched off affordable (AGENTS.md's priority 3).
    /// [`nvs_render::text::substitute`] answers a borrow for the same reason
    /// one sink over.
    fn nvs_core_html_escape(_ctx, args: [1]) {
        let text = text(&args[0], "the text")?;

        // Both halves are a scan and neither fires on ordinary text, so they
        // are asked before anything is allocated.
        let mut unterminated = Vec::new();
        nvs_render::bidi::for_each_unterminated(text, |offset, _| unterminated.push(offset));
        if unterminated.is_empty() && !text.chars().any(|c| escaped(c).is_some()) {
            #[expect(
                unsafe_code,
                reason = "the argument slot holds a live reference for the length of \
                          the call, which is `Value::retain`'s whole obligation"
            )]
            unsafe {
                args[0].retain();
            }
            return Ok(args[0]);
        }

        // Every escape is longer than what it replaces, so the input's length
        // is a floor and never a wasted reservation.
        let mut out = String::with_capacity(text.len());
        let mut cuts = unterminated.into_iter().peekable();
        for (offset, c) in text.char_indices() {
            if cuts.peek() == Some(&offset) {
                cuts.next();
                out.push(REPLACEMENT);
                continue;
            }
            match escaped(c) {
                Some(reference) => out.push_str(reference),
                None => out.push(c),
            }
        }
        Ok(Value::str(NvsStr::new(out.as_bytes())))
    }
}

/// What an unterminated directional control becomes — ADR 0024 § 5's last
/// bullet, which writes the character out.
///
/// The same replacement `nvs_render::text` uses for it, and deliberately not a
/// character reference: the control is being *removed*, not shown, and
/// `&#8235;` in the output would be an escaped payload rather than a neutral
/// one.
const REPLACEMENT: char = '\u{FFFD}';

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::CLASSES;

    /// ADR 0024 § 3, asked of the registry rather than of the body: `escape`
    /// launders, its answer is unqualified, and the qualifier it removes is
    /// `tainted` and not the other axis.
    ///
    /// The **and for no other** half is what makes this more than a row read
    /// back. § 3's rule is that a launderer is narrow and sink-named, so the
    /// claim under test is a *set*: exactly one member of the whole registry
    /// is spelled for the HTML sink, and it is this one. A second `Core\Html`
    /// row that also laundered — a `sanitize` written as a launderer, which is
    /// what § 3 spends a paragraph refusing — fails here on the day it is
    /// added, which is the day the decision to widen the sink's escape hatch
    /// is actually being made.
    #[test]
    fn html_escape_launders_for_the_html_sink_and_for_no_other() {
        let escape = CLASS
            .members()
            .find(|method| method.name == "escape")
            .expect("`Core\\Html::escape` is registered");

        // The mark on the parameter is what admits a `tainted` argument, and
        // the unqualified return type is what stops carrying it — the two
        // together are the whole of "this member launders".
        assert!(
            matches!(escape.params, [CoreTy::Text(Qual::Launder)]),
            "`escape` takes one `Qual::Launder` text parameter, not {}",
            escape.params.len()
        );
        assert!(
            matches!(escape.return_ty, CoreTy::Str),
            "`escape` answers a plain, unqualified `string`"
        );

        // `Reveal` is the other axis and a different decision: ADR 0060 § 5's
        // "a verified signature does not launder" is the same shape one axis
        // over, and `Qual`'s own docs close that roster at two classes. A
        // launderer for HTML that also revealed a `secret` would be laundering
        // confidentiality on the strength of an escaping argument.
        assert!(
            !escape
                .params
                .iter()
                .any(|param| matches!(param, CoreTy::Text(Qual::Reveal))),
            "escaping neutralizes injection, not confidentiality"
        );

        // The set claim. Every launderer in the registry names its own sink in
        // its own doc comment; this asserts that exactly one of them is the
        // HTML sink's, so `Core\Html` gaining a second one is a failure here
        // rather than a quiet widening.
        let html_launderers: Vec<&'static str> = CLASSES
            .iter()
            .filter(|class| class.name == r"Core\Html")
            .flat_map(CoreClass::members)
            .filter(|method| {
                method
                    .params
                    .iter()
                    .any(|param| matches!(param, CoreTy::Text(Qual::Launder)))
            })
            .map(|method| method.name)
            .collect();
        assert_eq!(
            html_launderers,
            vec!["escape"],
            "ADR 0024 § 3's launderer for the HTML sink is one member and is named for it"
        );
    }

    /// The transformation, over the boundary each of the five characters sits
    /// on: a character reference is produced for every one of them and for
    /// nothing else in ASCII.
    ///
    /// Counted rather than read off five lines, so a member that escaped a
    /// sixth character — `/`, which several PHP escapers add — fails here.
    #[test]
    fn exactly_five_ascii_characters_are_escaped() {
        let escaped_set: Vec<char> = (0u8..128)
            .map(char::from)
            .filter(|c| escaped(*c).is_some())
            .collect();
        assert_eq!(escaped_set, vec!['"', '&', '\'', '<', '>']);
    }
}
