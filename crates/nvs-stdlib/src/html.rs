//! `Core\Html` — `rule:security/launderers-are-sink-named`'s narrow,
//! sink-named launderer over the sink § 5 makes out of HTML text, and
//! `rule:core-classes/html-parsing`'s WHATWG parser onto `Core\Xml`'s tree.
//!
//! § 3 writes `Core\Html::escape(tainted string): Core\Html\Markup` out as
//! *the* worked example of what a launderer is allowed to be: one member, one
//! sink, and a contract naming it. `sanitize` is the second member of that
//! shape here and the last — it names the same sink and launders the other
//! thing that arrives at it, untrusted markup rather than untrusted text. What
//! § 3 refuses is a launderer naming **no** sink, and there is no generic
//! `clean()` here ever: a value safe for HTML text is not safe for a shell
//! argument, and a catch-all buys the false confidence the qualifier exists to
//! prevent.
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
//! # The rebuild, and where its policy is
//!
//! [`nvs_core_html_sanitize`] is `rule:core-classes/html-sanitize`'s member and
//! is three things in a row, each of which can be read without the others:
//! [`parse`] reads the document, [`rebuilt`] decides what of it survives
//! [`ELEMENTS`], and [`source`] writes what is left back out under the
//! serialisation half of `rule:core-classes/html-parsing` — WHATWG's rules
//! through this door, [`crate::xml`]'s writer through the other. Only the
//! middle one holds a policy, and that policy is a closed list rather than an
//! argument, so what a caller can change about it is nothing.
//!
//! # Where each half of the reason rule is enforced
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
//! # Every door onto a `Markup`, and the sink that needs none
//!
//! [`MARKUP`] is registered *and* reachable: every way
//! `rule:core-classes/html-auto-escape` and `rule:core-classes/html-literal`
//! give a program to obtain one is here — [`MARKUP_SYMBOL`] for `as Markup` on
//! a source literal, [`MARKUP_CONCAT_SYMBOL`] for `Markup + Markup`,
//! [`nvs_core_html_join`] for a list of fragments and one separator, the escape
//! itself, which `rule:security/launderer-answers-a-carrier` turned from the
//! first two's poor relation into the ordinary one, and the pair a markup
//! literal's own lowering reaches, [`ESCAPE_TEXT_SYMBOL`] and
//! [`MARKUP_TEXT_SYMBOL`], which answer a hole's bytes rather than a carrier
//! per hole.
//!
//! **The sink's automatic lift is none of them, and none of it is here.** Every
//! non-`Markup` value `echo` is handed inside an HTTP request is escaped with
//! no call written at the site, and the decision is the *sink's*: `nvs_runtime`'s
//! `write_rendered` reads the transform off the carrier already named by the sink
//! in force — `Core\Html\Markup` for the `OutputSink::Body` an isolate answering
//! a request builds, and the terminal's substitution for every other one — so no
//! `echo` site and no member on this class arbitrates it.
//! `rule:tooling/echo-always-has-a-sink` is that table and those two modules'
//! own comments are the home of which column is read where. What this module
//! owes that path is the escape itself and only that: [`nvs_core_html_escape`]
//! and the sink both call `nvs_render::html::escape`, which is what makes the
//! launderer and the sink incapable of disagreeing about what an `&` becomes.
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
    doc: None,
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
            name: "join",
            names: &["parts", "separator"],
            // A `Markup` per element rather than a `string`: the answer is the
            // carrier the HTML sink writes raw, so a `string` element would be
            // a way to put unescaped computed text inside one — which is the
            // bypass `rule:core-classes/html-auto-escape` closes. Every part
            // arrives having already passed whichever rule made it markup.
            params: &[
                CoreTy::Array(&CoreTy::Instance(MARKUP_NAME)),
                CoreTy::Instance(MARKUP_NAME),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(MARKUP_NAME),
            symbol: "nvs_core_html_join",
            doc: Some(&JOIN_DOC),
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
        CoreMethod {
            name: "sanitize",
            names: &["document"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Instance(MARKUP_NAME),
            symbol: "nvs_core_html_sanitize",
            doc: Some(&SANITIZE_DOC),
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
/// Every way of obtaining one either writes the trust in the source or is a
/// member of [`CLASS`] that takes trusted operands: a markup literal, whose
/// segments are what the author typed; `as Markup` on a *source literal*,
/// which is the trust level that literal already carried; `Markup + Markup`
/// and `Core\Html::join`, which compose fragments already trusted; the
/// launderer, which earns the carrier by escaping; and the sink's own
/// escape-and-lift of everything else. A constructor member *here* would be
/// none of those — it would take a runtime `string`, which is exactly the
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
    doc: None,
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

/// The symbol a markup literal's hole lowers to — `rule:core-classes/html-literal`'s
/// escape, answering the escaped **bytes** where [`nvs_core_html_escape`]
/// answers a carrier.
///
/// One transformation, two answer shapes, and the position is what picks: a
/// hole is one piece of a literal that becomes a single `Markup` holding the
/// joined bytes, so a carrier per hole would be a carrier the join unwraps
/// again. What that rule's *What it costs to run* promises is one object
/// allocation for the whole literal, however many holes it has.
///
/// Row-less for [`MARKUP_SYMBOL`]'s reason: `escape` is the written spelling
/// and this is a lowering's, so the literal's own lowering is the only thing
/// allowed to reach it.
pub const ESCAPE_TEXT_SYMBOL: &str = "nvs_core_html_escape_text";

/// The symbol a markup literal's `Markup`-holding hole lowers to — the raw
/// splice `rule:core-classes/html-literal` grants it, answering the carrier's
/// own bytes.
///
/// Row-less for [`ESCAPE_TEXT_SYMBOL`]'s reason, and narrower than
/// `Core\Html::toSource`, which is the *written* way out of the carrier and
/// demands a reason for being one (`rule:core-classes/html-to-source`). Nothing
/// a program writes reaches this: the bytes it hands back never become a
/// `string` a program can hold, only another carrier's slot.
pub const MARKUP_TEXT_SYMBOL: &str = "nvs_core_html_markup_text";

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

/// `Core\Html::join`'s reference card — `rule:core-api/reference-card`.
const JOIN_DOC: MethodDoc = MethodDoc {
    short: "Concatenates a list of `Core\\Html\\Markup` fragments in order, writing `$separator` \
            between each pair — the list form of `Markup + Markup`, which is what a page composed \
            from fragments writes instead of folding the operator over them.",
    params: &[
        ParamDoc {
            name: "parts",
            desc: "The fragments to write out, in the order they are held.",
            shape: &[],
        },
        ParamDoc {
            name: "separator",
            desc: "The markup written between each pair of parts — never before the first or \
                   after the last. An empty markup joins the parts with nothing between them.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Html\\Markup` carrying every part's bytes in order, and empty markup for an \
          empty list. Nothing is escaped on the way: each part and the separator are already \
          carriers, so re-escaping one would corrupt the markup it was built for.",
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

/// `Core\Html::sanitize`'s reference card — `rule:core-api/reference-card`.
const SANITIZE_DOC: MethodDoc = MethodDoc {
    short: "Parses `$document` as HTML and answers a document rebuilt from the elements and \
            attributes the allowlist holds — the launderer for untrusted markup, as opposed to \
            untrusted text, which is `escape`'s.",
    params: &[ParamDoc {
        name: "document",
        desc: "The markup to rebuild. Any text at all is a document, since the parse behind this \
               has no way to refuse one.",
        shape: &[],
    }],
    ret: "A `Core\\Html\\Markup` carrying the rebuilt document, which is why the result may be \
          written into a page without being escaped again. Nothing is filtered and nothing is \
          escaped in place: an element the allowlist does not hold contributes no tag, an \
          attribute it does not hold is not written, and a `script`, `style` or other raw-text \
          element is gone with its content. The allowlist is closed — there is no argument that \
          extends it, because a policy the caller writes is a policy whose holes are the \
          caller's. Text, and the order of what is kept, are the document's own.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_html_escape" => (nvs_core_html_escape as *const ()).cast(),
        "nvs_core_html_join" => (nvs_core_html_join as *const ()).cast(),
        "nvs_core_html_parse" => (nvs_core_html_parse as *const ()).cast(),
        "nvs_core_html_sanitize" => (nvs_core_html_sanitize as *const ()).cast(),
        "nvs_core_html_to_source" => (nvs_core_html_to_source as *const ()).cast(),
        MARKUP_SYMBOL => (nvs_core_html_markup as *const ()).cast(),
        MARKUP_CONCAT_SYMBOL => (nvs_core_html_markup_concat as *const ()).cast(),
        ESCAPE_TEXT_SYMBOL => (nvs_core_html_escape_text as *const ()).cast(),
        MARKUP_TEXT_SYMBOL => (nvs_core_html_markup_text as *const ()).cast(),
        _ => return None,
    })
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

/// [`nvs_render::html::escape`] over a `Value`, which is the whole of
/// `rule:security/launderers-are-sink-named`'s transform for the HTML sink:
/// `&`, `<`, `>`, `"` and `'` written as character references and every
/// unterminated bidirectional control replaced.
///
/// **The table is not here.** The HTML sink applies the same transform to
/// every non-carrier value `echo` writes into a response, and a copy of it in
/// this module would be a second answer to what an `&` becomes — so the
/// launderer calls the sink's own, exactly as
/// [`nvs_core_html_escape_text`] and `nvs_runtime`'s `echo` do.
///
/// `value` is borrowed, as a `CoreCall`'s argument always is, and the answer
/// carries **one reference of its own** for whoever called to hand on — the
/// argument's own, retained, where there was nothing to escape, and a fresh
/// allocation where there was. That is what lets [`nvs_core_html_escape`] pass
/// it straight to [`crate::instance::build`], which takes a slot's reference
/// over, and [`ESCAPE_TEXT_SYMBOL`] hand it back as the answer.
///
/// # Why the unchanged case still carries the argument's own bytes
///
/// `rule:core-classes/html-auto-escape` makes this the sink's *only* behaviour:
/// every non-`Markup` interpolation into an HTML response passes through here,
/// whether or not it is tainted. So the input with nothing to escape is not an
/// edge case, it is most of a page — and handing that path's bytes straight on
/// keeps it at one scan and no *string* allocation, which is what makes a rule
/// that cannot be switched off affordable (AGENTS.md's priority 3).
/// [`nvs_render::text::substitute`] answers a borrow for the same reason one
/// sink over.
fn escaped_text(value: Value, subject: &str) -> Result<Value, Fault> {
    let text = text(&value, subject)?;
    // The borrow the escape answers *is* the "nothing to escape" answer, so
    // the unchanged case is recognised here rather than scanned for a second
    // time.
    let std::borrow::Cow::Owned(escaped) = nvs_render::html::escape(text) else {
        #[expect(
            unsafe_code,
            reason = "the argument slot holds a live reference for the length of \
                      the call, which is `Value::retain`'s whole obligation"
        )]
        unsafe {
            value.retain();
        }
        return Ok(value);
    };
    Ok(Value::str(NvsStr::new(escaped.as_bytes())))
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
    /// The transform is [`escaped_text`], shared with [`ESCAPE_TEXT_SYMBOL`],
    /// and its doc comment holds why the unchanged input still travels on its
    /// own bytes. What is left here is the lift.
    ///
    /// **What the carrier itself spends:** one object allocation per call,
    /// charged to the request exactly as [`nvs_core_html_markup`]'s lift is.
    /// That is the price `rule:security/launderer-answers-a-carrier` names and it is paid on every escape,
    /// including the unchanged one — the alternative is a `string` answer the
    /// sink escapes again, which costs a second scan *and* a wrong document.
    fn nvs_core_html_escape(_ctx, args: [1]) {
        // `instance::build` takes the slot's reference over, and that is
        // exactly the one `escaped_text` hands back.
        let escaped = escaped_text(args[0], r"`Core\Html::escape`'s `$text`")?;
        Ok(crate::instance::build(&MARKUP, [escaped]))
    }
}

nvs_runtime::nvs_helper! {
    /// A markup literal's hole, escaped — `rule:core-classes/html-literal`'s
    /// hole rule, and the whole of what [`ESCAPE_TEXT_SYMBOL`] does.
    ///
    /// The same transform `Core\Html::escape` runs, answering the bytes it
    /// produced instead of a carrier holding them, because the piece this is
    /// one of is on its way into a carrier already.
    ///
    /// **What it spends:** nothing beyond the escape itself — no object, and no
    /// string where the hole's text had nothing to escape.
    fn nvs_core_html_escape_text(_ctx, args: [1]) {
        escaped_text(args[0], "a markup literal's hole")
    }
}

nvs_runtime::nvs_helper! {
    /// A markup literal's `Markup`-holding hole, spliced raw — the whole of
    /// what [`MARKUP_TEXT_SYMBOL`] does.
    ///
    /// **Nothing is escaped**, which is `rule:core-classes/html-literal`'s rule
    /// rather than an omission: a hole already holding a carrier is
    /// `Markup + Markup` written in interpolation syntax, and escaping a
    /// fragment that passed whichever rule made it a `Markup` would corrupt the
    /// markup it was built for.
    ///
    /// **What it spends:** nothing. The slot's bytes travel under one more
    /// reference into the carrier the literal is building, so a spliced
    /// fragment is not copied.
    fn nvs_core_html_markup_text(_ctx, args: [1]) {
        let slot = markup_slot(args[0], "a markup literal's spliced hole")?;
        #[expect(
            unsafe_code,
            reason = "the slot holds a live reference for the length of the call, \
                      which is `Value::retain`'s whole obligation"
        )]
        unsafe {
            slot.retain();
        }
        Ok(slot)
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
pub(crate) fn markup_slot(value: Value, position: &str) -> Result<Value, Fault> {
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
    /// `Core\Html::join(array<Core\Html\Markup> $parts, Core\Html\Markup $separator): Core\Html\Markup`
    /// — `rule:core-classes/html-literal`'s composition over a list.
    ///
    /// **Nothing is escaped and nothing is trusted here**, which is the rule
    /// rather than an omission, and it is [`nvs_core_html_markup_concat`]'s
    /// argument over a list instead of a pair: every element is already a
    /// carrier, so each one passed whichever rule made it markup, and
    /// re-escaping a fragment would corrupt the markup it was lifted for. What
    /// the list buys over folding `+` is the separator and the walk — `n`
    /// fragments composed with the operator allocate `n - 1` intermediate
    /// carriers, and this allocates one.
    ///
    /// **What it spends:** one string allocation and one object allocation per
    /// call, both charged to the request, whatever the list holds. No element
    /// is touched: a `Markup` is a value type, so every part and the separator
    /// read back exactly as they were passed.
    fn nvs_core_html_join(_ctx, args: [2]) {
        // Unreachable from source: parameter 0 is `array<Core\Html\Markup>` in
        // [`CLASS`] above, so a non-container subject is `E0401` at the
        // checker. `crate::arr`'s `nvs_core_arr_count` states that judgement in
        // full.
        let parts = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Html::join expected {:?} for the parts, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let separator = markup_slot(args[1], r"`Core\Html::join`'s `$separator`")?;
        let separator = text(&separator, r"`Core\Html::join`'s `$separator`")?;

        let mut out = String::new();
        for (at, part) in crate::str::Elements::of(parts).enumerate() {
            if at > 0 {
                out.push_str(separator);
            }
            let held = markup_slot(part, r"`Core\Html::join`'s `$parts` element")?;
            out.push_str(text(&held, r"`Core\Html::join`'s `$parts` element")?);
        }
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
    /// **An empty one is refused here**, and a computed one never reaches this
    /// body because `E0805` refuses it where it is written; the module doc's
    /// *Where each half of the reason rule is enforced* is the home of that
    /// split.
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
        Ok(crate::xml::instance_of(parse(document), &[]))
    }
}

// ============================================================================
// Serialization — the WHATWG half of the two doors
// ============================================================================

/// The elements written as a start tag and nothing else.
///
/// WHATWG's void set, whole. An end tag for one of these is not a close but a
/// parse error, so writing one would change what the output means rather than
/// pad it: `<br></br>` reparses as two line breaks. The list is the
/// algorithm's and is deliberately not shortened to the elements anyone still
/// writes — `basefont` and `bgsound` are obsolete to *write* and are still
/// void to *read*, and reading is what a serialiser answers to.
const VOID: &[&str] = &[
    "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "img", "input",
    "keygen", "link", "meta", "param", "source", "track", "wbr",
];

/// The elements whose character data is written back literally.
///
/// The parsing algorithm gives each of these a raw-text content model, so
/// their text never held markup and escaping it would change what it says:
/// `&amp;` inside a `<script>` is five characters of program, not one
/// ampersand. `noscript` belongs here because [`parse`] runs with scripting
/// enabled, which is the reading that member is written against.
///
/// **This list is where a tree no parse produced stops round-tripping**, and
/// the boundary is WHATWG's rather than one drawn here: text put inside a
/// `<script>` by hand comes back out as source a reparse reads as elements
/// again. Nothing this module serialises can be in that position —
/// [`ELEMENTS`] holds none of these names, so [`rebuilt`] drops every one of
/// them with its content — and anything that ever serialises a tree from
/// somewhere other than the sanitizer owes the same answer before it does.
const RAW_TEXT: &[&str] = &[
    "style",
    "script",
    "xmp",
    "iframe",
    "noembed",
    "noframes",
    "plaintext",
    "noscript",
];

/// One thing the walk in [`source`] has left to write.
///
/// A stack of these rather than a recursive walk, for the reason
/// [`crate::xml::instance_of`] is iterative too: the tree this reads is
/// [`DEPTH_CEILING`] deep at its deepest, and a native frame per node at that
/// depth is a crash where a bound was supposed to be.
enum Step<'a> {
    /// A node to write, and whether its parent's content model makes character
    /// data under it literal.
    Write(&'a Parsed, bool),
    /// The end tag of an element whose children are already on the stack.
    End(&'a str),
}

/// `tree`'s children as HTML source, by the WHATWG fragment serialisation
/// algorithm.
///
/// The children rather than the node, which is that algorithm's own shape and
/// the one both callers want: a document node serialises to the document, and
/// an element serialises to what is inside it.
///
/// **This is written here rather than in [`crate::xml`] because serialization
/// follows the door** (`rule:core-classes/html-parsing`): the two parsers share
/// a node family, not a set of writing rules. `<br>` has no end tag here and
/// must have one there, an unescaped `>` is text here and is refused there, and
/// a member that took a flag to pick between them would be the one ambiguity
/// that rule retired.
fn source(tree: &Parsed) -> String {
    let mut out = String::new();
    let mut work = Vec::new();
    stacked(tree, false, &mut work);
    while let Some(step) = work.pop() {
        match step {
            Step::End(name) => {
                out.push_str("</");
                out.push_str(name);
                out.push('>');
            }
            Step::Write(node, literal) => written_out(node, literal, &mut out, &mut work),
        }
    }
    out
}

/// `node`'s children pushed onto `work` so that popping yields them in
/// document order, where `literal` is what `node`'s content model makes of
/// character data under it.
fn stacked<'a>(node: &'a Parsed, literal: bool, work: &mut Vec<Step<'a>>) {
    work.extend(
        node.children
            .iter()
            .rev()
            .map(|child| Step::Write(child, literal)),
    );
}

/// `node` written to `out`, with whatever it leaves for later pushed onto
/// `work`.
fn written_out<'a>(node: &'a Parsed, literal: bool, out: &mut String, work: &mut Vec<Step<'a>>) {
    match node.kind {
        // A document inside a document is not something either parser builds;
        // if one is ever handed here it is a bag of children, which is what
        // the document node is at the top too.
        Kind::Document => stacked(node, false, work),
        Kind::Text => {
            if literal {
                out.push_str(&node.text);
            } else {
                escape_into(&node.text, false, out);
            }
        }
        // Comment data is written as it stands, which is the algorithm: there
        // is no escape defined for it, and inventing one would make the
        // comment say something the tree does not.
        Kind::Comment => {
            out.push_str("<!--");
            out.push_str(&node.text);
            out.push_str("-->");
        }
        Kind::ProcessingInstruction => {
            out.push_str("<?");
            out.push_str(&node.name);
            out.push(' ');
            out.push_str(&node.text);
            out.push('>');
        }
        Kind::Element => {
            out.push('<');
            out.push_str(&node.name);
            for (name, value) in &node.attributes {
                out.push(' ');
                out.push_str(name);
                out.push_str("=\"");
                escape_into(value, true, out);
                out.push('"');
            }
            out.push('>');
            if VOID.contains(&node.name.as_str()) {
                return;
            }
            // The parse of one of these three drops a leading newline, so a
            // serialiser that did not put one back would shorten the text by
            // one character every round trip.
            if matches!(node.name.as_str(), "pre" | "textarea" | "listing")
                && node
                    .children
                    .first()
                    .is_some_and(|first| first.kind == Kind::Text && first.text.starts_with('\n'))
            {
                out.push('\n');
            }
            work.push(Step::End(&node.name));
            stacked(node, RAW_TEXT.contains(&node.name.as_str()), work);
        }
    }
}

/// `text` appended to `out` under the algorithm's escape, where `attribute`
/// says which of its two sets applies.
///
/// **Not [`escaped`]'s five**, and the difference is not an oversight in
/// either direction. That set is a *launderer's*: it answers text that is safe
/// wherever it is dropped, including inside an unquoted attribute, so it
/// escapes both quote characters. This one is the *serialiser's*: it writes
/// every attribute value between double quotes itself, so a `'` in one means
/// an apostrophe and escaping it would put `&#39;` where the document said a
/// character. Escaping `<` and `>` in an attribute would do the same, which is
/// why the two sets are disjoint on four of the six characters between them.
fn escape_into(text: &str, attribute: bool, out: &mut String) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            // The one non-delimiter in either set: a no-break space is
            // indistinguishable from a space in source, and a round trip that
            // wrote it raw would turn it into one.
            '\u{A0}' => out.push_str("&nbsp;"),
            '"' if attribute => out.push_str("&quot;"),
            '<' if !attribute => out.push_str("&lt;"),
            '>' if !attribute => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
}

// ============================================================================
// The allowlist, and the rebuild that is `sanitize`
// ============================================================================

/// Every element `sanitize` writes, and the attributes each may carry beyond
/// [`GLOBAL`]'s.
///
/// **Closed, and sorted, and neither is incidental.** Closed is
/// `rule:core-classes/html-sanitize`: a sanitizer whose policy its caller
/// writes is a sanitizer whose holes are the caller's, and this member takes no
/// argument that could reach this list. An element a real application needs is
/// added here, in a commit that says why. Sorted is what makes
/// [`allowed`]'s binary search total, and
/// `the_allowlist_is_closed_and_is_not_configurable_by_a_caller` is what holds
/// the order.
///
/// What is *not* here is as decided as what is. No `class` and no `id`, the
/// two attributes a receiving page **selects on**. `id` is the DOM-clobbering
/// surface, where an attribute name shadows a property a page's own script
/// reads. `class` is the first attribute a real application asks for and is
/// refused for the reason that survives asking: its value chooses which of the
/// receiving page's own rules and handlers apply to attacker-supplied content
/// — a stylesheet that positions `.modal-overlay` over the page, a script that
/// binds a handler to every `.delete`. Neither can be met with a value test
/// here, because the meaning of the value lives in a document this member
/// never sees, and a test it cannot write is a hole
/// [`GLOBAL`]'s three deliberately do not open. An application that needs this
/// content styled styles the element it puts the answer *inside*, which is
/// markup it wrote itself. No `style`, which is a second grammar with its own
/// injection story and no parser here. No `target`, whose `_blank` hands the
/// opened page a reference back. Nothing from SVG or MathML, whose namespaces
/// are where mXSS lives, since a name that case-corrects on parse is a name
/// whose serialisation and reparse can disagree.
const ELEMENTS: &[(&str, &[&str])] = &[
    ("a", &["href"]),
    ("abbr", &[]),
    ("b", &[]),
    ("blockquote", &["cite"]),
    ("br", &[]),
    ("caption", &[]),
    ("cite", &[]),
    ("code", &[]),
    ("col", &["span"]),
    ("colgroup", &["span"]),
    ("dd", &[]),
    ("del", &["cite", "datetime"]),
    ("dfn", &[]),
    ("div", &[]),
    ("dl", &[]),
    ("dt", &[]),
    ("em", &[]),
    ("figcaption", &[]),
    ("figure", &[]),
    ("h1", &[]),
    ("h2", &[]),
    ("h3", &[]),
    ("h4", &[]),
    ("h5", &[]),
    ("h6", &[]),
    ("hr", &[]),
    ("i", &[]),
    ("img", &["alt", "height", "src", "width"]),
    ("ins", &["cite", "datetime"]),
    ("kbd", &[]),
    ("li", &["value"]),
    ("mark", &[]),
    ("ol", &["start"]),
    ("p", &[]),
    ("pre", &[]),
    ("q", &["cite"]),
    ("s", &[]),
    ("samp", &[]),
    ("small", &[]),
    ("span", &[]),
    ("strong", &[]),
    ("sub", &[]),
    ("sup", &[]),
    ("table", &[]),
    ("tbody", &[]),
    ("td", &["colspan", "rowspan"]),
    ("tfoot", &[]),
    ("th", &["colspan", "rowspan", "scope"]),
    ("thead", &[]),
    ("tr", &[]),
    ("u", &[]),
    ("ul", &[]),
    ("var", &[]),
    ("wbr", &[]),
];

/// The attributes any allowlisted element may carry.
///
/// Three, and each says something about the text rather than about the page:
/// what it is called, what language it is in, and which way it reads. None of
/// them is a URL, a script or a style, so none needs a value test.
const GLOBAL: &[&str] = &["dir", "lang", "title"];

/// The attributes whose value is a URL, and therefore the ones [`addressable`]
/// has an opinion about.
const URL: &[&str] = &["cite", "href", "src"];

/// The schemes a URL attribute may name.
///
/// The two that fetch and the one that composes a message. `javascript:` and
/// `data:` are the two that *run*, and there is no version of this list that
/// admits either — which is why the test is a list of what may run rather than
/// a list of what may not, the same shape as the element list one level up.
const SCHEMES: &[&str] = &["http", "https", "mailto"];

/// The attributes `name` may carry, or `None` for an element the allowlist
/// does not hold.
fn allowed(name: &str) -> Option<&'static [&'static str]> {
    ELEMENTS
        .binary_search_by(|(element, _)| (*element).cmp(name))
        .ok()
        .map(|at| ELEMENTS[at].1)
}

