//! `Core\Html` — `rule:security/launderers-are-sink-named`'s narrow,
//! sink-named launderer over the sink `rule:core-classes/html-auto-escape`
//! makes out of HTML text, and `rule:core-classes/html-parsing`'s WHATWG
//! parser onto `Core\Xml`'s tree.
//!
//! The launderer rule writes `Core\Html::escape(tainted string): Core\Html\Markup` out as
//! *the* worked example of what a launderer is allowed to be: one member, one
//! sink, and a contract naming it. `sanitize` is the second member of that
//! shape here and the last — it names the same sink and launders the other
//! thing that arrives at it, untrusted markup rather than untrusted text. What
//! that rule refuses is a launderer naming **no** sink, and there is no generic
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
//! and [`nvs_core_html_to_source`] is the one way back to the bytes
//! (`rule:core-classes/html-to-source`). Every
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
//! the text still compiles, because `rule:core-classes/html-to-source`'s
//! argument is that the hatch be
//! *greppable and justified* rather than inline, which is
//! `Core\Secret::reveal`'s own position on the same question
//! ([`crate::secret`]).
//!
//! # Every door onto a `Markup`, and the sink that needs none
//!
//! [`MARKUP`] is registered *and* reachable: every way
//! `rule:core-classes/html-auto-escape` and `rule:core-classes/html-template`
//! give a program to obtain one is here — [`MARKUP_SYMBOL`] for `as Markup` on
//! a string literal, [`MARKUP_CONCAT_SYMBOL`] for `Markup + Markup`,
//! [`nvs_core_html_join`] for a list of fragments and one separator, the escape
//! itself, which `rule:security/launderer-answers-a-carrier` turned from the
//! first two's poor relation into the ordinary one, and the pair an html
//! template's own lowering reaches, [`ESCAPE_TEXT_SYMBOL`] and
//! [`MARKUP_TEXT_SYMBOL`], which answer a hole's bytes rather than a carrier
//! per hole.
//!
//! **A page built with `+` in a loop is linear.** `$page = $page + $row` on a
//! local lowers to [`MARKUP_APPEND_SYMBOL`] rather than
//! [`MARKUP_CONCAT_SYMBOL`] (`nvs_ir::lower::stmt`'s
//! `lower_carrier_self_append`), and where `$page` is the object's one owner
//! its text grows in place under `nvs_runtime::string`'s doubling, as a string
//! `.=` does. What that spends is the doubling's own: a page built this way
//! holds up to twice its bytes until it is freed, charged to the request
//! (`rule:programs/memory-priority`). Every other `+` copies both operands into
//! a new carrier, because something else may still read the left one.
//! `Core\Cli\Text` takes the same lowering through
//! [`crate::cli::TEXT_APPEND_SYMBOL`].
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
//! makes the answer safe in text and inside a quoted attribute of either
//! quote, and escaping one fewer character has never been the reason a page
//! was fast. It is **not** safe in an unquoted attribute: a space, a tab, `=`
//! or a backtick there ends the value or starts another attribute, and none of
//! them is escaped. A template hole in one is accepted and gets this same
//! escape, which is the blind spot `rule:core-classes/html-template` names and
//! declines to close; the attribute's quotes are the page's to write.
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
use std::collections::{HashMap, HashSet};
use std::panic::AssertUnwindSafe;

use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, QualName};
use nvs_runtime::{Fault, NvsStr, Tag, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};
use crate::xml::{DEPTH_CEILING, Kind, Parsed};

/// `Core\Html`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Makes HTML safe to put into a page. It escapes text, cleans untrusted HTML, and \
            parses HTML documents.",
};

/// `Core\Html\Markup`'s class card — `rule:core-api/reference-card`.
const MARKUP_CARD: ClassDoc = ClassDoc {
    short: "HTML that is safe to put into a page. You get it from an html template, \
            ``html`…` ``, or from `Core\\Html::escape`, never from a plain string.",
};

