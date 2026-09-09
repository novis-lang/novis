//! `Core\Html` — `rule:security/launderers-are-sink-named`'s narrow,
//! sink-named launderer over the sink § 5 makes out of HTML text, and
//! `rule:core-classes/html-parsing`'s WHATWG parser onto `Core\Xml`'s tree.
//!
//! § 3 writes `Core\Html::escape(tainted string): Core\Html\Markup` out as
//! *the* worked example of what a launderer is allowed to be: one member, one
//! sink, and a contract naming it. This module is that member. There is
//! deliberately no `sanitize()` here yet and no generic `clean()` ever — a
//! value safe for HTML text is not safe for a shell argument, and § 3's whole
//! argument is that a catch-all buys the false confidence the qualifier exists
//! to prevent.
//!
//! # Why the answer is a carrier and not a `string`
//!
//! `rule:security/launderer-answers-a-carrier`
//! asks two questions of every launderer and this is the one member on the
//! roster that answers yes to both: the HTML sink launders on its own, so a
//! second application is one the source does not show, and escaping is not
//! idempotent, so that second application changes the output — `&` becomes
//! `&amp;` becomes `&amp;amp;`. Answering [`MARKUP`] is what makes the eager
//! `htmlspecialchars` habit stop compiling instead of shipping `&amp;amp;`,
//! and [`nvs_core_html_to_source`] is § 3's one way back to the bytes. Every
//! other launderer in the registry keeps its plain type, which is the same
//! predicate answering no.
//!
//! # The WHATWG parse, and where its boundary is
//!
//! [`nvs_core_html_parse`] is `rule:core-classes/html-parsing`'s second door
//! onto [`crate::xml`]'s tree, and it **cannot fail**: implied tags, misnested
//! formatting and foster parenting are the algorithm's *specified* output
//! rather than a guess, so there is nothing for
//! `rule:errors/ambiguous-input-refused` to refuse. That is the opposite
//! contract to `Core\Xml::parse` over the same nodes, and the pair is what
//! replaces one object flipping between the two under a flag.
//!
//! The engine is `html5ever` and the tree builder is [`Sink`], ours. The
//! boundary between the two is where the one-node-family clause is enforced:
//! [`crate::xml::Parsed`] and [`Kind`] are the only things [`Sink`] builds, so
//! a construct this door could produce that the other could not would have to
//! be a sixth kind, which there is no way to write. What that costs in
//! placement decisions — the doctype dropped, a `<template>`'s contents kept on
//! the element, a declarative shadow root refused — is on each of [`Sink`]'s
//! own members.
//!
//! # Known gaps
//!
//! `rule:core-api/tier-roster` gives this class one more thing than it has:
//! `sanitize`, the rebuild that answers a [`MARKUP`]. It waits on nothing now
//! that the parse is here, since rebuilding a document means walking the tree
//! this member already answers.
//!
//! `rule:core-classes/html-to-source` asks two things of [`nvs_core_html_to_source`]'s `$reason` and
//! each is enforced in the one place that can answer it. **A computed reason is
//! refused where it is written**, by `nvs_types::reasons` under `E0805` — that
//! is a judgement about the *source*, and there is nothing the body could read
//! to make it. **An empty one is the body's**, below, because emptiness is a
//! property of the text and the checker gains nothing by racing it there; that
//! pass's own module doc is the home of the split. A `const REASON` holding
//! the text still compiles, because § 3's argument is that the hatch be
//! *greppable and justified* rather than inline, which is
//! `Core\Secret::reveal`'s own position on the same question
//! ([`crate::secret`]).
//!
//! [`MARKUP`] is registered *and* reachable: `rule:core-classes/html-auto-escape`'s three ways to
//! obtain one are all here — [`MARKUP_SYMBOL`] for `as Markup` on a source
//! literal, [`MARKUP_CONCAT_SYMBOL`] for `Markup + Markup`, and the escape
//! itself, which `rule:security/launderer-answers-a-carrier` turned from the first two's poor relation into
//! the ordinary one. What still waits is the sink's **automatic** lift — every
//! non-`Markup` interpolation into an HTML response escaped and wrapped with
//! no call written at the site — and it waits on that response existing, which
//! is the same wait `Core\Request` is on. The *predicate* `rule:security/launderer-answers-a-carrier` reads
//! does not wait on it: § 5 already decided the sink launders on its own, and
//! the return type is written against that decision rather than against what
//! is on disk.
//!
//! # Why the escape set is fixed at five, with no argument
//!
//! `escape` writes `&`, `<`, `>`, `"` and `'` as references, always. PHP
//! spells the same operation as `htmlspecialchars($s, $flags, $encoding,
//! $double)` — four arguments, of which the first is a bitmask whose default
//! left `'` unescaped until PHP 8.1 and produced a decade of attribute-context
//! XSS. `rule:core-api/shape-rules` R6
//! forbids the bitmask outright, and the safe member of every pair the flags
//! chose between is the only one worth having: escaping both quote characters
//! makes the answer safe in an unquoted-attribute position as well as in text,
//! and escaping one fewer character has never been the reason a page was fast.
//! `$encoding` has no analogue because a `string` is UTF-8 by
//! `rule:types/bytes`, and `$double`
//! has none because "do not escape what already looks escaped" is exactly the
//! repair `rule:errors/ambiguous-input-refused`
//! refuses: `&amp;` in the input is text that said `&amp;`, and it comes back
//! as `&amp;amp;`.
//!
//! `'` becomes `&#39;` rather than `&apos;` because the named reference is
//! XML's and HTML 4 does not define it; the numeric one is understood by every
//! parser that has ever existed.
//!
//! # Why the bidi row is here and not in `nvs_render`
//!
//! `rule:core-classes/html-auto-escape`'s last bullet puts an unterminated bidirectional control on
//! this member: escaping the five characters says nothing about *display
//! order*, so a payload that reverses the rendering of the text after it
//! survives the escape untouched.
//! `rule:security/bidi-predicate`
//! owns the predicate, and this is its third caller — the lexer refuses a
//! source span, the terminal sink substitutes, and this sink substitutes too.
//! What it is *not* is [`nvs_render::text::substitute`]: that function is
//! `rule:tooling/terminal-output-is-a-sink`'s terminal table, which also turns every C0 byte into a
//! Control Picture. A newline is legitimate HTML text, and rewriting it as `␊`
//! would corrupt every escaped document, so this member calls the bidi
//! predicate directly and leaves the C0 rows to the sink that wants them.
//!
//! A *balanced* control passes through: mixed-direction text is what those
//! code points are for, and `rule:security/bidi-predicate`'s whole position is that banning them
//! breaks Arabic and Hebrew.

