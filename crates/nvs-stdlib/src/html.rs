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
//! this class two more things than it has: `sanitize` and
//! [ADR 0122](../../../../docs/adr/0122-html-parsing-is-a-whatwg-entry-on-core-html-over-core-xmls-tree.md)'s
//! WHATWG parser over `Core\Xml`'s tree, both of which wait on that tree
//! existing at all.
//!
//! [`MARKUP`] is registered *and* reachable: two of ADR 0024 § 5's three ways
//! to obtain one are here, as [`MARKUP_SYMBOL`] for `as Markup` on a source
//! literal and [`MARKUP_CONCAT_SYMBOL`] for `Markup + Markup`. The third is
//! the sink's own **escape-and-lift** — every non-`Markup` interpolation into
//! an HTML response is escaped through `escape` and wrapped — and it waits on
//! that response existing, which is the same wait `Core\Request` is on.
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

/// `Core\Html\Markup`'s fully-qualified name, taken from the runtime constant
/// that decides which classes the HTML sink renders raw.
///
/// Written this way rather than spelled again, for
/// [`nvs_runtime::CARRIER_CLI_TEXT`]'s reason one carrier over: the class a
/// program writes and the class [`nvs_runtime::value_to_string`] renders
/// cannot drift apart if there is only one string.
pub const MARKUP_NAME: &str = nvs_runtime::CARRIER_HTML_MARKUP;

/// ADR 0024 § 5's `Core\Html\Markup` — the HTML sink's only raw-write bypass.
///
/// **Memberless, and that is the design rather than an unfinished roster.**
/// § 5 gives three ways to obtain one and every one of them is a language
/// construct: `as Markup` on a *source literal*, which is the trust level the
/// literal already carried; `Markup + Markup`, which composes two trusted
/// fragments; and the sink's own escape-and-lift of everything else, which
/// runs [`CLASS`]'s `escape` and wraps the answer. A constructor member would
/// be a fourth, and it would take a runtime `string` — which is exactly the
/// bypass § 5's first bullet closes ("compute the escape-defeating payload at
/// runtime, then cast it"). `Core\Cli\Text::plain` is the same shape one sink
/// over and *does* have that member, because its argument is laundered on the
/// way in; nothing here can launder markup, since raw markup is the whole
/// point of the type.
///
/// So the class exists to be **named**, the way
/// [`crate::script::HANDLE`] does: it gives the checker's target a registered
/// layout, and its one slot is where the trusted bytes live.
/// [`nvs_runtime::CARRIER_TEXT_SLOT`] is that index, shared with the other
/// carrier so `value_to_string` renders either without asking this crate
/// anything.
pub(crate) const MARKUP: CoreClass = CoreClass {
    name: MARKUP_NAME,
    methods: &[],
    instance: &[],
    slots: &["text"],
    constants: &[],
};

/// The symbol `<literal> as Core\Html\Markup` lowers to — ADR 0024 § 5's lift
/// of a trusted source literal into [`MARKUP`].
///
/// No [`CoreMethod`] row, for the same reason [`MARKUP`] has no members at
/// all: a member taking a `string` is precisely the runtime-computed bypass
/// § 5 closes, so the only thing allowed to call this is the lowering of the
/// construct that spells it, and a row would make it callable by name.
/// `crate::script`'s two symbols are the same arrangement one construct over,
/// and its module doc is the home of why a symbol without a row is a shape
/// rather than an oversight.
///
/// `nvs-ir` reaches it through `nvs_types`, which is the only edge there is:
/// `nvs-runtime` owns every `nvs_ir::Helper` symbol and cannot reach this
/// crate's layout for a `Core` class, so the lift is a `CoreCall` rather than
/// a helper row.
pub const MARKUP_SYMBOL: &str = "nvs_core_html_markup";

/// The symbol `Markup + Markup` lowers to — ADR 0024 § 5's composition rule,
/// which is the second and last way a program obtains a [`MARKUP`].
///
/// Row-less for [`MARKUP_SYMBOL`]'s reason and by the same argument: `+` is
/// the spelling § 5 gives composition, so the operator's own lowering is the
/// only thing allowed to reach this, and a member row would be a third way in
/// that took its operands from anywhere.
pub const MARKUP_CONCAT_SYMBOL: &str = "nvs_core_html_markup_concat";

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
        MARKUP_SYMBOL => (nvs_core_html_markup as *const ()).cast(),
        MARKUP_CONCAT_SYMBOL => (nvs_core_html_markup_concat as *const ()).cast(),
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