/// `rule:security/launderers-are-sink-named`'s launderer for the HTML sink, and `rule:core-classes/html-to-source`'s one way back
/// out of the carrier it answers.
///
/// `rule:security/launderers-are-sink-named`'s own worked example of a launderer, and here
/// because the class beneath it is the one this crate defers to
/// whenever a `tainted` value has to be written into a document: HTML is
/// the sink that escapes by default, and this is what it escapes with.
/// [`crate::html`] owns why `sanitize` and the WHATWG parser are not here.
// rank: 1030
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Html",
    doc: Some(&CARD),
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
            name: "later",
            names: &["fn"],
            params: &[
                CoreTy::CallableSig(&[], &CoreTy::Mixed),
                CoreTy::Options(LATER_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Instance(MARKUP_NAME),
            symbol: "nvs_core_html_later",
            doc: Some(&LATER_DOC),
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
/// member of [`CLASS`] that takes trusted operands: an html template, whose
/// segments are what the author typed; `as Markup` on a *string literal*,
/// which is the trust level that string already carried; `Markup + Markup`
/// and `Core\Html::join`, which compose fragments already trusted; the
/// launderer, which earns the carrier by escaping; and the sink's own
/// escape-and-lift of everything else. A constructor member *here* would be
/// none of those — it would take a runtime `string`, which is exactly the
/// bypass `rule:core-classes/html-auto-escape` closes ("compute the escape-defeating payload at
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
///
/// `rule:core-classes/html-auto-escape`'s carrier for that same sink, memberless: `nvs_runtime`
/// already renders it and `nvs_types` already refuses a `tainted` or
/// computed conversion to it, so what this row adds is the registered
/// layout the slot lives in. [`crate::html`] owns why it has no
/// constructor.
// rank: 1040
pub(crate) const MARKUP: CoreClass = CoreClass {
    name: MARKUP_NAME,
    doc: Some(&MARKUP_CARD),
    methods: &[],
    instance: &[],
    slots: &["text"],
    constants: &[],
};

/// The symbol `<string literal> as Core\Html\Markup` lowers to —
/// `rule:core-classes/html-auto-escape`'s lift of a trusted string literal into [`MARKUP`].
///
/// No [`CoreMethod`] row, for the same reason [`MARKUP`] has no members at
/// all: a member taking a `string` is precisely the runtime-computed bypass
/// the rule closes, so the only thing allowed to call this is the lowering of the
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
/// the spelling the rule gives composition, so the operator's own lowering is the
/// only thing allowed to reach this, and a member row would be a third way in
/// that took its operands from anywhere.
pub const MARKUP_CONCAT_SYMBOL: &str = "nvs_core_html_markup_concat";

/// The symbol `$page = $page + $b` lowers to where `$page` is a local holding a
/// `Markup` — [`MARKUP_CONCAT_SYMBOL`]'s composition, appending into the left
/// operand where nothing else holds it.
///
/// Row-less for [`MARKUP_CONCAT_SYMBOL`]'s reason. It is reached only from that
/// one statement shape, because writing into the left operand is sound only
/// where its holder is re-pointed at the answer.
pub const MARKUP_APPEND_SYMBOL: &str = "nvs_core_html_markup_append";

/// The symbol an html template's hole lowers to — `rule:core-classes/html-template`'s
/// escape, answering the escaped **bytes** where [`nvs_core_html_escape`]
/// answers a carrier.
///
/// One transformation, two answer shapes, and the position is what picks: a
/// hole is one piece of a template that becomes a single `Markup` holding the
/// joined bytes, so a carrier per hole would be a carrier the join unwraps
/// again. What that rule's *What it costs to run* promises is one object
/// allocation for the whole template, however many holes it has.
///
/// Row-less for [`MARKUP_SYMBOL`]'s reason: `escape` is the written spelling
/// and this is a lowering's, so the template's own lowering is the only thing
/// allowed to reach it.
pub const ESCAPE_TEXT_SYMBOL: &str = "nvs_core_html_escape_text";

/// The symbol an html template's `Markup`-holding hole lowers to — the raw
/// splice `rule:core-classes/html-template` grants it, answering the carrier's
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
    short: "Escapes `&`, `<`, `>`, `\"` and `'` in `$text` as character references. It also \
            replaces every unterminated bidirectional control character with `\u{FFFD}`. You can \
            write the result into a page, where a `tainted` string is not allowed.",
    params: &[ParamDoc {
        name: "text",
        desc: "The text to write into an HTML document. It is shown as text, not read as HTML.",
        shape: &[],
    }],
    ret: "A `Core\\Html\\Markup` with the escaped text. It is safe in element content and in an \
          attribute value with either kind of quotes. It is not a `string`, so the page does not \
          escape it a second time. `toSource` returns its text as a `string`. Text without the \
          five characters and without an unterminated control character does not change. The \
          five characters are always escaped, and no option turns this off. Input that already \
          looks like a reference is escaped again, because `&amp;` in the input is the text \
          `&amp;`.",
    errors: &[],
};

/// `Core\Html::join`'s reference card — `rule:core-api/reference-card`.
const JOIN_DOC: MethodDoc = MethodDoc {
    short: "Joins a list of `Core\\Html\\Markup` fragments in order, with `$separator` between \
            each two. It does the same as `Markup + Markup` for a whole list.",
    params: &[
        ParamDoc {
            name: "parts",
            desc: "The fragments to join, in list order.",
            shape: &[],
        },
        ParamDoc {
            name: "separator",
            desc: "The markup written between each two parts. It is not written before the first \
                   part or after the last. With empty markup, nothing is written between parts.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Html\\Markup` with every part in order. An empty list gives empty markup. \
          Nothing is escaped, because each part and the separator are already safe markup. \
          Escaping them again would break that markup.",
    errors: &[],
};

/// `Core\Html::later`'s options, in the order the ABI passes them.
const LATER_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "placeholder",
        ty: CoreTy::Instance(MARKUP_NAME),
        default: Const::Null,
    },
    CoreOption {
        name: "error",
        ty: CoreTy::Instance(MARKUP_NAME),
        default: Const::Null,
    },
    CoreOption {
        name: "deadline",
        ty: CoreTy::Instance(crate::time::DURATION_NAME),
        default: Const::Null,
    },
];

/// `Core\Html::later`'s reference card — `rule:core-api/reference-card`.
const LATER_DOC: MethodDoc = MethodDoc {
    short: "Runs `$fn` as a separate task and returns a placeholder. Write the placeholder where \
            the output belongs. When the page is finished, the output of `$fn` replaces the \
            placeholder. Several `later` calls run at the same time.",
    params: &[
        ParamDoc {
            name: "fn",
            desc: "The function to run. It takes no arguments. Its output is what it echoes, \
                   followed by the `Core\\Html\\Markup` it returns, if any.",
            shape: &[],
        },
        ParamDoc {
            name: "placeholder",
            desc: "The markup the page shows until the output is ready. The default is empty.",
            shape: &[],
        },
        ParamDoc {
            name: "error",
            desc: "The markup shown instead of the output when `$fn` throws an error or runs \
                   past its `deadline`. The default is empty.",
            shape: &[],
        },
        ParamDoc {
            name: "deadline",
            desc: "The longest time `$fn` may run. Without it, only the request's own limits \
                   apply.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Html\\Markup` placeholder. Outside an HTML response, `$fn` runs at once and \
          the result is its output.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "When the page writes the same placeholder twice. A placeholder that the page never \
               writes is not an error: its `$fn` does not run, and a warning is logged.",
    }],
};

/// `Core\Html::toSource`'s reference card — `rule:core-api/reference-card`.
const TO_SOURCE_DOC: MethodDoc = MethodDoc {
    short: "Returns the source text of a `Core\\Html\\Markup`. There is no `Markup as string` \
            cast, so this is the only way to get the text. Each call needs a written reason, so \
            it is easy to find in code.",
    params: &[
        ParamDoc {
            name: "markup",
            desc: "The markup whose text you need.",
            shape: &[],
        },
        ParamDoc {
            name: "reason",
            desc: "Why this code needs the text and not the markup, written for the next person \
                   who reads it. Nothing else reads it. An empty reason is not allowed.",
            shape: &[],
        },
    ],
    ret: "The source text of the markup, as a plain `string`. Typical uses are caching a \
          rendered fragment, storing it in a database column, or writing it to a file. Another \
          use is passing it to an output that is not HTML.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$reason` is empty. Each call must say why it needs the text.",
    }],
};

/// `Core\Html::parse`'s reference card — `rule:core-api/reference-card`.
const PARSE_DOC: MethodDoc = MethodDoc {
    short: "Parses `$document` as HTML with the WHATWG algorithm, which browsers use. It returns \
            the document node of the same kind of tree that `Core\\Xml::parse` builds.",
    params: &[ParamDoc {
        name: "document",
        desc: "The document text. Any text is a valid document, so this method never throws an \
               error.",
        shape: &[],
    }],
    ret: "The `Core\\Xml\\Node` of kind `Document`. Its children are the top level of the parsed \
          page. For any input that is not empty, this is one `html` element with `head` and \
          `body` in it. That is true even when the text does not contain these tags. Missing \
          tags, wrongly nested tags and stray content go exactly where a browser puts them. So \
          the tree is the page a browser would show. A `<!DOCTYPE …>` gives no node, because \
          the tree has no kind for it. Names are lower case in HTML. In SVG and MathML they get \
          the correct case, as the algorithm says. Elements nested deeper than 1000 levels are \
          placed at level 1000, so walking the tree can never run out of stack.",
    errors: &[],
};

/// `Core\Html::sanitize`'s reference card — `rule:core-api/reference-card`.
const SANITIZE_DOC: MethodDoc = MethodDoc {
    short: "Parses `$document` as HTML and builds a new document from only the allowed elements \
            and attributes. Use it for untrusted HTML. For untrusted text, use `escape`.",
    params: &[ParamDoc {
        name: "document",
        desc: "The HTML to rebuild. Any text is a valid document, so this method never throws \
               an error.",
        shape: &[],
    }],
    ret: "A `Core\\Html\\Markup` with the rebuilt document. You can write it into a page without \
          escaping it again. The method builds a new document and does not edit the old one. An \
          element that is not allowed loses its tag, but its content is kept. An attribute \
          that is not allowed is not written. A `script`, `style` or other raw-text element is \
          removed with its content. The list of allowed elements is fixed, and no argument can extend it. A list \
          you write yourself could have security holes. The text and the order of the kept \
          content stay as in the document.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_html_escape" => (nvs_core_html_escape as *const ()).cast(),
        "nvs_core_html_join" => (nvs_core_html_join as *const ()).cast(),
        "nvs_core_html_later" => (nvs_core_html_later as *const ()).cast(),
        "nvs_core_html_parse" => (nvs_core_html_parse as *const ()).cast(),
        "nvs_core_html_sanitize" => (nvs_core_html_sanitize as *const ()).cast(),
        "nvs_core_html_to_source" => (nvs_core_html_to_source as *const ()).cast(),
        MARKUP_SYMBOL => (nvs_core_html_markup as *const ()).cast(),
        MARKUP_CONCAT_SYMBOL => (nvs_core_html_markup_concat as *const ()).cast(),
        MARKUP_APPEND_SYMBOL => (nvs_core_html_markup_append as *const ()).cast(),
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
/// that cannot be switched off affordable in latency.
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
    Ok(Value::text(&escaped))
}