use std::borrow::Cow;
use std::cell::{Ref, RefCell};

use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, QualName};
use nvs_runtime::{Fault, NvsStr, Tag, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};
use crate::xml::{DEPTH_CEILING, Kind, Parsed};

/// `rule:security/launderers-are-sink-named`'s launderer for the HTML sink, and `rule:core-classes/html-to-source`'s one way back
/// out of the carrier it answers.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Html",
    methods: &[
        CoreMethod {
            name: "escape",
            names: &["text"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Instance(MARKUP_NAME),
            symbol: "nvs_core_html_escape",
            doc: Some(&ESCAPE_DOC),
        },
        CoreMethod {
            name: "toSource",
            names: &["markup", "reason"],
            params: &[CoreTy::Instance(MARKUP_NAME), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_html_to_source",
            doc: Some(&TO_SOURCE_DOC),
        },
        CoreMethod {
            name: "parse",
            names: &["document"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Instance(crate::xml::NODE_NAME),
            symbol: "nvs_core_html_parse",
            doc: Some(&PARSE_DOC),
        },
    ],
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

/// `rule:core-classes/html-auto-escape`'s `Core\Html\Markup` — the HTML sink's only raw-write bypass.
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

/// The symbol `<literal> as Core\Html\Markup` lowers to — `rule:core-classes/html-auto-escape`'s lift
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

/// The symbol `Markup + Markup` lowers to — `rule:core-classes/html-auto-escape`'s composition rule,
/// which is the second and last way a program obtains a [`MARKUP`].
///
/// Row-less for [`MARKUP_SYMBOL`]'s reason and by the same argument: `+` is
/// the spelling § 5 gives composition, so the operator's own lowering is the
/// only thing allowed to reach this, and a member row would be a third way in
/// that took its operands from anywhere.
pub const MARKUP_CONCAT_SYMBOL: &str = "nvs_core_html_markup_concat";

/// `Core\Html::escape`'s reference card — `rule:core-api/reference-card`.
const ESCAPE_DOC: MethodDoc = MethodDoc {
    short: "Writes `&`, `<`, `>`, `\"` and `'` in `$text` as character references, and replaces \
            every unterminated bidirectional control with `\u{FFFD}` — the launderer for the HTML \
            sink, so its result is accepted where a `tainted` string is not.",
    params: &[ParamDoc {
        name: "text",
        desc: "The text to write into an HTML document, as text rather than as markup.",
        shape: &[],
    }],
    ret: "A `Core\\Html\\Markup` carrying the escaped text, safe in element content and in an \
          attribute value quoted either way. It is not a `string`, which is what stops the sink \
          escaping it a second time; `toSource` is the way back to the bytes. Text with none of \
          the five characters and no unterminated control is carried through unchanged. The five \
          are escaped \
          unconditionally: there is no flag, and an input that already reads as a reference is \
          escaped again, since `&amp;` in the input is text that said `&amp;`.",
    errors: &[],
};

/// `Core\Html::toSource`'s reference card — `rule:core-api/reference-card`.
const TO_SOURCE_DOC: MethodDoc = MethodDoc {
    short: "Hands back the source text a `Core\\Html\\Markup` carries — the one way out of the \
            carrier, since there is no `Markup as string` conversion. Rare, greppable, and it \
            carries a written reason at the site.",
    params: &[
        ParamDoc {
            name: "markup",
            desc: "The markup whose bytes are wanted rather than its guarantee.",
            shape: &[],
        },
        ParamDoc {
            name: "reason",
            desc: "Why this call site needs the text and not the carrier, written for the next \
                   reader. Nothing else reads it, and an empty one is refused.",
            shape: &[],
        },
    ],
    ret: "The markup's source text, as a plain `string`. Caching a rendered fragment, storing one \
          in a column, writing one to a file and handing one to a sink that is not this one are \
          the legitimate callers.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$reason` is empty. A reason nobody had to write is a reason nobody wrote, so the \
               hatch refuses to open without one.",
    }],
};

/// `Core\Html::parse`'s reference card — `rule:core-api/reference-card`.
const PARSE_DOC: MethodDoc = MethodDoc {
    short: "Parses `$document` as HTML by the WHATWG algorithm — the one browsers run — and answers \
            the document node of the same tree `Core\\Xml::parse` builds.",
    params: &[ParamDoc {
        name: "document",
        desc: "The document text. Any text at all is a document: this member has no way to refuse \
               one.",
        shape: &[],
    }],
    ret: "The `Core\\Xml\\Node` of kind `Document` whose children are what the algorithm put at the \
          top level — for all but an empty input, one `html` element with `head` and `body` under \
          it, whether or not the text wrote those tags. Implied tags, misnested tags and \
          stray content are placed exactly where a browser places them, so what comes back is what \
          a page would have rendered rather than a reading of what was written. A `<!DOCTYPE …>` \
          leaves no node, since the tree has no kind for one. Names are lower-cased in the HTML \
          namespace and case-corrected in the SVG and MathML ones, as the algorithm specifies. \
          Nesting deeper than a thousand elements is flattened onto the thousandth rather than \
          held, so no document can make the walk of the answer exhaust the stack.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_html_escape" => (nvs_core_html_escape as *const ()).cast(),
        "nvs_core_html_parse" => (nvs_core_html_parse as *const ()).cast(),
        "nvs_core_html_to_source" => (nvs_core_html_to_source as *const ()).cast(),
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
/// The tag check is `rule:types/bytes`'s UTF-8 guarantee itself: `bytes` is its own tag
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
    /// `Core\Html::escape(tainted string $text): Core\Html\Markup` — `rule:security/launderers-are-sink-named`
    /// 's launderer for the sink § 5 describes, replacing
    /// `htmlspecialchars`.
    ///
    /// Removing the qualifier is the *registry row's* job, not this body's:
    /// `tainted` has no run-time representation at all, so what the checker
    /// reads is [`Qual::Launder`] on the parameter and `CoreTy::Instance` on
    /// the answer. What runs here is the transformation that makes that
    /// judgement true, plus the lift into [`MARKUP`] that
    /// `rule:security/launderer-answers-a-carrier`
    /// requires of it: the HTML sink launders on its own and its transform
    /// is not idempotent, so an answer the sink could not tell from unescaped
    /// text is one it would escape a second time.
    ///
    /// # Why the unchanged case still carries the argument's own bytes
    ///
    /// `rule:core-classes/html-auto-escape` makes this the sink's *only* behaviour: every non-`Markup`
    /// interpolation into an HTML response passes through here, whether or not
    /// it is tainted. So the input with nothing to escape is not an edge case,
    /// it is most of a page — and handing that path's bytes straight to the
    /// carrier keeps it at one scan and no *string* allocation, which is what
    /// makes a rule that cannot be switched off affordable (AGENTS.md's
    /// priority 3). [`nvs_render::text::substitute`] answers a borrow for the
    /// same reason one sink over.
    ///
    /// **What the carrier itself spends:** one object allocation per call,
    /// charged to the request exactly as [`nvs_core_html_markup`]'s lift is.
    /// That is the price `rule:security/launderer-answers-a-carrier` names and it is paid on every escape,
    /// including the unchanged one — the alternative is a `string` answer the
    /// sink escapes again, which costs a second scan *and* a wrong document.
    fn nvs_core_html_escape(_ctx, args: [1]) {
        let text = text(&args[0], r"`Core\Html::escape`'s `$text`")?;

        // Both halves are a scan and neither fires on ordinary text, so they
        // are asked before anything is allocated.
        let mut unterminated = Vec::new();
        nvs_render::bidi::for_each_unterminated(text, |offset, _| unterminated.push(offset));
        if unterminated.is_empty() && !text.chars().any(|c| escaped(c).is_some()) {
            // `instance::build` takes over the slot's reference, and a
            // `CoreCall`'s arguments are borrowed — so the reference the
            // carrier ends up holding is taken here, as the lift does it.
            #[expect(
                unsafe_code,
                reason = "the argument slot holds a live reference for the length of \
                          the call, which is `Value::retain`'s whole obligation"
            )]
            unsafe {
                args[0].retain();
            }
            return Ok(crate::instance::build(&MARKUP, [args[0]]));
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
        Ok(crate::instance::build(
            &MARKUP,
            [Value::str(NvsStr::new(out.as_bytes()))],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `"<b>" as Core\Html\Markup` — `rule:core-classes/html-auto-escape`'s lift, and the whole of
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
    /// `$a + $b` over two `Core\Html\Markup` — `rule:core-classes/html-auto-escape`'s composition
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

nvs_runtime::nvs_helper! {
    /// `Core\Html::toSource(Core\Html\Markup $markup, string $reason): string`
    /// — `rule:core-classes/html-to-source`'s one way back out of the carrier.
    ///
    /// **There is no `Markup as string` conversion, and this is why there is a
    /// member instead.** A cast would reopen the hole in a keystroke —
    /// `Core\Html::escape($x) as string . $tainted` is the bug the carrier
    /// removes — so the way out is greppable by name and carries a written
    /// reason at the site, exactly as `Core\Secret::reveal` does one axis over.
    ///
    /// The reason is otherwise read by nobody: [`crate::secret`]'s module doc
    /// is the home of that argument, and this member takes the same
    /// [`Qual::Neutral`] text for the same reasons — no byte of it reaches the
    /// answer, and a `secret` justification is refused by the ordinary rule.
    /// **An empty one is refused here**, which is the half of § 3's rule that
    /// can be enforced at all today: the other half wants the reason to be a
    /// *source literal*, and that is a compile-time judgement with nowhere to
    /// declare its diagnostic — both type bands are full (`E0499`, `E0799`), so
    /// widening them is a decision of its own rather than a line in this
    /// member. The module's *Known gaps* records it.
    ///
    /// **What it spends:** one comparison. The answer is the slot's own
    /// [`NvsStr`] with one more reference on it, so the bytes are never copied
    /// and the carrier the caller passed is left exactly where it was.
    fn nvs_core_html_to_source(_ctx, args: [2]) {
        let reason = text(&args[1], r"`Core\Html::toSource`'s `$reason`")?;
        if reason.is_empty() {
            return Err(nvs_runtime::Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                "Core\\Html::toSource(): the reason is written for the next \
                 reader, so an empty one is refused"
                    .to_owned(),
            ));
        }

        let source = markup_slot(args[0], r"`Core\Html::toSource`'s `$markup`")?;

        // Handed back to Novis code, so the reference is this call's to take —
        // `crate::instance::slot` borrows and says so.
        #[expect(
            unsafe_code,
            reason = "the slot's `Value` is owned by a receiver that is live for \
                      the length of the call, which is `Value::retain`'s whole \
                      obligation"
        )]
        unsafe {
            source.retain();
        }
        Ok(source)
    }
}

/// What an unterminated directional control becomes — `rule:core-classes/html-auto-escape`'s last
/// bullet, which writes the character out.
///
/// The same replacement `nvs_render::text` uses for it, and deliberately not a
/// character reference: the control is being *removed*, not shown, and
/// `&#8235;` in the output would be an escaped payload rather than a neutral
/// one.
const REPLACEMENT: char = '\u{FFFD}';

// ============================================================================
// The WHATWG parse
// ============================================================================

/// One node of the arena [`Sink`] fills, before the whole of it becomes a
/// [`Parsed`] tree.
///
/// A flat `Vec` with indices for edges rather than a tree of owned nodes,
/// because the tree builder *moves* nodes between parents — the adoption agency
/// algorithm, which is most of what HTML's error recovery is — and reparenting
/// a subtree here is an index write rather than a walk.
#[derive(Debug)]
struct Node {
    /// Which of [`Kind`]'s five this is.
    kind: Kind,
    /// An element's qualified name, kept whole because [`TreeSink::elem_name`]
    /// hands it back to the tree builder, which reads the namespace as well as
    /// the local name to decide what foreign content means.
    qual: Option<QualName>,
    /// The name the [`Parsed`] node carries: an element's tag or a processing
    /// instruction's target, with the prefix the document wrote.
    name: String,
    /// The character data this node carries itself.
    text: String,
    /// An element's attributes, in written order.
    attributes: Vec<(String, String)>,
    /// This node's children, in document order.
    children: Vec<usize>,
    /// What this node hangs off, or `None` for the document and for a node the
    /// builder has detached.
    parent: Option<usize>,
    /// Whether this is a MathML `annotation-xml` acting as an HTML integration
    /// point, which is a property of the attributes it was created with and
    /// decides how content nested inside it is parsed.
    integration_point: bool,
}

impl Node {
    /// An empty node of `kind`.
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            qual: None,
            name: String::new(),
            text: String::new(),
            attributes: Vec::new(),
            children: Vec::new(),
            parent: None,
            integration_point: false,
        }
    }

    /// A text node carrying `text`.
    fn text(text: &str) -> Self {
        let mut node = Self::new(Kind::Text);
        node.text.push_str(text);
        node
    }
}