/// Whether `value` names an address the rebuild is willing to write.
///
/// The test is on the **scheme**, because that is the whole of what makes a URL
/// attribute a code-execution sink. A value naming no scheme at all is a
/// relative reference: it resolves against the page that received it and can
/// reach nothing the page could not already.
///
/// Tab, line feed and carriage return come out before the scheme is read,
/// because every browser removes them from a URL and `java&#9;script:` is the
/// oldest bypass there is. A colon that arrives after a `/`, `?` or `#` is
/// punctuation inside a path rather than a scheme's own, which is how a URL
/// parser reads it too.
fn addressable(value: &str) -> bool {
    let value: String = value
        .chars()
        .filter(|ch| !matches!(ch, '\t' | '\n' | '\r'))
        .collect();
    let value = value.trim_start_matches(|ch: char| ch <= ' ');
    let Some(colon) = value.find(':') else {
        return true;
    };
    let scheme = &value[..colon];
    scheme.contains(['/', '?', '#'])
        || SCHEMES
            .iter()
            .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
}

/// What the rebuild does with one node it has reached.
enum Verdict {
    /// The node is not written, and neither is anything under it.
    Dropped,
    /// The node is written whole and has nothing under it to walk.
    Leaf(Parsed),
    /// The node opens: what is written is here, and its children are still to
    /// be decided. A [`Kind::Document`] is the tree's own word for a bag of
    /// children with no element of its own, which is what an unwrapped node
    /// leaves behind.
    Opened(Parsed),
}