/// A `string`, or the fault a non-`string` tag produces. `subject` names what
/// was expected to be text and heads the message.
///
/// The tag check is ADR 0009's UTF-8 guarantee itself: `bytes` is its own tag
/// over the same allocation and reaches `None` here, which is what keeps a
/// binary payload out of a text format.
fn text<'a>(value: &'a Value, subject: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        // Unreachable from source: `escape`'s row declares one `CoreTy::Text`
        // parameter, so a `bytes` argument — the only other tag over this
        // allocation — is `E0401` (*expected `string`, found `bytes`*) at the
        // call and never reaches this body, and a carrier's slot holds what
        // this module put there. The check stays because the ABI is
        // `*const Value` and nothing in it carries either promise.
        Fault::fatal(format!(
            "{subject} expected {:?}, got tag {}",
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
        let text = text(&args[0], r"`Core\Html::escape`'s `$text`")?;

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

nvs_runtime::nvs_helper! {
    /// `"<b>" as Core\Html\Markup` — ADR 0024 § 5's lift, and the whole of
    /// what [`MARKUP_SYMBOL`] does.
    ///
    /// The *trust* decision is not here and cannot be: `nvs_types::expr::quals`
    /// has already refused a `tainted` operand, a `secret` one and anything
    /// computed (`E0417`), so by the time this runs the argument is a source
    /// literal the author wrote and the only thing left is to put it in the
    /// carrier's one slot. A body that re-asked the question would be asking
    /// it of a value that no longer remembers where it came from, which is
    /// exactly why § 5's rule is a compile-time one.
    ///
    /// **What it spends:** one object allocation per lift, which is
    /// [`crate::instance`]'s cost and charged to the request like every other
    /// `Core` instance. The literal's bytes are not copied — the slot holds
    /// one more reference to the same [`NvsStr`].
    fn nvs_core_html_markup(_ctx, args: [1]) {
        // Unreachable from source for `text`'s own reason: `nvs-ir` emits this
        // over a `Ty::Str` the checker proved is a literal. The check stays
        // because the ABI is `*const Value`, and the slot it fills is the one
        // `nvs_runtime::value_to_string` writes out raw.
        text(&args[0], r"the literal lifted by `as Core\Html\Markup`")?;

        // A `CoreCall`'s arguments are borrowed and `instance::build` takes
        // over each slot's reference, so the reference the carrier ends up
        // holding is taken here rather than handed over by the caller.
        #[expect(
            unsafe_code,
            reason = "the argument slot holds a live reference for the length of \
                      the call, which is `Value::retain`'s whole obligation"
        )]
        unsafe {
            args[0].retain();
        }
        Ok(crate::instance::build(&MARKUP, [args[0]]))
    }
}

/// One `Core\Html\Markup` operand's trusted bytes — slot
/// [`nvs_runtime::CARRIER_TEXT_SLOT`], **borrowed**, exactly as
/// [`crate::instance::slot`] hands it over.
///
/// Returned as a [`Value`] rather than as a `&str` because the borrow has to
/// outlive this call: the slot's own `Value` is what owns the reference the
/// text is read through.
fn markup_slot(value: Value, position: &str) -> Result<Value, Fault> {
    let object = crate::instance::receiver(value, &MARKUP, position)?;
    Ok(crate::instance::slot(
        object,
        nvs_runtime::CARRIER_TEXT_SLOT,
    ))
}

nvs_runtime::nvs_helper! {
    /// `$a + $b` over two `Core\Html\Markup` — ADR 0024 § 5's composition
    /// rule, and the whole of what [`MARKUP_CONCAT_SYMBOL`] does.
    ///
    /// **Nothing is checked and nothing is escaped**, which is the rule rather
    /// than an omission: § 5 grants composition precisely because both
    /// fragments already passed whichever rule made them `Markup`, so
    /// re-escaping either here would corrupt the markup it was lifted for.
    /// The pair is the operator table's own — `nvs_types::expr::operators`
    /// admits `Markup + Markup` and refuses every other object beside `+` — so
    /// the only judgement left is the one the tag check below makes.
    ///
    /// **What it spends:** one string allocation and one object allocation per
    /// composition, both charged to the request. Neither operand is touched: a
    /// `Markup` is a value type, so `$a + $b` leaves both of them where they
    /// were, and a chain of `n` fragments is `n - 1` of these.
    fn nvs_core_html_markup_concat(_ctx, args: [2]) {
        let left = markup_slot(args[0], "the left operand")?;
        let right = markup_slot(args[1], "the right operand")?;
        let left = text(&left, "the left operand of `Markup + Markup`")?;
        let right = text(&right, "the right operand of `Markup + Markup`")?;

        let mut out = String::with_capacity(left.len() + right.len());
        out.push_str(left);
        out.push_str(right);
        Ok(crate::instance::build(
            &MARKUP,
            [Value::str(NvsStr::new(out.as_bytes()))],
        ))
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

    /// ADR 0024 § 5's carrier, in the two facts neither crate that acts on it
    /// can check for itself: `nvs_runtime::CARRIER_TEXT_SLOT` is the index
    /// this class's registered layout gives `text`, and this class's name is
    /// one `nvs_runtime` renders raw. `Core\Cli\Text` asserts the same pair
    /// one sink over, which is what makes a carrier a *set* rather than a
    /// special case.
    ///
    /// The third assertion is § 5's own shape: the class is **memberless**,
    /// because every way of obtaining a `Markup` is a language construct and a
    /// constructor member would be a fourth that took a runtime string. The
    /// const's doc comment is the home of that argument; this fails on the day
    /// a member is added to it, which is the day the bypass is being widened.
    #[test]
    fn the_markup_carrier_is_named_slotted_and_memberless() {
        assert_eq!(MARKUP.slot("text"), nvs_runtime::CARRIER_TEXT_SLOT);
        assert!(nvs_runtime::is_carrier(MARKUP_NAME));
        assert!(
            crate::registry::class_renders(MARKUP_NAME),
            "the HTML sink writes a `Markup` out as the bytes it already holds"
        );
        assert_eq!(
            MARKUP.members().count(),
            0,
            "a `Markup` is obtained by `as` on a literal, by `+`, or by the sink's own \
             escape-and-lift — never by a call"
        );
    }

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