/// The bytes of an optional `Markup` option, or nothing for one the call did
/// not name.
fn option_bytes(value: Value, position: &str) -> Result<Vec<u8>, Fault> {
    if value.obj_ptr().is_none() {
        return Ok(Vec::new());
    }
    let slot = markup_slot(value, position)?;
    Ok(slot.as_str_bytes().unwrap_or_default().to_vec())
}

nvs_runtime::nvs_helper! {
    /// `Core\Html::later(callable $fn, {placeholder?, error?, deadline?}): Markup`
    /// — `rule:core-classes/html-later`.
    ///
    /// In an HTML response the body is a registration: the callable and its two
    /// fragments go to the request's slots and the answer is the placeholder.
    /// `nvs_runtime::later` owns when a slot runs and how its output reaches
    /// the page. Anywhere else there is no page to assemble, so the callable
    /// runs here and its output is the answer.
    fn nvs_core_html_later(ctx, args: [4]) {
        // Unreachable from source: the parameter is a callable, so `null` is
        // refused at the call.
        if args[0].tag() == Some(Tag::Null) {
            return Err(Fault::fatal(
                "Core\\Html::later expected a callable, got null".to_string(),
            ));
        }
        let placeholder = option_bytes(args[1], "`Core\\Html::later`'s `placeholder`")?;
        let error = option_bytes(args[2], "`Core\\Html::later`'s `error`")?;
        let deadline = if args[3].obj_ptr().is_some() {
            let nanos = crate::time::nanos_of(args, 3, "deadline")?;
            Some(std::time::Duration::from_nanos(u64::try_from(nanos).unwrap_or(0)))
        } else {
            None
        };
        if ctx.carrier() == MARKUP_NAME {
            #[expect(
                unsafe_code,
                reason = "the argument is borrowed from the caller's frame and the \
                          slot keeps it past this call, so it needs a reference of its own"
            )]
            // SAFETY: the caller's argument slot is live for this call; the
            // reference taken here is the one the slot releases.
            unsafe {
                args[0].retain();
            }
            let marker = ctx.register_later(args[0], &placeholder, &error, deadline);
            #[expect(
                clippy::disallowed_methods,
                reason = "ASCII markers and a token around the placeholder `Markup`'s string"
            )]
            let marker = Value::str(NvsStr::new(&marker));
            return Ok(crate::instance::build(&MARKUP, [marker]));
        }
        ctx.begin_capture();
        let answer = nvs_runtime::call_callable(ctx, args[0], &[]);
        let mut bytes = ctx.end_capture().unwrap_or_default();
        let answer = answer?;
        if answer.obj_ptr().is_some() {
            let slot = markup_slot(answer, "the `Markup` `Core\\Html::later`'s callable returned");
            if let Ok(slot) = slot {
                bytes.extend_from_slice(slot.as_text().unwrap_or_default().as_bytes());
            }
        }
        #[expect(
            unsafe_code,
            reason = "`call_callable` answered with a reference this frame owns"
        )]
        // SAFETY: as above, and nothing reads `answer` after this.
        unsafe {
            answer.release();
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| crate::out::not_text("Core\\Html::later"))?;
        Ok(crate::instance::build(&MARKUP, [Value::text(text)]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Html::escape(tainted string $text): Core\Html\Markup` — `rule:security/launderers-are-sink-named`
    /// 's launderer for the sink `rule:core-classes/html-auto-escape` describes, replacing
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
    /// An html template's hole, escaped — `rule:core-classes/html-template`'s
    /// hole rule, and the whole of what [`ESCAPE_TEXT_SYMBOL`] does.
    ///
    /// The same transform `Core\Html::escape` runs, answering the bytes it
    /// produced instead of a carrier holding them, because the piece this is
    /// one of is on its way into a carrier already.
    ///
    /// **What it spends:** nothing beyond the escape itself — no object, and no
    /// string where the hole's text had nothing to escape.
    fn nvs_core_html_escape_text(_ctx, args: [1]) {
        escaped_text(args[0], "an html template's hole")
    }
}

nvs_runtime::nvs_helper! {
    /// An html template's `Markup`-holding hole, spliced raw — the whole of
    /// what [`MARKUP_TEXT_SYMBOL`] does.
    ///
    /// **Nothing is escaped**, which is `rule:core-classes/html-template`'s rule
    /// rather than an omission: a hole already holding a carrier is
    /// `Markup + Markup` written in interpolation syntax, and escaping a
    /// fragment that passed whichever rule made it a `Markup` would corrupt the
    /// markup it was built for.
    ///
    /// **What it spends:** nothing. The slot's bytes travel under one more
    /// reference into the carrier the template is building, so a spliced
    /// fragment is not copied.
    fn nvs_core_html_markup_text(_ctx, args: [1]) {
        let slot = markup_slot(args[0], "an html template's spliced hole")?;
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
    /// computed (`E0417`), so by the time this runs the argument is a string
    /// literal the author wrote and the only thing left is to put it in the
    /// carrier's one slot. A body that re-asked the question would be asking
    /// it of a value that no longer remembers where it came from, which is
    /// exactly why `rule:core-classes/html-auto-escape` is a compile-time one.
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
    /// than an omission: the rule grants composition precisely because both
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
        markup_concat(args[0], args[1])
    }
}

/// [`nvs_core_html_markup_concat`]'s body: a new `Markup` holding both
/// operands' bytes, and neither operand touched.
fn markup_concat(left: Value, right: Value) -> Result<Value, Fault> {
    let left = markup_slot(left, "the left operand")?;
    let right = markup_slot(right, "the right operand")?;
    let left = text(&left, "the left operand of `Markup + Markup`")?;
    let right = text(&right, "the right operand of `Markup + Markup`")?;

    let mut out = String::with_capacity(left.len() + right.len());
    out.push_str(left);
    out.push_str(right);
    Ok(crate::instance::build(&MARKUP, [Value::text(&out)]))
}

nvs_runtime::nvs_helper! {
    /// `$page = $page + $b` over two `Core\Html\Markup` — the same composition
    /// as [`nvs_core_html_markup_concat`], and the whole of what
    /// [`MARKUP_APPEND_SYMBOL`] does.
    ///
    /// `nvs_ir` calls this only where the variable that holds the left operand
    /// is re-pointed at the answer. When that variable is the left object's one
    /// owner, nothing else can see the object, so its text grows in place and
    /// the answer is the same object with one more reference. The statement then
    /// releases the variable's old reference. Any other left operand, and a
    /// right operand that is the same object, takes the copy.
    ///
    /// **What it spends:** nothing per composition on the in-place path, where
    /// the text's doubling (`nvs_runtime::string`'s § *Capacity*) is the whole
    /// cost: a page built in a loop holds up to twice its bytes. The copy path
    /// spends what [`nvs_core_html_markup_concat`] does.
    fn nvs_core_html_markup_append(_ctx, args: [2]) {
        let carrier = crate::instance::receiver(args[0], &MARKUP, "the left operand")?;
        let right = markup_slot(args[1], "the right operand")?;
        text(&right, "the right operand of `Markup + Markup`")?;
        let left = crate::instance::slot(carrier, nvs_runtime::CARRIER_TEXT_SLOT);
        text(&left, "the left operand of `Markup + Markup`")?;
        if args[0].bits() == args[1].bits() || !crate::instance::sole_owner(carrier) {
            return markup_concat(args[0], args[1]);
        }
        crate::instance::append_to_text_slot(carrier, nvs_runtime::CARRIER_TEXT_SLOT, right);
        #[expect(
            unsafe_code,
            reason = "the left argument is a live object for the length of the call, \
                      and the answer is a second reference to it"
        )]
        unsafe {
            args[0].retain();
        }
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Html::join(array<Core\Html\Markup> $parts, Core\Html\Markup $separator): Core\Html\Markup`
    /// — `rule:core-classes/html-template`'s composition over a list.
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
    /// call, both charged to the request, whatever the list holds, and one
    /// pass over the list that sizes the string. That size is asked of the
    /// request's memory limit before any byte is copied, so a page that does
    /// not fit is a `FATAL` and not a buffer past the limit. No element
    /// is touched: a `Markup` is a value type, so every part and the separator
    /// read back exactly as they were passed.
    fn nvs_core_html_join(ctx, args: [2]) {
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

        // One counting pass sizes the page, so a list that repeats one long
        // fragment many times is refused before any byte is copied. The ask
        // is twice the page, because the buffer and the carrier's copy of it
        // are held at once. The breach is already recorded on a refusal.
        let mut size = 0_usize;
        for (at, part) in crate::str::Elements::of(parts).enumerate() {
            if at > 0 {
                size = size.saturating_add(separator.len());
            }
            let held = markup_slot(part, r"`Core\Html::join`'s `$parts` element")?;
            size = size.saturating_add(text(&held, r"`Core\Html::join`'s `$parts` element")?.len());
        }
        if !nvs_runtime::budget::affords(size.saturating_mul(2)) {
            return Err(ctx.memory_breach().unwrap_or_else(|| {
                // no case can reach this: the request ends at its memory
                // `FATAL`, so no program reads this text.
                // `html_join_past_the_memory_limit_is_refused_before_a_byte_is_copied`
                // asserts it.
                Fault::fatal(
                    "Core\\Html::join: the page does not fit in the request's memory limit"
                        .to_owned(),
                )
            }));
        }
        let mut out = String::with_capacity(size);
        for (at, part) in crate::str::Elements::of(parts).enumerate() {
            if at > 0 {
                out.push_str(separator);
            }
            let held = markup_slot(part, r"`Core\Html::join`'s `$parts` element")?;
            out.push_str(text(&held, r"`Core\Html::join`'s `$parts` element")?);
        }
        Ok(crate::instance::build(
            &MARKUP,
            [Value::text(&out)],
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
        node.text = copied(text);
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
    /// The attribute names of each element [`TreeSink::add_attrs_if_missing`]
    /// has added to, by handle.
    ///
    /// Only `<html>` and `<body>` are ever added to, and a document may repeat
    /// either tag as often as it likes. Building the set once per element is
    /// what keeps each repeat in proportion to its own attributes rather than
    /// to every attribute the element already has.
    attribute_names: RefCell<HashMap<usize, HashSet<String>>>,
}

impl Sink {
    /// A sink holding nothing but the document node every handle is relative
    /// to.
    fn new() -> Self {
        Self {
            nodes: RefCell::new(vec![Node::new(Kind::Document)]),
            attribute_names: RefCell::new(HashMap::new()),
        }
    }

    /// `node`, added to the arena, as the handle the builder will refer to it
    /// by.
    ///
    /// **Every node asks the request's memory limit first**, because a tree
    /// is not proportional to its source: reconstructing formatting elements
    /// clones a whole list of them for every paragraph, so a short document
    /// can build a tree many times its size, and the allocator's own compare
    /// is noticed only when the member returns. A `false` from
    /// [`nvs_runtime::budget::affords`] has already recorded the breach, and
    /// [`OverLimit`] unwinds out of `html5ever` to [`bounded`], because
    /// [`TreeSink`] has no way to stop a parse.
    ///
    /// What is asked is the arena's doubling when it is full, which is the
    /// largest single step a parse takes: two million nodes are about 300 MB
    /// of room. Each `create_` method asks for the node's own strings before
    /// it copies them.
    fn push(&self, node: Node) -> usize {
        let mut nodes = self.nodes.borrow_mut();
        afford(room(&nodes, 0));
        nodes.push(node);
        nodes.len() - 1
    }

    /// Which parent `target` hangs off and where among its children it sits, or
    /// `None` for a node with no parent.
    ///
    /// The search starts at the last child. Foster parenting asks for the
    /// place of the open `<table>`, which is its parent's last child, once for
    /// every node it inserts, so a search from the front would cost a table
    /// with many misplaced nodes time in the square of their number.
    fn locate(&self, target: usize) -> Option<(usize, usize)> {
        let nodes = self.nodes.borrow();
        let parent = nodes[target].parent?;
        let at = nodes[parent].children.iter().rposition(|&n| n == target)?;
        Some((parent, at))
    }

    /// Makes `child` the last of `parent`'s children.
    fn attach(&self, parent: usize, child: usize) {
        self.detach(child);
        let mut nodes = self.nodes.borrow_mut();
        afford(room(&nodes[parent].children, 0));
        nodes[child].parent = Some(parent);
        nodes[parent].children.push(child);
    }

    /// Puts `child` among `parent`'s children at `at`.
    fn insert(&self, parent: usize, at: usize, child: usize) {
        let mut nodes = self.nodes.borrow_mut();
        afford(room(&nodes[parent].children, 0));
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
        afford(grown(
            nodes[at].text.len(),
            nodes[at].text.capacity(),
            text.len(),
        ));
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
        afford(attrs.iter().fold(
            size_of::<(String, String)>() * attrs.len(),
            |bytes, attr| {
                bytes
                    .saturating_add(attr.name.local.len())
                    .saturating_add(attr.value.len())
            },
        ));
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
        node.text = copied(&text);
        self.push(node)
    }

    /// Unreachable from an HTML document — the tokenizer reads `<?…>` as a
    /// bogus comment — and written out anyway because the node family has this
    /// kind and `xml5ever` drives the same trait onto it.
    fn create_pi(&self, target: StrTendril, data: StrTendril) -> usize {
        let mut node = Node::new(Kind::ProcessingInstruction);
        node.name = copied(&target);
        node.text = copied(&data);
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
        if attrs.is_empty() {
            return;
        }
        let mut nodes = self.nodes.borrow_mut();
        let node = &mut nodes[*target];
        let mut index = self.attribute_names.borrow_mut();
        let had = index.entry(*target).or_insert_with(|| {
            node.attributes
                .iter()
                .map(|(name, _)| name.clone())
                .collect()
        });
        for attr in attrs {
            let name = written(&attr.name);
            let bytes = name
                .len()
                .saturating_mul(2)
                .saturating_add(attr.value.len());
            afford(room(&node.attributes, bytes));
            if had.insert(name.clone()) {
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

/// Unwinds with [`OverLimit`] unless the request can afford `bytes` more.
///
/// [`room`] and [`grown`] never answer zero, and the attribute sum counts at
/// least the empty vector's room, because an ask of zero bytes past the limit
/// records a refusal that names zero bytes.
fn afford(bytes: usize) {
    if !nvs_runtime::budget::affords(bytes) {
        std::panic::resume_unwind(Box::new(OverLimit));
    }
}

/// The bytes one more push onto `items` allocates, plus `extra` the item brings.
///
/// A full vector doubles, so that is the room asked for: a node of a million
/// children holds a million children's room, and the next push asks for twice
/// that.
fn room<T>(items: &Vec<T>, extra: usize) -> usize {
    let slots = if items.len() == items.capacity() {
        items.capacity().saturating_mul(2).max(4)
    } else {
        1
    };
    slots.saturating_mul(size_of::<T>()).saturating_add(extra)
}

/// The bytes a string of `len` bytes in `capacity` allocates to take `more`
/// bytes, and at least one.
fn grown(len: usize, capacity: usize, more: usize) -> usize {
    let wanted = len.saturating_add(more);
    if wanted > capacity {
        wanted.max(capacity.saturating_mul(2))
    } else {
        1
    }
}

/// `text` as an owned string, once the request can afford its bytes.
fn copied(text: &str) -> String {
    if !text.is_empty() {
        afford(text.len());
    }
    text.to_owned()
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
/// native stack, and security and request isolation are not traded for
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

/// How many elements may be open at once while a document is parsed.
///
/// `rule:core-classes/html-parsing` caps the stack of open elements here, as
/// WebKit does and at the depth Chromium, Firefox and WebKit all cap a tree
/// at: a start tag that arrives with this many elements open first closes the
/// current one, and the new element becomes its sibling. Every scope check
/// the tree builder makes walks that stack, so the cap is what keeps each
/// token's cost bounded and a parse's time in proportion to its document.
///
/// Under Miri the cap is 16. The tests that reach the cap parse a document
/// deeper than it, and at 512 each one takes longer than Miri's test timeout.
/// The code paths are the same at either depth, and every test here passes
/// natively with the cap at 16.
const OPEN_ELEMENTS_CAP: usize = if cfg!(miri) { 16 } else { 512 };

/// `document`, parsed by the WHATWG algorithm under [`OPEN_ELEMENTS_CAP`].
///
/// `ParseOpts::default()` leaves scripting *enabled*, which is what a browser
/// with JavaScript on does and therefore what the agreement this member is
/// built on is about: `<noscript>`'s content is raw text rather than markup.
/// It is also the safer of the two readings for anything built over this, since
/// content a scripting browser would never build elements from does not become
/// elements here either.
fn parse(document: &str) -> Parsed {
    html5ever::parse_document(Sink::new(), options()).one(document)
}

/// The options [`parse`] runs `html5ever` with.
fn options() -> html5ever::ParseOpts {
    let mut opts = html5ever::ParseOpts::default();
    opts.tree_builder.max_open_elements = Some(OPEN_ELEMENTS_CAP);
    opts
}

/// The payload [`Sink::push`] unwinds with when the request cannot afford one
/// more node. `resume_unwind` runs no panic hook, so nothing is printed.
struct OverLimit;

/// [`parse`], stopped at the request's memory limit.
///
/// The unwind is caught here and nowhere else. Every value it passes through
/// is dropped on the way out — the builder, its stack and the half-built
/// arena — so the request is left holding what it held before the call, and
/// any other panic keeps unwinding to the request's own boundary.
///
/// # Errors
///
/// A [`Fault::Fatal`] naming `member` when the tree would pass the limit.
/// [`nvs_runtime::run_helper`] reports the breach in its place.
fn bounded(document: &str, member: &str) -> Result<Parsed, Fault> {
    std::panic::catch_unwind(AssertUnwindSafe(|| parse(document))).map_err(|payload| {
        if payload.is::<OverLimit>() {
            Fault::fatal(format!(
                "{member}: the document's tree is larger than the request's memory limit allows"
            ))
        } else {
            std::panic::resume_unwind(payload)
        }
    })
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
        Ok(crate::xml::instance_of(bounded(document, r"Core\Html::parse")?, &[]))
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
/// attribute a code-execution sink, and on the **host**. A value naming no
/// scheme is a relative reference, and the one written here never names a
/// host: it resolves against the page that received it and can reach nothing
/// the page could not already.
///
/// So a value starting with two of `/` and `\` is not written. It is
/// scheme-relative and names whatever host follows, and a browser reads a
/// backslash there as a slash. `/path`, `?q` and `#frag` stay.
///
/// Tab, line feed and carriage return come out before either is read, because
/// every browser removes them from a URL and `java&#9;script:` is the oldest
/// bypass there is. A colon that arrives after a `/`, `?` or `#` is
/// punctuation inside a path rather than a scheme's own, which is how a URL
/// parser reads it too.
fn addressable(value: &str) -> bool {
    let value: String = value
        .chars()
        .filter(|ch| !matches!(ch, '\t' | '\n' | '\r'))
        .collect();
    let value = value.trim_start_matches(|ch: char| ch <= ' ');
    let slash = |ch: Option<char>| matches!(ch, Some('/' | '\\'));
    let mut first = value.chars();
    if slash(first.next()) && slash(first.next()) {
        return false;
    }
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
    /// survived, plus the source it is written back to — all three attributed
    /// to the request, and the first two dropped before this returns. The
    /// parse's tree is refused node by node at the memory limit ([`bounded`]);
    /// the other two are no larger than it, and a crossing there is noticed
    /// when this returns. `rule:programs/memory-priority` is what
    /// buys that: the rebuild is a tree operation because the alternative is a
    /// pass over text, and a pass over text is what every sanitizer that has
    /// been bypassed was.
    fn nvs_core_html_sanitize(_ctx, args: [1]) {
        let document = text(&args[0], r"`Core\Html::sanitize`'s `$document`")?;
        let out = source(&rebuilt(&bounded(document, r"Core\Html::sanitize")?));
        Ok(crate::instance::build(
            &MARKUP,
            [Value::text(&out)],
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use nvs_runtime::{Ctx, NvsArray, OutputSink, call};

    use super::*;
    use crate::registry::CLASSES;

    /// [`Sink`], counting the names the tree builder reads.
    ///
    /// Every step of every walk the builder makes over its stack of open
    /// elements reads one name, so the count is the length of those walks.
    struct Counted {
        sink: Sink,
        reads: Cell<u64>,
    }

    impl TreeSink for Counted {
        type Handle = usize;
        type Output = (Parsed, u64);
        type ElemName<'a> = Ref<'a, QualName>;

        fn finish(self) -> (Parsed, u64) {
            (self.sink.finish(), self.reads.get())
        }
        fn parse_error(&self, why: Cow<'static, str>) {
            self.sink.parse_error(why);
        }
        fn get_document(&self) -> usize {
            self.sink.get_document()
        }
        fn elem_name<'a>(&'a self, target: &'a usize) -> Ref<'a, QualName> {
            self.reads.set(self.reads.get() + 1);
            self.sink.elem_name(target)
        }
        fn create_element(
            &self,
            name: QualName,
            attrs: Vec<Attribute>,
            flags: ElementFlags,
        ) -> usize {
            self.sink.create_element(name, attrs, flags)
        }
        fn create_comment(&self, text: StrTendril) -> usize {
            self.sink.create_comment(text)
        }
        fn create_pi(&self, target: StrTendril, data: StrTendril) -> usize {
            self.sink.create_pi(target, data)
        }
        fn append(&self, parent: &usize, child: NodeOrText<usize>) {
            self.sink.append(parent, child);
        }
        fn append_before_sibling(&self, sibling: &usize, new_node: NodeOrText<usize>) {
            self.sink.append_before_sibling(sibling, new_node);
        }
        fn append_based_on_parent_node(
            &self,
            element: &usize,
            prev_element: &usize,
            child: NodeOrText<usize>,
        ) {
            self.sink
                .append_based_on_parent_node(element, prev_element, child);
        }
        fn append_doctype_to_document(
            &self,
            name: StrTendril,
            public: StrTendril,
            system: StrTendril,
        ) {
            self.sink.append_doctype_to_document(name, public, system);
        }
        fn get_template_contents(&self, target: &usize) -> usize {
            self.sink.get_template_contents(target)
        }
        fn same_node(&self, x: &usize, y: &usize) -> bool {
            self.sink.same_node(x, y)
        }
        fn set_quirks_mode(&self, mode: QuirksMode) {
            self.sink.set_quirks_mode(mode);
        }
        fn add_attrs_if_missing(&self, target: &usize, attrs: Vec<Attribute>) {
            self.sink.add_attrs_if_missing(target, attrs);
        }
        fn remove_from_parent(&self, target: &usize) {
            self.sink.remove_from_parent(target);
        }
        fn reparent_children(&self, node: &usize, new_parent: &usize) {
            self.sink.reparent_children(node, new_parent);
        }
        fn is_mathml_annotation_xml_integration_point(&self, handle: &usize) -> bool {
            self.sink.is_mathml_annotation_xml_integration_point(handle)
        }
        fn allow_declarative_shadow_roots(&self, intended_parent: &usize) -> bool {
            self.sink.allow_declarative_shadow_roots(intended_parent)
        }
    }

    /// `document` parsed as [`parse`] parses it, and the steps its walks
    /// over the stack of open elements took.
    fn parsed_counting_steps(document: &str) -> (Parsed, u64) {
        let counted = Counted {
            sink: Sink::new(),
            reads: Cell::new(0),
        };
        html5ever::parse_document(counted, options()).one(document)
    }

    /// A `Core\Html\Markup` over `source`, built the way every member that
    /// answers one builds it — the carrier's one slot holding the bytes.
    fn markup(source: &str) -> Value {
        crate::instance::build(&MARKUP, [Value::text(source)])
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
        list.append(markup("<li>one</li>"));
        list.append(markup("<li>two</li>"));
        list.append(markup("<li>three</li>"));
        let parts = Value::array(list);
        let separator = markup("<br>");

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

    /// Parts whose page is larger than the memory limit are refused before a
    /// byte of the page is copied.
    // covers: Core\Html::join
    #[test]
    fn html_join_past_the_memory_limit_is_refused_before_a_byte_is_copied() {
        const LIMIT: usize = 64 << 10;
        let mut list = NvsArray::new();
        list.append(markup(&"<p>x</p>".repeat(LIMIT / 8)));
        list.append(markup(&"<p>y</p>".repeat(LIMIT / 8)));
        let parts = Value::array(list);
        let separator = markup("<br>");
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_memory_limit(LIMIT);

        let answered = call(super::nvs_core_html_join, &mut ctx, &[parts, separator]);

        assert!(answered.is_err(), "a page of twice the limit does not fit");
        assert!(nvs_runtime::budget::refused(), "the breach is recorded");
        assert!(ctx.memory_peak() < LIMIT, "no byte of the page was copied");
        drop(ctx);
        #[expect(unsafe_code, reason = "this test owns the references it built")]
        unsafe {
            separator.release();
            parts.release();
        }
    }

    /// `$page = $page + $b` appends into `$page`'s own object where `$page` is
    /// its one owner, and copies where a second variable also holds it.
    #[test]
    fn markup_append_grows_a_sole_owner_in_place_and_copies_a_shared_one() {
        let page = markup("<p>");
        let row = markup("a & b");
        let mut ctx = Ctx::new(OutputSink::Sink);

        let grown = call(super::nvs_core_html_markup_append, &mut ctx, &[page, row])
            .expect("appending carriers never fails");
        assert_eq!(grown.bits(), page.bits());
        assert_eq!(carried(grown), "<p>a & b");

        // A second owner makes the append a copy, and the first owner's text
        // stays as it was.
        let copied = call(super::nvs_core_html_markup_append, &mut ctx, &[page, row])
            .expect("appending carriers never fails");
        assert_ne!(copied.bits(), page.bits());
        assert_eq!(carried(copied), "<p>a & ba & b");
        assert_eq!(carried(page), "<p>a & b");

        #[expect(
            unsafe_code,
            reason = "this test owns the references it built above and the two the \
                      helper answered with"
        )]
        unsafe {
            copied.release();
            grown.release();
            page.release();
            row.release();
        }
    }

    /// The boundary the walk has to get right on its own: with no part to
    /// write, there is no pair for the separator to go between, so the answer
    /// is a carrier holding nothing rather than the separator by itself.
    #[test]
    fn html_join_over_an_empty_list_answers_an_empty_markup() {
        let parts = Value::array(NvsArray::new());
        let separator = markup("<br>");

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

    /// `rule:core-classes/html-template`'s "it neither trusts nor escapes
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
        list.append(markup("<b>a & b</b>"));
        list.append(markup("<i>'q' > \"p\"</i>"));
        let parts = Value::array(list);
        let separator = markup(" & ");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let joined = call(super::nvs_core_html_join, &mut ctx, &[parts, separator])
            .expect("joining carriers never fails");
        assert_eq!(carried(joined), "<b>a & b</b> & <i>'q' > \"p\"</i>");

        let text = Value::text("a & b");
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
        let text = Value::text("<a title='x'>\"&\"</a>\u{202E}!");
        let escaped = call(super::nvs_core_html_escape, &mut ctx, &[text])
            .expect("escaping text never fails");
        assert_eq!(
            carried(escaped),
            "&lt;a title=&#39;x&#39;&gt;&quot;&amp;&quot;&lt;/a&gt;\u{FFFD}!"
        );

        let plain = Value::text("Ada Lovelace");
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
        let document = Value::text(
            "<p onclick=x() class=c>Hi <b>there</b><script>alert(1)</script></p>\
              <font>kept</font> <a href=\" javascript:alert(1)\">a</a> <a href=/help>b</a>",
        );
        let once = call(super::nvs_core_html_sanitize, &mut ctx, &[document])
            .expect("the rebuild has no failure mode");
        let written = carried(once);
        assert_eq!(
            written,
            "<p>Hi <b>there</b></p>kept <a>a</a> <a href=\"/help\">b</a>"
        );

        let again = Value::text(&written);
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
        let fragment = markup("<b>Tom &amp; Jerry</b>");
        let reason = Value::text("storing the rendered card");
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

        let empty = Value::text("");
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
    /// The third assertion is `rule:core-classes/html-auto-escape`'s own shape: the class is **memberless**,
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
            "a `Markup` is obtained from an html template, from `as` on a string literal, \
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
    /// back. `rule:security/launderers-are-sink-named` says a launderer is narrow and *sink-named*, so the
    /// claim under test is a set: the members spelled for the HTML sink are
    /// [`CLASS`]'s `escape` and `sanitize` and nothing else in the registry,
    /// and a third row here fails this on the day it is added — which is the
    /// day the decision to widen the sink's escape hatch is actually being
    /// made.
    ///
    /// Two launderers on one class is not the catch-all that rule refuses, and the
    /// distinction is the one it draws: what it forbids is a
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
    /// The predicate makes the carrier a *derived* answer: a launderer answers one when
    /// its sink launders on its own **and** its transform is not idempotent,
    /// and the plain unqualified type otherwise. So the roster below is the
    /// rule's table, transcribed, and the test asserts agreement in
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
    /// rather than that sink's escaper — which is why the rule's table has a row per
    /// sink and not per member, and why `Core\Cli::escape` is the row that
    /// speaks for the terminal. `Core\Http::allowUrl` answers
    /// `Core\Http\Target`, which is a pinned-address capability handle and not
    /// a text carrier at all: the column here is *`nvs_runtime::is_carrier`* —
    /// "the sink writes this value's bytes out raw" — and `rule:http-server/allow-url-pins-the-address`'s sink
    /// neither auto-launders nor escapes anything.
    ///
    /// `Core\Taint::assertTrusted` is the third, and the one row that names no
    /// sink at all: the predicate still answers, because the transform is the
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
            // Both answer `Core\Ldap\Dn`, a value only a DN parameter takes,
            // for `Core\Http::allowUrl`'s reason: nothing auto-escapes into a
            // DN, so it is not a text carrier.
            (r"Core\Ldap\Dn::child", false),
            (r"Core\Ldap\Dn::of", false),
            (r"Core\Path::fromCwd", false),
            (r"Core\Regex::quote", false),
            (r"Core\Taint::assertTrusted", false),
            (r"Core\Taint::assertTrustedBytes", false),
            (r"Core\Uri::encodeComponent", false),
            (r"Core\Uri::encodeFormValue", false),
            // The writing half of `Core\Xml` is a launderer once per member that puts
            // something into a document, and none of them answers a carrier:
            // nothing auto-escapes into an XML document the way `echo` does
            // into an HTTP response, so the predicate's first condition fails
            // before its second is asked. `crate::xml`'s `WRITER` is where each
            // member names the sink it launders for. `instruction` is not on
            // the roster: XML escapes nothing inside an instruction, so its
            // parameters are sinks and refuse a tainted value outright.
            (r"Core\Xml\Writer::attribute", false),
            (r"Core\Xml\Writer::cdata", false),
            (r"Core\Xml\Writer::comment", false),
            (r"Core\Xml\Writer::content", false),
            (r"Core\Xml\Writer::doctype", false),
            (r"Core\Xml\Writer::startElement", false),
        ];

        let mut roster: Vec<(String, bool)> = CLASSES
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
        let cases: [(&str, &str); 2] = [
            (
                "<ul><li>one<li>two</ul>",
                "<html><head></head><body><ul><li>one</li><li>two</li></ul></body></html>",
            ),
            ("", "<html><head></head><body></body></html>"),
        ];
        let mut ctx = Ctx::new(OutputSink::Sink);
        for (document, expected) in cases {
            let source = Value::text(document);
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
    /// The mode half is what makes this more than a row read back. The rule retires
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
    /// *Beside* `escape` is the part worth asserting. `rule:security/launderers-are-sink-named`
    /// admits a launderer only when it is narrow and names its sink, so this row has to be the
    /// same *shape* as the one that rule works through — same qualifier on
    /// the way in, same carrier on the way out — and differ only in what it
    /// launders. A `sanitize` answering a plain `string` would be the shape
    /// `rule:security/launderer-answers-a-carrier` refuses for this sink, and
    /// its output would be escaped a second time on the way into a page.
    ///
    /// The last assertion is the one that fails at *run* time when it is
    /// missed: a symbol with no [`address`] arm links and then panics naming
    /// itself the first time a program calls the member.
    #[test]
    fn sanitize_is_a_launderer_beside_escape() {
        nvs_footprint::card(CLASS.name);
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

    /// A tree past the request's memory limit stops the parse at the limit.
    ///
    /// A hundred thousand paragraphs are 400 KB of source and a tree of tens
    /// of megabytes. Both members that parse are refused under a 1 MiB limit,
    /// and the peak each leaves on its context stays near that limit. The peak
    /// is what tells a parse stopped at the limit from one that built the
    /// whole tree first and was noticed on return.
    // covers: Core\Html::parse, Core\Html::sanitize
    #[test]
    fn a_tree_past_the_memory_limit_stops_the_parse_at_the_limit() {
        let members: [(nvs_runtime::NvsFn, &str); 2] = [
            (nvs_core_html_parse, r"Core\Html::parse"),
            (nvs_core_html_sanitize, r"Core\Html::sanitize"),
        ];
        for (member, name) in members {
            // Made before the context, so the source is not on its balance.
            // Under Miri the limit and the document are a quarter of the size.
            let (paragraphs, limit) = if cfg!(miri) {
                (25_000, 256 << 10)
            } else {
                (100_000, 1 << 20)
            };
            let document = Value::text(&"<p>x".repeat(paragraphs));
            let mut ctx = Ctx::new(OutputSink::Sink);
            ctx.set_memory_limit(limit);
            let answered = call(member, &mut ctx, &[document]);
            let refusal = ctx.take_pending().map(std::borrow::Cow::into_owned);
            assert!(answered.is_err(), "{name} refuses a tree past the limit");
            let refusal = refusal.expect("a refusal leaves its message in the context");
            assert!(refusal.contains("memory limit"), "{name}: {refusal}");
            assert!(
                ctx.memory_peak() < 2 * limit,
                "{name} stopped at the limit rather than after the whole tree: peak {}",
                ctx.memory_peak()
            );
            drop(ctx);
            #[expect(unsafe_code, reason = "the case owns the document it made")]
            unsafe {
                document.release();
            }
        }
    }

    /// A document of many sibling elements stops before the arena doubles past
    /// the limit.
    ///
    /// Each node asks for its room, but the arena's doubling is the largest
    /// step: half a million line breaks are 2 MB of source and an arena of
    /// about 70 MB. Asked for only one node at a time, the doubling made the
    /// peak near three times the arena that was full.
    // covers: Core\Html::parse
    #[test]
    fn many_siblings_stop_at_the_memory_limit_before_the_arena_doubles() {
        // Under Miri, 4,000 line breaks build an arena of about 560 KB, more than
        // twice the 256 KiB limit.
        let (count, limit) = if cfg!(miri) {
            (4_000, 1 << 18)
        } else {
            (500_000, 8 << 20)
        };
        let document = Value::text(&"<br>".repeat(count));
        let mut ctx = Ctx::new(OutputSink::Sink);
        ctx.set_memory_limit(limit);
        assert!(
            call(nvs_core_html_parse, &mut ctx, &[document]).is_err(),
            "the parse refuses past the limit"
        );
        assert!(
            ctx.memory_peak() < limit + limit / 4,
            "{} bytes at the peak",
            ctx.memory_peak()
        );
        drop(ctx);
        #[expect(unsafe_code, reason = "the case owns the document it made")]
        unsafe {
            document.release();
        }
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

    /// HTML-2: a reference that starts with two slashes names a host, and a
    /// browser reads a backslash there as a slash, so all four pairs are
    /// dropped. Tab, line feed and carriage return come out first, as a
    /// browser removes them. A path, a query and a fragment name no host and
    /// stay.
    #[test]
    fn a_reference_that_names_another_host_is_dropped_and_a_path_query_or_fragment_stays() {
        const DROPPED: &[&str] = &[
            "//evil.example/x",
            "\\\\evil.example/x",
            "/\\evil.example/x",
            "\\/evil.example/x",
            " //evil.example",
            "/\t/evil.example",
            "\n\\\r\\evil.example",
            "//evil.example/a:b",
        ];
        for value in DROPPED {
            let document = format!("<a href=\"{value}\">x</a><img src=\"{value}\">");
            assert_eq!(
                source(&rebuilt(&parse(&document))),
                "<a>x</a><img>",
                "sanitizing {document:?}"
            );
        }

        const KEPT: &[&str] = &["/path", "?q", "#frag", "page/a:b", "/a//b"];
        for value in KEPT {
            let document = format!("<a href=\"{value}\">x</a>");
            assert_eq!(
                source(&rebuilt(&parse(&document))),
                document,
                "sanitizing {document:?}"
            );
        }
    }

    /// `rule:core-classes/html-parsing`'s cap on the stack of open elements,
    /// read off the tree it leaves.
    ///
    /// `html` and `body` are the first two open elements, so the first
    /// `OPEN_ELEMENTS_CAP - 2` `div`s nest. Each `div` after them closes the
    /// one before it and becomes its sibling. Every `div` is still in the tree,
    /// no element is deeper than the cap, and the text lands in the last
    /// `div`, as it does in WebKit.
    #[test]
    fn a_start_tag_at_the_cap_closes_the_current_element_and_becomes_its_sibling() {
        let count = OPEN_ELEMENTS_CAP + 10;
        let tree = parse(&("<div>".repeat(count) + "bottom"));
        let nodes = walked(&tree);
        let divs: Vec<usize> = nodes
            .iter()
            .filter(|(_, kind, name)| *kind == Kind::Element && *name == "div")
            .map(|(depth, _, _)| *depth)
            .collect();
        assert_eq!(divs.len(), count, "no element is dropped");
        assert_eq!(divs.iter().max(), Some(&OPEN_ELEMENTS_CAP));
        assert_eq!(
            divs.iter()
                .filter(|&&depth| depth == OPEN_ELEMENTS_CAP)
                .count(),
            count - (OPEN_ELEMENTS_CAP - 3),
            "every div past the cap is a sibling of the div at the cap"
        );
        let (depth, kind, _) = *nodes.last().expect("the tree has nodes");
        assert_eq!((depth, kind), (OPEN_ELEMENTS_CAP + 1, Kind::Text));
    }

    /// The cap bounds the work each token costs, whatever the shape of the
    /// document past it.
    ///
    /// Without the cap, each of these costs steps in the square of its length:
    /// every `div` checks the whole stack for an open `p`, every unmatched end
    /// tag searches the whole stack for its element, and every `form` searches
    /// it for a `template`. With the cap, no walk is longer than
    /// [`OPEN_ELEMENTS_CAP`] and a few elements a formatting element reopens,
    /// so the steps per tag stay under a small multiple of the cap.
    #[test]
    fn every_walk_over_the_stack_of_open_elements_is_bounded_by_the_cap() {
        const COUNT: usize = if cfg!(miri) { 48 } else { 20_000 };
        let cases = [
            ("<div>".repeat(COUNT), COUNT),
            ("<span>".repeat(COUNT) + &"</x>".repeat(COUNT), 2 * COUNT),
            ("<b>".repeat(COUNT) + &"</i>".repeat(COUNT), 2 * COUNT),
            ("<ul><li>".repeat(COUNT / 2), COUNT),
            ("<form><span>".repeat(COUNT / 2), COUNT),
            ("<table><tr><td>".repeat(COUNT / 3), COUNT),
        ];
        for (document, tags) in cases {
            let (tree, steps) = parsed_counting_steps(&document);
            assert_eq!(tree.kind, Kind::Document);
            let per_tag = steps / tags as u64;
            assert!(
                per_tag <= 4 * OPEN_ELEMENTS_CAP as u64,
                "{}…: {steps} steps is {per_tag} for each tag",
                &document[..24]
            );
        }
    }

    /// The special elements the cap can close still parse. Each is closed by
    /// an end tag for its own name, which is a token the WHATWG algorithm
    /// already has a rule for, so no input reaches a panic.
    #[test]
    fn the_cap_closes_tables_templates_selects_and_foreign_content_without_failing() {
        let deep = OPEN_ELEMENTS_CAP + if cfg!(miri) { 8 } else { 50 };
        let documents = [
            "<table>".repeat(deep),
            "<table><tr><td>".repeat(deep),
            "<template>".repeat(deep) + "x",
            "<select><option>".repeat(deep),
            "<svg>".repeat(deep) + "<foreignObject><div>x",
            "<math><mi>".repeat(deep) + "x",
            "<p><button>".repeat(deep),
            "<b><p>".repeat(deep) + "x",
            "<textarea>".to_owned() + &"<div>".repeat(deep),
        ];
        for document in &documents {
            assert_eq!(parse(document).kind, Kind::Document, "{}…", &document[..20]);
        }
    }

    /// A tag's attributes are kept in written order, and a name written a
    /// second time is dropped with its value, the first one winning. That holds
    /// for a tag with a few attributes and for one with enough that the
    /// tokenizer looks its names up in a set.
    #[test]
    fn a_duplicate_attribute_is_dropped_and_the_first_value_kept_for_any_count() {
        let few = parse("<p a=1 b=2 a=3>");
        let many: String = (0..40).map(|i| format!(" n{i}={i}")).collect();
        let many = parse(&format!("<p{many} n3=late n39=late z=last>"));
        let attributes = |tree: &Parsed| -> Vec<(String, String)> {
            let body = &tree.children[0].children[1];
            body.children[0].attributes.clone()
        };
        assert_eq!(
            attributes(&few),
            [("a", "1"), ("b", "2")].map(|(n, v)| (n.to_owned(), v.to_owned()))
        );
        let many = attributes(&many);
        assert_eq!(many.len(), 41);
        assert_eq!(many[3], ("n3".to_owned(), "3".to_owned()));
        assert_eq!(many[39], ("n39".to_owned(), "39".to_owned()));
        assert_eq!(many[40], ("z".to_owned(), "last".to_owned()));
    }

    /// A tag with a hundred thousand attributes keeps every one of them.
    ///
    /// The duplicate check runs once for each attribute, against the ones
    /// before it on the same tag. A set makes that check constant, so this tag
    /// parses in time in proportion to its length. Under Miri the tag has
    /// three hundred attributes, and each of them still checks the set.
    #[test]
    fn a_tag_with_a_hundred_thousand_attributes_keeps_them_all() {
        let count: usize = if cfg!(miri) { 300 } else { 100_000 };
        let attributes: String = (0..count).map(|i| format!(" a{i}={i}")).collect();
        let tree = parse(&format!("<p{attributes}>x"));
        let paragraph = &tree.children[0].children[1].children[0];
        assert_eq!(paragraph.name, "p");
        assert_eq!(paragraph.attributes.len(), count);
        assert_eq!(paragraph.attributes[count - 1].1, (count - 1).to_string());
    }

    /// A repeated `<html>` or `<body>` tag adds only the attributes the element
    /// does not have yet, and the element keeps the value written first.
    #[test]
    fn a_repeated_html_tag_adds_only_the_attributes_its_element_lacks() {
        let tree =
            parse("<html lang=en><html lang=fr dir=ltr><body a=1><body a=2 b=3><html dir=rtl x=y>");
        let html = &tree.children[0];
        let body = &html.children[1];
        let pairs = |node: &Parsed| -> Vec<String> {
            node.attributes
                .iter()
                .map(|(n, v)| format!("{n}={v}"))
                .collect()
        };
        assert_eq!(pairs(html), ["lang=en", "dir=ltr", "x=y"]);
        assert_eq!(pairs(body), ["a=1", "b=3"]);
    }

    /// The active formatting elements still follow the Noah's Ark rule: at
    /// most three entries with the same name and the same attributes are
    /// kept, and attributes written in a different order are the same.
    ///
    /// `</p>` closes the `b`s with the paragraph, and the text in the second
    /// paragraph reopens every entry that is kept, so that paragraph shows how
    /// many there are. Four identical `b`s leave three, four that differ only
    /// in attribute order leave three too, and four different ones leave four.
    #[test]
    fn noahs_ark_keeps_three_equal_formatting_elements_in_any_attribute_order() {
        let cases = [
            ("<p><b a=1><b a=1><b a=1><b a=1></p><p>x", 3),
            ("<p><b a=1 c=2><b c=2 a=1><b a=1 c=2><b c=2 a=1></p><p>x", 3),
            ("<p><b a=1><b a=2><b a=3><b a=4></p><p>x", 4),
        ];
        for (document, reopened) in cases {
            let tree = parse(document);
            let nodes = walked(&tree);
            let paragraph = nodes
                .iter()
                .rposition(|(_, _, name)| *name == "p")
                .expect("the document has two paragraphs");
            let in_paragraph = nodes[paragraph + 1..]
                .iter()
                .filter(|(_, kind, name)| *kind == Kind::Element && *name == "b")
                .count();
            assert_eq!(in_paragraph, reopened, "{document}");
        }
    }

    /// Foster parenting puts misplaced content in front of the open table, in
    /// the order the document wrote it.
    #[test]
    fn foster_parented_nodes_land_before_the_table_in_written_order() {
        let tree = parse("<table>a<i>1</i>b<i>2</i><tr><td>c</table>");
        assert_eq!(
            source(&tree.children[0].children[1]),
            "a<i>1</i>b<i>2</i><table><tbody><tr><td>c</td></tr></tbody></table>"
        );
    }
}