/// What [`rebuilt`] does with `node`.
fn verdict(node: &Parsed) -> Verdict {
    match node.kind {
        Kind::Text => {
            let mut built = Parsed::new(Kind::Text);
            built.text.push_str(&node.text);
            Verdict::Leaf(built)
        }
        // A comment is not content, and its data is written back with no
        // escape at all ([`written_out`]), so a `-->` inside one would close
        // it early and everything after it would reparse as markup. That is
        // the mXSS shape exactly, and dropping comments is what makes the
        // round trip a fixed point rather than something to argue about.
        Kind::Comment | Kind::ProcessingInstruction => Verdict::Dropped,
        Kind::Document => Verdict::Opened(Parsed::new(Kind::Document)),
        Kind::Element => match allowed(&node.name) {
            Some(attributes) => {
                let mut built = Parsed::new(Kind::Element);
                built.name.push_str(&node.name);
                built.attributes = node
                    .attributes
                    .iter()
                    .filter(|(name, value)| {
                        (GLOBAL.contains(&name.as_str()) || attributes.contains(&name.as_str()))
                            && (!URL.contains(&name.as_str()) || addressable(value))
                    })
                    .cloned()
                    .collect();
                Verdict::Opened(built)
            }
            // A raw-text element's children are neither markup nor text —
            // [`RAW_TEXT`] is where that is written down — so keeping them
            // would be writing a program's source into a document as if it
            // were prose. A `<template>` goes the same way: its content is
            // inert where it sits and is markup again the moment anything
            // clones it.
            None if RAW_TEXT.contains(&node.name.as_str()) || node.name == "template" => {
                Verdict::Dropped
            }
            // Everything else is unwrapped rather than dropped, and that is
            // what keeps the member usable: `html`, `head` and `body` are
            // elements the algorithm inserts around any fragment at all, so
            // dropping an unknown element with its content would drop every
            // document. What is under one is ordinary content that has been
            // decided on its own terms one turn of the walk later.
            None => Verdict::Opened(Parsed::new(Kind::Document)),
        },
    }
}