/// The tree builder `html5ever` drives — the whole of the boundary between that
/// crate and Novis's own nodes.
///
/// `markup5ever_rcdom` is the sample DOM this replaces, and not taking it is
/// what `rule:core-classes/html-parsing`'s one-node-family clause needs: a
/// parse that built someone else's DOM and walked the result into [`Parsed`]
/// would hold the document twice and put a second node family in this crate for
/// the length of every parse, which is the drift the clause exists to prevent.
/// Nothing below decides what a node *is* — [`Kind`] and [`Parsed`] are
/// [`crate::xml`]'s, and a construct HTML could produce that XML could not would
/// have to be a sixth kind, which there is no way to write.
#[derive(Debug)]
struct Sink {
    /// Every node built so far. Index 0 is the document, and an index is what
    /// [`TreeSink::Handle`] is.
    ///
    /// A `RefCell` because [`TreeSink`] takes `&self` throughout: the tree
    /// builder holds handles into this while it calls, so the interior
    /// mutability is the trait's shape rather than a choice made here. Every
    /// borrow below is one statement long for that reason.
    nodes: RefCell<Vec<Node>>,
}

impl Sink {
    /// A sink holding nothing but the document node every handle is relative
    /// to.
    fn new() -> Self {
        Self {
            nodes: RefCell::new(vec![Node::new(Kind::Document)]),
        }
    }

    /// `node`, added to the arena, as the handle the builder will refer to it
    /// by.
    fn push(&self, node: Node) -> usize {
        let mut nodes = self.nodes.borrow_mut();
        nodes.push(node);
        nodes.len() - 1
    }

    /// Which parent `target` hangs off and where among its children it sits, or
    /// `None` for a node with no parent.
    fn locate(&self, target: usize) -> Option<(usize, usize)> {
        let nodes = self.nodes.borrow();
        let parent = nodes[target].parent?;
        let at = nodes[parent].children.iter().position(|&n| n == target)?;
        Some((parent, at))
    }

    /// Makes `child` the last of `parent`'s children.
    fn attach(&self, parent: usize, child: usize) {
        self.detach(child);
        let mut nodes = self.nodes.borrow_mut();
        nodes[child].parent = Some(parent);
        nodes[parent].children.push(child);
    }

    /// Puts `child` among `parent`'s children at `at`.
    fn insert(&self, parent: usize, at: usize, child: usize) {
        let mut nodes = self.nodes.borrow_mut();
        nodes[child].parent = Some(parent);
        nodes[parent].children.insert(at, child);
    }

    /// Takes `target` out of whatever it hangs off, leaving it in the arena
    /// with no parent.
    fn detach(&self, target: usize) {
        let Some((parent, at)) = self.locate(target) else {
            return;
        };
        let mut nodes = self.nodes.borrow_mut();
        nodes[parent].children.remove(at);
        nodes[target].parent = None;
    }

    /// Appends `text` to the node at `at` if there is one and it is text, and
    /// reports whether it did.
    ///
    /// [`TreeSink`]'s contract is that adjacent text is one node, so this is
    /// what keeps a document that arrived in ten buffers from producing ten
    /// text nodes where a browser holds one.
    fn merged(&self, at: Option<usize>, text: &str) -> bool {
        let Some(at) = at else {
            return false;
        };
        let mut nodes = self.nodes.borrow_mut();
        if nodes[at].kind != Kind::Text {
            return false;
        }
        nodes[at].text.push_str(text);
        true
    }
}

impl TreeSink for Sink {
    type Handle = usize;
    type Output = Parsed;
    type ElemName<'a> = Ref<'a, QualName>;

    fn finish(self) -> Parsed {
        let mut nodes = self.nodes.into_inner();
        let mut document = taken(&mut nodes[0]);
        fill(&mut nodes, 0, 1, &mut document.children);
        document
    }