/// `tree` rebuilt from [`ELEMENTS`] — every node that survives, in the order
/// it was written, and nothing else.
///
/// **A rebuild rather than a filter**, which `rule:core-classes/html-sanitize`
/// is about: nothing here reads the source text, deletes from it or repairs
/// it. A node either has a place in the grammar this list describes, in which
/// case it is built afresh from its name and the attributes that list holds, or
/// it has none and contributes no element. What comes back is therefore
/// something this module could have built from nothing, which is what makes the
/// serialisation of it predictable — the property "what looks dangerous" can
/// never have, since that list is one an attacker extends.
///
/// Iterative for [`Step`]'s reason: the tree under it is a parse's, so it is
/// [`DEPTH_CEILING`] deep at its deepest.
fn rebuilt(tree: &Parsed) -> Parsed {
    let mut stack = vec![(tree.children.iter(), Parsed::new(Kind::Document))];
    loop {
        let next = stack
            .last_mut()
            .expect("the root frame is popped by the return below")
            .0
            .next();
        if let Some(node) = next {
            match verdict(node) {
                Verdict::Dropped => {}
                Verdict::Leaf(built) => stack
                    .last_mut()
                    .expect("the frame the child was read from is still open")
                    .1
                    .children
                    .push(built),
                Verdict::Opened(built) => stack.push((node.children.iter(), built)),
            }
            continue;
        }
        let (_, built) = stack.pop().expect("the frame just read to its end");
        let Some(parent) = stack.last_mut() else {
            return built;
        };
        if built.kind == Kind::Document {
            parent.1.children.extend(built.children);
        } else {
            parent.1.children.push(built);
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Html::sanitize(tainted string $document): Core\Html\Markup` —
    /// `rule:core-classes/html-sanitize`'s launderer, and the second one on
    /// this class.
    ///
    /// Three members in a row and each is one of the three things a document
    /// can be: [`parse`] reads it, [`rebuilt`] decides what of it survives, and
    /// [`source`] writes what is left. The middle one is the only one with a
    /// policy in it, which is why the other two are written against the
    /// algorithm alone and can be read without it.
    ///
    /// **Why the answer is a carrier** is `rule:security/launderer-answers-a-carrier`,
    /// the same predicate [`nvs_core_html_escape`] answers: the HTML sink
    /// launders on its own, so a `string` answer here would be escaped a second
    /// time and a rebuilt document would arrive as its own source text.
    ///
    /// **What it spends:** the parse's tree, plus a second tree holding what
    /// survived, plus the source it is written back to — all three proportional
    /// to the document, all three attributed to the request, and the first two
    /// dropped before this returns. `rule:programs/memory-priority` is what
    /// buys that: the rebuild is a tree operation because the alternative is a
    /// pass over text, and a pass over text is what every sanitizer that has
    /// been bypassed was.
    fn nvs_core_html_sanitize(_ctx, args: [1]) {
        let document = text(&args[0], r"`Core\Html::sanitize`'s `$document`")?;
        let out = source(&rebuilt(&parse(document)));
        Ok(crate::instance::build(
            &MARKUP,
            [Value::str(NvsStr::new(out.as_bytes()))],
        ))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsArray, OutputSink, call};

    use super::*;
    use crate::registry::CLASSES;

    /// A `Core\Html\Markup` over `source`, built the way every member that
    /// answers one builds it — the carrier's one slot holding the bytes.
    fn markup(source: &[u8]) -> Value {
        crate::instance::build(&MARKUP, [Value::str(NvsStr::new(source))])
    }

    /// The bytes a `Core\Html\Markup` carries, read back out of that slot.
    ///
    /// It asserts what it reads: a value this fails on is not a carrier at
    /// all, which is the half of "answers a `Markup`" a byte comparison alone
    /// would miss.
    fn carried(value: Value) -> String {
        let held =
            markup_slot(value, "a test's markup").expect("the value is a `Core\\Html\\Markup`");
        text(&held, "a test's markup")
            .expect("a carrier's slot holds text")
            .to_owned()
    }

    /// `Core\Html::join`, end to end through the `rule:errors/propagation`
    /// boundary compiled code reaches it at: the parts land in the order the
    /// list holds them, and the separator lands between each pair and nowhere
    /// else — not before the first part and not after the last.
    // covers: Core\Html::join
    #[test]
    fn html_join_writes_every_part_in_order_with_its_separator_between() {
        let mut list = NvsArray::new();
        list.append(markup(b"<li>one</li>"));
        list.append(markup(b"<li>two</li>"));
        list.append(markup(b"<li>three</li>"));
        let parts = Value::array(list);
        let separator = markup(b"<br>");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let joined = call(super::nvs_core_html_join, &mut ctx, &[parts, separator])
            .expect("joining carriers never fails");
        assert_eq!(
            carried(joined),
            "<li>one</li><br><li>two</li><br><li>three</li>"
        );

        // A `Markup` is a value type, so the composition left every operand
        // where it was — the same claim `Markup + Markup` makes of its pair.
        assert_eq!(carried(separator), "<br>");

        #[expect(
            unsafe_code,
            reason = "this test owns the references it built above, and the \
                      helper borrowed rather than consumed them"
        )]
        unsafe {
            joined.release();
            separator.release();
            parts.release();
        }
    }

    /// The boundary the walk has to get right on its own: with no part to
    /// write, there is no pair for the separator to go between, so the answer
    /// is a carrier holding nothing rather than the separator by itself.
    #[test]
    fn html_join_over_an_empty_list_answers_an_empty_markup() {
        let parts = Value::array(NvsArray::new());
        let separator = markup(b"<br>");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let joined = call(super::nvs_core_html_join, &mut ctx, &[parts, separator])
            .expect("joining nothing never fails");
        // [`carried`] is what asserts the answer is a `Markup` and not a
        // `null` standing in for "there was nothing to build".
        assert_eq!(carried(joined), "");

        #[expect(
            unsafe_code,
            reason = "this test owns the references it built above, and the \
                      helper borrowed rather than consumed them"
        )]
        unsafe {
            joined.release();
            separator.release();
            parts.release();
        }
    }

    /// `rule:core-classes/html-literal`'s "it neither trusts nor escapes
    /// anything", in the five characters that would show it: every part and
    /// the separator arrive as carriers, so each one's bytes come back exactly
    /// as they were written.
    ///
    /// The contrast is the assertion worth having. `Core\Html::escape` over
    /// the same text writes the references, so a `join` that escaped would be
    /// escaping a fragment a second time — which is what
    /// `rule:security/launderer-answers-a-carrier` says the carrier exists to
    /// make unreachable.
    #[test]
    fn html_join_escapes_nothing_because_every_part_is_already_a_carrier() {
        let mut list = NvsArray::new();
        list.append(markup(b"<b>a & b</b>"));
        list.append(markup(b"<i>'q' > \"p\"</i>"));
        let parts = Value::array(list);
        let separator = markup(b" & ");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let joined = call(super::nvs_core_html_join, &mut ctx, &[parts, separator])
            .expect("joining carriers never fails");
        assert_eq!(carried(joined), "<b>a & b</b> & <i>'q' > \"p\"</i>");

        let text = Value::str(NvsStr::new(b"a & b"));
        let escaped = call(super::nvs_core_html_escape, &mut ctx, &[text])
            .expect("escaping text never fails");
        assert_eq!(
            carried(escaped),
            "a &amp; b",
            "the member that does write the references is the launderer, and \
             it is the only one"
        );

        #[expect(
            unsafe_code,
            reason = "this test owns the references it built above, and the \
                      helpers borrowed rather than consumed them"
        )]
        unsafe {
            escaped.release();
            text.release();
            joined.release();
            separator.release();
            parts.release();
        }
    }

    /// `Core\Html::escape` end to end through the boundary compiled code
    /// reaches it at: the five characters and an unterminated directional
    /// control are rewritten in one pass, and a text with nothing to rewrite
    /// comes back in a carrier holding exactly its own bytes.
    // covers: Core\Html::escape
    #[test]
    fn html_escape_writes_five_references_and_replaces_an_open_control() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let text = Value::str(NvsStr::new("<a title='x'>\"&\"</a>\u{202E}!".as_bytes()));
        let escaped = call(super::nvs_core_html_escape, &mut ctx, &[text])
            .expect("escaping text never fails");
        assert_eq!(
            carried(escaped),
            "&lt;a title=&#39;x&#39;&gt;&quot;&amp;&quot;&lt;/a&gt;\u{FFFD}!"
        );

        let plain = Value::str(NvsStr::new(b"Ada Lovelace"));
        let unchanged = call(super::nvs_core_html_escape, &mut ctx, &[plain])
            .expect("escaping text never fails");
        assert_eq!(carried(unchanged), "Ada Lovelace");

        #[expect(
            unsafe_code,
            reason = "this test owns the references it built above, and the \
                      member borrowed rather than consumed them"
        )]
        unsafe {
            unchanged.release();
            plain.release();
            escaped.release();
            text.release();
        }
    }

    /// `Core\Html::sanitize` end to end through the boundary compiled code
    /// reaches it at: a raw-text element goes with its content, an element the
    /// list lacks leaves its text behind, an attribute the list does not grant
    /// and a `href` naming a scheme that runs are not written, and the
    /// wrapper the parse adds round a fragment is not in the answer. Handing
    /// the answer's own bytes back changes nothing, which is the fixed point
    /// the member is written around, asked here of the carrier a program gets.
    // covers: Core\Html::sanitize
    #[test]
    fn html_sanitize_rebuilds_a_fragment_and_its_answer_is_a_fixed_point() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let document = Value::str(NvsStr::new(
            b"<p onclick=x() class=c>Hi <b>there</b><script>alert(1)</script></p>\
              <font>kept</font> <a href=\" javascript:alert(1)\">a</a> <a href=/help>b</a>",
        ));
        let once = call(super::nvs_core_html_sanitize, &mut ctx, &[document])
            .expect("the rebuild has no failure mode");
        let written = carried(once);
        assert_eq!(
            written,
            "<p>Hi <b>there</b></p>kept <a>a</a> <a href=\"/help\">b</a>"
        );

        let again = Value::str(NvsStr::new(written.as_bytes()));
        let twice = call(super::nvs_core_html_sanitize, &mut ctx, &[again])
            .expect("the rebuild has no failure mode");
        assert_eq!(carried(twice), written);

        #[expect(
            unsafe_code,
            reason = "this test owns the references it built above, and the \
                      member borrowed rather than consumed them"
        )]
        unsafe {
            twice.release();
            again.release();
            once.release();
            document.release();
        }
    }

    /// `Core\Html::toSource` end to end through the boundary compiled code
    /// reaches it at: the answer is a plain string holding the carrier's bytes
    /// unchanged — no reference is decoded — and the carrier still holds the
    /// same bytes afterwards. An empty reason is refused before the carrier is
    /// read, so a call with nothing written at the site never gets the bytes.
    // covers: Core\Html::toSource
    #[test]
    fn html_to_source_answers_the_carried_bytes_and_refuses_an_empty_reason() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let fragment = markup(b"<b>Tom &amp; Jerry</b>");
        let reason = Value::str(NvsStr::new(b"storing the rendered card"));
        let source = call(
            super::nvs_core_html_to_source,
            &mut ctx,
            &[fragment, reason],
        )
        .expect("a written reason is accepted");
        assert_eq!(
            text(&source, "the answer").expect("the answer is a string"),
            "<b>Tom &amp; Jerry</b>"
        );
        assert_eq!(carried(fragment), "<b>Tom &amp; Jerry</b>");

        let empty = Value::str(NvsStr::new(b""));
        assert!(call(super::nvs_core_html_to_source, &mut ctx, &[fragment, empty]).is_err());
        let refused = ctx.take_pending().expect("an empty reason throws");
        assert!(refused.contains("an empty one is refused"), "{refused}");

        #[expect(
            unsafe_code,
            reason = "this test owns the references it built above, and the \
                      member borrowed rather than consumed them"
        )]
        unsafe {
            empty.release();
            source.release();
            reason.release();
            fragment.release();
        }
    }

    /// `rule:core-classes/html-auto-escape`'s carrier, in the two facts neither crate that acts on it
    /// can check for itself: `nvs_runtime::CARRIER_TEXT_SLOT` is the index
    /// this class's registered layout gives `text`, and this class's name is
    /// one `nvs_runtime` renders raw. `Core\Cli\Text` asserts the same pair
    /// one sink over, which is what makes a carrier a *set* rather than a
    /// special case.
    ///
    /// The third assertion is § 5's own shape: the class is **memberless**,
    /// because a `Markup` is obtained where the trust is written or from a
    /// member of [`CLASS`] over operands that already carry it, and a
    /// constructor *on the carrier* would take a runtime string instead. The
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
            "a `Markup` is obtained from a markup literal, from `as` on a source literal, \
             from `+` or `Core\\Html::join` over fragments already trusted, from the \
             launderer, or from the sink's own escape-and-lift — never from a member on \
             the carrier itself"
        );
    }

    /// `rule:security/launderers-are-sink-named`, asked of the registry rather than of the body: `escape`
    /// launders, its answer is unqualified, and the qualifier it removes is
    /// `tainted` and not the other axis.
    ///
    /// The **and for no other** half is what makes this more than a row read
    /// back. § 3's rule is that a launderer is narrow and *sink-named*, so the
    /// claim under test is a set: the members spelled for the HTML sink are
    /// [`CLASS`]'s `escape` and `sanitize` and nothing else in the registry,
    /// and a third row here fails this on the day it is added — which is the
    /// day the decision to widen the sink's escape hatch is actually being
    /// made.
    ///
    /// Two launderers on one class is not the catch-all § 3 refuses, and the
    /// distinction is the one that section draws: what it forbids is a
    /// `sanitize()` or `clean()` that names *no* sink, on the argument that a
    /// value safe for HTML text is not safe for a shell argument. Both of
    /// these name this sink, and they launder different things into it —
    /// `escape` takes text that must arrive as text, `sanitize` takes markup
    /// that must arrive as markup (`rule:core-classes/html-sanitize`). Neither
    /// would do the other's job, which is what "narrow" means.
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
        // its own doc comment; this asserts which of them are the HTML sink's,
        // so a launderer appearing anywhere else for this sink — or a third one
        // here — is a failure rather than a quiet widening.
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
            vec!["escape", "sanitize"],
            "`rule:security/launderers-are-sink-named`'s launderers for the HTML sink are these two, and both are named for it"
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
    /// both directions. The HTML sink's two are the only yes, and for one
    /// reason: that sink auto-escapes, and neither transform is idempotent —
    /// `&` → `&amp;` → `&amp;amp;` for `escape`, and a rebuilt document
    /// arriving as its own source text for `sanitize`. `Core\Cli::escape` is
    /// the near miss the name-half of this
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
            (r"Core\Html::sanitize", true),
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

    /// `Core\Html::parse`, end to end through the boundary compiled code
    /// reaches it at, and read back through the member a program writes the
    /// tree out with: the tags a fragment left out are supplied, and an item
    /// nobody closed is closed where the next one opens.
    ///
    /// The empty document is the other end of the same claim — no input at
    /// all still answers the three elements every document has.
    // covers: Core\Html::parse
    #[test]
    fn html_parse_answers_a_whole_document_for_a_fragment_and_for_nothing() {
        let cases: [(&[u8], &str); 2] = [
            (
                b"<ul><li>one<li>two</ul>",
                "<html><head></head><body><ul><li>one</li><li>two</li></ul></body></html>",
            ),
            (b"", "<html><head></head><body></body></html>"),
        ];
        let mut ctx = Ctx::new(OutputSink::Sink);
        for (document, expected) in cases {
            let source = Value::str(NvsStr::new(document));
            let tree = call(super::nvs_core_html_parse, &mut ctx, &[source])
                .expect("the parse has no failure mode");
            let written = call(crate::xml::nvs_core_xml_node_source, &mut ctx, &[tree])
                .expect("a parsed document writes back out");
            assert_eq!(written.as_text(), Some(expected));
            #[expect(
                unsafe_code,
                reason = "this test owns the three references it built or was handed, and \
                          neither member consumed its argument"
            )]
            unsafe {
                written.release();
                tree.release();
                source.release();
            }
        }
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

    /// `rule:core-classes/html-sanitize`'s member as `rule:security/tainted-qualifier`
    /// sees it: the qualifier comes off a `tainted` argument, and what it
    /// answers is the sink's carrier rather than a `string`.
    ///
    /// *Beside* `escape` is the part worth asserting. § 3 admits a launderer
    /// only when it is narrow and names its sink, so this row has to be the
    /// same *shape* as the one that section works through — same qualifier on
    /// the way in, same carrier on the way out — and differ only in what it
    /// launders. A `sanitize` answering a plain `string` would be the shape
    /// `rule:security/launderer-answers-a-carrier` refuses for this sink, and
    /// its output would be escaped a second time on the way into a page.
    ///
    /// The last assertion is the one that fails at *run* time when it is
    /// missed: a symbol with no [`address`] arm links and then panics naming
    /// itself the first time a program calls the member.
    #[test]
    fn sanitize_is_an_adr_0024_launderer_beside_escape() {
        let sanitize = CLASS
            .members()
            .find(|method| method.name == "sanitize")
            .expect("`Core\\Html::sanitize` is registered");

        assert!(
            matches!(sanitize.params, [CoreTy::Text(Qual::Launder)]),
            "`sanitize` takes one `Qual::Launder` text parameter, not {}",
            sanitize.params.len()
        );
        assert!(
            matches!(sanitize.return_ty, CoreTy::Instance(name) if name == MARKUP_NAME),
            "`sanitize` answers the HTML sink's carrier — `rule:security/launderer-answers-a-carrier`"
        );
        assert!(
            !sanitize
                .params
                .iter()
                .any(|param| matches!(param, CoreTy::Text(Qual::Reveal))),
            "rebuilding a document neutralizes injection, not confidentiality"
        );
        assert!(
            sanitize.doc.is_some(),
            "`rule:core-api/reference-card` gives every row a card"
        );
        assert!(
            address(sanitize.symbol).is_some(),
            "`{}` resolves, or the first call to the member panics naming it",
            sanitize.symbol
        );
    }

    /// `rule:core-classes/html-sanitize`'s *closed* half, which is a property
    /// of the table and of the row together rather than of any one line.
    ///
    /// The row half is that nothing a caller writes can reach the policy: one
    /// parameter, which is the document, and no default — so there is no
    /// options bag, no flag and no list to pass. The table half is four
    /// invariants, each of which a plausible-looking edit breaks silently.
    /// Sorted is what makes [`allowed`]'s binary search total, and an
    /// out-of-order name is not refused anywhere — it is simply never found.
    /// Disjoint from [`RAW_TEXT`] is what keeps the serialiser's literal-text
    /// branch unreachable from this member's own output, which is the mXSS
    /// property one level down. The name test is the denylist that would
    /// otherwise be a *policy*: an `on…` handler, a `style`, an `id` or a
    /// `target` cannot be added to a row without failing here, so widening the
    /// list stays a decision rather than a typo. And every URL attribute has to
    /// appear in the table, or [`addressable`] is guarding a name nothing grants.
    #[test]
    fn the_allowlist_is_closed_and_is_not_configurable_by_a_caller() {
        let sanitize = CLASS
            .members()
            .find(|method| method.name == "sanitize")
            .expect("`Core\\Html::sanitize` is registered");
        assert_eq!(
            sanitize.names,
            ["document"],
            "the document, and nothing that describes what to do with it"
        );
        assert!(
            sanitize.defaults.is_empty(),
            "a default is a knob, and this member has none"
        );

        let sorted = ELEMENTS
            .windows(2)
            .filter(|pair| pair[0].0 < pair[1].0)
            .count();
        assert_eq!(
            sorted,
            ELEMENTS.len() - 1,
            "the list is searched by bisection, so an unsorted name is one that is never found"
        );

        let overlapping: Vec<&str> = ELEMENTS
            .iter()
            .map(|(element, _)| *element)
            .filter(|element| RAW_TEXT.contains(element) || *element == "template")
            .collect();
        assert!(
            overlapping.is_empty(),
            "an element whose content model is raw text is dropped, never kept: {overlapping:?}"
        );

        let named: Vec<&str> = ELEMENTS
            .iter()
            .flat_map(|(_, attributes)| attributes.iter().copied())
            .chain(GLOBAL.iter().copied())
            .filter(|attribute| {
                attribute.starts_with("on")
                    || matches!(*attribute, "style" | "id" | "class" | "target" | "srcdoc")
            })
            .collect();
        assert!(
            named.is_empty(),
            "a handler, a second grammar, a clobbering surface or a window \
             reference is not an attribute this list grants: {named:?}"
        );

        let guarded = URL
            .iter()
            .filter(|url| {
                ELEMENTS
                    .iter()
                    .any(|(_, attributes)| attributes.contains(url))
            })
            .count();
        assert_eq!(
            guarded,
            URL.len(),
            "every attribute the scheme test guards is one some element may carry"
        );
    }

    /// The acceptance property, and the reason
    /// `rule:core-classes/html-sanitize` asks for a rebuild rather than a
    /// filter: **mutation XSS**.
    ///
    /// Every payload below is a document whose *parse* differs from its
    /// source — a sanitizer that reads text and deletes from it approves the
    /// source, and the browser then builds elements the sanitizer never saw.
    /// The corpus is the shapes that class comes in: raw-text elements whose
    /// content reparses as markup (`listing`, `xmp`, `noscript`, `style`), the
    /// foreign-content integration points where namespace switching does the
    /// same, comments and CDATA that close early, and URL schemes that run.
    ///
    /// Two properties are asserted over the whole corpus by **counting**, so a
    /// member that answers plausibly for nineteen payloads and mutates on the
    /// twentieth fails here rather than reading correctly line by line.
    /// *Closed* is that reparsing the answer yields nothing outside
    /// [`ELEMENTS`] — the wrapper the algorithm puts round any fragment aside —
    /// which is the property that makes the answer safe to write into a page.
    /// *Fixed point* is that sanitizing the answer again changes nothing, which
    /// is the property that says the first answer is what a browser will
    /// actually build: if the two ever differ, the document the sanitizer
    /// approved is not the document that renders.
    #[test]
    fn parse_sanitize_serialise_reparse_reaches_a_fixed_point_over_the_mxss_corpus() {
        const CORPUS: &[&str] = &[
            "<listing>&lt;img src=x onerror=alert(1)&gt;</listing>",
            "<xmp><p>&lt;/xmp&gt;&lt;img src=x onerror=alert(1)&gt;</p></xmp>",
            "<noscript><p title=\"</noscript><img src=x onerror=alert(1)>\">",
            "<style><img src=x onerror=alert(1)></style>",
            "<svg></p><style><a id=\"</style><img src=x onerror=alert(1)>\">",
            "<math><mtext><table><mglyph><style><!--</style><img src=x onerror=alert(1)>",
            "<form><math><mtext></form><form><mglyph><style></math><img src onerror=alert(1)>",
            "<!--><img src=x onerror=alert(1)>-->",
            "<![CDATA[<img src=x onerror=alert(1)>]]>",
            "<a href=\"javascript&colon;alert(1)\">x</a>",
            "<a href=\"java\tscript:alert(1)\">x</a>",
            "<a href=\"  JaVaScRiPt:alert(1)\">x</a>",
            "<img src=\"data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==\">",
            "<p title=\"&quot;><img src=x onerror=alert(1)>\">t</p>",
            "<div><p>&lt;/div&gt;&lt;script&gt;alert(1)&lt;/script&gt;</p></div>",
            "<template><script>alert(1)</script></template>",
            "<iframe srcdoc=\"&lt;script&gt;alert(1)&lt;/script&gt;\"></iframe>",
            "<table><td background=\"javascript:alert(1)\">x",
            "<b><noembed></b><img src=x onerror=alert(1)></noembed>",
            "<p>a\u{a0}b &amp;lt;img src=x onerror=alert(1)&amp;gt;</p>",
            "<pre>\n\nkept</pre>",
            "<textarea><p>&lt;/textarea&gt;&lt;img src=x onerror=alert(1)&gt;</p>",
        ];

        // The wrapper `Core\Html::parse` puts round any fragment at all, which
        // is in the reparse of every answer including the empty one and is not
        // in the answer itself.
        const WRAPPER: &[&str] = &["html", "head", "body"];

        let cleaned = |document: &str| source(&rebuilt(&parse(document)));

        // What failed rather than what printed: a crate the language server
        // links may write to neither stream, so the detail a counting test owes
        // its reader rides on the assertion itself
        // (`rule:ide/stdout-belongs-to-the-protocol`).
        let mut mutating = Vec::new();
        let mut escaping = Vec::new();
        let mut closed = 0;
        let mut fixed = 0;
        for payload in CORPUS {
            let once = cleaned(payload);
            let twice = cleaned(&once);
            if once == twice {
                fixed += 1;
            } else {
                mutating.push(format!("{payload:?}: {once:?} then {twice:?}"));
            }

            let reparsed = parse(&once);
            let mut escapes = vec![&reparsed];
            let mut stray = Vec::new();
            while let Some(node) = escapes.pop() {
                if node.kind == Kind::Element && !WRAPPER.contains(&node.name.as_str()) {
                    match allowed(&node.name) {
                        Some(attributes) => stray.extend(
                            node.attributes
                                .iter()
                                .map(|(name, _)| name.clone())
                                .filter(|name| {
                                    !GLOBAL.contains(&name.as_str())
                                        && !attributes.contains(&name.as_str())
                                }),
                        ),
                        None => stray.push(node.name.clone()),
                    }
                }
                escapes.extend(node.children.iter());
            }
            if stray.is_empty() {
                closed += 1;
            } else {
                escaping.push(format!("{payload:?} -> {stray:?} in {once:?}"));
            }
        }

        assert_eq!(
            fixed,
            CORPUS.len(),
            "sanitizing an answer again must change nothing, or the document \
             approved is not the document that renders: {mutating:?}"
        );
        assert_eq!(
            closed,
            CORPUS.len(),
            "the reparse of an answer holds nothing outside the allowlist: {escaping:?}"
        );
    }

    /// `rule:core-classes/html-sanitize`'s *rebuild, never filter* half, in the
    /// one behaviour that tells the two designs apart from outside.
    ///
    /// A filter that escapes what it does not like leaves the source in the
    /// page as visible text: `<script>alert(1)</script>` becomes
    /// `&lt;script&gt;…`, which a reader sees. A rebuild has nowhere to put it —
    /// the element simply has no place in the grammar, so it contributes no
    /// tag at all. The two halves below are the two answers that are *not* the
    /// same: a raw-text element goes with its content, because its character
    /// data was never text, and every other unlisted element is unwrapped,
    /// because `html`, `head` and `body` are elements the algorithm inserts
    /// round any fragment and dropping their content would drop every document.
    #[test]
    fn an_element_outside_the_allowlist_is_dropped_rather_than_escaped_in_place() {
        const CASES: &[(&str, &str)] = &[
            ("<script>alert(1)</script>", ""),
            ("<p>a<script>alert(1)</script>b</p>", "<p>ab</p>"),
            ("<style>p{color:red}</style><p>t</p>", "<p>t</p>"),
            ("<marquee>hello</marquee>", "hello"),
            ("<p onclick=\"boom()\">hi</p>", "<p>hi</p>"),
            ("<a href=\"javascript:alert(1)\">x</a>", "<a>x</a>"),
            ("<a href=\"/page\">x</a>", "<a href=\"/page\">x</a>"),
            ("<img src=x onerror=alert(1)>", "<img src=\"x\">"),
            ("<!-- <b>c</b> --><p>t</p>", "<p>t</p>"),
        ];

        for (document, want) in CASES {
            assert_eq!(
                &source(&rebuilt(&parse(document))),
                want,
                "sanitizing {document:?}"
            );
        }

        let escaped: Vec<&str> = CASES
            .iter()
            .filter(|(_, want)| want.contains("&lt;") || want.contains("&gt;"))
            .map(|(document, _)| *document)
            .collect();
        assert!(
            escaped.is_empty(),
            "an element with no place in the grammar leaves no tag, escaped or \
             otherwise: {escaped:?}"
        );
    }
}