    /// Discarded, and that is `rule:core-classes/html-parsing` rather than a
    /// gap: every one of these names a place the algorithm has already said
    /// what to do, so the recovery is the specified output and there is nothing
    /// a caller could act on. A parse that reported them would be offering a
    /// failure this member does not have.
    fn parse_error(&self, _why: Cow<'static, str>) {}

    fn get_document(&self) -> usize {
        0
    }

    fn elem_name<'a>(&'a self, target: &'a usize) -> Ref<'a, QualName> {
        Ref::map(self.nodes.borrow(), |nodes| {
            nodes[*target]
                .qual
                .as_ref()
                .expect("the tree builder asks only an element for its name")
        })
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> usize {
        let mut node = Node::new(Kind::Element);
        node.name = written(&name);
        node.attributes = attrs
            .into_iter()
            .map(|attr| (written(&attr.name), attr.value.to_string()))
            .collect();
        node.integration_point = flags.mathml_annotation_xml_integration_point;
        node.qual = Some(name);
        self.push(node)
    }

    fn create_comment(&self, text: StrTendril) -> usize {
        let mut node = Node::new(Kind::Comment);
        node.text = text.to_string();
        self.push(node)
    }

    /// Unreachable from an HTML document — the tokenizer reads `<?…>` as a
    /// bogus comment — and written out anyway because the node family has this
    /// kind and `xml5ever` drives the same trait onto it.
    fn create_pi(&self, target: StrTendril, data: StrTendril) -> usize {
        let mut node = Node::new(Kind::ProcessingInstruction);
        node.name = target.to_string();
        node.text = data.to_string();
        self.push(node)
    }

    fn append(&self, parent: &usize, child: NodeOrText<usize>) {
        match child {
            NodeOrText::AppendNode(node) => self.attach(*parent, node),
            NodeOrText::AppendText(text) => {
                let last = self.nodes.borrow()[*parent].children.last().copied();
                if !self.merged(last, &text) {
                    let node = self.push(Node::text(&text));
                    self.attach(*parent, node);
                }
            }
        }
    }

    fn append_before_sibling(&self, sibling: &usize, new_node: NodeOrText<usize>) {
        match new_node {
            NodeOrText::AppendNode(node) => {
                // The detach comes first and the position is read after it:
                // `node` may have an old parent, and if that parent is this one
                // then removing it moves every sibling after it down one.
                self.detach(node);
                if let Some((parent, at)) = self.locate(*sibling) {
                    self.insert(parent, at, node);
                }
            }
            NodeOrText::AppendText(text) => {
                let Some((parent, at)) = self.locate(*sibling) else {
                    return;
                };
                let before = at
                    .checked_sub(1)
                    .map(|before| self.nodes.borrow()[parent].children[before]);
                if !self.merged(before, &text) {
                    let node = self.push(Node::text(&text));
                    self.insert(parent, at, node);
                }
            }
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &usize,
        prev_element: &usize,
        child: NodeOrText<usize>,
    ) {
        if self.nodes.borrow()[*element].parent.is_some() {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    /// Dropped, because [`Kind`] is closed at five and none of them is a
    /// doctype. `Core\Xml` refuses a `<!DOCTYPE …>` outright rather than
    /// reading one, so neither door lets a document type declaration mean
    /// anything, and this is that same answer where refusing is not allowed:
    /// the declaration names no entity that could be expanded and leaves no
    /// node that could be read.
    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public: StrTendril,
        _system: StrTendril,
    ) {
    }

    /// A `<template>`'s contents are the element's own children.
    ///
    /// The DOM keeps them in a separate fragment, and the family has no
    /// fragment node to keep them in. Handing the element back as its own
    /// contents puts them where a program walking the tree would look for them
    /// — `$template->children()` — instead of nowhere, which is what a fragment
    /// nothing is attached to would mean here.
    fn get_template_contents(&self, target: &usize) -> usize {
        *target
    }

    fn same_node(&self, x: &usize, y: &usize) -> bool {
        x == y
    }

    /// Discarded: quirks mode changes how a *renderer* lays a document out and
    /// changes nothing about the tree, and this member answers a tree.
    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn add_attrs_if_missing(&self, target: &usize, attrs: Vec<Attribute>) {
        let mut nodes = self.nodes.borrow_mut();
        let node = &mut nodes[*target];
        for attr in attrs {
            let name = written(&attr.name);
            if node.attributes.iter().all(|(had, _)| *had != name) {
                node.attributes.push((name, attr.value.to_string()));
            }
        }
    }

    fn remove_from_parent(&self, target: &usize) {
        self.detach(*target);
    }

    fn reparent_children(&self, node: &usize, new_parent: &usize) {
        let moved = std::mem::take(&mut self.nodes.borrow_mut()[*node].children);
        let mut nodes = self.nodes.borrow_mut();
        for &child in &moved {
            nodes[child].parent = Some(*new_parent);
        }
        nodes[*new_parent].children.extend(moved);
    }

    fn is_mathml_annotation_xml_integration_point(&self, handle: &usize) -> bool {
        self.nodes.borrow()[*handle].integration_point
    }

    /// Refused, so a `<template shadowrootmode>` stays the template element it
    /// was written as. A shadow root is a sixth kind of node under another
    /// name, and the family is closed at five; a program that wants what the
    /// template holds reads its children.
    fn allow_declarative_shadow_roots(&self, _intended_parent: &usize) -> bool {
        false
    }
}

/// A qualified name as the document wrote it — `prefix:local` where there is a
/// prefix and `local` where there is not.
///
/// The namespace URI is deliberately not in it. It is not what a tag looks like
/// in the source, `Core\Xml`'s own parse puts the written name in the same
/// field, and a name that differed between the two doors would be the drift
/// `rule:core-classes/html-parsing` forbids.
fn written(name: &QualName) -> String {
    match &name.prefix {
        Some(prefix) => format!("{prefix}:{}", name.local),
        None => name.local.to_string(),
    }
}

/// The node at `node` as a childless [`Parsed`], moving its text out of the
/// arena rather than copying it.
///
/// Every node is visited exactly once, so what is left behind is never read
/// again — which is what keeps a parse from holding the document's text a third
/// time while it converts.
fn taken(node: &mut Node) -> Parsed {
    let mut built = Parsed::new(node.kind);
    built.name = std::mem::take(&mut node.name);
    built.text = std::mem::take(&mut node.text);
    built.attributes = std::mem::take(&mut node.attributes);
    built
}

/// Every child of `at`, converted into `into`, where those children sit at
/// `depth`.
///
/// **Recursion here is bounded by [`DEPTH_CEILING`], and the bound is
/// load-bearing rather than tidy.** `rule:core-classes/html-parsing` gives this
/// parse no way to fail, so `<div>` written ten thousand times is a document
/// that must produce a tree; a walk that recursed over it would exhaust the
/// native stack, and a crash is not something AGENTS.md's priority 1 trades for
/// fidelity. `Core\Xml` holds the same ceiling from the other side by refusing
/// a document past it, which it is allowed to do and this is not.
///
/// Nothing is dropped. A node deeper than the ceiling is attached to the
/// deepest ancestor still inside it, so all of the document's content arrives
/// and only its nesting flattens — and it flattens a thousand elements past
/// where any document a person wrote ends.
fn fill(nodes: &mut [Node], at: usize, depth: usize, into: &mut Vec<Parsed>) {
    let children = std::mem::take(&mut nodes[at].children);
    for child in children {
        let mut node = taken(&mut nodes[child]);
        if depth < DEPTH_CEILING {
            fill(nodes, child, depth + 1, &mut node.children);
            into.push(node);
        } else {
            into.push(node);
            flatten(nodes, child, into);
        }
    }
}

/// Every descendant of `at`, appended to `into` as siblings rather than as a
/// nesting — the ceiling's own walk.
///
/// Iterative, which is the whole point: the depth it is reading is the depth
/// [`fill`] refused to recurse over.
fn flatten(nodes: &mut [Node], at: usize, into: &mut Vec<Parsed>) {
    let mut work = Vec::new();
    stack(nodes, at, &mut work);
    while let Some(child) = work.pop() {
        into.push(taken(&mut nodes[child]));
        stack(nodes, child, &mut work);
    }
}

/// `at`'s children pushed onto `work` so that popping yields them in document
/// order, and their own children after each of them.
fn stack(nodes: &mut [Node], at: usize, work: &mut Vec<usize>) {
    let children = std::mem::take(&mut nodes[at].children);
    work.extend(children.into_iter().rev());
}

/// `document`, parsed by the WHATWG algorithm.
///
/// `ParseOpts::default()` leaves scripting *enabled*, which is what a browser
/// with JavaScript on does and therefore what the agreement this member is
/// built on is about: `<noscript>`'s content is raw text rather than markup.
/// It is also the safer of the two readings for anything built over this, since
/// content a scripting browser would never build elements from does not become
/// elements here either.
fn parse(document: &str) -> Parsed {
    html5ever::parse_document(Sink::new(), html5ever::ParseOpts::default()).one(document)
}

nvs_runtime::nvs_helper! {
    /// `Core\Html::parse(string $document): Core\Xml\Node` — replacing
    /// `DOMDocument::loadHTML` and PHP 8.4's `Dom\HTMLDocument::createFromString`.
    ///
    /// There is no error path, and that is the member's contract rather than an
    /// omission: `rule:core-classes/html-parsing` takes the WHATWG algorithm
    /// whole, and under it implied tags, misnested tags and stray content have
    /// a *specified* placement that every conforming parser agrees on. Nothing
    /// is guessed, so `rule:errors/ambiguous-input-refused` has no ambiguity to
    /// refuse. [`crate::xml::nvs_core_xml_parse`] keeps the opposite contract
    /// over the same tree, which is the pair the rule is about.
    fn nvs_core_html_parse(_ctx, args: [1]) {
        // unreachable from source: the parameter is `CoreTy::Text`, so anything
        // that is not a `string` is `E0401` at the call site.
        let Some(document) = args[0].as_text() else {
            return Err(Fault::fatal(format!(
                "Core\\Html::parse expected a `string`, got tag {}",
                args[0].tag_byte()
            )));
        };
        Ok(crate::xml::instance_of(parse(document)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::CLASSES;

    /// `rule:core-classes/html-auto-escape`'s carrier, in the two facts neither crate that acts on it
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

    /// `rule:security/launderers-are-sink-named`, asked of the registry rather than of the body: `escape`
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
            matches!(escape.return_ty, CoreTy::Instance(name) if name == MARKUP_NAME),
            "`escape` answers an unqualified `Core\\Html\\Markup` — `rule:security/launderer-answers-a-carrier`"
        );

        // `Reveal` is the other axis and a different decision: `rule:security/verification-does-not-launder`'s
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
            "`rule:security/launderers-are-sink-named`'s launderer for the HTML sink is one member and is named for it"
        );
    }

    /// `rule:core-classes/html-escape-answers-markup`, asked of the whole route rather than of the row: the
    /// carrier `escape` answers is the one the HTML sink writes raw, its slot
    /// is the one the bytes go into, and the class it names is registered.
    ///
    /// The three together are what "the escaped value is a carrier" means at
    /// run time. A row answering `CoreTy::Instance` of a class the sink did not
    /// render would compile, pass the row-shaped assertion in
    /// [`html_escape_launders_for_the_html_sink_and_for_no_other`], and print
    /// the escaped text with `Core\Html\Markup` where the markup should be.
    #[test]
    fn html_escape_answers_the_markup_carrier() {
        let escape = CLASS
            .members()
            .find(|method| method.name == "escape")
            .expect("`Core\\Html::escape` is registered");
        let CoreTy::Instance(answered) = escape.return_ty else {
            panic!("`escape` answers an instance, not {:?}", escape.return_ty)
        };

        assert_eq!(answered, MARKUP.name, "and the instance is this class");
        assert!(
            nvs_runtime::is_carrier(answered),
            "the answer is written raw by the HTML sink, which is the whole \
             point of it not being a `string`"
        );
        assert_eq!(
            MARKUP.slot("text"),
            nvs_runtime::CARRIER_TEXT_SLOT,
            "and the escaped bytes go in the slot that sink reads"
        );

        // The composition half, which `rule:core-classes/html-escape-answers-markup` says is already built: an
        // escaped fragment is usable with `+` without a second escape, so the
        // operator table has to have the row the answer's type needs.
        assert!(
            crate::registry::CLASSES
                .iter()
                .any(|class| class.name == answered),
            "`CoreTy::Instance` resolves against the registry, so the answer \
             names a class a program can be handed"
        );
    }

    /// `rule:security/launderer-answers-a-carrier`'s predicate, asked of **every** launderer in the registry
    /// rather than of this one — the claim is a set, exactly as
    /// [`html_escape_launders_for_the_html_sink_and_for_no_other`]'s is.
    ///
    /// § 1 makes the carrier a *derived* answer: a launderer answers one when
    /// its sink launders on its own **and** its transform is not idempotent,
    /// and the plain unqualified type otherwise. So the roster below is the
    /// table in that section, transcribed, and the test asserts agreement in
    /// both directions. `Core\Html::escape` is the only yes: the HTML sink
    /// auto-escapes and `&` → `&amp;` → `&amp;amp;` changes under a second
    /// application. `Core\Cli::escape` is the near miss the name-half of this
    /// test exists for — the terminal *also* launders on its own, and its
    /// escape is idempotent because the glyph it substitutes holds no `ESC`,
    /// so it keeps its `string` (`rule:tooling/terminal-output-is-a-sink`).
    ///
    /// Two rows read wrong at a glance and neither is a counterexample.
    /// `Core\Cli\Text::plain` and `styled` answer a carrier while their sink is
    /// idempotent, because they are *constructors* for the terminal's carrier
    /// rather than that sink's escaper — which is why § 1's table has a row per
    /// sink and not per member, and why `Core\Cli::escape` is the row that
    /// speaks for the terminal. `Core\Http::allowUrl` answers
    /// `Core\Http\Target`, which is a pinned-address capability handle and not
    /// a text carrier at all: the column here is *`nvs_runtime::is_carrier`* —
    /// "the sink writes this value's bytes out raw" — and `rule:http-server/allow-url-pins-the-address`'s sink
    /// neither auto-launders nor escapes anything.
    ///
    /// `Core\Taint::assertTrusted` is the third, and the one row that names no
    /// sink at all: § 1's predicate still answers, because the transform is the
    /// identity and so idempotent by inspection, and a value the developer has
    /// just sworn is trusted re-entering an auto-escaping sink is the case that
    /// predicate exists to let pass. `nvs_stdlib::registry`'s `Qual` doc
    /// comment is the home of why one row is allowed to name all of them.
    ///
    /// The roster is asserted whole, so a launderer added anywhere fails here
    /// until someone places it against the predicate — which is the day the
    /// decision is actually being made.
    #[test]
    fn every_launderer_for_an_auto_escaping_sink_answers_a_carrier() {
        // `rule:security/launderer-answers-a-carrier`'s table, as `(member, does it answer a carrier)`. A
        // `true` row is a sink that both auto-launders and is non-idempotent.
        const PLACED: &[(&str, bool)] = &[
            (r"Core\Cli::escape", false),
            (r"Core\Cli\Text::plain", true),
            (r"Core\Cli\Text::styled", true),
            (r"Core\Db::quoteIdentifier", false),
            (r"Core\Html::escape", true),
            (r"Core\Http::allowUrl", false),
            (r"Core\IO::within", false),
            (r"Core\Regex::quote", false),
            (r"Core\SignedCookie::open", false),
            (r"Core\Taint::assertTrusted", false),
            (r"Core\Uri::encodeComponent", false),
            (r"Core\Uri::encodeFormValue", false),
            // The writing half of § 17 is a launderer once per member that puts
            // something into a document, and none of them answers a carrier:
            // nothing auto-escapes into an XML document the way `echo` does
            // into an HTTP response, so the predicate's first condition fails
            // before its second is asked. `crate::xml`'s `WRITER` is where each
            // member names the sink it launders for.
            (r"Core\Xml\Writer::attribute", false),
            (r"Core\Xml\Writer::cdata", false),
            (r"Core\Xml\Writer::comment", false),
            (r"Core\Xml\Writer::content", false),
            (r"Core\Xml\Writer::doctype", false),
            (r"Core\Xml\Writer::instruction", false),
            (r"Core\Xml\Writer::startElement", false),
        ];

        let mut roster: Vec<(String, bool)> = CLASSES
            .iter()
            .flat_map(|class| class.members().map(move |method| (class.name, method)))
            .filter(|(_, method)| {
                method
                    .params
                    .iter()
                    .any(|param| matches!(param, CoreTy::Text(Qual::Launder)))
            })
            .map(|(class, method)| {
                let answers_carrier = match method.return_ty {
                    CoreTy::Instance(name) => nvs_runtime::is_carrier(name),
                    _ => false,
                };
                (format!("{class}::{}", method.name), answers_carrier)
            })
            .collect();
        roster.sort();

        let placed: Vec<(String, bool)> = PLACED
            .iter()
            .map(|(name, carrier)| ((*name).to_owned(), *carrier))
            .collect();
        assert_eq!(
            roster, placed,
            "every launderer is placed against `rule:security/launderer-answers-a-carrier`'s two conditions, \
             and answers a carrier exactly when both hold"
        );
    }

    /// `rule:core-classes/html-to-source`'s escape hatch, in the three things that make it one: it is
    /// the **only** member that takes a `Markup` and answers a `string`, it
    /// takes a written reason, and the reason is ordinary text rather than a
    /// second qualified position.
    ///
    /// The first assertion is the one worth having. `Markup as string` does not
    /// exist, so the way out is this row and a program can be read for it by
    /// name — a second member answering the bytes would be an ungreppable
    /// second door, and it would compile.
    #[test]
    fn to_source_is_the_only_way_out_of_markup_and_it_takes_a_reason() {
        let ways_out: Vec<String> = CLASSES
            .iter()
            .flat_map(|class| class.members().map(move |method| (class.name, method)))
            .filter(|(_, method)| {
                matches!(method.return_ty, CoreTy::Str | CoreTy::Bytes)
                    && method.params.iter().any(
                        |param| matches!(param, CoreTy::Instance(name) if *name == MARKUP_NAME),
                    )
            })
            .map(|(class, method)| format!("{class}::{}", method.name))
            .collect();
        assert_eq!(
            ways_out,
            vec![r"Core\Html::toSource"],
            "`rule:core-classes/html-to-source` gives the carrier one exit, and it is named for what \
             it hands back"
        );

        let to_source = CLASS
            .members()
            .find(|method| method.name == "toSource")
            .expect("`Core\\Html::toSource` is registered");
        assert_eq!(
            to_source.names,
            ["markup", "reason"],
            "the reason is written at the call site, which is what makes the \
             hatch readable rather than merely narrow"
        );
        assert!(
            matches!(
                to_source.params,
                [CoreTy::Instance(name), CoreTy::Text(Qual::Neutral)] if *name == MARKUP_NAME
            ),
            "the reason is [`Qual::Neutral`] for `Core\\Secret::reveal`'s \
             reason: no byte of it reaches the answer, so a `secret` \
             justification is refused by the ordinary rule"
        );
        assert!(
            to_source.defaults.is_empty(),
            "neither argument has a default — a reason nobody had to write is a \
             reason nobody wrote"
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

    /// Every node of `tree`, in document order, as `(depth, kind, name)`.
    fn walked(tree: &Parsed) -> Vec<(usize, Kind, &str)> {
        let mut seen = vec![(0, tree.kind, tree.name.as_str())];
        let mut work: Vec<(usize, &Parsed)> = tree.children.iter().rev().map(|c| (1, c)).collect();
        while let Some((depth, node)) = work.pop() {
            seen.push((depth, node.kind, node.name.as_str()));
            work.extend(node.children.iter().rev().map(|c| (depth + 1, c)));
        }
        seen
    }

    /// `rule:core-classes/html-parsing`'s never-fails half, over the inputs a
    /// parser that *could* fail would fail on.
    ///
    /// Counted rather than read off a line, because what is under test is a
    /// property of the whole table: a member with one refusal in it takes the
    /// count down, and a member that answered plausibly for eleven of twelve
    /// still fails here. The signature carries the same claim at compile time —
    /// [`parse`] answers a [`Parsed`] and not a `Result`, so there is no error
    /// path to leave untested — and this is the runtime half, that no input
    /// reaches a panic or an empty answer either.
    #[test]
    fn tag_soup_produces_a_document_because_the_parser_has_no_failure_mode() {
        let soup = [
            "",
            "<",
            "<<<<",
            "</>",
            "<p<p<p>",
            "<a href=\"unterminated>text",
            "<!-- unterminated comment",
            "<![CDATA[not xml]]>",
            "<?not a processing instruction?>",
            "<script>var a = '</p>';</script>",
            "&notanentity;",
            "<table><td><table><td>",
            "\u{0}\u{FFFF}",
            "<!DOCTYPE html SYSTEM \"http://example.invalid/dtd\">",
            "<svg><foreignObject><div><table><tr>",
            "</html></body><p>after the end",
        ];
        let documents = soup
            .iter()
            .filter(|text| parse(text).kind == Kind::Document)
            .count();
        assert_eq!(
            documents,
            soup.len(),
            "every one of these is a document, because the WHATWG algorithm \
             specifies an answer for each of them"
        );
    }

    /// `rule:core-classes/html-parsing`'s one-node-family half.
    ///
    /// The strongest half of it is not asserted here at all — [`parse`]'s
    /// return type is [`crate::xml::Parsed`], so a node of this door's own
    /// invention would not compile. What is left for a test is that the *kinds*
    /// this door reaches are the family's and no wider, asked as a set over a
    /// document written to produce every one of them that HTML can, so a parse
    /// that grew a sixth ordinal fails here rather than at the door a program
    /// walks the tree through.
    #[test]
    fn the_parser_produces_core_xmls_own_node_family() {
        let tree = parse("<!DOCTYPE html><!--note--><p id=x>text<br></p>");
        let mut kinds: Vec<Kind> = walked(&tree).into_iter().map(|(_, kind, _)| kind).collect();
        kinds.sort_by_key(|kind| kind.ordinal());
        kinds.dedup();
        assert_eq!(
            kinds,
            vec![Kind::Element, Kind::Text, Kind::Comment, Kind::Document],
            "the four kinds an HTML document can hold, and the fifth — a \
             processing instruction — is one the tokenizer reads as a comment, \
             which is why it is absent rather than missing"
        );
        assert!(
            Kind::ALL.contains(&Kind::ProcessingInstruction),
            "the family is `Core\\Xml`'s whole family and not the subset this \
             door reaches; a kind only the other door produces is still one \
             a program walking either tree may meet"
        );
        assert_eq!(
            walked(&tree)
                .iter()
                .filter(|(_, kind, _)| *kind == Kind::Element)
                .map(|(_, _, name)| *name)
                .collect::<Vec<_>>(),
            vec!["html", "head", "body", "p", "br"],
            "a doctype leaves no node: the family has no kind for one, so this \
             door drops what `Core\\Xml` refuses"
        );
    }

    /// `rule:core-classes/html-parsing`'s placement, asked of the registry: the
    /// WHATWG parse is an entry on **this** class, and neither door carries the
    /// other's mode.
    ///
    /// The mode half is what makes this more than a row read back. § 1 retired
    /// `DOMDocument`'s `loadXML`/`loadHTML` pair — one object flipping between
    /// refuse-hard and recover-always — so the claim under test is that neither
    /// `parse` has anywhere to put a flag: one parameter each, no default and no
    /// options bag, which is the shape a mode would have to arrive in. The
    /// return types agreeing is the other half, and it is the one-node-family
    /// clause: two doors, one `Core\Xml\Node`.
    #[test]
    fn it_is_an_entry_on_core_html_and_there_is_no_html_mode_on_the_xml_parser() {
        let doors = [(r"Core\Html", &CLASS), (r"Core\Xml", &crate::xml::CLASS)];
        for (class, roster) in doors {
            let parse = roster
                .members()
                .find(|member| member.name == "parse")
                .unwrap_or_else(|| panic!("{class} parses"));
            assert!(
                matches!(parse.params, [CoreTy::Text(Qual::Neutral)]),
                "{class}::parse takes the document and nothing else — an options \
                 bag or a second parameter is where a mode would arrive, and \
                 `rule:core-classes/html-parsing` is that neither door has one"
            );
            assert_eq!(parse.names, ["document"]);
            assert!(parse.defaults.is_empty());
            assert!(
                matches!(parse.return_ty, CoreTy::Instance(node) if node == crate::xml::NODE_NAME),
                "{class}::parse answers the one node family both parsers \
                 produce, which is what lets a walk be written once"
            );
        }
        assert_eq!(
            CLASS
                .members()
                .filter(|member| member.name.contains("arse"))
                .count(),
            1,
            "one WHATWG entry, not a parse plus a parseFragment or a \
             parseWithOptions — the second would be the mode under another name"
        );
    }
}
