//! `Core\Cli` — the terminal facts a program is allowed to ask for, and
//! `Core\Cli\Text`, the carrier of the terminal sink.
//!
//! `rule:tooling/the-terminal-profile-resolves-once` is
//! this module's half of that ADR: which of the three standard streams is a
//! terminal, how wide and how tall it is, and how much colour it can show.
//! `rule:tooling/echo-always-has-a-sink`
//! 's table pairs every context with a sink and every sink with a *carrier*:
//! `Core\Html\Markup` under an HTTP request, `Core\Cli\Text` everywhere else,
//! and § 5 makes that carrier the return of `Core\Out::capture`. So the carrier
//! had to exist before [`crate::out`] could, which is why it is in this file
//! and was in it alone for a while.
//!
//! # Every member here is one read of a profile resolved once
//!
//! `rule:tooling/the-terminal-profile-resolves-once` says the terminal profile is resolved **once per process, not
//! per call**, so two reads of `Core\Cli::width()` are the same number by
//! construction rather than by luck. That resolution is
//! [`nvs_runtime::terminal`] and not this module: it reaches the operating
//! system, and `rule:security/capability-check-at-the-door`
//! says a `Core` member may not — `tests/capability.rs`'s
//! `nvs_stdlib_reaches_the_os_only_through_the_gate` holds that shut by name.
//! That module's own docs own the caching, what it spends, and why no
//! capability gates it.
//!
//! What is left here is the surface: six rows, three enums, and the mapping
//! between the runtime's Rust `ColorDepth` and the ordinals
//! [`crate::registry::ENUMS`] gives `Core\Cli\ColorDepth`. The mapping is the
//! one thing this file can get wrong on its own, so
//! `tests::the_two_enums_agree_with_the_runtimes_own` holds the two rosters
//! together. Named rather than linked because this module is public — `rule:tooling/styling-is-a-value-not-a-grammar`
//! 's `Text + Text` is a row in `nvs_types`, which reaches `NAME` and
//! [`TEXT_CONCAT_SYMBOL`] through it — and a `#[cfg(test)]` item is not there
//! for a documentation build to resolve.
//!
//! # Why `Core\Cli\Shell` is here, taken by nothing in this file
//!
//! [`SHELL`] is the third enum and no member of `Core\Cli` reads it: `rule:tooling/commands-are-compiled`
//! 's `Core\Command::completions` is the one thing that does, and
//! [`crate::command`] is where that member lives. [`crate::registry::ENUMS`]
//! asks for one line per enum *declared beside the member that takes it*, and
//! this is the exception the rule is worth making: the name is
//! `Core\Cli\Shell`, so the `Cli` namespace is the roster a reader looks in,
//! and a second home for one case of it is how two rosters start disagreeing.
//! [`crate::router::METHOD`] sits the same way for the same reason — a
//! namespace prefix is not a module boundary.
//!
//! # What a `Text` is, and what it is not
//!
//! One slot, holding bytes that have already been through § 1's table, and one
//! member: `plain`. A `Text` is **the language's one raw path** — `echo` writes
//! a carrier through unchanged rather than substituting over it, which is what
//! `rule:tooling/terminal-output-is-a-sink`'s *"there is exactly one raw path, `Cli\Text`"* asks for and
//! what `nvs_runtime`'s `nvs_echo_value` implements. Keying that on the class
//! is deliberate: a `raw` bit riding on a `Tag::Str` would leave the carrier on
//! the first member that answered one, and `is_carrier_value`'s doc comment in
//! `nvs-runtime` owns the reasoning.
//!
//! What keeps that path from being a hole is that **no constructor accepts
//! bytes it has not neutralized**. [`nvs_core_cli_text_plain`] substitutes over
//! its argument, and [`built`] — the transfer builder `Core\Out::capture` uses
//! — is handed bytes that came out of a sink already. The control bytes a
//! `Text` may legitimately carry are `styled`'s, put there from a `Cli\Style`
//! and never taken from a caller's string.
//!
//! # What a style is, and where it is rendered
//!
//! `rule:tooling/styling-is-a-value-not-a-grammar`'s other half is here too: [`STYLE`] and [`COLOR`], the two
//! value types that exist so the carrier has something to wear that is not a
//! grammar. A colour is a class *constant that is an instance* —
//! `Color::RED` is `Color::index(1)` inlined at the use site, which
//! [`crate::registry::Const::Built`] has expressed since `Core\Time\Zone::UTC`
//! — so the sixteen names cost one allocation each where they are written and
//! nothing is shared between isolates.
//!
//! [`nvs_core_cli_text_styled`] keeps that style **as a run** beside the text
//! it wears, and the escape sequence is rendered by whatever writes, for the
//! stream it writes to, against the profile § 3 resolves once per process. So
//! one `Text` goes styled to a terminal standard output and plain to a
//! redirected standard error in the same run: [`rendered_at`] is where that
//! happens and [`depth_for`] is the per-stream half of the answer.
//!
//! [`nvs_core_cli_text_concat`] is § 2's `Text + Text`, the third and last way
//! a `Text` is obtained. It has no member row: the operator is the spelling,
//! and `nvs_types::expr::operators` admits the pair the way it admits
//! `Markup + Markup` — one rule over both sink carriers rather than one each.
//!
//! [`nvs_core_cli_escape`] is the same table reached as a *value* rather than
//! as an effect — `rule:security/launderers-are-sink-named`'s named launderer for this sink — and it calls
//! `nvs_render::text::substitute` exactly as the sink does, so the two cannot
//! come to disagree.
//!
//! # The prompts, and the two questions each one asks first
//!
//! `rule:tooling/a-prompt-is-a-core-member`'s `ask`, `confirm`, `select<T>` and `secret` are here, and
//! what they have in common is where they *do not* read: `nvs_runtime`'s
//! terminal module opens the controlling terminal by name, so a program whose
//! standard input is a pipe can still ask a question. This module's own half
//! is [`watched`] — whether **this request's** output reaches that terminal at
//! all — because a question written into an HTTP response body or a
//! `Core\Out::capture` buffer reaches nobody, and a read after it would hold
//! the core for a keystroke that is never coming. Both answers have to be yes
//! before anything is opened; otherwise § 4's second rule applies and the
//! prompt answers its `default` or throws `Core\Cli\NotInteractive`.
//!
//! Both answers being yes is still not a promise that anyone will *type*
//! something, so the read underneath is under a clock as well:
//! `nvs_runtime::terminal::ANSWER_DEADLINE`, after which the prompt takes the
//! same `default` and throws the same class with a sentence naming the
//! deadline instead of the missing terminal ([`unanswered`]). `rule:tooling/a-prompt-is-a-core-member`'s
//! "it never blocks" is those two rules together — the terminal that is not
//! there, and the terminal nobody is sitting at — and there is no argument on
//! any of the four members that lengthens the second.
//!
//! That pair is also what makes the prompts *testable*: a `.nvst` case runs as
//! a child with its output piped, so every prompt in one takes the
//! not-interactive path by construction rather than by luck, and a case can
//! freeze what it answers without a terminal or a person anywhere near it.
//!
//! A test can also *answer* them, which is `rule:tooling/a-prompt-is-a-core-member`'s last paragraph and is
//! [`ask_terminal`]'s first line: `Core\Test::scriptAnswers` writes a queue onto
//! the request's own context, and a prompt takes its oldest line ahead of both
//! questions above. Ahead of them, because a flow whose answers depended on
//! whether the suite ran from a developer's shell or from CI is exactly what a
//! scripted queue exists to remove. `select` and `multiSelect` decide whether
//! there is anyone to ask *before* they ask — they render a menu first — so
//! those two read [`answerable`] rather than [`watched`], and that is the whole
//! of what the queue costs this module.
//!
//! # A served request's words are empty, and that is the contract
//!
//! [`nvs_core_cli_arguments`] answers whatever the launcher wrote with
//! `Ctx::set_command_line`, and `nvs-cli` is the launcher that writes one — so
//! inside an HTTP request the member answers an empty list rather than
//! throwing. Empty is the true answer and not a stand-in for a missing one: the
//! words a process was started with are the launcher's fact, and a served
//! request was not started with any, so there is nothing this module could
//! reach to fill the list with. PHP has the same shape under a web SAPI, where
//! `$argv` is simply absent, and the neutral answer is the direction with no
//! failure mode in it — a library that reads its own arguments to pick a
//! default goes on working when the same code is reached from a handler.

use nvs_runtime::terminal::{Answer, ColorDepth, Echo, Stream};
use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    CaseDoc, ClassDoc, Const, CoreClass, CoreConst, CoreEnum, CoreMethod, CoreOption, CoreTy,
    EnumDoc, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// `Core\Cli`'s fully-qualified name, in one place so the registry row and
/// every refusal that names the class cannot drift apart.
pub(crate) const CLASS_NAME: &str = r"Core\Cli";

/// `Core\Cli`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Works with the terminal in a command-line program. It gives the program's arguments \
            and the terminal's size and colours, asks the user questions, and shows output that \
            updates in place.",
};

/// `rule:tooling/the-terminal-profile-resolves-once`'s profile, § 1's launderer and § 4's prompts, as registry
/// rows. See [`crate::registry::CLASSES`].
///
/// Fifteen members, which is the whole of § 3's table and § 4's prompts:
/// `displayWidth` was the last row owed and is here, over the UAX #11 table
/// `nvs_runtime::terminal` now carries.
///
/// In the spec's own order (§ 15), which is why `arguments` is first, `write`
/// second and `escape`
/// third: the words the program was started with come before the effect, and
/// the effect before the launderer that performs the same table
/// as a value, and [`nvs_core_cli_escape`] owns why that launderer could land
/// ahead of it. § 4's five
/// prompts are all here now — `multiSelect` is the one whose answer is a set
/// rather than a value, so it is `select`'s menu under a second reading loop
/// rather than another row of the shape below. `live` and `progress` are last
/// because § 5 is the section after the prompts, and they are one region with
/// two ways of writing a frame rather than two surfaces.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: CLASS_NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "arguments",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::TaintedStr),
            symbol: "nvs_core_cli_arguments",
            doc: Some(&ARGUMENTS_DOC),
        },
        CoreMethod {
            name: "write",
            names: &["value"],
            params: &[CoreTy::Union(WRITABLE), CoreTy::Options(WRITE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_cli_write",
            doc: Some(&WRITE_DOC),
        },
        CoreMethod {
            name: "escape",
            names: &["text"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_cli_escape",
            doc: Some(&ESCAPE_DOC),
        },
        CoreMethod {
            name: "isTty",
            names: &["stream"],
            params: &[CoreTy::Enum(STREAM_NAME)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_cli_is_tty",
            doc: Some(&IS_TTY_DOC),
        },
        CoreMethod {
            name: "width",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_cli_width",
            doc: Some(&WIDTH_DOC),
        },
        CoreMethod {
            name: "height",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_cli_height",
            doc: Some(&HEIGHT_DOC),
        },
        CoreMethod {
            name: "colorDepth",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(COLOR_DEPTH_NAME),
            symbol: "nvs_core_cli_color_depth",
            doc: Some(&COLOR_DEPTH_MEMBER_DOC),
        },
        CoreMethod {
            name: "displayWidth",
            names: &["value"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_cli_display_width",
            doc: Some(&DISPLAY_WIDTH_DOC),
        },
        CoreMethod {
            name: "ask",
            names: &["question"],
            params: &[CoreTy::Text(Qual::Neutral), CoreTy::Options(ASK_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::TaintedStr,
            symbol: "nvs_core_cli_ask",
            doc: Some(&ASK_DOC),
        },
        CoreMethod {
            name: "confirm",
            names: &["question"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Options(CONFIRM_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_cli_confirm",
            doc: Some(&CONFIRM_DOC),
        },
        CoreMethod {
            name: "select",
            // `choices` rather than § 4's own `$options`: `rule:core-api/shape-rules` R2 reserves
            // `options` as the name every member's trailing bag is callable
            // by, so a positional sharing it would be ambiguous at a named
            // call site. That ADR's rule is the later and wider one, and § 4's
            // table now writes `$choices` for the same reason.
            names: &["question", "choices"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Options(SELECT_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Var("T"),
            symbol: "nvs_core_cli_select",
            doc: Some(&SELECT_DOC),
        },
        CoreMethod {
            name: "multiSelect",
            names: &["question", "choices"],
            params: &[
                CoreTy::Text(Qual::Neutral),
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Options(MULTI_SELECT_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_cli_multi_select",
            doc: Some(&MULTI_SELECT_DOC),
        },
        CoreMethod {
            name: "secret",
            names: &["question"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::SecretTaintedStr,
            symbol: "nvs_core_cli_secret",
            doc: Some(&SECRET_DOC),
        },
        CoreMethod {
            name: "live",
            names: &["body"],
            // The one member here whose answer is the *body's* — `rule:tooling/in-place-output-is-a-scoped-live-region`
            // writes `live<T>(callable(Live): T $body): T`, so the region is
            // scenery around a call that computes whatever it was going to
            // compute, and `T` binds from the body's own declared return type.
            params: &[CoreTy::CallableSig(
                &[CoreTy::Instance(LIVE_NAME)],
                &CoreTy::Var("T"),
            )],
            defaults: &[],
            return_ty: CoreTy::Var("T"),
            symbol: "nvs_core_cli_live",
            doc: Some(&LIVE_DOC),
        },
        CoreMethod {
            name: "progress",
            names: &["total", "body"],
            params: &[
                CoreTy::Uint,
                CoreTy::CallableSig(&[CoreTy::Instance(PROGRESS_NAME)], &CoreTy::Var("T")),
            ],
            defaults: &[],
            return_ty: CoreTy::Var("T"),
            symbol: "nvs_core_cli_progress",
            doc: Some(&PROGRESS_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Cli::escape`'s reference card — `rule:core-api/reference-card`.
const ESCAPE_DOC: MethodDoc = MethodDoc {
    short: "Answers `$text` with every control byte replaced by a visible, inert glyph — `ESC` as \
            `␛`, a bare `CR` as `␍`, `DEL` as `␡`, a C1 code point or an unterminated \
            bidirectional control as `�` — while `LF` and `TAB` pass through. This is the terminal \
            sink's own table as a value; `echo` already performs it, so a program needs this only \
            to hold the neutralized form.",
    params: &[ParamDoc {
        name: "text",
        desc: "The text to neutralize. Its `tainted` qualifier is removed, because the terminal \
               is the sink this launders for and nothing is left in the answer for it to act on.",
        shape: &[],
    }],
    ret: "The same text with the substitutions applied, and no other change — this is not an HTML \
          escaper, so `<`, `&` and `\"` are returned as themselves.",
    errors: &[],
};

/// `Core\Cli::isTty`'s reference card — `rule:core-api/reference-card`.
const IS_TTY_DOC: MethodDoc = MethodDoc {
    short: "Reports whether one standard stream is attached to a terminal — `posix_isatty` and \
            `stream_isatty`, which PHP splits between two extensions. Resolved once for the \
            process, so two calls in one run cannot disagree.",
    params: &[ParamDoc {
        name: "stream",
        desc: "Which stream to ask about. It is a parameter rather than a single process-wide \
               answer because colour on standard output while standard input is a pipe is the \
               common case, and one `isTty()` cannot express it.",
        shape: &[],
    }],
    ret: "`true` when that stream is a terminal, `false` when it is a pipe, a file or closed.",
    errors: &[],
};

/// `Core\Cli::width`'s reference card — `rule:core-api/reference-card`.
const WIDTH_DOC: MethodDoc = MethodDoc {
    short: "The controlling terminal's width in columns — `tput cols`, without a child process. \
            Resolved once for the process.",
    params: &[],
    ret: "The column count, or `80` when no standard stream is a terminal. Never `0`, so a \
          caller may subtract a margin from it without checking.",
    errors: &[],
};

/// `Core\Cli::height`'s reference card — `rule:core-api/reference-card`.
const HEIGHT_DOC: MethodDoc = MethodDoc {
    short: "The controlling terminal's height in rows — `tput lines`. Resolved once for the \
            process, alongside the width it was read with.",
    params: &[],
    ret: "The row count, or `24` when no standard stream is a terminal. Never `0`, for \
          `width`'s reason.",
    errors: &[],
};

/// `Core\Cli::colorDepth`'s reference card — `rule:core-api/reference-card`.
const COLOR_DEPTH_MEMBER_DOC: MethodDoc = MethodDoc {
    short: "How much colour standard output can show, honouring `NO_COLOR`, `CLICOLOR_FORCE`, \
            `FORCE_COLOR`, `COLORTERM` and `TERM`. A program does not normally ask: it writes \
            `Cli\\Text` and the sink degrades to what the terminal has. Resolved once for the \
            process.",
    params: &[],
    ret: "The depth as a `Core\\Cli\\ColorDepth` case — `None` whenever standard output is not \
          a terminal and nothing forced colour on, which is what makes `myprog | grep` and a CI \
          log plain.",
    errors: &[],
};

/// `Core\Cli::displayWidth`'s reference card — `rule:core-api/reference-card`.
const DISPLAY_WIDTH_DOC: MethodDoc = MethodDoc {
    short: "How many terminal columns `$value` will occupy when it is written — UAX #11 widths \
            over grapheme clusters, replacing `mb_strwidth`. A CJK ideograph and a fullwidth \
            Latin letter are two columns, a combining mark is none, and a control byte is the \
            one its Control Picture costs, because that is what the terminal sink shows.",
    params: &[ParamDoc {
        name: "value",
        desc: "The text to measure. A `tainted` one is accepted: a column count carries nothing \
               back out of it.",
        shape: &[],
    }],
    ret: "The column count. A tab advances to the next multiple of eight, and a newline ends the \
          row — so a value spanning several rows answers the width of its widest one, which is \
          what a box is padded to.",
    errors: &[],
};

/// `Core\Cli::write`'s subject — `rule:tooling/the-terminal-profile-resolves-once`'s `string|Cli\Text $value`, and
/// the one parameter in this file that admits the carrier as well as the text.
///
/// [`Qual::Neutral`] rather than [`Qual::Sink`] is § 1's *"regardless of
/// qualifier"* in the type system: the terminal substitutes over whatever it is
/// given, so a `tainted` argument is ordinary here where it would be refused at
/// a SQL or a shell sink — the neutralizing *is* the laundering, performed on
/// the way out instead of demanded on the way in. The `secret` axis is
/// untouched by that, and `Neutral` refuses one for the reason
/// [`Qual::Reveal`]'s roster of two is closed: substituting a control byte does
/// nothing for confidentiality (`rule:security/secret-sinks-refuse`).
const WRITABLE: &[CoreTy] = &[CoreTy::Text(Qual::Neutral), CoreTy::Instance(NAME)];

/// `Core\Cli::write`'s trailing options — `rule:tooling/the-terminal-profile-resolves-once`'s
/// `{stream?: Cli\Stream, newline?: bool}`.
///
/// Both carry a value rather than [`Const::Null`], unlike every other bag in
/// this file: neither option has an "absent" meaning distinct from its default.
/// A call that names no stream writes to standard output, which is what `print`
/// and `echo` already do, and a call that names no `newline` writes exactly the
/// bytes it was handed — `fwrite`'s behaviour and not `echo PHP_EOL`'s, because
/// a member that silently appended a byte could not be used to build a line.
const WRITE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "stream",
        ty: CoreTy::Enum(STREAM_NAME),
        default: Const::EnumCase(STREAM_NAME, "Out"),
    },
    CoreOption {
        name: "newline",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// `Core\Cli::arguments`'s reference card — `rule:core-api/reference-card`.
const ARGUMENTS_DOC: MethodDoc = MethodDoc {
    short: "The words this program was started with, past the program itself — PHP's `$argv` and \
            `$argc` in one place. A program that declares a `#[Command]` reads its arguments off \
            the table `Core\\Command::run` matched them against instead; this is the raw list, for \
            a program that parses its own.",
    params: &[],
    ret: "An `array<tainted string>` in the order the shell wrote them, empty for a program \
          started with none. Every element is `tainted`: the words came from outside the program's \
          own text, so a path, a URL or a query built from one passes its own launderer first. The \
          program's own name is not an element — nothing indexes it away, and `$argv[0]` has no \
          spelling here.",
    errors: &[],
};

/// `Core\Cli::write`'s reference card — `rule:core-api/reference-card`.
const WRITE_DOC: MethodDoc = MethodDoc {
    short: "Writes `$value` to a standard stream, replacing `fwrite(STDOUT, …)` and `print`. A \
            `string` has every control byte replaced by the visible glyph `Core\\Cli::escape` \
            gives it; a `Core\\Cli\\Text` is written through unchanged, because it is the sink's \
            own carrier and its bytes have already been neutralized.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The text to write. A `tainted` one is written like any other — the \
                   substitution is what makes the terminal safe, so nothing has to be laundered \
                   first — while a `secret` one is refused at compile time.",
            shape: &[],
        },
        ParamDoc {
            name: "stream",
            desc: "Which standard stream to write to. `Out` when omitted; `Err` writes to this \
                   request's diagnostic channel, which a `Core\\Out::capture` does not take. \
                   `In` throws.",
            shape: &[],
        },
        ParamDoc {
            name: "newline",
            desc: "Whether to append one `LF` after the value. `false` when omitted, so the \
                   member writes exactly what it was handed.",
            shape: &[],
        },
    ],
    ret: "Nothing. The bytes are on the stream, or the write failed and the request is over.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$stream` is `Core\\Cli\\Stream::In`, which a program reads and never writes. The \
               case exists for `Core\\Cli::isTty`, which asks its question about all three.",
    }],
};

/// `Core\Cli::ask`'s trailing options — `rule:tooling/a-prompt-is-a-core-member`'s
/// `{default?: string, validate?: callable}`.
///
/// Both defaults are [`Const::Null`] rather than a value of the option's own
/// type, which is that variant's documented case twice over: a `callable` has
/// no "no callback" spelling, and an *absent* `default` is what decides
/// between answering and throwing `Core\Cli\NotInteractive` — so `""` could
/// not stand in for it.
const ASK_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "default",
        ty: CoreTy::Text(Qual::Contagious),
        default: Const::Null,
    },
    CoreOption {
        name: "validate",
        ty: CoreTy::Callable,
        default: Const::Null,
    },
];

/// `Core\Cli::confirm`'s trailing options — § 4's `{default?: bool}`.
///
/// [`Const::Null`] again, and here the distinction it draws is the whole
/// member: `false` is an answer, absence is not, and only absence makes an
/// unattended run throw rather than proceed.
const CONFIRM_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "default",
    ty: CoreTy::Bool,
    default: Const::Null,
}];

/// `Core\Cli::select`'s trailing options — § 4's
/// `{labels?: callable, default?: T}`.
///
/// `default` is typed as the same variable the options array binds, so a
/// default that is not one of the values offered is a compile error rather
/// than a run that silently answers something absent from the list.
const SELECT_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "labels",
        ty: CoreTy::Callable,
        default: Const::Null,
    },
    CoreOption {
        name: "default",
        ty: CoreTy::Var("T"),
        default: Const::Null,
    },
];

/// `Core\Cli::multiSelect`'s bag — [`SELECT_OPTIONS`] without the `default`,
/// which `rule:tooling/a-prompt-is-a-core-member`'s table omits and which this member has nothing to do
/// with: an empty line already names the empty set, so the one thing a
/// `default` would be for is already spelled.
const MULTI_SELECT_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "labels",
    ty: CoreTy::Callable,
    default: Const::Null,
}];

/// `Core\Cli::ask`'s reference card — `rule:core-api/reference-card`.
const ASK_DOC: MethodDoc = MethodDoc {
    short: "Asks `$question` at the controlling terminal and answers the line typed back — \
            `readline`, without the GNU library and without reading standard input, so a program \
            reading piped data can still ask. With no terminal it answers `default` if one was \
            given and throws otherwise; it never blocks waiting for an answer nobody can give.",
    params: &[
        ParamDoc {
            name: "question",
            desc: "What to write at the terminal. It goes through the same substitution `echo` \
                   performs, so a question built from untrusted text cannot move the cursor or \
                   repaint the screen around its own answer.",
            shape: &[],
        },
        ParamDoc {
            name: "default",
            desc: "What to answer when the terminal is not there, and what an empty line means \
                   when it is. Absent makes both of those a `Core\\Cli\\NotInteractive`.",
            shape: &[],
        },
        ParamDoc {
            name: "validate",
            desc: "Called with each answer; a falsy verdict asks again. It runs only where there \
                   is a terminal to ask again at, so it never sees a `default`.",
            shape: &[],
        },
    ],
    ret: "The line typed, without its ending, and `tainted` whatever it says — it came from \
          outside the program, exactly as a request body did.",
    errors: &[ErrorDoc {
        error: "Core\\Cli\\NotInteractive",
        desc: "There is no controlling terminal to ask, its input ended, or nobody answered within \
               the prompt deadline — and the call named no `default`.",
    }],
};

/// `Core\Cli::confirm`'s reference card — `rule:core-api/reference-card`.
const CONFIRM_DOC: MethodDoc = MethodDoc {
    short: "Asks `$question` as a yes/no question, showing which way the `Enter` key goes, and \
            answers what was typed. An answer that is neither is asked again rather than read as \
            `false`.",
    params: &[
        ParamDoc {
            name: "question",
            desc: "What to write at the terminal, substituted as `ask` substitutes it.",
            shape: &[],
        },
        ParamDoc {
            name: "default",
            desc: "What an empty line means, and what an unattended run answers. Absent makes an \
                   unattended run throw, and makes an empty line ask again.",
            shape: &[],
        },
    ],
    ret: "`true` for yes and `false` for no — a plain `bool` and never a `tainted` one, because \
          nothing of what was typed survives into a closed two-case answer.",
    errors: &[ErrorDoc {
        error: "Core\\Cli\\NotInteractive",
        desc: "There is no controlling terminal to ask, its input ended, or nobody answered within \
               the prompt deadline — and the call named no `default`.",
    }],
};

/// `Core\Cli::select`'s reference card — `rule:core-api/reference-card`.
const SELECT_DOC: MethodDoc = MethodDoc {
    short: "Offers `$choices` as a numbered list and answers the one chosen — the value itself, \
            never its position, so nothing at the call site indexes back into the array.",
    params: &[
        ParamDoc {
            name: "question",
            desc: "What to write above the list, substituted as `ask` substitutes it.",
            shape: &[],
        },
        ParamDoc {
            name: "choices",
            desc: "The values to choose between, listed in their own order. An empty array is a \
                   `LogicError`: there is no answer to hand back.",
            shape: &[],
        },
        ParamDoc {
            name: "labels",
            desc: "Called with each option to produce the line shown for it. Absent renders each \
                   option as text the way `echo` would.",
            shape: &[],
        },
        ParamDoc {
            name: "default",
            desc: "What an empty line chooses, and what an unattended run answers. Typed as the \
                   choices' own element type, so it cannot be a value that is not on offer.",
            shape: &[],
        },
    ],
    ret: "The chosen element of `$choices`, with that array's element type.",
    errors: &[
        ErrorDoc {
            error: "Core\\Cli\\NotInteractive",
            desc: "There is no controlling terminal to ask, its input ended, or nobody answered \
                   within the prompt deadline — and the call named no `default`.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$choices` is empty, so there is nothing that could be chosen.",
        },
    ],
};

/// `Core\Cli::multiSelect`'s reference card — `rule:core-api/reference-card`.
const MULTI_SELECT_DOC: MethodDoc = MethodDoc {
    short: "Offers `$choices` as a numbered list and answers every one chosen, as the values \
            themselves — `select` where the answer is a set, so the numbers are typed together \
            on one line and an empty line names none of them.",
    params: &[
        ParamDoc {
            name: "question",
            desc: "What to write above the list, substituted as `ask` substitutes it. The \
                   accepted range and separator are appended to it, the way `confirm` appends \
                   `[y/n]`.",
            shape: &[],
        },
        ParamDoc {
            name: "choices",
            desc: "The values to choose between, listed in their own order. An empty array is a \
                   `LogicError`: there is nothing that could be chosen.",
            shape: &[],
        },
        ParamDoc {
            name: "labels",
            desc: "Called with each option to produce the line shown for it. Absent renders each \
                   option as text the way `echo` would.",
            shape: &[],
        },
    ],
    ret: "The chosen elements of `$choices`, in that array's own order and each at most once, \
          whatever order they were typed in. An empty line answers an empty array; there is no \
          `default`, so a run with no terminal throws instead.",
    errors: &[
        ErrorDoc {
            error: "Core\\Cli\\NotInteractive",
            desc: "There is no controlling terminal to ask, its input ended, or nobody answered \
                   within the prompt deadline. Unlike the other prompts this one has no \
                   `default` to fall back on.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$choices` is empty, so there is nothing that could be chosen.",
        },
    ],
};

/// `Core\Cli::secret`'s reference card — `rule:core-api/reference-card`.
const SECRET_DOC: MethodDoc = MethodDoc {
    short: "Asks `$question` with the terminal's echo turned off, so a password is not left on \
            the screen or in a scrollback buffer — PHP's `readline` has no spelling for this at \
            all and every program shells out to `stty -echo` for it.",
    params: &[ParamDoc {
        name: "question",
        desc: "What to write at the terminal, substituted as `ask` substitutes it.",
        shape: &[],
    }],
    ret: "The line typed, `secret` and `tainted` at once: output, logs, dumps, `Throwable` \
          messages and serialization all refuse it, and it still has to be laundered for any \
          sink it reaches.",
    errors: &[ErrorDoc {
        error: "Core\\Cli\\NotInteractive",
        desc: "There is no controlling terminal to ask, its input ended, or nobody answered within \
               the prompt deadline. `secret` takes no `default`, because a password nobody typed \
               is not a password.",
    }],
};

/// `Core\Cli::live`'s reference card — `rule:core-api/reference-card`.
const LIVE_DOC: MethodDoc = MethodDoc {
    short: "Runs `$body` with a live region open on the terminal, and answers whatever `$body` \
            answered. `$body` receives a `Core\\Cli\\Live` whose `set` replaces the region's rows \
            in place — the scoped replacement for `moveUp`/`clearLine` cursor primitives, which \
            break the moment output is piped and leave a shell unusable when a program dies \
            holding them.",
    params: &[ParamDoc {
        name: "body",
        desc: "The work to do while the region is open. Its own return value is this member's, so \
               a region costs a call site nothing in what it can compute.",
        shape: &[],
    }],
    ret: "Exactly what `$body` answered, at `$body`'s own type. The terminal is restored on every \
          path out — a return, a throw, a fatal, an internal panic — and a run whose output is \
          not a terminal renders nothing at all rather than a smear of escape sequences.",
    errors: &[],
};

/// `Core\Cli\Live`'s fully-qualified name — see [`STREAM_NAME`].
pub(crate) const LIVE_NAME: &str = r"Core\Cli\Live";

/// [`LIVE`]'s one slot, by index: which region on this core's stack the handle
/// names. See [`REGIONS`].
const LIVE_DEPTH: usize = 0;

/// `Core\Cli\Live`'s class card — `rule:core-api/reference-card`.
const LIVE_CARD: ClassDoc = ClassDoc {
    short: "An area of the terminal that your program redraws while it works. `Core\\Cli::live` \
            gives one to your function. Each update replaces what the area showed before.",
};

/// `rule:tooling/in-place-output-is-a-scoped-live-region`'s `Cli\Live` — the handle `$body` is handed, and the whole of
/// what a program may do to a live region.
///
/// One member, because § 5's table writes one: a region is *replaced* rather
/// than appended to, which is what makes it a region and not a log. There is no
/// `close` and no `clear` — the scope is the `Core\Cli::live` call, and a
/// member for ending one early would be the free-form cursor control that
/// section refuses.
pub(crate) const LIVE: CoreClass = CoreClass {
    name: LIVE_NAME,
    doc: Some(&LIVE_CARD),
    methods: &[],
    instance: &[CoreMethod {
        name: "set",
        names: &["lines"],
        // A `Cli\Text` per row rather than a `string`: the region writes to the
        // terminal sink, and `rule:security/capture-answers-the-carrier`'s carrier is what has already been
        // through it. A `string` here would be a second, unsubstituted way onto
        // the screen — which is `rule:tooling/terminal-output-is-a-sink`'s whole subject.
        params: &[CoreTy::Array(&CoreTy::Instance(NAME))],
        defaults: &[],
        return_ty: CoreTy::Void,
        symbol: "nvs_core_cli_live_set",
        doc: Some(&SET_DOC),
    }],
    slots: &["depth"],
    constants: &[],
};

/// `Core\Cli\Live::set`'s reference card — `rule:core-api/reference-card`.
const SET_DOC: MethodDoc = MethodDoc {
    short: "Replaces the region's rows with `$lines`. The runtime owns the cursor: it coalesces \
            frames on a timer rather than repainting per call, diffs against what is on screen, \
            and writes nothing at all where the run has no terminal.",
    params: &[ParamDoc {
        name: "lines",
        desc: "The region's rows, one `Core\\Cli\\Text` each, replacing whatever it held. A row \
               wider than the terminal is clamped rather than wrapped, because a wrapped row is \
               a region that can no longer be repainted.",
        shape: &[],
    }],
    ret: "Nothing. What was drawn is on screen, or was coalesced into the next frame — a program \
          cannot observe which, and the last frame handed in is always painted before the region \
          closes.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The handle's own region is not the one holding the terminal — it has closed, or an \
               inner `Core\\Cli::live` is open inside it. A `Cli\\Live` is scoped to the call that \
               made it and escaping one is a program bug.",
    }],
};

/// `Core\Cli::progress`'s reference card — `rule:core-api/reference-card`.
const PROGRESS_DOC: MethodDoc = MethodDoc {
    short: "Runs `$body` with a progress bar open on the terminal, and answers whatever `$body` \
            answered. A closed, named behaviour over `live`'s general one: the region is a bar \
            the runtime draws, so counting toward `$total` is all a program says.",
    params: &[
        ParamDoc {
            name: "total",
            desc: "How many units of work the bar is scaled to. A total of `0` is work already \
                   done, and draws a full bar rather than refusing.",
            shape: &[],
        },
        ParamDoc {
            name: "body",
            desc: "The work to do while the bar is open. Its own return value is this member's.",
            shape: &[],
        },
    ],
    ret: "Exactly what `$body` answered, at `$body`'s own type — `live`'s contract, since this is \
          that member with a frame the runtime writes.",
    errors: &[],
};

/// `Core\Cli\Progress`'s fully-qualified name — see [`STREAM_NAME`].
pub(crate) const PROGRESS_NAME: &str = r"Core\Cli\Progress";

/// [`PROGRESS`]'s slots, by index: the region it draws in, what it counts
/// toward, how far it has come, and the caption beside the bar.
const PROGRESS_DEPTH: usize = 0;

/// See [`PROGRESS_DEPTH`].
const PROGRESS_TOTAL: usize = 1;

/// See [`PROGRESS_DEPTH`].
const PROGRESS_DONE: usize = 2;

/// See [`PROGRESS_DEPTH`].
const PROGRESS_LABEL: usize = 3;

/// `advance`'s options — `rule:tooling/in-place-output-is-a-scoped-live-region`'s `{by?: uint, label?: string}`.
///
/// `by` defaults to one because counting one thing at a time is what a loop
/// does. `label` defaults to [`Const::Null`] rather than to the empty string,
/// and the difference is observable: absent leaves the caption as it was, where
/// an empty one would clear it on every step of a loop that only counts.
const ADVANCE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "by",
        ty: CoreTy::Uint,
        default: Const::Uint(1),
    },
    CoreOption {
        name: "label",
        // `Qual::Launder` for `Core\Cli\Text::plain`'s reason: the caption goes
        // to the terminal sink, and § 1's substitution is what neutralizes it.
        ty: CoreTy::Text(Qual::Launder),
        default: Const::Null,
    },
];

/// `Core\Cli\Progress`'s class card — `rule:core-api/reference-card`.
const PROGRESS_CARD: ClassDoc = ClassDoc {
    short: "A progress bar that `Core\\Cli::progress` gives to your function. Each step moves the \
            counter forward and draws the bar again.",
};

/// `rule:tooling/in-place-output-is-a-scoped-live-region`'s `Cli\Progress` — the handle `progress`'s body is handed.
///
/// One member, as § 5's table writes it. `progress` over `live` is not an ADR
/// 0063 R17 violation: R17 forbids a procedural twin of a class API and a class
/// wrapper around a static, and a closed, named behaviour built on a general
/// one is neither — which is why this class has a counter and no `set`.
pub(crate) const PROGRESS: CoreClass = CoreClass {
    name: PROGRESS_NAME,
    doc: Some(&PROGRESS_CARD),
    methods: &[],
    instance: &[CoreMethod {
        name: "advance",
        names: &[],
        params: &[CoreTy::Options(ADVANCE_OPTIONS)],
        defaults: &[],
        return_ty: CoreTy::Void,
        symbol: "nvs_core_cli_progress_advance",
        doc: Some(&ADVANCE_DOC),
    }],
    slots: &["depth", "total", "done", "label"],
    constants: &[],
};

/// `Core\Cli\Progress::advance`'s reference card — `rule:core-api/reference-card`.
const ADVANCE_DOC: MethodDoc = MethodDoc {
    short: "Counts `by` units of work as done and redraws the bar, optionally changing the \
            caption beside it.",
    params: &[
        ParamDoc {
            name: "by",
            desc: "How many units this step finished; omitted, one.",
            shape: &[],
        },
        ParamDoc {
            name: "label",
            desc: "The caption beside the bar, substituted as every other terminal write is; \
                   omitted, the caption is left as it was.",
            shape: &[],
        },
    ],
    ret: "Nothing. A bar past its total reads as complete rather than as more than complete, so \
          a loop that miscounts draws a finished bar instead of a wrong one.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The handle's own region is not the one holding the terminal — it has closed, or an \
               inner region is open inside it, exactly as `Core\\Cli\\Live::set` refuses.",
    }],
};

/// `Core\Cli\Stream`'s fully-qualified name, written once — [`STREAM`] declares
/// it and the [`CoreTy::Enum`] naming it resolves against
/// [`crate::registry::ENUMS`], so the two cannot drift apart.
pub(crate) const STREAM_NAME: &str = r"Core\Cli\Stream";

/// `rule:tooling/the-terminal-profile-resolves-once`'s `Cli\Stream` — which standard stream a question is about.
///
/// The ordinals are [`nvs_runtime::terminal::Stream`]'s own declaration order,
/// which is what [`stream_of`] converts between.
pub(crate) const STREAM: CoreEnum = CoreEnum {
    name: STREAM_NAME,
    cases: &[("In", 0), ("Out", 1), ("Err", 2)],
    doc: Some(&STREAM_DOC),
};

/// [`STREAM`]'s reference card — `rule:core-api/reference-card`.
const STREAM_DOC: EnumDoc = EnumDoc {
    short: "One of the three standard streams a process begins with. It exists because \"is a \
            terminal\" is always a question about one of them and never about the process.",
    cases: &[
        CaseDoc {
            name: "In",
            desc: "Standard input — what a prompt reads and what a pipeline feeds.",
        },
        CaseDoc {
            name: "Out",
            desc: "Standard output — what `echo` writes, and the stream the colour depth is \
                   decided for.",
        },
        CaseDoc {
            name: "Err",
            desc: "Standard error — diagnostics, which stay visible when output is redirected.",
        },
    ],
};

/// `Core\Cli\ColorDepth`'s fully-qualified name — see [`STREAM_NAME`].
pub(crate) const COLOR_DEPTH_NAME: &str = r"Core\Cli\ColorDepth";

/// `rule:tooling/the-terminal-profile-resolves-once`'s `Cli\ColorDepth` — how much colour a terminal can show.
///
/// Ordered from least to most, so the sink's `truecolor → 256 → 16 → none`
/// degradation is a comparison on the ordinal rather than a table.
pub(crate) const COLOR_DEPTH: CoreEnum = CoreEnum {
    name: COLOR_DEPTH_NAME,
    cases: &[("None", 0), ("Ansi16", 1), ("Ansi256", 2), ("TrueColor", 3)],
    doc: Some(&COLOR_DEPTH_DOC),
};

/// [`COLOR_DEPTH`]'s reference card — `rule:core-api/reference-card`.
const COLOR_DEPTH_DOC: EnumDoc = EnumDoc {
    short: "How much colour standard output can show. The cases ascend, so a sink degrading to \
            what a terminal has is a comparison rather than a lookup.",
    cases: &[
        CaseDoc {
            name: "None",
            desc: "No colour at all — a pipe, a file, `NO_COLOR`, or `TERM=dumb`. Styling is \
                   dropped entirely rather than approximated.",
        },
        CaseDoc {
            name: "Ansi16",
            desc: "The eight ANSI colours and their bright halves, which every terminal has.",
        },
        CaseDoc {
            name: "Ansi256",
            desc: "The 256-entry indexed palette, reported by a `TERM` naming `256color`.",
        },
        CaseDoc {
            name: "TrueColor",
            desc: "24-bit colour, reported by `COLORTERM=truecolor` and by a Windows console \
                   that accepted virtual terminal processing.",
        },
    ],
};

/// `Core\Cli\Shell`'s fully-qualified name — see [`STREAM_NAME`].
pub(crate) const SHELL_NAME: &str = r"Core\Cli\Shell";

/// `rule:tooling/commands-are-compiled`'s `Cli\Shell` — the shell `Core\Command::completions` writes a
/// script for, and a closed roster like [`crate::router::METHOD`]: § 6 names
/// `bash`, `zsh`, `fish` and `pwsh` and nothing else.
///
/// The three POSIX-family shells come first and PowerShell last, which is
/// documentation rather than a bound: unlike `METHOD`'s CSRF tail nothing reads
/// a *range* of this roster, and `completions` answers one case at a time. A
/// fifth shell is a case here **and** a generator beside it, because a case
/// nothing can generate for is a name a program can write and pass nowhere,
/// which is [`crate::registry::ENUMS`]' own test for admitting an entry.
pub(crate) const SHELL: CoreEnum = CoreEnum {
    name: SHELL_NAME,
    cases: &[("Bash", 0), ("Zsh", 1), ("Fish", 2), ("Pwsh", 3)],
    doc: Some(&SHELL_DOC),
};

/// [`SHELL`]'s reference card — `rule:core-api/reference-card`.
const SHELL_DOC: EnumDoc = EnumDoc {
    short: "Which shell `Core\\Command::completions` writes a completion script for. Four cases \
            and no catch-all: a script is generated in the named shell's own syntax, so a case \
            with no generator behind it would complete nothing.",
    cases: &[
        CaseDoc {
            name: "Bash",
            desc: "GNU Bash, whose script registers a function with `complete -F`.",
        },
        CaseDoc {
            name: "Zsh",
            desc: "Z shell, whose script is a `#compdef` function driving `_arguments`.",
        },
        CaseDoc {
            name: "Fish",
            desc: "fish, whose script is one `complete -c` line per command and per option.",
        },
        CaseDoc {
            name: "Pwsh",
            desc: "PowerShell — 7 and Windows PowerShell alike, whose script calls \
                   `Register-ArgumentCompleter`.",
        },
    ],
};

// ------------------------------------------------------------------ the members

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_cli_arguments" => (nvs_core_cli_arguments as *const ()).cast(),
        "nvs_core_cli_write" => (nvs_core_cli_write as *const ()).cast(),
        "nvs_core_cli_escape" => (nvs_core_cli_escape as *const ()).cast(),
        "nvs_core_cli_is_tty" => (nvs_core_cli_is_tty as *const ()).cast(),
        "nvs_core_cli_width" => (nvs_core_cli_width as *const ()).cast(),
        "nvs_core_cli_height" => (nvs_core_cli_height as *const ()).cast(),
        "nvs_core_cli_color_depth" => (nvs_core_cli_color_depth as *const ()).cast(),
        "nvs_core_cli_display_width" => (nvs_core_cli_display_width as *const ()).cast(),
        "nvs_core_cli_text_plain" => (nvs_core_cli_text_plain as *const ()).cast(),
        "nvs_core_cli_text_styled" => (nvs_core_cli_text_styled as *const ()).cast(),
        "nvs_core_cli_text_text" => (nvs_core_cli_text_text as *const ()).cast(),
        TEXT_CONCAT_SYMBOL => (nvs_core_cli_text_concat as *const ()).cast(),
        "nvs_core_cli_color_index" => (nvs_core_cli_color_index as *const ()).cast(),
        "nvs_core_cli_color_rgb" => (nvs_core_cli_color_rgb as *const ()).cast(),
        "nvs_core_cli_style_of" => (nvs_core_cli_style_of as *const ()).cast(),
        "nvs_core_cli_live" => (nvs_core_cli_live as *const ()).cast(),
        "nvs_core_cli_progress" => (nvs_core_cli_progress as *const ()).cast(),
        "nvs_core_cli_progress_advance" => (nvs_core_cli_progress_advance as *const ()).cast(),
        "nvs_core_cli_live_set" => (nvs_core_cli_live_set as *const ()).cast(),
        "nvs_core_cli_ask" => (nvs_core_cli_ask as *const ()).cast(),
        "nvs_core_cli_confirm" => (nvs_core_cli_confirm as *const ()).cast(),
        "nvs_core_cli_select" => (nvs_core_cli_select as *const ()).cast(),
        "nvs_core_cli_multi_select" => (nvs_core_cli_multi_select as *const ()).cast(),
        "nvs_core_cli_secret" => (nvs_core_cli_secret as *const ()).cast(),
        _ => return None,
    })
}

/// The runtime's [`Stream`] a `Core\Cli\Stream` argument names.
///
/// # Errors
///
/// A [`Fault::fatal`] for a value that is no case of the enum — the same
/// treatment [`crate::hash`] gives its `Core\Digest` argument, and for the same
/// reason.
fn stream_of(value: &Value, member: &str) -> Result<Stream, Fault> {
    match value.as_int() {
        Some(0) => Ok(Stream::In),
        Some(1) => Ok(Stream::Out),
        Some(2) => Ok(Stream::Err),
        // This is unreachable from source: both callers' parameter is
        // `CoreTy::Enum(STREAM_NAME)`, so `E0401` refuses anything that is not
        // one of the three cases before a single instruction of this body runs,
        // and compiled code writes the ordinal itself.
        _ => Err(Fault::fatal(format!(
            "Core\\Cli::{member} expected a `Core\\Cli\\Stream` case, got tag {} value {:?}",
            value.tag_byte(),
            value.as_int()
        ))),
    }
}

/// The [`COLOR_DEPTH`] ordinal for one of the runtime's depths.
///
/// Written out rather than derived from the Rust enum's discriminant: the two
/// rosters are declared in different crates for different readers, and the cast
/// that looked equivalent would silently answer the wrong case the first time
/// either gained an entry.
fn depth_ordinal(depth: ColorDepth) -> i64 {
    match depth {
        ColorDepth::None => 0,
        ColorDepth::Ansi16 => 1,
        ColorDepth::Ansi256 => 2,
        ColorDepth::TrueColor => 3,
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::arguments(): array<tainted string>` — spec § 15's raw
    /// argument list, replacing `$argv` and `$argc`.
    ///
    /// **It reads no operating system**, which is the whole of why this member
    /// is three lines. `rule:security/capability-check-at-the-door` keeps `argv` out of this crate, so the
    /// words arrive through `Ctx::set_command_line`, written by the launcher
    /// that started the program — `nvs-cli`'s own `main` beside
    /// `set_program_name`, whose comment owns why a served request has a
    /// command line only in the sense that the *server* was started with one.
    /// So a request that nobody started with words reads an empty array here
    /// rather than the server's own, and this member has no capability
    /// question to ask.
    ///
    /// `Core\Command::run` reads the same list off the same context, and that
    /// is the point rather than a duplication: one program declares
    /// `#[Command]` and reads a matched table, another parses its own words,
    /// and both are looking at what the shell wrote. The elements are
    /// `tainted` for the reason `Core\Env`'s values are — they come from
    /// outside the program's own text — and there is no `$argv[0]`, because
    /// the name a program was invoked by is `Ctx::program_name`'s question and
    /// is answered where a completion script needs it.
    ///
    /// **What it spends:** one array of one `string` per word, per call. The
    /// list is copied rather than shared because a `string` in it is a Novis
    /// value and the context holds Rust `String`s; the words are the shell's
    /// own line, so the size is bounded by what an operating system would let
    /// a process be started with at all.
    fn nvs_core_cli_arguments(ctx, _args: [0]) {
        let mut out = NvsArray::new();
        for word in ctx.command_line() {
            out.append(Value::str(NvsStr::new(word.as_bytes())));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::write(string|Cli\Text $value, {stream?: Cli\Stream, newline?: bool}): void`
    /// — `rule:tooling/the-terminal-profile-resolves-once`'s first row, replacing `fwrite(STDOUT, …)` and `print`.
    ///
    /// # It is the sink `echo` already is, reached with a stream named
    ///
    /// § 1 has one substitution table and this member performs the same one, on
    /// the same two cases `nvs_runtime`'s `nvs_echo_value` separates: a
    /// `Core\Cli\Text` is the language's single raw path and is written
    /// through unchanged, everything else is neutralized on the way out. The
    /// carrier is recognised **by its class**, which is `is_carrier_value`'s
    /// own rule and the reason it works: no constructor of a `Text` accepts
    /// bytes it has not already put through this table, so trusting the class
    /// is trusting a closed set of producers rather than a bit a caller could
    /// arrange.
    ///
    /// What `echo` cannot say is *which* stream, and that is the whole of why
    /// this row exists rather than being a second spelling of the same
    /// statement. `Stream::Out` is this request's output — so a
    /// `Core\Out::capture` in force takes it, `Ctx::write_output`'s own rule —
    /// and `Stream::Err` is its **diagnostic** channel, which a capture
    /// deliberately does not take (`rule:errors/debug-dump`). A context has exactly those
    /// two channels; there is no third for a member to invent.
    ///
    /// `Stream::In` throws rather than being absent from the option's type:
    /// `Cli\Stream` is one enum because `rule:tooling/the-terminal-profile-resolves-once` wants `isTty` to ask about
    /// all three, and a second two-case enum spelled only for this parameter
    /// would be the "no operation is reachable two ways" rule broken sideways —
    /// two rosters of the same three streams, disagreeing the first time one
    /// gains a case.
    ///
    /// **What it spends:** one allocation for a `string`, the newline included
    /// — the substitution borrows when there is nothing to replace, and the
    /// buffer is sized for the byte that ends the line before it is filled. A
    /// second write to reach the same stream would be a second trip through the
    /// sink for one byte, and this member is bounded by the terminal it writes
    /// to rather than by the copy (`rule:programs/memory-priority`'s ordering: priority 3 over 5). A
    /// `Core\Cli\Text` costs its runs' own rendering on top of that, and one
    /// more allocation when a newline grows the buffer that rendering sized.
    fn nvs_core_cli_write(ctx, args: [3]) {
        let stream = stream_of(&args[1], "write")?;
        if matches!(stream, Stream::In) {
            // A literal rather than a `format!`, for the reason
            // `Core\Cli\Live::set`'s refusal is one: `conformance_coverage`'s
            // error-path gate matches a site by the stem before its first hole.
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                "Core\\Cli::write(): standard input is not a stream a program writes to — \
                 `Core\\Cli\\Stream::In` is a case so that `Core\\Cli::isTty` can ask its \
                 question about all three",
            ));
        }
        let newline = args[2].as_bool() == Some(true);
        let mut bytes = match args[0].tag() {
            Some(Tag::Object) => {
                let receiver = crate::instance::receiver(args[0], &TEXT, "write")?;
                // **This is the sink.** The runs are rendered here, for the
                // stream this call names, which is what lets one `Text` go
                // styled to a terminal standard output and plain to a
                // redirected standard error in the same run. The carrier slot
                // holds standard output's own rendering, which `echo` takes
                // and this member never reads.
                rendered_at(crate::instance::slot(receiver, TEXT_RUNS), depth_of(stream))?
                    .into_bytes()
            }
            // Every other tag, and not just `Tag::Str`.
            // Unreachable from source: the row's parameter is a `CoreTy::Union`
            // of the two, so `E0401` refuses anything that is neither the
            // carrier nor a `string` before an instruction of this body runs.
            _ => {
                let text = args[0].as_text().ok_or_else(|| {
                    Fault::fatal(format!(
                        "Core\\Cli::write expected a `string` or a `Core\\Cli\\Text`, got tag {}",
                        args[0].tag_byte()
                    ))
                })?;
                // One allocation for the whole write: the substitution borrows
                // when there is nothing to replace, and the buffer is sized
                // for the newline before it is filled. An `into_owned`
                // followed by a `push` allocates a second time, because a
                // `String` built from a borrow has no room left in it.
                let rendered = nvs_render::text::substitute(text);
                let mut bytes = Vec::with_capacity(rendered.len() + usize::from(newline));
                bytes.extend_from_slice(rendered.as_bytes());
                bytes
            }
        };
        if newline {
            bytes.push(b'\n');
        }
        match stream {
            Stream::Err => ctx.write_diagnostic(&bytes),
            // `Stream::In` was refused above, before anything was rendered.
            Stream::In | Stream::Out => ctx.write_output(&bytes),
        }
        // Unreachable from source, on `Core\Debug::dump`'s reasoning rather
        // than on a diagnostic: `OutputSink::Buffer` and `Sink` never fail,
        // which `Ctx::write_output` and `Ctx::write_diagnostic` both state in
        // their own `# Errors`, and nothing in the language closes a descriptor
        // the host handed the process.
        .map_err(|error| Fault::fatal(format!("Core\\Cli::write could not write: {error}")))?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::escape(tainted string $text): string` — `rule:tooling/terminal-output-is-a-sink`'s named
    /// launderer for the terminal sink, and PHP's missing counterpart to
    /// `htmlspecialchars`.
    ///
    /// # Why this exists when `echo` already substitutes
    ///
    /// `rule:tooling/terminal-output-is-a-sink` puts the substitution at the sink and states outright that
    /// *ordinary output does not need this member*. What needs it is a program
    /// that wants the neutralized text **as a value** — to interpolate into a
    /// `Core\Str::format` template, to measure, or to compare — and, under
    /// `rule:security/launderers-are-sink-named`
    /// , to hand a `tainted string` to something else that refuses one. That
    /// is the whole of its job, which is why it could land before `write` did:
    /// the sink is `echo`, `write` is a second spelling of it, and neither is
    /// what this member answers.
    ///
    /// # One table, one implementation
    ///
    /// [`nvs_render::text::substitute`] is called rather than restated, so this
    /// member and [`nvs_runtime`]'s `echo` cannot answer differently — which is
    /// the property that would otherwise fail silently, since a launderer that
    /// neutralizes *less* than its sink is exactly the false confidence `rule:security/launderers-are-sink-named`
    /// refuses a generic `sanitize()` over. That module owns the table and
    /// its rows' reasoning; `rule:security/bidi-predicate`
    /// owns the bidi row's predicate.
    ///
    /// # Why the qualifier comes off
    ///
    /// [`Qual::Launder`] on the parameter, per `rule:security/launderers-are-sink-named`'s rule that a
    /// launderer names the one sink it is safe for: this one is safe for the
    /// terminal and for nothing else. The answer is still not safe in an HTML
    /// document, in a shell argument or in a SQL identifier, and each of those
    /// has its own launderer for exactly that reason.
    fn nvs_core_cli_escape(_ctx, args: [1]) {
        // A fatal rather than a throw, because nothing may catch a broken ABI:
        // the row's parameter is `CoreTy::Text`, so `E0401` refuses anything
        // that is not a `string` before a single instruction of this body runs,
        // which makes the message below unreachable from source.
        let text = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Cli::escape expected a `string`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        Ok(Value::str(NvsStr::new(
            nvs_render::text::substitute(text).as_bytes(),
        )))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::isTty(Cli\Stream $stream): bool` — replacing `posix_isatty`
    /// and `stream_isatty`, which answer the same question from two extensions.
    ///
    /// Reads the cached profile; `nvs_runtime::terminal` owns why there is one.
    fn nvs_core_cli_is_tty(_ctx, args: [1]) {
        let stream = stream_of(&args[0], "isTty")?;
        Ok(Value::bool(nvs_runtime::terminal::profile().is_tty(stream)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::width(): uint` — replacing a `tput cols` subprocess, which is
    /// how a PHP program asks today.
    fn nvs_core_cli_width(_ctx, _args: [0]) {
        Ok(Value::uint(u64::from(nvs_runtime::terminal::profile().width())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::height(): uint` — `tput lines`, read in the same call the
    /// width was.
    fn nvs_core_cli_height(_ctx, _args: [0]) {
        Ok(Value::uint(u64::from(nvs_runtime::terminal::profile().height())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::displayWidth(string $value): uint` — replacing `mb_strwidth`,
    /// which counts `mbstring`'s idea of a wide character and has no answer at
    /// all for a combining mark.
    ///
    /// The count is `nvs_runtime::terminal::display_width` and every decision
    /// inside it is that function's doc comment — what a control byte costs,
    /// why a newline answers the widest row and why a tab lands on a stop.
    /// This body is the surface: a `string` in, a `uint` out.
    ///
    /// It reads no profile. A column is the same width whether or not anything
    /// is watching, so this is the one member of this class that answers the
    /// same number with every standard stream redirected — which is what makes
    /// it assertable in a `.nvst` case at all.
    fn nvs_core_cli_display_width(_ctx, args: [1]) {
        // `escape`'s arm, for `escape`'s reason, and unreachable from source
        // for the same one: the row's parameter is `CoreTy::Text`, so `E0401`
        // refuses anything that is not a `string` before a single instruction
        // of this body runs.
        let text = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Cli::displayWidth expected a `string`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        // `try_from` rather than `as`, for the reason `Core\Str::length`'s body
        // states at length: `usize` is no wider than `u64` on any target
        // `deny.toml` builds for, so this conversion is total and the arm
        // below is unreachable from source — no diagnostic states that one,
        // because it is a property of the target rather than of the call. It
        // is the price of not writing a cast the lints this crate denies would
        // need a silence for, not a boundary a program can reach.
        let columns = u64::try_from(nvs_runtime::terminal::display_width(text))
            .map_err(|_| Fault::fatal("Core\\Cli::displayWidth counted past `uint`"))?;
        Ok(Value::uint(columns))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::colorDepth(): Cli\ColorDepth` — a question PHP has no
    /// spelling for at all, which is why every CLI package ships its own
    /// heuristic.
    ///
    /// An enum answers as its ordinal, exactly as a user-declared enum does
    /// (`rule:enums/closed-integer-type`).
    fn nvs_core_cli_color_depth(_ctx, _args: [0]) {
        Ok(Value::int(depth_ordinal(
            nvs_runtime::terminal::profile().color_depth(),
        )))
    }
}

// ------------------------------------------------------------------ the prompts

/// The question a prompt writes, neutralized exactly as `echo` neutralizes
/// what it is handed.
///
/// `rule:tooling/terminal-output-is-a-sink`'s substitution over the question and not only over the answer:
/// a question is ordinary output, and a program that interpolates a filename
/// or a claim from a token into one is writing untrusted bytes at a terminal
/// like any other. [`nvs_core_cli_escape`]'s own docs own the table.
///
/// # Errors
///
/// A [`Fault::fatal`] for an argument that is not a `string`, which is
/// unreachable from source: every prompt's first parameter is a
/// [`CoreTy::Text`], so `E0401` refuses anything else a phase earlier.
fn question_of(value: &Value, member: &str) -> Result<String, Fault> {
    let text = value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Cli::{member} expected a `string` question, got tag {}",
            value.tag_byte()
        ))
    })?;
    Ok(nvs_render::text::substitute(text).to_string())
}

/// Whether this call has anyone to ask — `rule:tooling/a-prompt-is-a-core-member`'s "with no controlling
/// terminal" as a predicate, and the reason a prompt never blocks a program
/// nobody is watching.
///
/// Two questions, and both have to answer yes. `nvs_runtime::terminal` asks
/// whether the *process* has a terminal at all; this adds whether *this
/// request's output* reaches it, because a question written into an HTTP
/// response body or a `Core\Out::capture` buffer is a question nobody sees —
/// and a read after it would hold the core until a keystroke that is never
/// coming.
fn watched(ctx: &nvs_runtime::Ctx) -> bool {
    ctx.output_reaches_the_terminal() && nvs_runtime::terminal::is_interactive()
}

/// One line off the controlling terminal, or which kind of silence came back
/// instead — nobody to ask, or nobody answering inside
/// [`nvs_runtime::terminal::ANSWER_DEADLINE`].
fn ask_terminal(ctx: &mut nvs_runtime::Ctx, question: &str, echo: Echo) -> Answer {
    // `rule:tooling/a-prompt-is-a-core-member`'s last paragraph, and it is *ahead* of both other answers on
    // purpose. All five prompts reach the terminal through here, so the queue
    // costs none of them a path of their own; and a scripted answer wins over a
    // terminal that is there, because a test whose result depended on whether
    // the developer ran it from a shell or from CI is the thing this exists to
    // remove. `Core\Test::scriptAnswers` is the only filler, so outside a test
    // this is one not-taken branch on an empty queue.
    if let Some(scripted) = ctx.take_scripted_answer() {
        return Answer::Line(scripted);
    }
    if !watched(ctx) {
        return Answer::Ended;
    }
    nvs_runtime::terminal::prompt(question, echo)
}

/// Whether this call has anyone to ask *or* an answer already written down —
/// [`watched`] for the members that decide before they ask.
///
/// `select` and `multiSelect` build a menu, which runs the caller's `labels`
/// callback, so they ask this first rather than rendering lines nobody will
/// read. Asking [`watched`] alone there would put those two members' scripted
/// answers out of reach while the other three drained the same queue.
fn answerable(ctx: &nvs_runtime::Ctx) -> bool {
    ctx.has_scripted_answer() || watched(ctx)
}

/// What a prompt with nowhere to read and nothing to fall back on throws —
/// `rule:tooling/a-prompt-is-a-core-member`'s `Core\Cli\NotInteractive`, which is in
/// `nvs_hir::errors::TREE` so that a program can `catch` it by name.
///
/// One function for all four prompts, so the sentence a program sees is the
/// same wherever it came from; `tests/conformance/core/cli-prompts-are-not-interactive-without-a-terminal.nvst`
/// is what freezes it.
///
/// It opens on the same clause as [`timed_out`] and then says which silence it
/// was, because the two are one judgement — the question could not be answered
/// and the call named no `default` — reached by two routes. That shared opening
/// is also what lets one case discharge both for
/// `tests/conformance_coverage.rs`'s error-path gate, whose own doc says a stem
/// two sites share freezes as one line.
fn not_interactive(member: &str) -> Fault {
    Fault::thrown_as(
        nvs_runtime::ThrownClass::CliNotInteractive,
        format!("no answer for Core\\Cli::{member}: there is no controlling terminal"),
    )
}

/// What a prompt that *was* asked and got no answer throws — the same class,
/// because a program catching `Core\Cli\NotInteractive` is asking "could this
/// question be answered", and a terminal nobody is sitting at answers no.
///
/// Only the second clause differs: [`not_interactive`]'s names a terminal that
/// does not exist, which would be false here, where one was opened and written
/// to. `rule:tooling/a-prompt-is-a-core-member`'s deadline paragraph is the rule and
/// `nvs_runtime::terminal`'s module doc owns how the bound is built.
fn timed_out(member: &str) -> Fault {
    Fault::thrown_as(
        nvs_runtime::ThrownClass::CliNotInteractive,
        format!(
            "no answer for Core\\Cli::{member}: nothing typed within {}s",
            nvs_runtime::terminal::ANSWER_DEADLINE.as_secs()
        ),
    )
}

/// What a prompt hands back for a silence: the `default` if the call named one,
/// and otherwise the throw that says which silence it was.
///
/// One function for all four prompts and for both silences, so that a member
/// cannot grow its own answer to a deadline — the failure mode `rule:tooling/a-prompt-is-a-core-member`'s
/// "it never blocks" exists to prevent is exactly a path that quietly waits
/// instead.
fn unanswered(quiet: &Answer, member: &str, fallback: Value) -> Result<Value, Fault> {
    if given(fallback) {
        return Ok(handed_back(fallback));
    }
    Err(match quiet {
        Answer::TimedOut => timed_out(member),
        // `Line` cannot arrive — a caller reaches this only where the match
        // above it took the line — and `Ended` is the ordinary unattended run.
        Answer::Ended | Answer::Line(_) => not_interactive(member),
    })
}

/// Whether an option was given at all — the [`Const::Null`] an omitting call
/// site passes.
fn given(value: Value) -> bool {
    !matches!(value.tag(), Some(Tag::Null) | None)
}

/// One more reference on a value this frame is about to hand back, since every
/// argument is borrowed from the caller's frame.
fn handed_back(value: Value) -> Value {
    #[expect(
        unsafe_code,
        reason = "the value is an argument, owned by the caller's frame for the \
                  length of this call, so the reference this helper returns has \
                  to be one of its own — `Value::retain` is a no-op for the \
                  unboxed tags a `bool` or a `null` default arrives as"
    )]
    unsafe {
        value.retain();
    }
    value
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::ask(string $question, {default?: string, validate?: callable}): tainted string`
    /// — `rule:tooling/a-prompt-is-a-core-member`'s first prompt, replacing `readline` and the
    /// `fgets(STDIN)` every PHP script writes instead of it.
    ///
    /// # It is not `fgets(STDIN)`, and that is the point
    ///
    /// § 4: *prompts read the controlling terminal, not standard input*, so
    /// `cat data.csv | myprog` can still ask a question. `nvs_runtime::terminal`
    /// is where the device is opened by name; nothing in this module reads the
    /// standard input stream at all, which
    /// `a_prompt_reads_the_controlling_terminal_and_not_stdin` holds
    /// shut over this file's own source.
    ///
    /// # Why the answer is `tainted` and the question is `Qual::Neutral`
    ///
    /// The answer came from outside the program, so it is
    /// [`CoreTy::TaintedStr`] — a promise about the *value*, unconditional,
    /// exactly as `rule:security/verification-does-not-launder`'s verified claims are. The question's own
    /// classification is [`Qual::Neutral`] rather than [`Qual::Sink`] because
    /// the terminal substitutes instead of refusing (§ 1) and because the
    /// answer's qualifier does not depend on the question's: a prompt built
    /// from a literal and a prompt built from a request parameter both answer
    /// something untrusted.
    fn nvs_core_cli_ask(ctx, args: [3]) {
        let question = question_of(&args[0], "ask")?;
        let (fallback, validate) = (args[1], args[2]);
        loop {
            let answer = match ask_terminal(ctx, &question, Echo::Shown) {
                Answer::Line(line) => line,
                ref quiet => return unanswered(quiet, "ask", fallback),
            };
            if answer.is_empty() && given(fallback) {
                return Ok(handed_back(fallback));
            }
            let value = Value::str(NvsStr::new(answer.as_bytes()));
            if !given(validate) {
                return Ok(value);
            }
            let verdict = nvs_runtime::call_callable(ctx, validate, &[value]);
            let accepted = match verdict {
                Ok(verdict) => {
                    let truthy = nvs_runtime::value_truthy(verdict);
                    #[expect(
                        unsafe_code,
                        reason = "the verdict is a fresh value this frame owns, \
                                  and a predicate answering a heap value would \
                                  otherwise leak one reference per attempt"
                    )]
                    unsafe {
                        verdict.release();
                    }
                    truthy
                }
                Err(fault) => {
                    #[expect(
                        unsafe_code,
                        reason = "the answer this frame built is owed a release \
                                  on the failing edge as much as on the taken one"
                    )]
                    unsafe {
                        value.release();
                    }
                    return Err(fault);
                }
            };
            if accepted {
                return Ok(value);
            }
            #[expect(
                unsafe_code,
                reason = "a refused answer is dropped here rather than returned, \
                          so the reference this frame built goes with it"
            )]
            unsafe {
                value.release();
            }
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::confirm(string $question, {default?: bool}): bool` — § 4's
    /// second prompt, and the one whose answer is **not** `tainted`.
    ///
    /// `rule:security/taint-propagation` launders a checked conversion, and a closed two-case
    /// answer set is exactly one: nothing of what was typed survives into the
    /// `bool`, so there is no untrusted content left for a sink to act on.
    ///
    /// An answer that is neither yes nor no is asked again rather than read as
    /// "no", because a program that deletes on `false` would otherwise treat a
    /// typo as consent. An **empty** line takes the `default` where one was
    /// given, which is what the `[Y/n]` in the question is promising.
    fn nvs_core_cli_confirm(ctx, args: [2]) {
        let fallback = args[1];
        let shown = match (given(fallback), fallback.as_bool() == Some(true)) {
            (false, _) => " [y/n] ",
            (true, true) => " [Y/n] ",
            (true, false) => " [y/N] ",
        };
        let question = format!("{}{shown}", question_of(&args[0], "confirm")?);
        loop {
            let answer = match ask_terminal(ctx, &question, Echo::Shown) {
                Answer::Line(line) => line,
                ref quiet => return unanswered(quiet, "confirm", fallback),
            };
            match answer.trim().to_ascii_lowercase().as_str() {
                "y" | "yes" => return Ok(Value::bool(true)),
                "n" | "no" => return Ok(Value::bool(false)),
                "" if given(fallback) => return Ok(handed_back(fallback)),
                // Anything else, and an empty line with no default: ask again.
                _ => {}
            }
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::select<T>(string $question, array<T> $choices, {labels?: callable, default?: T}): T`
    /// — § 4's third prompt.
    ///
    /// **It answers the value, not the index**, which is
    /// `rule:core-api/shape-rules` R4 and R5
    /// and is what earns the generic: the compiler knows the result's type
    /// from the options array, so no call site casts and none indexes back
    /// into the list it just passed.
    ///
    /// The list is written above the question, one `1) label` per line, and
    /// the answer is read as that number. A number outside the list is asked
    /// again rather than clamped — clamping would silently choose a
    /// neighbour, which on a menu is the one failure mode nobody checks for.
    fn nvs_core_cli_select(ctx, args: [4]) {
        let question = question_of(&args[0], "select")?;
        let (labels, fallback) = (args[2], args[3]);
        let options = options_of(args[1], "select")?;
        if options.is_empty() {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                "Core\\Cli::select was given nothing to choose between",
            ));
        }

        // The empty list above is refused whether or not anyone is watching —
        // it is a bug in the program either way — but the menu is only *built*
        // where it can be read or answered, so an unattended run calls no
        // `labels` callback and renders no line it would then discard.
        if !answerable(ctx) {
            return if given(fallback) {
                Ok(handed_back(fallback))
            } else {
                Err(not_interactive("select"))
            };
        }

        let mut menu = menu_of(ctx, &options, labels)?;
        menu.push_str(&question);
        menu.push(' ');

        loop {
            let answer = match ask_terminal(ctx, &menu, Echo::Shown) {
                Answer::Line(line) => line,
                ref quiet => return unanswered(quiet, "select", fallback),
            };
            let answer = answer.trim();
            if answer.is_empty() && given(fallback) {
                return Ok(handed_back(fallback));
            }
            if let Ok(chosen) = answer.parse::<usize>()
                && let Some(option) = chosen.checked_sub(1).and_then(|at| options.get(at))
            {
                return Ok(handed_back(*option));
            }
        }
    }
}

/// The menu `select` and `multiSelect` write above their question: one
/// `1) label` per line, in the options' own order.
///
/// One builder for both, because the numbering a caller types against is the
/// same numbering — a second copy is how the two would come to disagree about
/// what `2` means, which is the one thing a menu cannot afford.
fn menu_of(ctx: &mut nvs_runtime::Ctx, options: &[Value], labels: Value) -> Result<String, Fault> {
    let mut menu = String::new();
    for (at, option) in options.iter().enumerate() {
        menu.push_str(&format!(
            "{}) {}\n",
            at + 1,
            label_of(ctx, labels, *option)?
        ));
    }
    Ok(menu)
}

/// Which options `answer` names, as one flag per option — or `None` where a
/// token was not a number on the menu, which [`nvs_core_cli_multi_select`] asks
/// again rather than guessing at.
///
/// A flag per option rather than the numbers as typed, because that is what
/// makes the answer a **set**: an option named twice is chosen once, and the
/// order it comes back in is the menu's rather than the typing's. Splitting on
/// commas and spaces only — rather than on every non-digit — is what keeps
/// `1-3` a refusal instead of a range spelling silently read as two. An empty
/// token is dropped rather than refused, so a trailing separator is forgiven:
/// it has exactly one reading, which is not true of anything else here.
fn chosen_of(answer: &str, offered: usize) -> Option<Vec<bool>> {
    let mut chosen = vec![false; offered];
    for token in answer
        .split([',', ' ', '\t'])
        .filter(|token| !token.is_empty())
    {
        let at = token.parse::<usize>().ok()?.checked_sub(1)?;
        *chosen.get_mut(at)? = true;
    }
    Some(chosen)
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::multiSelect<T>(string $question, array<T> $choices, {labels?: callable}): array<T>`
    /// — § 4's fourth prompt, and the only one whose answer is a set.
    ///
    /// It is [`nvs_core_cli_select`] with a second reading loop and nothing
    /// else: the same menu from the same [`menu_of`], the same numbering, the
    /// same refusal of anything off the list. What differs is the parse — a
    /// line of numbers rather than one — and three decisions that follow from
    /// the answer being a set rather than a value:
    ///
    /// - **The order is the menu's, and each choice appears once.** Numbers
    ///   typed as `3,1,3` answer options 1 and 3 in that order. The order
    ///   someone typed is not information they meant to give, and an answer
    ///   paired against `$choices` at the call site is the whole reason § 4
    ///   returns values rather than indices.
    /// - **An empty line is the empty array, not a silence.** Choosing none is
    ///   an answer a multi-select has to be able to give, and it is the only
    ///   spelling for it — the alternative forces every caller to add a "none
    ///   of these" choice of its own. § 4's table gives this member no
    ///   `default` precisely because it needs none.
    /// - **A token that is not a number on the menu is asked again**, exactly
    ///   as `select` re-asks: `1-3` is refused rather than read as a range,
    ///   because reading it as one would silently drop option 2.
    ///
    /// The accepted range and separator are appended to the question, which is
    /// `confirm`'s `[y/n]` rather than a new idea: the shape of the answer
    /// belongs to the member, so the member is what can state it.
    fn nvs_core_cli_multi_select(ctx, args: [3]) {
        let question = question_of(&args[0], "multiSelect")?;
        let labels = args[2];
        let options = options_of(args[1], "multiSelect")?;
        if options.is_empty() {
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                "Core\\Cli::multiSelect was given nothing to choose between",
            ));
        }

        // `select`'s reasoning: the empty list is a bug either way, but the
        // menu is only built where it can be read or answered, so an unattended
        // run calls no `labels` callback.
        if !answerable(ctx) {
            return Err(not_interactive("multiSelect"));
        }

        let mut menu = menu_of(ctx, &options, labels)?;
        menu.push_str(&question);
        menu.push_str(&format!(" [1-{}, comma-separated] ", options.len()));

        loop {
            let answer = match ask_terminal(ctx, &menu, Echo::Shown) {
                Answer::Line(line) => line,
                // No `default` in § 4's row, so the shared helper is reached
                // with a `null` and always throws.
                ref quiet => return unanswered(quiet, "multiSelect", Value::null()),
            };
            let Some(chosen) = chosen_of(answer.trim(), options.len()) else {
                continue;
            };
            let mut picked = NvsArray::new();
            for (at, option) in options.iter().enumerate() {
                if chosen[at] {
                    picked.append(handed_back(*option));
                }
            }
            return Ok(Value::array(picked));
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::secret(string $question): secret tainted string` — § 4's
    /// fifth prompt, and the clearest demonstration of why
    /// `rule:security/secret-qualifier`'s
    /// qualifier was worth having: a password typed here structurally cannot
    /// be echoed, logged, dumped, put in a `Throwable` message or serialized,
    /// and it cost one row's return type to say so.
    ///
    /// The echo is turned off at the terminal rather than overwritten
    /// afterwards, so the characters are never on the screen at all — the
    /// difference matters for a scrollback buffer, for a screen recording and
    /// for anyone standing behind the person typing.
    ///
    /// There is no `default`: a password nobody typed is not a password, so an
    /// unattended run throws [`not_interactive`] rather than proceeding with
    /// something a configuration file could have chosen.
    fn nvs_core_cli_secret(ctx, args: [1]) {
        let question = question_of(&args[0], "secret")?;
        let answer = match ask_terminal(ctx, &question, Echo::Hidden) {
            Answer::Line(line) => line,
            // `secret` names no `default` at all, so the shared helper is
            // reached with a `null` and always throws — which is the rule, not
            // a shortcut: a password nobody typed is not a password.
            ref quiet => return unanswered(quiet, "secret", Value::null()),
        };
        Ok(Value::str(NvsStr::new(answer.as_bytes())))
    }
}

/// The values of a `select` options array, in their own order, each borrowed
/// from the array the caller still owns.
///
/// # Errors
///
/// A [`Fault::fatal`] for an argument that is not an array, which is
/// unreachable from source: the parameter is a [`CoreTy::Array`], so `E0401`
/// refuses anything else a phase earlier.
fn options_of(value: Value, member: &str) -> Result<Vec<Value>, Fault> {
    let array = value.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Cli::{member} expected an `array`, got tag {}",
            value.tag_byte()
        ))
    })?;
    let array = crate::arr::borrowed(array);
    let mut options = Vec::new();
    let mut from = 0_usize;
    while let Some(slot) = array.next_slot(from) {
        from = slot + 1;
        options.push(
            array
                .value_at(slot)
                .expect("next_slot only names live entries"),
        );
    }
    Ok(options)
}

/// The line `select` shows for one option: what `labels` answered for it, or
/// the option rendered the way `echo` would render it.
///
/// `nvs_runtime::stringify` rather than `value_to_string` directly, so an
/// object option renders through `rule:classes/stringable`'s `toString` — a menu of
/// `Core\Time\Zone`s should read as its zones, and a class with no renderer
/// throws here rather than printing a placeholder nobody can choose between.
fn label_of(ctx: &mut nvs_runtime::Ctx, labels: Value, option: Value) -> Result<String, Fault> {
    let rendered = if given(labels) {
        let answered = nvs_runtime::call_callable(ctx, labels, &[option])?;
        let text = nvs_runtime::stringify(ctx, answered);
        #[expect(
            unsafe_code,
            reason = "the callback's answer is a fresh value this frame owns, \
                      and the rendering above took its own reference"
        )]
        unsafe {
            answered.release();
        }
        text?
    } else {
        nvs_runtime::stringify(ctx, option)?
    };
    let line = rendered.as_text().unwrap_or_default().to_owned();
    #[expect(
        unsafe_code,
        reason = "`stringify` answers a fresh reference, which this frame owes a \
                  release once the bytes are copied out of it"
    )]
    unsafe {
        rendered.release();
    }
    Ok(nvs_render::text::substitute(&line).to_string())
}

// ------------------------------------------------------------------ the carrier

/// The carrier class's fully-qualified name, as
/// [`CoreTy::Instance`] spells it.
///
/// Taken from `nvs_runtime::CARRIER_CLI_TEXT` rather than written again here:
/// the *sink* decides what its carrier is (`rule:tooling/echo-always-has-a-sink`), the sink lives in
/// `nvs-runtime`, and `nvs_runtime::value_to_string` renders whatever that
/// constant names. Two spellings could disagree and the render would silently
/// stop happening.
///
/// `pub` for the one edge `crate::html::MARKUP_NAME` already has: `rule:tooling/styling-is-a-value-not-a-grammar`
/// 's `Text + Text` is a row in `nvs_types`' operator table, that crate has
/// no `nvs-runtime` dependency to read the runtime constant through, and a
/// third spelling of the name is the drift this comment is about. It reaches
/// it as `nvs_types::CORE_CLI_TEXT_CLASS`.
pub const NAME: &str = nvs_runtime::CARRIER_CLI_TEXT;

/// `Core\Cli\Text`'s class card — `rule:core-api/reference-card`.
const TEXT_CARD: ClassDoc = ClassDoc {
    short: "Text for the terminal, which can have colours and styles. You join two pieces with \
            `+`. Outside an HTTP request, `Core\\Out::capture` returns the captured output as one.",
};

/// Spec § 13's `Core\Cli\Text` — `rule:security/capture-answers-the-carrier`'s slot, and `rule:tooling/styling-is-a-value-not-a-grammar`'s first
/// constructor over it.
///
/// Two slots: the rendering standard output takes, which is the one
/// `nvs_runtime::value_to_string` renders a carrier as, and the [`TEXT_RUNS`]
/// every other stream is rendered from.
pub(crate) const TEXT: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&TEXT_CARD),
    methods: &[
        CoreMethod {
            name: "plain",
            names: &["text"],
            params: &[CoreTy::Text(Qual::Launder)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_cli_text_plain",
            doc: Some(&PLAIN_DOC),
        },
        CoreMethod {
            name: "styled",
            names: &["text", "style"],
            params: &[CoreTy::Text(Qual::Launder), CoreTy::Instance(STYLE_NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_cli_text_styled",
            doc: Some(&STYLED_DOC),
        },
    ],
    instance: &[CoreMethod {
        name: "text",
        names: &[],
        params: &[],
        defaults: &[],
        return_ty: CoreTy::Str,
        symbol: "nvs_core_cli_text_text",
        doc: Some(&TEXT_TEXT_DOC),
    }],
    slots: &["text", "runs"],
    constants: &[],
};

/// `Core\Cli\Text::plain`'s reference card — `rule:core-api/reference-card`.
const PLAIN_DOC: MethodDoc = MethodDoc {
    short: "Answers `$text` as a `Core\\Cli\\Text`, with every control byte already replaced by the \
            visible glyph `Core\\Cli::escape` gives it. This is the terminal sink's own carrier: \
            `echo` writes a `Text` through unchanged, which is why the substitution happens here \
            instead.",
    params: &[ParamDoc {
        name: "text",
        desc: "The text to carry. Its `tainted` qualifier is removed, for the same reason \
               `Core\\Cli::escape` removes it: the terminal is the sink this neutralizes for, and \
               nothing is left in the answer for it to act on.",
        shape: &[],
    }],
    ret: "A `Core\\Cli\\Text` carrying the neutralized form. It composes with another `Text` and \
          is written by `echo`; it carries no styling, which is `styled`'s.",
    errors: &[],
};

/// `Core\Cli\Text::text`'s reference card — `rule:core-api/reference-card`.
const TEXT_TEXT_DOC: MethodDoc = MethodDoc {
    short: "Answers what this `Core\\Cli\\Text` says, as a `string`, leaving its styling out: the \
            text of every run joined, already control-byte-substituted. This is what a \
            `Core\\Out::capture` filter reads before it writes a new carrier.",
    params: &[],
    ret: "The runs' own text, with no escape sequence in it that a `Cli\\Style` put there. It is \
          safe to hand straight back to `plain` or `styled`, the terminal sink's substitution \
          being idempotent.",
    errors: &[],
};

/// `Core\Cli\Text::styled`'s reference card — `rule:core-api/reference-card`.
const STYLED_DOC: MethodDoc = MethodDoc {
    short: "Answers `$text` as a `Core\\Cli\\Text` wearing `$style`, with the text itself \
            neutralized exactly as `plain` neutralizes it — so the only control bytes in the \
            answer are the ones the style put there. Replaces the `\"\\e[31m…\"` string every PHP \
            CLI program builds by hand.",
    params: &[
        ParamDoc {
            name: "text",
            desc: "The text to carry. Its `tainted` qualifier is removed for `plain`'s reason, \
                   and an escape sequence inside it is substituted rather than obeyed.",
            shape: &[],
        },
        ParamDoc {
            name: "style",
            desc: "The style to wear, as a value — Novis has no markup or escape grammar to write \
                   one in.",
            shape: &[],
        },
    ],
    ret: "A `Core\\Cli\\Text` carrying the neutralized text between the style's own escape \
          sequence and a reset. The styling is rendered for the terminal this process actually \
          has, so it is absent entirely when standard output is not one.",
    errors: &[],
};

/// Slot 1 of a [`TEXT`] — its runs, two entries each: the substituted body,
/// and the [`STYLE`] that body wears or `null` for one that wears none.
///
/// **The runs are what a `Text` is.** `rule:tooling/the-terminal-profile-resolves-once`
/// drops styling entirely on a stream that is no terminal, and which stream a
/// value goes to is not known until something writes it — so a style rendered
/// into bytes at construction would fix one answer for every stream, which is
/// exactly what [`nvs_core_cli_write`] renders per stream instead.
///
/// Flat rather than an array of pairs because the pairing is this module's
/// own: no member answers a run and none takes one, so the layout owes the
/// surface nothing.
const TEXT_RUNS: usize = 1;

/// A `Core\Cli\Text` of one unstyled run carrying `text`, which must be a
/// `Tag::Str` value the caller is transferring.
///
/// **This transfers bytes; it does not neutralize them.** Every caller owes
/// that itself, and there are two: [`crate::out`]'s `capture`, whose bytes came
/// out of the sink already, and [`nvs_core_cli_text_plain`], which substitutes
/// over its argument first. That is the whole of what keeps `rule:tooling/terminal-output-is-a-sink`'s raw
/// path closed — `nvs_runtime::helpers::is_carrier_value` owns why the sink
/// trusts the class rather than the bytes.
///
/// An unstyled run renders as its own bytes at every depth, so the carrier
/// slot is the text itself and this constructor asks the profile nothing.
pub(crate) fn built(text: nvs_runtime::Value) -> nvs_runtime::Value {
    let mut runs = NvsArray::new();
    #[expect(
        unsafe_code,
        reason = "the run keeps a reference of its own to the bytes the carrier \
                  slot also holds, and the caller transferred exactly one"
    )]
    unsafe {
        text.retain();
    }
    runs.append(text);
    runs.append(Value::null());
    crate::instance::build(&TEXT, [text, Value::array(runs)])
}

/// A `Core\Cli\Text` of `runs`, whose [`nvs_runtime::CARRIER_TEXT_SLOT`] holds
/// them rendered for **standard output**.
///
/// That slot is the one `nvs_runtime::value_to_string` renders a carrier as,
/// and it has no stream to ask about: `echo` writes the request's output
/// channel, so standard output's rendering is the one it owes. Every other
/// stream is rendered from the runs by whatever writes to it.
///
/// # Errors
///
/// Whatever [`rendered_at`] answers, with the runs released — so a fatal
/// leaves no array behind and no half-built instance.
fn of_runs(runs: NvsArray) -> Result<Value, Fault> {
    let runs = Value::array(runs);
    match rendered_at(runs, depth_of(Stream::Out)) {
        Ok(rendered) => Ok(crate::instance::build(
            &TEXT,
            [Value::str(NvsStr::new(rendered.as_bytes())), runs],
        )),
        Err(fault) => {
            #[expect(
                unsafe_code,
                reason = "this frame owns the array's only reference, and no \
                          instance was built to take it over"
            )]
            unsafe {
                runs.release();
            }
            Err(fault)
        }
    }
}

/// Every run of `runs`, rendered for a stream that can show `depth`.
///
/// The bytes are the runs' own, substituted by whichever constructor built
/// them; what `depth` decides is the escape sequence in front of each body and
/// the reset after it, and [`ColorDepth::None`] — the answer for a redirected
/// stream — drops both. That is `rule:tooling/the-terminal-profile-resolves-once`'s
/// *"when the stream is not a terminal, styling is dropped entirely"*, applied
/// where the stream is known.
///
/// **What it spends:** one string the size of the answer and one `Vec` of the
/// run entries, per call, both freed before the bytes reach the stream.
///
/// # Errors
///
/// A `Fault::fatal` for a shape this module did not build, which is
/// unreachable from source: the runs of a `Text` are written by [`built`] and
/// by [`of_runs`]'s callers and read by nothing a program can reach.
fn rendered_at(runs: Value, depth: ColorDepth) -> Result<String, Fault> {
    let array = runs.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "a `{NAME}` holds its runs in an array, got tag {}",
            runs.tag_byte()
        ))
    })?;
    let held: Vec<Value> = crate::str::Elements::of(array).collect();
    let mut rendered = String::new();
    for run in held.chunks(2) {
        let body = run[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "a `{NAME}` run carries its body as a `string`, got tag {}",
                run[0].tag_byte()
            ))
        })?;
        let worn = run.get(1).copied().unwrap_or_else(Value::null);
        let opening = if worn.tag() == Some(Tag::Object) {
            let style = crate::instance::receiver(worn, &STYLE, "a run")?;
            sgr(
                ink_of(crate::instance::slot(style, STYLE_COLOR), "color")?,
                ink_of(crate::instance::slot(style, STYLE_BACKGROUND), "background")?,
                crate::instance::slot(style, STYLE_FLAGS)
                    .as_int()
                    .unwrap_or(0),
                depth,
            )
        } else {
            String::new()
        };
        if opening.is_empty() {
            rendered.push_str(body);
        } else {
            // One reset closes everything the opening sequence set, so a run
            // never leaks its own styling into the one written after it.
            rendered.push_str(&opening);
            rendered.push_str(body);
            rendered.push_str("\u{1B}[0m");
        }
    }
    Ok(rendered)
}

/// How much colour `stream` may show in this process — [`depth_for`] over the
/// profile `rule:tooling/the-terminal-profile-resolves-once` resolves once.
fn depth_of(stream: Stream) -> ColorDepth {
    let profile = nvs_runtime::terminal::profile();
    depth_for(
        profile.color_depth(),
        profile.is_tty(Stream::Out),
        profile.is_tty(stream),
    )
}

/// One stream's colour depth, from the **process** depth and whether standard
/// output and that stream are terminals.
///
/// A pure function of the three, so a test can ask it about a terminal the
/// process running it does not have.
///
/// The depth itself stays a process answer — `Cli::colorDepth` takes no stream,
/// and `nvs_runtime::terminal` resolves it from standard output and the forcing
/// variables together. What varies by stream is whether that depth reaches it:
/// a depth that survived a standard output which is **not** a terminal was
/// forced, and a forced depth is the run's answer everywhere; otherwise a
/// stream shows what the process can when it is a terminal, and nothing when it
/// is redirected.
fn depth_for(process: ColorDepth, out_is_tty: bool, stream_is_tty: bool) -> ColorDepth {
    if process == ColorDepth::None {
        return ColorDepth::None;
    }
    if !out_is_tty || stream_is_tty {
        return process;
    }
    ColorDepth::None
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli\Text::plain(string $text): Cli\Text` — `rule:tooling/styling-is-a-value-not-a-grammar`'s first
    /// constructor, and the half of § 1's raw path that keeps it from being a
    /// hole.
    ///
    /// `echo` writes a carrier through **unchanged**, so a `Text` is the one
    /// value in the language whose bytes the terminal sink does not inspect.
    /// What makes that safe is that neither constructor accepts bytes it has
    /// not neutralized: § 2's *"not a trust assertion a developer can be
    /// tricked into making, it is a constructor that cannot produce an injected
    /// sequence"*. So this is exactly [`nvs_core_cli_escape`]'s table with a
    /// carrier around the answer, and the two call the same
    /// `nvs_render::text::substitute` rather than each holding a copy.
    ///
    /// The control bytes a `Text` may legitimately hold are `styled`'s, which
    /// puts them there itself from a `Cli\Style` — never from its own argument.
    fn nvs_core_cli_text_plain(_ctx, args: [1]) {
        // A fatal rather than a throw, for the reason `escape`'s body gives:
        // `E0401` refuses a non-`string` argument a phase earlier, so this
        // message is unreachable from source and a broken ABI is not catchable.
        let text = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Cli\\Text::plain expected a `string`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        Ok(built(Value::str(NvsStr::new(
            nvs_render::text::substitute(text).as_bytes(),
        ))))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli\Text::text(): string` — the runs' own bodies, joined, with no
    /// styling: what this `Text` says rather than how it will look.
    ///
    /// # Why this carrier may be read back, and `Core\Html\Markup` may not
    ///
    /// `rule:security/capture-answers-the-carrier` gives its reason as the
    /// second escape: bytes that have been through a sink, handed back as a
    /// `string` and re-emitted, would be escaped twice and the page corrupted.
    /// The terminal sink cannot do that to itself — `rule:tooling/terminal-output-is-a-sink`'s
    /// substitution replaces a control byte with a *visible glyph*, which is
    /// not a control byte, so a second pass has nothing left to replace. HTML
    /// escaping is not idempotent (`&amp;` escapes again), which is why the
    /// read half is here and there is none on the markup carrier.
    ///
    /// **The styling is left out**, and that is the other half of what makes
    /// the answer safe to pass back into a constructor: a run's escape
    /// sequence would be substituted on the way in and come back as a Control
    /// Picture, so what can be read is what was read *out of* the program's
    /// own text. [`crate::out`]'s `{through:}` is what wanted this — its
    /// callable is handed the captured carrier and answers one, so without a
    /// read it could only replace what it was given.
    ///
    /// **What it spends:** one string the size of the text, per call.
    fn nvs_core_cli_text_text(_ctx, args: [1]) {
        let receiver = crate::instance::receiver(args[0], &TEXT, "text")?;
        let plain = rendered_at(
            crate::instance::slot(receiver, TEXT_RUNS),
            ColorDepth::None,
        )?;
        Ok(Value::str(NvsStr::new(plain.as_bytes())))
    }
}

/// The symbol `Text + Text` lowers to — `rule:tooling/styling-is-a-value-not-a-grammar`'s composition rule, and
/// the third way a program obtains a [`TEXT`].
///
/// Row-less exactly as [`crate::html::MARKUP_CONCAT_SYMBOL`] is, and for the
/// same reason: `+` is the spelling § 2 gives composition, so the operator's
/// own lowering is the only thing allowed to reach this. A member row would be
/// a way in that took its operands from anywhere, and both operands here are
/// trusted **because they are already a `Text`** rather than because this body
/// checked anything.
///
/// `nvs-ir` reaches it through `nvs_types`, as `CORE_CLI_TEXT_CONCAT`.
pub const TEXT_CONCAT_SYMBOL: &str = "nvs_core_cli_text_concat";

nvs_runtime::nvs_helper! {
    /// `$a + $b` over two `Core\Cli\Text` — `rule:tooling/styling-is-a-value-not-a-grammar`'s *"`Text + Text` is
    /// `Text`, immutable (R20), composing the way `Markup` already does"*, and
    /// the whole of what [`TEXT_CONCAT_SYMBOL`] does.
    ///
    /// **Nothing is substituted here**, which is the rule rather than an
    /// omission: every control byte either operand holds was put there by
    /// `Cli\Style`, since § 2's two constructors substitute on the way in and
    /// nothing else builds a `Text` from source. Substituting again would
    /// neutralize a style the program asked for, and the result would be the
    /// escape sequence printed as a Control Picture.
    ///
    /// A styled run closes its own sequence with a reset as it is rendered
    /// ([`rendered_at`]), so the sum carries no styling across the seam and
    /// neither operand's appearance changes.
    ///
    /// The pair is the operator table's own — `nvs_types::expr::operators`'
    /// `carrier_composition_result` admits a carrier beside its own kind and
    /// refuses every other object beside `+` — so the only judgement left is
    /// the tag check below.
    ///
    /// **What it spends:** one array holding both operands' runs, one string
    /// for the sink's rendering of them and one object, all charged to the
    /// request. Neither operand is touched: a `Text` is immutable, so `$a + $b`
    /// leaves both where they were, and a chain of `n` fragments is `n - 1` of
    /// these.
    fn nvs_core_cli_text_concat(_ctx, args: [2]) {
        let mut runs = NvsArray::new();
        for (operand, position) in [(args[0], "the left operand"), (args[1], "the right operand")] {
            let carrier = crate::instance::receiver(operand, &TEXT, position)?;
            let held = crate::instance::slot(carrier, TEXT_RUNS);
            // Unreachable from source: a carrier's slots hold what this module
            // put there. The check stays because the ABI is `*const Value`,
            // which carries no promise.
            let array = held.array_ptr().ok_or_else(|| {
                Fault::fatal(format!(
                    "{position} of `Text + Text` holds its runs in an array, got tag {}",
                    held.tag_byte()
                ))
            })?;
            for entry in crate::str::Elements::of(array) {
                #[expect(
                    unsafe_code,
                    reason = "the sum keeps a reference of its own to every run \
                              its operands lend it, and both operands are live \
                              for the length of this call"
                )]
                unsafe {
                    entry.retain();
                }
                runs.append(entry);
            }
        }
        of_runs(runs)
    }
}

// ------------------------------------------------------ the styling value types

/// `Core\Cli\Color`'s fully-qualified name — see [`STREAM_NAME`].
pub(crate) const COLOR_NAME: &str = r"Core\Cli\Color";

/// The symbol `Color::index` registers, written once: the sixteen named colours
/// are [`Const::Built`] constants *over it*, so a second spelling here would be
/// a constant that builds nothing.
const INDEX_SYMBOL: &str = "nvs_core_cli_color_index";

/// Slot 0 of a [`COLOR`] — which of the two colour spaces its `value` is in,
/// as [`INK_INDEXED`] or [`INK_RGB`].
const COLOR_KIND: usize = 0;

/// Slot 1 of a [`COLOR`] — a palette entry `0..=255`, or a packed
/// `r << 16 | g << 8 | b`, according to slot 0.
const COLOR_VALUE: usize = 1;

/// [`COLOR_KIND`] for a palette entry.
const INK_INDEXED: i64 = 0;

/// [`COLOR_KIND`] for 24-bit colour.
const INK_RGB: i64 = 1;

/// `Core\Cli\Color`'s class card — `rule:core-api/reference-card`.
const COLOR_CARD: ClassDoc = ClassDoc {
    short: "A terminal colour: one of the sixteen named colours, or any RGB colour. A terminal \
            that shows fewer colours gets the nearest colour it can show.",
};

/// `rule:tooling/styling-is-a-value-not-a-grammar`'s `Cli\Color` — **a value type, not an enum**.
///
/// `rule:enums/closed-integer-type`'s closed
/// named integer type does not fit a set with sixteen million members, so the
/// sixteen the terminal names are class constants and the rest is constructed.
/// Two slots rather than one packed integer because the two colour spaces are
/// genuinely different questions — a palette entry is resolved by the
/// terminal's own theme and a triple is not — and [`sgr_ink`] degrades between
/// them, which a single encoded number would make an arithmetic puzzle.
pub(crate) const COLOR: CoreClass = CoreClass {
    name: COLOR_NAME,
    doc: Some(&COLOR_CARD),
    methods: &[
        CoreMethod {
            name: "index",
            names: &["index"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Instance(COLOR_NAME),
            symbol: INDEX_SYMBOL,
            doc: Some(&INDEX_DOC),
        },
        CoreMethod {
            name: "rgb",
            names: &["red", "green", "blue"],
            params: &[CoreTy::Uint, CoreTy::Uint, CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Instance(COLOR_NAME),
            symbol: "nvs_core_cli_color_rgb",
            doc: Some(&RGB_DOC),
        },
    ],
    instance: &[],
    slots: &["kind", "value"],
    constants: NAMED,
};

/// `Core\Cli\Color::index`'s reference card — `rule:core-api/reference-card`.
const INDEX_DOC: MethodDoc = MethodDoc {
    short: "A colour from the terminal's 256-entry palette, whose first sixteen entries are the \
            named constants on this class.",
    params: &[ParamDoc {
        name: "index",
        desc: "The palette entry, `0` to `255`.",
        shape: &[],
    }],
    ret: "The colour that entry names, which a terminal with a smaller palette renders as the \
          nearest of the sixteen it has.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$index` is above `255`, which no palette has an entry for.",
    }],
};

/// `Core\Cli\Color::rgb`'s reference card — `rule:core-api/reference-card`.
const RGB_DOC: MethodDoc = MethodDoc {
    short: "A 24-bit colour, for the terminals that have one — the sixteen million members that \
            are why this class is a value type and not an enum.",
    params: &[
        ParamDoc {
            name: "red",
            desc: "The red channel, `0` to `255`.",
            shape: &[],
        },
        ParamDoc {
            name: "green",
            desc: "The green channel, `0` to `255`.",
            shape: &[],
        },
        ParamDoc {
            name: "blue",
            desc: "The blue channel, `0` to `255`.",
            shape: &[],
        },
    ],
    ret: "The colour, which is written as itself on a true-colour terminal and as the nearest \
          palette entry on one without.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "A channel is above `255`.",
    }],
};

/// `rule:tooling/styling-is-a-value-not-a-grammar`'s *"the sixteen named colours are class constants"*, as the
/// [`Const::Built`] rows that makes them: `Color::RED` is `Color::index(1)`
/// inlined at the use site, so it is one per-request allocation like any other
/// object and nothing is shared between isolates.
///
/// The ordinals are the ANSI palette's own, which is what lets [`sgr_ink`] emit
/// `30 + index` for the first eight and `90 + index - 8` for their bright
/// halves without a table.
const NAMED: &[CoreConst] = &[
    named(
        "BLACK",
        0,
        "ANSI palette entry 0 — black, as the terminal's theme renders it.",
    ),
    named("RED", 1, "ANSI palette entry 1 — red."),
    named("GREEN", 2, "ANSI palette entry 2 — green."),
    named("YELLOW", 3, "ANSI palette entry 3 — yellow."),
    named("BLUE", 4, "ANSI palette entry 4 — blue."),
    named("MAGENTA", 5, "ANSI palette entry 5 — magenta."),
    named("CYAN", 6, "ANSI palette entry 6 — cyan."),
    named(
        "WHITE",
        7,
        "ANSI palette entry 7 — white, which a light theme renders as near-black.",
    ),
    named(
        "BRIGHT_BLACK",
        8,
        "ANSI palette entry 8 — the grey a terminal shows for dimmed text.",
    ),
    named("BRIGHT_RED", 9, "ANSI palette entry 9 — bright red."),
    named("BRIGHT_GREEN", 10, "ANSI palette entry 10 — bright green."),
    named(
        "BRIGHT_YELLOW",
        11,
        "ANSI palette entry 11 — bright yellow.",
    ),
    named("BRIGHT_BLUE", 12, "ANSI palette entry 12 — bright blue."),
    named(
        "BRIGHT_MAGENTA",
        13,
        "ANSI palette entry 13 — bright magenta.",
    ),
    named("BRIGHT_CYAN", 14, "ANSI palette entry 14 — bright cyan."),
    named("BRIGHT_WHITE", 15, "ANSI palette entry 15 — bright white."),
];

/// One [`NAMED`] row: the palette entry `index`, as the call to
/// [`INDEX_SYMBOL`] that builds it.
///
/// The arguments are indexed out of [`INDEX_ARGS`] rather than built here
/// because a `const fn` cannot make a `&'static` slice, and a roster of sixteen
/// hand-written `Const::Built` blocks is sixteen places for the symbol to be
/// misspelled.
const fn named(name: &'static str, index: usize, desc: &'static str) -> CoreConst {
    CoreConst {
        name,
        ty: CoreTy::Instance(COLOR_NAME),
        value: Const::Built {
            symbol: INDEX_SYMBOL,
            args: INDEX_ARGS[index],
        },
        desc,
    }
}

/// The sixteen single-argument lists [`named`] indexes, one per palette entry.
const INDEX_ARGS: [&[Const]; 16] = [
    &[Const::Uint(0)],
    &[Const::Uint(1)],
    &[Const::Uint(2)],
    &[Const::Uint(3)],
    &[Const::Uint(4)],
    &[Const::Uint(5)],
    &[Const::Uint(6)],
    &[Const::Uint(7)],
    &[Const::Uint(8)],
    &[Const::Uint(9)],
    &[Const::Uint(10)],
    &[Const::Uint(11)],
    &[Const::Uint(12)],
    &[Const::Uint(13)],
    &[Const::Uint(14)],
    &[Const::Uint(15)],
];

/// `Core\Cli\Style`'s fully-qualified name — see [`STREAM_NAME`].
pub(crate) const STYLE_NAME: &str = r"Core\Cli\Style";

/// Slot 0 of a [`STYLE`] — its foreground [`COLOR`], or `null`.
const STYLE_COLOR: usize = 0;

/// Slot 1 of a [`STYLE`] — its background [`COLOR`], or `null`.
const STYLE_BACKGROUND: usize = 1;

/// Slot 2 of a [`STYLE`] — the five attributes, as the bits below.
const STYLE_FLAGS: usize = 2;

/// [`STYLE_FLAGS`]' bits, in [`STYLE_OPTIONS`]' own order, each paired with the
/// SGR parameter it emits.
///
/// One integer slot rather than five boolean ones because a style is read as a
/// whole every time it is read at all — [`sgr`] walks this once — and five
/// slots would be five `Value`s to release for a fact that fits in three bits
/// of one.
const ATTRIBUTES: [(i64, &str); 5] = [(1, "1"), (2, "2"), (4, "3"), (8, "4"), (16, "9")];

/// `rule:tooling/styling-is-a-value-not-a-grammar`'s `Cli\Style::of` options — R2's one trailing shape, and the
/// whole surface of what a style is.
///
/// Every option is absent by default and an absent colour is [`Const::Null`],
/// which is that variant's own case: there is no "no colour" `Color`, and
/// inventing one would make `Style::of({})` and `Style::of({color: …})`
/// different shapes of the same thing.
const STYLE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "color",
        ty: CoreTy::Instance(COLOR_NAME),
        default: Const::Null,
    },
    CoreOption {
        name: "background",
        ty: CoreTy::Instance(COLOR_NAME),
        default: Const::Null,
    },
    CoreOption {
        name: "bold",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "dim",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "italic",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "underline",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "strikethrough",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// `Core\Cli\Style`'s class card — `rule:core-api/reference-card`.
const STYLE_CARD: ClassDoc = ClassDoc {
    short: "How a piece of terminal text looks: its colours, and whether it is bold, underlined or \
            struck through. A style does not change after you create it.",
};

/// `rule:tooling/styling-is-a-value-not-a-grammar`'s `Cli\Style` — what a `Text` wears, as a value.
///
/// One member, because a style is constructed and then read: `rule:core-api/shape-rules` R5's
/// `of` for the canonical construction, R2's one trailing shape for the
/// options, R20's immutability for everything after.
pub(crate) const STYLE: CoreClass = CoreClass {
    name: STYLE_NAME,
    doc: Some(&STYLE_CARD),
    methods: &[CoreMethod {
        name: "of",
        names: &[],
        params: &[CoreTy::Options(STYLE_OPTIONS)],
        defaults: &[],
        return_ty: CoreTy::Instance(STYLE_NAME),
        symbol: "nvs_core_cli_style_of",
        doc: Some(&STYLE_OF_DOC),
    }],
    instance: &[],
    slots: &["color", "background", "flags"],
    constants: &[],
};

/// `Core\Cli\Style::of`'s reference card — `rule:core-api/reference-card`.
const STYLE_OF_DOC: MethodDoc = MethodDoc {
    short: "A style, as a value — the replacement for the `\"\\e[1;31m\"` string and the \
            `\"<bold><red>\"` markup, neither of which Novis has a grammar for.",
    params: &[
        ParamDoc {
            name: "color",
            desc: "The foreground colour. Absent leaves the terminal's own.",
            shape: &[],
        },
        ParamDoc {
            name: "background",
            desc: "The background colour. Absent leaves the terminal's own.",
            shape: &[],
        },
        ParamDoc {
            name: "bold",
            desc: "Whether the text is bold.",
            shape: &[],
        },
        ParamDoc {
            name: "dim",
            desc: "Whether the text is dimmed.",
            shape: &[],
        },
        ParamDoc {
            name: "italic",
            desc: "Whether the text is italic, which a minority of terminals render.",
            shape: &[],
        },
        ParamDoc {
            name: "underline",
            desc: "Whether the text is underlined.",
            shape: &[],
        },
        ParamDoc {
            name: "strikethrough",
            desc: "Whether the text is struck through.",
            shape: &[],
        },
    ],
    ret: "The style, which `Core\\Cli\\Text::styled` renders for the terminal this process \
          actually has.",
    errors: &[],
};

/// A colour, read out of a [`COLOR`] instance's two slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ink {
    /// [`INK_INDEXED`] or [`INK_RGB`].
    kind: i64,
    /// The palette entry, or the packed triple.
    value: u64,
}

/// The SGR parameters for one [`Ink`] at `depth`, degraded to what the terminal
/// has — `rule:tooling/the-terminal-profile-resolves-once`'s `truecolor → 256 → 16`, which is a comparison on
/// [`ColorDepth`]'s ascending ordinals rather than a table.
///
/// `background` picks the `4x`/`10x` half of the same numbering, which is the
/// `+ 10` every one of these parameters carries.
fn sgr_ink(ink: Ink, depth: ColorDepth, background: bool) -> String {
    let offset: u32 = if background { 10 } else { 0 };
    // The sixteen every terminal has, including the ones with no palette at
    // all: `30..=37` and their bright `90..=97`, or the `4x`/`10x` half.
    let basic = |entry: u8| -> String {
        if entry < 8 {
            (u32::from(entry) + 30 + offset).to_string()
        } else {
            (u32::from(entry) - 8 + 90 + offset).to_string()
        }
    };
    let (red, green, blue) = match ink.kind {
        INK_RGB => {
            let channel = |shift: u32| u8::try_from((ink.value >> shift) & 0xFF).unwrap_or(0);
            (channel(16), channel(8), channel(0))
        }
        // A palette entry is emitted as itself wherever the terminal has a
        // palette, and resolved to its own colour where it does not.
        _ => {
            let entry = u8::try_from(ink.value).unwrap_or(u8::MAX);
            if entry < 16 {
                return basic(entry);
            }
            if depth >= ColorDepth::Ansi256 {
                return format!("{};5;{entry}", 38 + offset);
            }
            rgb_of_entry(entry)
        }
    };
    match depth {
        ColorDepth::TrueColor => format!("{};2;{red};{green};{blue}", 38 + offset),
        ColorDepth::Ansi256 => format!("{};5;{}", 38 + offset, entry_of_rgb(red, green, blue)),
        _ => basic(basic_of_rgb(red, green, blue)),
    }
}

/// The 256-palette entry closest to one 24-bit colour — the 6×6×6 cube, or the
/// 24-step grey ramp for a triple whose channels agree.
fn entry_of_rgb(red: u8, green: u8, blue: u8) -> u8 {
    if red == green && green == blue {
        if red < 8 {
            return 16;
        }
        if red > 248 {
            return 231;
        }
        let step = (u16::from(red) - 8) * 24 / 247;
        return 232 + u8::try_from(step).unwrap_or(23);
    }
    let step = |channel: u8| (u16::from(channel) * 5 + 127) / 255;
    let cube = 16 + 36 * step(red) + 6 * step(green) + step(blue);
    u8::try_from(cube).unwrap_or(231)
}

/// The 24-bit colour a palette entry above the basic sixteen stands for, so
/// that a terminal with no palette can be given the nearest of its own.
fn rgb_of_entry(entry: u8) -> (u8, u8, u8) {
    if entry >= 232 {
        let level = 8 + (entry - 232) * 10;
        return (level, level, level);
    }
    let cube = entry - 16;
    ((cube / 36) * 51, ((cube / 6) % 6) * 51, (cube % 6) * 51)
}

/// The basic-sixteen entry closest to one 24-bit colour.
///
/// The bright bit is a *maximum* above two thirds rather than a per-channel
/// test, because a terminal's bright half is the same hue at a higher
/// intensity; the per-channel cut is a third, which is where the ANSI palette's
/// own primaries sit.
fn basic_of_rgb(red: u8, green: u8, blue: u8) -> u8 {
    let bright = if red.max(green).max(blue) > 170 { 8 } else { 0 };
    let mut entry = 0;
    if red > 85 {
        entry |= 1;
    }
    if green > 85 {
        entry |= 2;
    }
    if blue > 85 {
        entry |= 4;
    }
    entry | bright
}

/// The escape sequence one style is written as at `depth`, or the empty string
/// for a style that says nothing — and for **every** style at
/// [`ColorDepth::None`], which is `rule:tooling/the-terminal-profile-resolves-once`'s *"when the stream is not a
/// terminal, styling is dropped entirely"*.
///
/// A pure function of the style and the depth, so
/// `styling_is_a_value_type_and_never_a_grammar` can ask it about a
/// terminal this process does not have.
fn sgr(color: Option<Ink>, background: Option<Ink>, flags: i64, depth: ColorDepth) -> String {
    if depth == ColorDepth::None {
        return String::new();
    }
    let mut parts: Vec<String> = Vec::new();
    for (bit, parameter) in ATTRIBUTES {
        if flags & bit != 0 {
            parts.push(parameter.to_owned());
        }
    }
    if let Some(ink) = color {
        parts.push(sgr_ink(ink, depth, false));
    }
    if let Some(ink) = background {
        parts.push(sgr_ink(ink, depth, true));
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("\u{1B}[{}m", parts.join(";"))
}

/// The colour in one `Style::of` option slot, or `None` for the absent one.
///
/// # Errors
///
/// A `Fault::fatal` for a slot that is neither, which is unreachable from
/// source: the option is `CoreTy::Instance`, so `E0401` refuses anything else a
/// phase earlier, and a broken ABI is not catchable.
fn ink_of(value: Value, option: &str) -> Result<Option<Ink>, Fault> {
    match value.tag() {
        Some(Tag::Null) => Ok(None),
        Some(Tag::Object) => {
            let object = crate::instance::receiver(value, &COLOR, option)?;
            Ok(Some(Ink {
                kind: crate::instance::slot(object, COLOR_KIND)
                    .as_int()
                    .unwrap_or(INK_INDEXED),
                value: crate::instance::slot(object, COLOR_VALUE)
                    .as_uint()
                    .unwrap_or(0),
            }))
        }
        // Unreachable from source: the option's declared type is
        // `CoreTy::Instance`, so `E0401` refuses anything that is neither a
        // `Core\Cli\Color` nor the omitted default a phase earlier, and a
        // broken ABI is not something a program may catch.
        _ => Err(Fault::fatal(format!(
            "Core\\Cli\\Style::of expected a `{COLOR_NAME}` or nothing for `{option}`, got tag {}",
            value.tag_byte()
        ))),
    }
}

/// One `Style::of` boolean option, which the checker has already refused
/// anything but a `bool` for.
fn flag_of(value: Value) -> i64 {
    i64::from(value.as_bool().unwrap_or(false))
}

/// One channel of `Color::rgb`, refused above `255`.
///
/// # Errors
///
/// A `RuntimeError` — a catchable throw rather than a fatal, because a channel
/// is ordinary arithmetic a program can get wrong.
fn channel_of(value: Value, name: &str) -> Result<u64, Fault> {
    // Unreachable from source: the parameter is `CoreTy::Uint`, so `E0401`
    // refuses anything else a phase earlier — unlike the range below, which is
    // arithmetic no signature can express.
    let channel = value.as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Cli\\Color::rgb expected a `uint` for `{name}`, got tag {}",
            value.tag_byte()
        ))
    })?;
    if channel > 255 {
        return Err(Fault::thrown(format!(
            "Core\\Cli\\Color::rgb: `{name}` is {channel}, and a channel is 0 to 255"
        )));
    }
    Ok(channel)
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli\Color::index(uint $index): Cli\Color` — `rule:tooling/styling-is-a-value-not-a-grammar`'s
    /// constructor for the 256-entry palette, and the one the sixteen named
    /// constants are built by.
    fn nvs_core_cli_color_index(_ctx, args: [1]) {
        // Unreachable from source: the parameter is `CoreTy::Uint`, so `E0401`
        // refuses anything else a phase earlier, and a broken ABI is not
        // catchable — the range check below *is* reachable and throws.
        let index = args[0].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Cli\\Color::index expected a `uint`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        if index > 255 {
            return Err(Fault::thrown(format!(
                "Core\\Cli\\Color::index: {index} is not a palette entry, which is 0 to 255"
            )));
        }
        Ok(crate::instance::build(
            &COLOR,
            [Value::int(INK_INDEXED), Value::uint(index)],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli\Color::rgb(uint $red, uint $green, uint $blue): Cli\Color` —
    /// § 2's constructor for the sixteen million entries that are why this
    /// class is a value type and not an enum.
    ///
    /// The three channels are packed into one slot because they are read back
    /// together and never separately: [`sgr_ink`] unpacks them in the one place
    /// that renders them.
    fn nvs_core_cli_color_rgb(_ctx, args: [3]) {
        let red = channel_of(args[0], "red")?;
        let green = channel_of(args[1], "green")?;
        let blue = channel_of(args[2], "blue")?;
        Ok(crate::instance::build(
            &COLOR,
            [
                Value::int(INK_RGB),
                Value::uint((red << 16) | (green << 8) | blue),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli\Style::of({color?, background?, bold?, dim?, italic?,
    /// underline?, strikethrough?}): Cli\Style` — `rule:tooling/styling-is-a-value-not-a-grammar`'s style, as a
    /// value rather than as a fifth grammar (`rule:core-api/shape-rules` R11 fixes the count at
    /// four).
    ///
    /// Seven arguments for one options bag: `nvs-ir` flattens it to one per
    /// option, so an omitted `{}` arrives as this row's own defaults and the
    /// body has no "was it given" question to ask.
    ///
    /// A colour argument is **borrowed**, as every `Core` member's is, so the
    /// two that are kept are retained on the way into the slot the style owns
    /// them in.
    fn nvs_core_cli_style_of(_ctx, args: [7]) {
        // Read before anything is retained: a refusal here must leave no
        // reference behind, and `ink_of` is the only step that can fail. The
        // colours themselves are stored as they arrived rather than as the
        // `Ink` this decodes, since `styled` reads the slots back.
        let _ = ink_of(args[0], "color")?;
        let _ = ink_of(args[1], "background")?;
        let mut flags = 0;
        for (index, &(bit, _)) in ATTRIBUTES.iter().enumerate() {
            if flag_of(args[index + 2]) != 0 {
                flags |= bit;
            }
        }
        #[expect(
            unsafe_code,
            reason = "both arguments are borrowed from the caller's frame, so \
                      the copies this style keeps in its own slots need a \
                      reference each — released with the style itself"
        )]
        unsafe {
            args[0].retain();
            args[1].retain();
        }
        Ok(crate::instance::build(
            &STYLE,
            [args[0], args[1], Value::int(flags)],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli\Text::styled(string $text, Cli\Style $style): Cli\Text` — ADR
    /// 0086 § 2's second constructor, and the only thing in the language that
    /// puts a control byte in front of a program's own text.
    ///
    /// # The text is neutralized and the style is not
    ///
    /// § 2: both constructors *"apply § 1's substitution to their input"*, so
    /// this calls the same `nvs_render::text::substitute`
    /// [`nvs_core_cli_text_plain`] does and then wraps the answer. The escape
    /// bytes in the result therefore came from a `Cli\Style` — a value the
    /// program built out of typed parts — and never from a string it was
    /// handed, which is the structural guarantee that makes `Text` a
    /// constructor rather than a trust assertion.
    ///
    /// # The style is kept, and the degradation happens at the sink
    ///
    /// What this stores is a run — the substituted bytes and the `Cli\Style`
    /// they wear — so the escape sequence is chosen by whatever writes, for the
    /// stream it writes to ([`rendered_at`]). One `Text` therefore goes styled
    /// to a terminal standard output and plain to a redirected standard error
    /// in the same run, which is `rule:tooling/the-terminal-profile-resolves-once`'s
    /// *"when the stream is not a terminal, styling is dropped entirely"* read
    /// per stream. The depth stays a process answer — `Cli::colorDepth` takes
    /// no stream — and what varies is whether it reaches the stream at all
    /// ([`depth_for`]).
    ///
    /// **What it spends:** one array of two entries per `Text`, beside the
    /// standard-output rendering [`of_runs`] leaves in the carrier slot for
    /// `echo` to write.
    ///
    /// The style is not checked here: a run whose second entry is not a
    /// `Cli\Style` is refused by [`rendered_at`] before [`of_runs`] builds
    /// anything, so a `Text` that exists carries a style this module can read.
    fn nvs_core_cli_text_styled(_ctx, args: [2]) {
        // Unreachable from source, for the reason `plain`'s body gives: the
        // parameter is `CoreTy::Text`, so `E0401` refuses a non-`string`
        // argument a phase earlier and a broken ABI is not catchable.
        let text = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Cli\\Text::styled expected a `string`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let mut runs = NvsArray::new();
        runs.append(Value::str(NvsStr::new(
            nvs_render::text::substitute(text).as_bytes(),
        )));
        #[expect(
            unsafe_code,
            reason = "the run keeps a reference of its own to the style this \
                      frame only borrows"
        )]
        unsafe {
            args[1].retain();
        }
        runs.append(args[1]);
        of_runs(runs)
    }
}

// ------------------------------------------------------------- the live region

thread_local! {
    /// The live regions open on this core, innermost last.
    ///
    /// A stack rather than one region, because `rule:tooling/in-place-output-is-a-scoped-live-region`'s shape is a scoped
    /// callable and calls nest — and a stack is what makes a
    /// [`Core\Cli\Live`](LIVE) handle a plain `int`: the handle names a depth,
    /// so a handle that outlived its region names a depth that is no longer
    /// there instead of a pointer that is no longer valid.
    ///
    /// Per core and never shared: § 8 gives the terminal to the main task of a
    /// CLI program, so a second core with a region open is a program that has
    /// already broken that rule.
    static REGIONS: std::cell::RefCell<Vec<nvs_runtime::terminal::Region>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// One open region's scope, as a value whose destruction closes it.
///
/// `rule:tooling/the-terminal-is-restored-on-every-exit-path` makes restoration an obligation on **every** exit path, and
/// [`nvs_core_cli_live`] cannot discharge that with a statement after the call:
/// a throw from `$body` skips it, and an internal panic
/// (`rule:errors/panics-bypass-user-code`) skips
/// every statement there is. So the end of a region is a `Drop`, here and in
/// [`nvs_runtime::terminal::Region`] both — this one ends the *scope*, that one
/// puts the *terminal* back.
///
/// It truncates rather than pops: an inner region whose own guard was somehow
/// skipped is closed by the outer one, so the stack can never keep a region
/// nobody can reach.
struct Open {
    /// The stack depth this guard restores the stack to.
    depth: usize,
}

impl Open {
    /// Opens a region and answers the guard that closes it.
    fn region() -> Self {
        REGIONS.with(|regions| {
            let mut regions = regions.borrow_mut();
            regions.push(nvs_runtime::terminal::Region::open());
            Self {
                depth: regions.len() - 1,
            }
        })
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        REGIONS.with(|regions| regions.borrow_mut().truncate(self.depth));
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::live<T>(callable $body): T` — `rule:tooling/in-place-output-is-a-scoped-live-region`'s scoped live
    /// region, and § 8's restoration obligation.
    ///
    /// The whole member is: open a region, run `$body` with a handle to it, and
    /// answer what `$body` answered. Nothing here inspects the outcome, because
    /// there is nothing to do differently on a throw — [`Open`] has already
    /// closed the region by the time the error leaves this frame, on that path
    /// and on the two that never reach a statement at all.
    ///
    /// **A region never refuses for want of a terminal.** § 5's "renders
    /// nothing at all when the stream is not a terminal" is the rule for this
    /// member, not § 4's silence: `$body` is the program's own work, and a
    /// piped run must do it. What throws is *claiming* the terminal, and an
    /// inert region claims nothing.
    fn nvs_core_cli_live(ctx, args: [1]) {
        let open = Open::region();
        let handle = crate::instance::build(
            &LIVE,
            [Value::int(i64::try_from(open.depth).unwrap_or(i64::MAX))],
        );
        let outcome = nvs_runtime::call_callable(ctx, args[0], &[handle]);
        #[expect(
            unsafe_code,
            reason = "`call_callable` takes its own reference to each argument, \
                      so the one this frame built is still ours to drop"
        )]
        unsafe {
            handle.release();
        }
        // Explicit, though the guard would do it on the way out either way: the
        // region closes *before* the body's answer is handed back, so a caller
        // that echoes it writes below the region rather than into it.
        drop(open);
        outcome
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli\Live::set(array<Cli\Text> $lines): void` — `rule:tooling/in-place-output-is-a-scoped-live-region`'s one
    /// member on a region.
    ///
    /// # Errors
    ///
    /// A thrown `LogicError` for a handle that is not the region holding the
    /// terminal. Two cases reach it and the sentence covers both, because they
    /// are one rule: a handle that escaped its `Core\Cli::live` call, and an
    /// outer handle painted while an inner region is open. Painting the second
    /// would interleave two regions' rows on one cursor, which is the state
    /// § 5 exists to make unreachable.
    fn nvs_core_cli_live_set(_ctx, args: [2]) {
        let receiver = crate::instance::receiver(args[0], &LIVE, "set")?;
        let depth = crate::instance::slot(receiver, LIVE_DEPTH)
            .as_int()
            .and_then(|depth| usize::try_from(depth).ok());
        // Unreachable from source: parameter 0 is `array<Core\Cli\Text>` in
        // [`LIVE`] above, so anything else is `E0401` at the checker, and a
        // slot this crate wrote is an `int` because nothing else writes it.
        let (Some(depth), Some(lines)) = (depth, args[1].array_ptr()) else {
            return Err(Fault::fatal(format!(
                "{LIVE_NAME}::set expected a depth and an array, got tags {} and {}",
                args[0].tag_byte(),
                args[1].tag_byte()
            )));
        };

        let mut frame = Vec::new();
        for row in crate::str::Elements::of(lines) {
            let carrier = crate::instance::receiver(row, &TEXT, "set")?;
            let held = crate::instance::slot(carrier, nvs_runtime::CARRIER_TEXT_SLOT);
            // Unreachable from source for the same reason: the element type is
            // the carrier, whose one slot only [`built`] ever fills.
            let text = held.as_text().ok_or_else(|| {
                Fault::fatal(format!(
                    "{LIVE_NAME}::set expected a `{NAME}` holding text, got tag {}",
                    held.tag_byte()
                ))
            })?;
            frame.push(text.to_owned());
        }

        if !innermost(depth) {
            // A literal rather than a `format!` over the name constants,
            // because `conformance_coverage`'s error-path gate finds a site by
            // the stem *before* its first hole — a message that opens with one
            // has no stem, and the case that catches it could not be matched
            // back to it.
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                "Core\\Cli\\Live::set(): this handle's region has closed, or an inner one is \
                 open inside it — a live region is scoped to the `Core\\Cli::live` call that \
                 made it, and only the innermost open region paints",
            ));
        }
        paint(depth, frame);
        Ok(Value::null())
    }
}

/// Whether the region at `depth` is the one holding the terminal.
///
/// Two handles ask this and both refuse in their own words: only one region
/// owns the cursor, so painting an outer one — or one whose `Core\Cli::live`
/// call has returned — would interleave two regions' rows on one cursor.
fn innermost(depth: usize) -> bool {
    REGIONS.with(|regions| depth + 1 == regions.borrow().len())
}

/// Hands `frame` to the region at `depth`, which [`innermost`] has just said is
/// there.
fn paint(depth: usize, frame: Vec<String>) {
    REGIONS.with(|regions| {
        if let Some(region) = regions.borrow_mut().get_mut(depth) {
            region.set(frame);
        }
    });
}

/// How many cells the bar itself occupies. The percentage and the caption
/// follow it, and the region clamps the row to the terminal's width — so this
/// is the one part of the row whose size is a choice rather than an answer.
const BAR_CELLS: u64 = 24;

/// One progress bar, as the row a region is handed: `[####--------]  33% label`.
///
/// A total of `0` reads as complete, and so does a `done` past its total: a
/// loop that miscounted draws a finished bar rather than a bar past its end,
/// which is the failure that would otherwise wrap the row.
fn bar(done: u64, total: u64, label: &str) -> String {
    let percent = if total == 0 {
        100
    } else {
        // Through `u128` so that a total near `u64::MAX` divides exactly rather
        // than saturating into a percentage of its own.
        u64::try_from(u128::from(done.min(total)) * 100 / u128::from(total)).unwrap_or(100)
    };
    let filled = usize::try_from(percent * BAR_CELLS / 100).unwrap_or(0);
    let cells = usize::try_from(BAR_CELLS).unwrap_or(0);
    let mut row = String::with_capacity(cells + label.len() + 8);
    row.push('[');
    for at in 0..cells {
        row.push(if at < filled { '#' } else { '-' });
    }
    row.push_str(&format!("] {percent:>3}%"));
    if !label.is_empty() {
        row.push(' ');
        row.push_str(label);
    }
    row
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli::progress<T>(uint $total, callable $body): T` — `rule:tooling/in-place-output-is-a-scoped-live-region`'s
    /// second row, over the region [`nvs_core_cli_live`] opens.
    ///
    /// The whole difference from `live` is who writes the frame: here the
    /// runtime does, from a counter, so a program says how far it has come and
    /// never how the bar is drawn. That is why the handle has no `set` — a
    /// progress bar a program could overwrite would be `live` with a worse
    /// name.
    fn nvs_core_cli_progress(ctx, args: [2]) {
        // Unreachable from source: parameter 0 is `uint` in `CLASS` above, so
        // anything else is `E0401: expected uint, found …` at the checker.
        let total = args[0].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Cli::progress expected a `uint` total, got tag {}",
                args[0].tag_byte()
            ))
        })?;

        let open = Open::region();
        let handle = crate::instance::build(
            &PROGRESS,
            [
                Value::int(i64::try_from(open.depth).unwrap_or(i64::MAX)),
                Value::uint(total),
                Value::uint(0),
                Value::str(NvsStr::new(b"")),
            ],
        );
        // The empty bar is drawn before the body runs, so a program that takes
        // a second to reach its first `advance` is on screen for it.
        paint(open.depth, vec![bar(0, total, "")]);
        let outcome = nvs_runtime::call_callable(ctx, args[1], &[handle]);
        #[expect(
            unsafe_code,
            reason = "`call_callable` takes its own reference to each argument, \
                      so the one this frame built is still ours to drop"
        )]
        unsafe {
            handle.release();
        }
        drop(open);
        outcome
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli\Progress::advance({by?: uint, label?: string}): void` — ADR
    /// 0086 § 5's one member on a bar.
    ///
    /// # Errors
    ///
    /// A thrown `LogicError` for a handle that is not the region holding the
    /// terminal, which is [`nvs_core_cli_live_set`]'s refusal for its reason —
    /// the two are one rule and each says it in its own member's words.
    fn nvs_core_cli_progress_advance(_ctx, args: [3]) {
        let receiver = crate::instance::receiver(args[0], &PROGRESS, "advance")?;
        // Unreachable from source: every slot here is one this module wrote
        // when it built the handle, and `by` is `uint` in `ADVANCE_OPTIONS`.
        let depth = crate::instance::slot(receiver, PROGRESS_DEPTH)
            .as_int()
            .and_then(|depth| usize::try_from(depth).ok())
            .ok_or_else(|| {
                Fault::fatal("Core\\Cli\\Progress::advance found no region depth on its handle")
            })?;
        let total = crate::instance::slot(receiver, PROGRESS_TOTAL)
            .as_uint()
            .unwrap_or(0);
        let done = crate::instance::slot(receiver, PROGRESS_DONE)
            .as_uint()
            .unwrap_or(0)
            .saturating_add(args[1].as_uint().unwrap_or(1));
        crate::instance::set_slot(receiver, PROGRESS_DONE, Value::uint(done));

        // An absent `label` leaves the caption as it was, which is what makes
        // `advance()` inside a counting loop cheap to write.
        let label = match args[2].as_text() {
            Some(given) => {
                let written = nvs_render::text::substitute(given).into_owned();
                crate::instance::set_slot(
                    receiver,
                    PROGRESS_LABEL,
                    Value::str(NvsStr::new(written.as_bytes())),
                );
                written
            }
            None => crate::instance::slot(receiver, PROGRESS_LABEL)
                .as_text()
                .unwrap_or_default()
                .to_owned(),
        };

        if !innermost(depth) {
            // A literal for [`nvs_core_cli_live_set`]'s reason.
            return Err(Fault::thrown_as(
                nvs_runtime::ThrownClass::Logic,
                "Core\\Cli\\Progress::advance(): this handle's region has closed, or an inner \
                 one is open inside it — a progress bar is scoped to the `Core\\Cli::progress` \
                 call that made it, and only the innermost open region paints",
            ));
        }
        paint(depth, vec![bar(done, total, &label)]);
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `nvs_runtime::CARRIER_TEXT_SLOT` is the index this class's registered
    /// layout gives `text`, and this class's name is one `nvs_runtime` renders
    /// — the two facts that make `echo` of a captured carrier work, and
    /// neither of them checkable from the crate that acts on them.
    #[test]
    fn the_carrier_slot_matches_the_registered_layout() {
        assert_eq!(TEXT.slot("text"), nvs_runtime::CARRIER_TEXT_SLOT);
        assert_eq!(TEXT.slot("runs"), TEXT_RUNS);
        assert!(nvs_runtime::is_carrier(NAME));
    }

    /// The text of every element of an `array<string>`.
    fn words_of(array: Value) -> Vec<String> {
        crate::str::Elements::of(array.array_ptr().expect("the words are an array"))
            .map(|word| word.as_text().expect("a word is a string").to_owned())
            .collect()
    }

    /// `rule:security/capability-check-at-the-door` keeps `argv` out of this
    /// crate, so the words this member answers are the ones a launcher wrote
    /// with `Ctx::set_command_line` and nothing else — a context given none
    /// answers an empty array rather than the process's own vector. The third
    /// assertion is the one a shared table would fail: each call copies, so a
    /// program that changes what it read cannot change what the next call
    /// reads.
    // covers: Core\Cli::arguments
    #[test]
    fn arguments_answer_the_words_the_launcher_wrote() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let none =
            nvs_runtime::call(nvs_core_cli_arguments, &mut ctx, &[]).expect("arguments answered");
        assert!(
            words_of(none).is_empty(),
            "a context given no command line answered something"
        );

        ctx.set_command_line(vec![
            "greet".to_owned(),
            "ada lovelace".to_owned(),
            "--dry-run".to_owned(),
        ]);
        let given =
            nvs_runtime::call(nvs_core_cli_arguments, &mut ctx, &[]).expect("arguments answered");
        assert_eq!(
            words_of(given),
            ["greet", "ada lovelace", "--dry-run"],
            "the words are the launcher's own, in its order and one element each"
        );

        let again =
            nvs_runtime::call(nvs_core_cli_arguments, &mut ctx, &[]).expect("arguments answered");
        assert_ne!(
            given.array_ptr(),
            again.array_ptr(),
            "two calls handed out one array between them"
        );

        #[expect(unsafe_code, reason = "each call's array is this test's to release")]
        unsafe {
            none.release();
            given.release();
            again.release();
        }
    }

    /// The member answers the *process profile's* own depth, as the ordinal
    /// [`COLOR_DEPTH`] declares for that case — and answers it again unchanged,
    /// which is the half a body reading the environment per call would fail
    /// while still looking right on one line.
    // covers: Core\Cli::colorDepth
    #[test]
    fn color_depth_answers_the_profiles_own_case_every_time() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let want = depth_ordinal(nvs_runtime::terminal::profile().color_depth());
        let first = nvs_runtime::call(nvs_core_cli_color_depth, &mut ctx, &[])
            .expect("colorDepth answered");
        assert_eq!(
            first.as_int(),
            Some(want),
            "the member and the profile disagree about this process"
        );
        assert!(
            COLOR_DEPTH
                .cases
                .iter()
                .any(|(_, declared)| *declared == want),
            "the answer is not one of the four cases a program can compare against"
        );

        let again = nvs_runtime::call(nvs_core_cli_color_depth, &mut ctx, &[])
            .expect("colorDepth answered");
        assert_eq!(
            again.as_int(),
            first.as_int(),
            "a second call reported a different terminal"
        );
    }

    /// `rule:tooling/a-prompt-is-a-core-member`'s *"it never blocks waiting for
    /// an answer nobody can give"*, over the arm every prompt reaches it
    /// through. [`unanswered`] is driven rather than the member, because a
    /// process running this test may well have a terminal, and the assertion is
    /// about what a prompt does when it has none.
    // covers: Core\Cli::ask
    #[test]
    fn a_prompt_with_nobody_to_ask_answers_its_default_or_throws() {
        let fallback = Value::str(NvsStr::new(b"ada"));
        let answered = unanswered(&Answer::Ended, "ask", fallback).expect("a default is an answer");
        assert_eq!(
            answered.as_text(),
            Some("ada"),
            "the default the call named is not what came back"
        );

        match unanswered(&Answer::Ended, "ask", Value::null()) {
            Err(Fault::Thrown(class, message)) => {
                assert!(
                    matches!(class, nvs_runtime::ThrownClass::CliNotInteractive),
                    "a silence is caught by `Core\\Cli\\NotInteractive` and nothing else"
                );
                assert!(
                    message.contains("there is no controlling terminal"),
                    "the message does not say which silence this was: {message}"
                );
            }
            other => panic!("a prompt with no default and no terminal answered {other:?}"),
        }

        // The deadline's silence is the same class and a different clause, so a
        // program catches one exception and an operator reads which happened.
        match unanswered(&Answer::TimedOut, "ask", Value::null()) {
            Err(Fault::Thrown(class, message)) => {
                assert!(matches!(class, nvs_runtime::ThrownClass::CliNotInteractive));
                assert!(
                    message.contains("nothing typed within"),
                    "a deadline and an absent terminal froze as one message: {message}"
                );
            }
            other => panic!("a prompt nobody answered in time answered {other:?}"),
        }

        #[expect(
            unsafe_code,
            reason = "`handed_back` took a reference of its own, so this test \
                      owes one release for the value it built and one for the \
                      answer"
        )]
        unsafe {
            answered.release();
            fallback.release();
        }
    }

    /// The half of `Core\Cli::confirm` that is its own: a closed answer set, so
    /// a line that is neither yes nor no is asked again rather than read as
    /// `false`, and an empty line takes the `default` the `[Y/n]` promised. A
    /// member reading the first character, or one taking anything unrecognised
    /// for a no, still looks right on `y` and `n` and fails here.
    ///
    /// The scripted queue is what makes the member itself drivable — it is read
    /// ahead of any terminal — so the assertion does not depend on whether the
    /// process running this test has one. Every call's queue ends on an answer
    /// the member accepts, since a drained queue is what sends a prompt to the
    /// terminal.
    // covers: Core\Cli::confirm
    #[test]
    fn confirm_reads_only_yes_and_no_and_asks_again_for_anything_else() {
        let question = Value::str(NvsStr::new(b"delete it?"));
        let mut ctx = nvs_runtime::Ctx::buffered();

        ctx.script_answers(["maybe".to_owned(), "  YES  ".to_owned()]);
        let yes = nvs_runtime::call(nvs_core_cli_confirm, &mut ctx, &[question, Value::null()])
            .expect("confirm answered");
        assert_eq!(
            yes.as_bool(),
            Some(true),
            "`maybe` was read as an answer, or `YES` was not"
        );
        assert!(
            !ctx.has_scripted_answer(),
            "the refused line was left in the queue for the next prompt to find"
        );

        ctx.script_answers(["n".to_owned()]);
        let no = nvs_runtime::call(
            nvs_core_cli_confirm,
            &mut ctx,
            &[question, Value::bool(true)],
        )
        .expect("confirm answered");
        assert_eq!(
            no.as_bool(),
            Some(false),
            "a person who said no was given the default instead"
        );

        // The empty line is the `Enter` key, and the question promised it the
        // default — the one answer that is read off the options rather than off
        // what was typed.
        for default in [true, false] {
            ctx.script_answers([String::new()]);
            let taken = nvs_runtime::call(
                nvs_core_cli_confirm,
                &mut ctx,
                &[question, Value::bool(default)],
            )
            .expect("confirm answered");
            assert_eq!(
                taken.as_bool(),
                Some(default),
                "`Enter` did not take the `{default}` the question showed"
            );
        }

        #[expect(
            unsafe_code,
            reason = "the question is a value this test built, and every answer \
                      above is an unboxed `bool` owing nothing"
        )]
        unsafe {
            question.release();
        }
    }

    /// The half of `Core\Cli::select` that is its own: the answer is the
    /// **value** and never the number it was typed as, which is
    /// `rule:core-api/shape-rules` R4 and R5, and a number off the menu is
    /// asked again rather than clamped to a neighbour. A member answering the
    /// index, or clamping `4` to the last choice, still looks right on `2` and
    /// fails here.
    ///
    /// The scripted queue drives the member itself, as [`nvs_core_cli_confirm`]'s
    /// test does, so the assertion holds whether or not the process running it
    /// has a terminal. The empty choice list needs no scripted answer at all: it
    /// is refused before anything is read, because it is a bug in the program
    /// whether or not anybody is watching. The class that refusal arrives as is
    /// `tests/hostile/core/Cli/select/01-a-menu-nobody-can-answer.nvs`'s first
    /// step, which catches `LogicError` by name; the sentence is this test's.
    // covers: Core\Cli::select
    #[test]
    fn select_answers_the_chosen_value_and_asks_again_for_a_number_off_the_menu() {
        let question = Value::str(NvsStr::new(b"which region?"));
        let mut choices = NvsArray::new();
        for region in ["eu-west", "us-east", "ap-south"] {
            choices.append(Value::str(NvsStr::new(region.as_bytes())));
        }
        let choices = Value::array(choices);
        let fallback = Value::str(NvsStr::new(b"ap-south"));
        let empty = Value::array(NvsArray::new());
        let mut ctx = nvs_runtime::Ctx::buffered();

        // `0` is below the menu, `4` is above it and `two` is not a number, so
        // each of the three is asked again; the numbering starts at one, so `2`
        // is the second choice rather than the third.
        ctx.script_answers(["0", "4", "two", "2"].map(str::to_owned));
        let chosen = nvs_runtime::call(
            nvs_core_cli_select,
            &mut ctx,
            &[question, choices, Value::null(), Value::null()],
        )
        .expect("select answered");
        assert_eq!(
            chosen.as_text(),
            Some("us-east"),
            "the answer is the number that was typed, or a number off the menu was taken for one"
        );
        assert!(
            !ctx.has_scripted_answer(),
            "a refused line was left in the queue for the next prompt to find"
        );

        // The empty line is the `Enter` key, and the default is what it takes —
        // never the first choice on the menu.
        ctx.script_answers([String::new()]);
        let taken = nvs_runtime::call(
            nvs_core_cli_select,
            &mut ctx,
            &[question, choices, Value::null(), fallback],
        )
        .expect("select answered");
        assert_eq!(
            taken.as_text(),
            Some("ap-south"),
            "`Enter` did not take the default the call named"
        );

        nvs_runtime::call(
            nvs_core_cli_select,
            &mut ctx,
            &[question, empty, Value::null(), fallback],
        )
        .expect_err("a menu with no choices on it was answered");
        assert_eq!(
            ctx.take_pending().map(std::borrow::Cow::into_owned),
            Some("Core\\Cli::select was given nothing to choose between".to_owned()),
            "the refusal does not say what the call was missing"
        );

        #[expect(
            unsafe_code,
            reason = "every value here is one this test built, and each answer \
                      came back with a reference of its own from `handed_back`"
        )]
        unsafe {
            chosen.release();
            taken.release();
            empty.release();
            fallback.release();
            choices.release();
            question.release();
        }
    }

    /// What a `Core\Cli::multiSelect` line is allowed to be, over the parse
    /// itself: the separators are the comma, the space and the tab, so `1-3` is
    /// asked again rather than read as a range that would drop choice 2. A token
    /// off the menu refuses the whole line, an empty token is forgiven, and a
    /// choice named twice is chosen once — the three decisions [`chosen_of`]
    /// records, none of which a `.nvst` case can sweep, since every refused line
    /// there costs a queued answer to recover from.
    ///
    /// The member is driven once beside the table, because the parse alone cannot
    /// show the two things a caller sees: the answer is a set of the **values**,
    /// and its order is the menu's rather than the typing's.
    // covers: Core\Cli::multiSelect
    #[test]
    fn a_multi_select_line_is_a_set_of_values_and_a_range_is_not_a_spelling() {
        assert_eq!(chosen_of("3,1,1", 3), Some(vec![true, false, true]));
        assert_eq!(chosen_of("2 3", 3), Some(vec![false, true, true]));
        assert_eq!(chosen_of("2,\t3,", 3), Some(vec![false, true, true]));
        assert_eq!(chosen_of("", 3), Some(vec![false, false, false]));
        for refused in ["1-3", "0", "4", "two", "1;2", "1.0"] {
            assert_eq!(
                chosen_of(refused, 3),
                None,
                "`{refused}` was read as a set rather than asked again"
            );
        }

        let question = Value::str(NvsStr::new(b"which regions?"));
        let mut choices = NvsArray::new();
        for region in ["eu-west", "us-east", "ap-south"] {
            choices.append(Value::str(NvsStr::new(region.as_bytes())));
        }
        let choices = Value::array(choices);
        let mut ctx = nvs_runtime::Ctx::buffered();

        ctx.script_answers(["3,1,3".to_owned()]);
        let chosen = nvs_runtime::call(
            nvs_core_cli_multi_select,
            &mut ctx,
            &[question, choices, Value::null()],
        )
        .expect("multiSelect answered");
        assert_eq!(
            words_of(chosen),
            vec!["eu-west".to_owned(), "ap-south".to_owned()],
            "the set is not in the menu's order, or a choice named twice came back twice"
        );

        #[expect(
            unsafe_code,
            reason = "the answer is a fresh array this frame owns, and the \
                      question and the choices are values this test built"
        )]
        unsafe {
            chosen.release();
            choices.release();
            question.release();
        }
    }

    /// What `Core\Cli::secret` promises about the line, and what its row
    /// promises about a run nobody is watching. The answer is the line exactly
    /// as it was typed: nothing is trimmed, nothing is mapped onto a closed set,
    /// and an empty line is still an answer. A member reading a password the way
    /// [`nvs_core_cli_confirm`] reads a yes would look right on `hunter2` and
    /// quietly change a password that is spaces at either end. The bytes are
    /// read here rather than in a case, because a case can only see them through
    /// `Core\Secret::reveal`, which is a second member in the path.
    ///
    /// The row is asserted beside the call for the other half of the promise:
    /// the return type carries both qualifiers, and `secret` is the one prompt
    /// with no options bag, so there is nowhere for a `default` to be written.
    // covers: Core\Cli::secret
    #[test]
    fn a_secret_is_the_line_as_typed_and_its_row_leaves_no_default_to_proceed_on() {
        let row = CLASS
            .methods
            .iter()
            .find(|method| method.name == "secret")
            .expect("§ 4's fifth prompt is a row on this class");
        assert!(
            matches!(row.return_ty, CoreTy::SecretTaintedStr),
            "a password answered as anything else can be echoed, logged and serialized"
        );
        assert!(
            !row.params
                .iter()
                .any(|param| matches!(param, CoreTy::Options(_))),
            "`secret` grew an options bag, which is where a `default` would go — and a \
             password nobody typed is not a password"
        );

        let question = Value::str(NvsStr::new(b"password?"));
        let mut ctx = nvs_runtime::Ctx::buffered();
        let mut answers = Vec::new();

        for typed in ["  pass word  ", "y", "", "\u{1b}[2J", "pässwörd"] {
            ctx.script_answers([typed.to_owned()]);
            let answer = nvs_runtime::call(nvs_core_cli_secret, &mut ctx, &[question])
                .expect("secret answered");
            assert_eq!(
                answer.as_text(),
                Some(typed),
                "the line was trimmed, neutralized or otherwise changed on the way back"
            );
            answers.push(answer);
        }

        #[expect(
            unsafe_code,
            reason = "every answer is a fresh string this frame owns, and the \
                      question is a value this test built"
        )]
        unsafe {
            for answer in answers {
                answer.release();
            }
            question.release();
        }
    }

    /// `rule:tooling/the-terminal-is-restored-on-every-exit-path` over the
    /// structure that discharges it. A region's end is [`Open`]'s `Drop` rather
    /// than a statement after the call, so the paths that never reach a
    /// statement — a throw out of `$body`, an internal panic — end it too. No
    /// program can see this: a case watches a stale handle refuse to paint,
    /// which is [`innermost`]'s answer rather than the stack's shape, so the
    /// stack itself is read here.
    ///
    /// The guard truncates rather than pops, and the last assertion is what
    /// that buys: an inner region whose own guard was skipped is closed by the
    /// outer one, so the stack can never keep a region nobody can reach.
    // covers: Core\Cli::live
    #[test]
    fn a_live_region_ends_with_its_scope_and_takes_every_inner_one_with_it() {
        let depth_now = || REGIONS.with(|regions| regions.borrow().len());
        assert_eq!(
            depth_now(),
            0,
            "this thread began with a region already open"
        );

        {
            let outer = Open::region();
            assert_eq!(depth_now(), 1);
            assert!(
                innermost(outer.depth),
                "the one open region is not the one allowed to paint"
            );

            let inner = Open::region();
            assert_eq!(depth_now(), 2);
            assert!(innermost(inner.depth));
            assert!(
                !innermost(outer.depth),
                "the outer handle paints while an inner region is open, which is \
                 two regions' rows interleaved on one cursor"
            );

            // The path a panic takes through an inner call: its guard never
            // runs at all.
            std::mem::forget(inner);
            assert_eq!(depth_now(), 2, "a guard that never ran closed a region");
        }

        assert_eq!(
            depth_now(),
            0,
            "the outer region's end left an inner one on the stack that nothing can reach"
        );
    }

    /// `Core\Cli\Progress`'s counter saturates instead of starting again, and a
    /// caption nobody gave is the caption that was there. Neither is observable
    /// from a program: the count is a slot nothing reads back, and the bar it
    /// scales is on a terminal a case does not have. A `done` that wrapped
    /// would draw a finished import as one that had barely begun, which is the
    /// single worst thing a progress bar can say.
    // covers: Core\Cli::progress
    #[test]
    fn a_progress_bar_saturates_and_keeps_the_caption_nobody_replaced() {
        let open = Open::region();
        let handle = crate::instance::build(
            &PROGRESS,
            [
                Value::int(i64::try_from(open.depth).unwrap_or(i64::MAX)),
                Value::uint(4),
                Value::uint(0),
                Value::str(NvsStr::new(b"fetching")),
            ],
        );
        let receiver =
            crate::instance::receiver(handle, &PROGRESS, "advance").expect("a handle this built");
        let mut ctx = nvs_runtime::Ctx::buffered();
        let counted = |receiver| crate::instance::slot(receiver, PROGRESS_DONE).as_uint();
        let said = |receiver| {
            crate::instance::slot(receiver, PROGRESS_LABEL)
                .as_text()
                .map(str::to_owned)
        };

        for _ in 0..2 {
            nvs_runtime::call(
                nvs_core_cli_progress_advance,
                &mut ctx,
                &[handle, Value::uint(u64::MAX), Value::null()],
            )
            .expect("advance counted");
        }
        assert_eq!(
            counted(receiver),
            Some(u64::MAX),
            "the count started again from zero, so a bar past its total reads as empty"
        );
        assert_eq!(
            said(receiver),
            Some("fetching".to_owned()),
            "a step that named no caption cleared the one already beside the bar"
        );

        let given = Value::str(NvsStr::new(b"building"));
        nvs_runtime::call(
            nvs_core_cli_progress_advance,
            &mut ctx,
            &[handle, Value::uint(1), given],
        )
        .expect("advance counted");
        assert_eq!(said(receiver), Some("building".to_owned()));

        #[expect(
            unsafe_code,
            reason = "the handle and the caption are values this test built, and \
                      the call took its own reference to each argument"
        )]
        unsafe {
            given.release();
            handle.release();
        }
    }

    /// The row `Core\Cli\Progress::advance` hands the region: it scales to the
    /// total, stops at a full bar rather than drawing past its end, and carries
    /// a caption the terminal reads as text. No `.nvst` case reaches any of the
    /// three, because a case has no terminal to paint on — what a program can
    /// see of this member is the refusal, which
    /// `tests/conformance/core/a-progress-handle-is-dead-once-its-bar-has-closed.nvst`
    /// pins.
    // covers: Core\Cli\Progress::advance
    #[test]
    fn advance_paints_a_bar_that_stops_at_full_and_a_caption_nothing_can_run() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        /// A screen a test can read back.
        struct Screen(Arc<Mutex<Vec<u8>>>);

        impl Write for Screen {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0
                    .lock()
                    .expect("this screen is never poisoned")
                    .extend_from_slice(bytes);
                Ok(bytes.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let screen = Arc::new(Mutex::new(Vec::new()));
        let open = Open::region();
        REGIONS.with(|regions| {
            regions.borrow_mut()[open.depth] =
                nvs_runtime::terminal::Region::painting_on(Box::new(Screen(Arc::clone(&screen))));
        });
        let handle = crate::instance::build(
            &PROGRESS,
            [
                Value::int(i64::try_from(open.depth).unwrap_or(i64::MAX)),
                Value::uint(4),
                Value::uint(0),
                Value::str(NvsStr::new(b"")),
            ],
        );
        let mut ctx = nvs_runtime::Ctx::buffered();

        // One unit of four. This frame lands at once, and every frame after it
        // is coalesced into the last — which is why the clamp is asserted from
        // the one the region owes at its close.
        nvs_runtime::call(
            nvs_core_cli_progress_advance,
            &mut ctx,
            &[handle, Value::uint(1), Value::null()],
        )
        .expect("the innermost region painted");

        // Three units past the total, with a caption that would clear the
        // screen if it arrived as a command.
        let caption = Value::str(NvsStr::new("\u{1B}[2Jimported".as_bytes()));
        nvs_runtime::call(
            nvs_core_cli_progress_advance,
            &mut ctx,
            &[handle, Value::uint(7), caption],
        )
        .expect("the innermost region painted");
        drop(open);

        let written = String::from_utf8(screen.lock().expect("the screen").clone())
            .expect("a region writes what it was handed, and that was text");
        assert!(
            written.contains("[######------------------]  25%"),
            "one unit of four did not draw a quarter of the cells, and this run wrote {written:?}"
        );
        assert!(
            written.contains("[########################] 100%"),
            "a count past the total drew something other than a full bar, so a loop that \
             miscounted reports work it never did: {written:?}"
        );
        assert!(
            written.contains('\u{241B}') && written.contains("imported"),
            "the caption is not on screen as the text it was"
        );
        assert!(
            !written.contains("\u{1B}[2J"),
            "a caption cleared the screen, so it reached the terminal as a command"
        );

        #[expect(
            unsafe_code,
            reason = "the handle and the caption are values this test built, and \
                      the call took its own reference to each argument"
        )]
        unsafe {
            caption.release();
            handle.release();
        }
    }

    /// What `Core\Cli::displayWidth` counts, and the arm no program can reach.
    /// A column is not a character and not a byte: a Japanese character takes
    /// two, a combining mark takes none, and a tab takes as many as the next
    /// stop is away. The last assertion drives the body's own refusal, which
    /// `E0401` keeps a `.nvst` case from ever reaching, since the row's
    /// parameter is `CoreTy::Text`.
    // covers: Core\Cli::displayWidth
    #[test]
    fn display_width_counts_columns_and_names_itself_on_a_value_no_source_can_pass() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        for (text, columns) in [("order", 5_u64), ("注文", 4), ("e\u{301}", 1), ("a\tb", 9)] {
            let value = Value::str(NvsStr::new(text.as_bytes()));
            let width = nvs_runtime::call(nvs_core_cli_display_width, &mut ctx, &[value])
                .expect("displayWidth answered");
            assert_eq!(
                width.as_uint(),
                Some(columns),
                "`{text}` was measured as something other than columns"
            );
            #[expect(
                unsafe_code,
                reason = "the text is a value this test built, and the count that \
                          came back is an unboxed `uint` owing nothing"
            )]
            unsafe {
                value.release();
            }
        }

        nvs_runtime::call(nvs_core_cli_display_width, &mut ctx, &[Value::uint(7)])
            .expect_err("a number was measured as if it were text");
        let refusal = ctx
            .take_pending()
            .expect("the refusal says nothing at all")
            .into_owned();
        assert!(
            refusal.starts_with("Core\\Cli::displayWidth expected a `string`"),
            "the message does not say which member was handed what: {refusal}"
        );
    }

    /// `Core\Cli::escape` through the member, for one text.
    fn escaped(ctx: &mut nvs_runtime::Ctx, text: &str) -> String {
        let value = Value::str(NvsStr::new(text.as_bytes()));
        let answer =
            nvs_runtime::call(nvs_core_cli_escape, ctx, &[value]).expect("escape answered");
        let owned = answer
            .as_text()
            .expect("escape answers a `string`")
            .to_owned();
        #[expect(
            unsafe_code,
            reason = "the text is a value this helper built and the answer is a \
                      fresh one it owns, so both go with the copy it hands back"
        )]
        unsafe {
            value.release();
            answer.release();
        }
        owned
    }

    /// What `Core\Cli::escape` answers is inert, and stays inert when it is
    /// escaped again. The sweep is every C0 byte plus `DEL`, a C1 code point and
    /// a bidirectional control nothing closes: the two that lay text out come
    /// back untouched, and nothing else comes back as something a terminal acts
    /// on. The second escaping is the half a table replacing a control byte with
    /// another control byte would fail while still looking right on `ESC`.
    // covers: Core\Cli::escape
    #[test]
    fn escape_leaves_nothing_a_terminal_acts_on_and_repeats_without_changing_it() {
        let mut ctx = nvs_runtime::Ctx::buffered();

        for byte in 0..=0x1F_u8 {
            let text =
                String::from_utf8(vec![b'a', byte, b'b']).expect("a C0 byte is one code point");
            let once = escaped(&mut ctx, &text);
            if byte == b'\n' || byte == b'\t' {
                assert_eq!(once, text, "the two that lay text out did not pass through");
                continue;
            }
            assert!(
                !once.chars().any(char::is_control),
                "byte {byte:#04x} came back as something a terminal still acts on"
            );
            assert_eq!(
                escaped(&mut ctx, &once),
                once,
                "escaping byte {byte:#04x} twice is not the same as escaping it once"
            );
        }

        for text in ["a\u{7F}b", "a\u{85}b"] {
            let once = escaped(&mut ctx, text);
            assert!(
                !once.chars().any(char::is_control),
                "`{text}` came back as something a terminal still acts on"
            );
            assert_eq!(escaped(&mut ctx, &once), once, "`{text}` is not idempotent");
        }

        // A bidirectional control with nothing to close it reorders every line
        // after it, which is how a name is made to read as another one.
        let reordered = escaped(&mut ctx, "invoice\u{202E}gpj.exe");
        assert!(
            !reordered.contains('\u{202E}'),
            "an unterminated bidirectional control survived: {reordered}"
        );
        assert_eq!(
            escaped(&mut ctx, &reordered),
            reordered,
            "the replacement glyph was replaced again"
        );
    }

    /// `Core\Cli::write`'s two streams are two channels, asserted on both sides:
    /// the line written to standard output is absent from the diagnostic buffer
    /// and the note written to standard error is absent from the output one.
    /// No `.nvst` case can see that half, since `--EXPECT--` reads standard
    /// output alone. The `newline` option rides along byte for byte, and the
    /// last assertion drives the body's own refusal, which `E0401` keeps a case
    /// from ever reaching, since the row's parameter is a [`CoreTy::Union`] of a
    /// `string` and the carrier.
    // covers: Core\Cli::write
    #[test]
    fn write_sends_each_stream_to_its_own_channel_and_names_itself_on_a_value_no_source_can_pass() {
        // A `Core\Cli\Stream` case arrives as its ordinal, which is what
        // compiled code writes for the option's `Const::EnumCase` default.
        const OUT: i64 = 1;
        const ERR: i64 = 2;

        let mut ctx = nvs_runtime::Ctx::buffered();
        ctx.set_diagnostic_sink(nvs_runtime::OutputSink::Buffer(Vec::new()));

        let result = Value::str(NvsStr::new(b"the result"));
        let note = Value::str(NvsStr::new(b"the progress note"));
        nvs_runtime::call(
            nvs_core_cli_write,
            &mut ctx,
            &[result, Value::int(OUT), Value::bool(true)],
        )
        .expect("write wrote the result");
        nvs_runtime::call(
            nvs_core_cli_write,
            &mut ctx,
            &[note, Value::int(ERR), Value::bool(false)],
        )
        .expect("write wrote the note");

        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(&b"the result\n"[..]),
            "standard output holds something other than the one line written to it"
        );
        assert_eq!(
            ctx.take_buffered_diagnostic().as_deref(),
            Some(&b"the progress note"[..]),
            "the diagnostic channel holds something other than the note it was given"
        );

        nvs_runtime::call(
            nvs_core_cli_write,
            &mut ctx,
            &[Value::uint(7), Value::int(OUT), Value::bool(false)],
        )
        .expect_err("a number was written as if it were text");
        let refusal = ctx
            .take_pending()
            .expect("the refusal says nothing at all")
            .into_owned();
        assert!(
            refusal.starts_with("Core\\Cli::write expected a `string` or a `Core\\Cli\\Text`"),
            "the message does not say which member was handed what: {refusal}"
        );

        #[expect(
            unsafe_code,
            reason = "both texts are values this test built, and the member \
                      answers `null`, which owns nothing"
        )]
        unsafe {
            result.release();
            note.release();
        }
    }

    /// `Core\Cli::isTty` answers the profile's own answer for each of the three
    /// streams, and answers it again unchanged — the half a body asking the
    /// operating system per call would fail while still looking right on one
    /// line. The last assertion drives [`stream_of`]'s refusal, which `E0401`
    /// keeps a `.nvst` case from ever reaching, since the row's parameter is a
    /// [`CoreTy::Enum`].
    // covers: Core\Cli::isTty
    #[test]
    fn is_tty_answers_per_stream_and_names_itself_on_a_case_no_source_can_pass() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        for (ordinal, stream) in [(0_i64, Stream::In), (1, Stream::Out), (2, Stream::Err)] {
            let want = nvs_runtime::terminal::profile().is_tty(stream);
            for _ in 0..2 {
                let answer =
                    nvs_runtime::call(nvs_core_cli_is_tty, &mut ctx, &[Value::int(ordinal)])
                        .expect("isTty answered");
                assert_eq!(
                    answer.as_bool(),
                    Some(want),
                    "{stream:?} was reported as something other than the profile's own answer"
                );
            }
        }

        nvs_runtime::call(nvs_core_cli_is_tty, &mut ctx, &[Value::int(3)])
            .expect_err("a fourth stream was asked about");
        let refusal = ctx
            .take_pending()
            .expect("the refusal says nothing at all")
            .into_owned();
        assert!(
            refusal.starts_with("Core\\Cli::isTty expected a `Core\\Cli\\Stream` case"),
            "the message does not say which member was handed what: {refusal}"
        );
    }

    /// `Core\Cli::width` and `Core\Cli::height` answer the profile's own two
    /// figures, and answer them again unchanged. The two are asserted together
    /// because one resolution carries both, so a body reading the terminal per
    /// member could answer a width from one size and a height from another and
    /// still look right on either line alone. Neither is `0`, which is what
    /// lets a caller subtract a margin without checking first.
    // covers: Core\Cli::width, Core\Cli::height
    #[test]
    fn width_and_height_answer_the_profiles_own_figures_every_time() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let profile = nvs_runtime::terminal::profile();
        for _ in 0..2 {
            let width =
                nvs_runtime::call(nvs_core_cli_width, &mut ctx, &[]).expect("width answered");
            let height =
                nvs_runtime::call(nvs_core_cli_height, &mut ctx, &[]).expect("height answered");
            assert_eq!(
                width.as_uint(),
                Some(u64::from(profile.width())),
                "the columns answered are not the profile's own"
            );
            assert_eq!(
                height.as_uint(),
                Some(u64::from(profile.height())),
                "the rows answered are not the profile's own"
            );
            assert!(
                width.as_uint() != Some(0) && height.as_uint() != Some(0),
                "a margin subtracted from this would wrap around"
            );
        }
    }

    /// One red, bold `Cli\Style`, built the way a program builds one.
    fn warning(ctx: &mut nvs_runtime::Ctx) -> Value {
        let red = nvs_runtime::call(nvs_core_cli_color_index, ctx, &[Value::uint(1)])
            .expect("Color::index answered");
        let options = [
            red,
            Value::null(),
            Value::bool(true),
            Value::bool(false),
            Value::bool(false),
            Value::bool(false),
            Value::bool(false),
        ];
        let style =
            nvs_runtime::call(nvs_core_cli_style_of, ctx, &options).expect("Style::of answered");
        #[expect(unsafe_code, reason = "the style holds the colour's own reference now")]
        unsafe {
            red.release();
        }
        style
    }

    /// The runs of `text`, as the entries this module wrote.
    fn runs_of(text: Value) -> Vec<Value> {
        let held = crate::instance::slot(text.obj_ptr().expect("a `Text` is an object"), TEXT_RUNS);
        crate::str::Elements::of(held.array_ptr().expect("the runs are an array")).collect()
    }

    /// `rule:tooling/styling-is-a-value-not-a-grammar`'s `Text`: what it holds
    /// is its runs — the bodies and the styles they wear — and the bytes are
    /// the *sink's* rendering of those, so composition carries both operands'
    /// runs through rather than their appearances.
    #[test]
    fn a_text_keeps_its_runs_and_is_rendered_at_the_sink() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let style = warning(&mut ctx);

        let head = Value::str(NvsStr::new("deleting ".as_bytes()));
        let warn = nvs_runtime::call(nvs_core_cli_text_styled, &mut ctx, &[head, style])
            .expect("Text::styled answered");
        let tail = Value::str(NvsStr::new("notes.txt".as_bytes()));
        let plain = nvs_runtime::call(nvs_core_cli_text_plain, &mut ctx, &[tail])
            .expect("Text::plain answered");
        let whole = nvs_runtime::call(nvs_core_cli_text_concat, &mut ctx, &[warn, plain])
            .expect("`Text + Text` answered");

        let held = runs_of(whole);
        assert_eq!(held.len(), 4, "a sum holds both operands' runs");
        assert_eq!(
            held[0].as_text(),
            Some("deleting "),
            "a run carries its body, not an escape sequence around it"
        );
        assert_eq!(
            held[1].tag(),
            Some(Tag::Object),
            "a styled run keeps the `Cli\\Style` itself"
        );
        assert_eq!(held[2].as_text(), Some("notes.txt"));
        assert!(held[3].obj_ptr().is_none(), "an unstyled run wears nothing");

        let carrier = whole.obj_ptr().expect("a `Text` is an object");
        let sink = rendered_at(
            crate::instance::slot(carrier, TEXT_RUNS),
            depth_of(Stream::Out),
        )
        .expect("the runs rendered for standard output");
        assert_eq!(
            crate::instance::slot(carrier, nvs_runtime::CARRIER_TEXT_SLOT).as_text(),
            Some(sink.as_str()),
            "the carrier slot is the runs rendered for the stream `echo` writes"
        );
        assert!(
            sink.ends_with("notes.txt"),
            "the rendering ends in the last run's own body"
        );

        #[expect(unsafe_code, reason = "each value owns the reference it releases")]
        unsafe {
            head.release();
            tail.release();
            style.release();
            warn.release();
            plain.release();
            whole.release();
        }
    }

    /// `rule:tooling/the-terminal-profile-resolves-once`'s *"when the stream is
    /// not a terminal, styling is dropped entirely"*, per stream: one `Text`
    /// written to both in the same run is styled on the terminal and plain on
    /// the pipe.
    ///
    /// The terminal is faked rather than found, and that is forced twice over:
    /// the profile resolves once per process, and the process running this test
    /// has every stream piped — which is also why the `.nvst` case that pins
    /// the *member* cannot see the difference. So the answer is given to
    /// [`depth_for`] and [`rendered_at`], which are pure functions of it.
    #[test]
    fn one_text_is_styled_on_a_terminal_stream_and_plain_on_a_redirected_one() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let style = warning(&mut ctx);
        let body = Value::str(NvsStr::new("danger".as_bytes()));
        let warn = nvs_runtime::call(nvs_core_cli_text_styled, &mut ctx, &[body, style])
            .expect("Text::styled answered");
        let runs = crate::instance::slot(warn.obj_ptr().expect("a `Text` is an object"), TEXT_RUNS);

        // The fake answer: standard output is a terminal showing the sixteen
        // colours, and standard error is redirected.
        let terminal = depth_for(ColorDepth::Ansi16, true, true);
        let redirected = depth_for(ColorDepth::Ansi16, true, false);
        assert_eq!(terminal, ColorDepth::Ansi16);
        assert_eq!(redirected, ColorDepth::None);

        assert_eq!(
            rendered_at(runs, terminal).expect("the runs rendered for the terminal"),
            "\u{1B}[1;31mdanger\u{1B}[0m",
            "a terminal stream shows the style the run wears"
        );
        assert_eq!(
            rendered_at(runs, redirected).expect("the runs rendered for the pipe"),
            "danger",
            "a redirected stream shows the body and nothing else"
        );

        // A depth that survived a standard output which is not a terminal was
        // forced, and a forced depth is the answer for every stream.
        assert_eq!(
            depth_for(ColorDepth::Ansi16, false, false),
            ColorDepth::Ansi16
        );
        assert_eq!(
            depth_for(ColorDepth::None, true, true),
            ColorDepth::None,
            "`NO_COLOR` and a dumb terminal outrank a stream that could show colour"
        );

        #[expect(unsafe_code, reason = "each value owns the reference it releases")]
        unsafe {
            body.release();
            style.release();
            warn.release();
        }
    }

    /// `rule:tooling/in-place-output-is-a-scoped-live-region` and `rule:tooling/the-terminal-is-restored-on-every-exit-path` with `rule:errors/panics-bypass-user-code`: a live region has an end, and the
    /// terminal is put back at that end on **every** path — including the one
    /// no user code runs on.
    ///
    /// An internal panic is the hardest of the four and the only one testable
    /// from Rust: a throw and a fatal both leave through `nvs_core_cli_live`'s
    /// own `Err`, while a panic leaves through nothing at all. So the region is
    /// held across a `catch_unwind` and its screen read back afterwards — which
    /// is what [`Region::painting_on`](nvs_runtime::terminal::Region::painting_on)
    /// exists for, since the alternative is a test that needs a terminal and a
    /// person in front of it. If restoration were a statement after the call
    /// rather than a destructor, the cursor would still be hidden here.
    ///
    /// The second half is the scope, over this module's own stack: [`Open`] is
    /// what makes a region's end the `Core\Cli::live` call's end, so a panic
    /// through one must leave nothing open. A region left on that stack is a
    /// depth a stale handle would still resolve against, which is the failure
    /// `Core\Cli\Live::set`'s refusal is written for.
    // covers: Core\Cli::live
    #[test]
    fn a_live_region_is_scoped_and_restores_the_terminal_on_a_panic() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        /// A screen a test can read back.
        struct Screen(Arc<Mutex<Vec<u8>>>);

        impl Write for Screen {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0
                    .lock()
                    .expect("this screen is never held across a panic")
                    .extend_from_slice(bytes);
                Ok(bytes.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let screen = Arc::new(Mutex::new(Vec::new()));
        let painted = Arc::clone(&screen);
        let told = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let fell = std::panic::catch_unwind(move || {
            let mut region = nvs_runtime::terminal::Region::painting_on(Box::new(Screen(painted)));
            region.set(vec!["scanning one".to_owned()]);
            panic!("the body of a `Core\\Cli::live` broke an invariant");
        });
        std::panic::set_hook(told);
        assert!(fell.is_err(), "the region does not swallow the panic");

        let written = String::from_utf8(screen.lock().expect("the screen").clone())
            .expect("a region writes what it was handed, and that was text");
        assert!(
            written.starts_with("\u{1B}[?25l"),
            "the cursor is hidden while a region is open, and this run wrote {written:?}"
        );
        assert!(
            written.ends_with("\u{1B}[?25h"),
            "and is shown again on the way out — panic included — but this run wrote {written:?}"
        );
        assert!(
            written.contains("scanning one"),
            "the frame the region was given before the panic still landed"
        );

        // The scope half. Nothing else in this crate touches the stack, so an
        // empty one before and after is the whole claim.
        assert!(REGIONS.with(|regions| regions.borrow().is_empty()));
        let told = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let fell = std::panic::catch_unwind(|| {
            let open = Open::region();
            assert_eq!(
                REGIONS.with(|regions| regions.borrow().len()),
                open.depth + 1
            );
            panic!("and again, with a region open on the stack");
        });
        std::panic::set_hook(told);
        assert!(fell.is_err());
        assert!(
            REGIONS.with(|regions| regions.borrow().is_empty()),
            "a panic through `Core\\Cli::live` leaves no region open"
        );
    }

    /// `rule:tooling/a-prompt-is-a-core-member`: a prompt reads the **controlling terminal**, so
    /// `cat data.csv | myprog` can still ask a question — and it never blocks
    /// where nobody can answer.
    ///
    /// Three assertions, because no one of them holds the rule alone. The
    /// first is behavioural and is the one a session can run anywhere: a
    /// request whose output is a buffer has nobody watching it, so
    /// [`ask_terminal`] answers `None` without opening anything, and a prompt
    /// with no default turns that into `Core\Cli\NotInteractive` rather than a
    /// wait. The second and third are over the *source*, because the
    /// alternative to them is a test that needs a terminal and a person: the
    /// device is opened by name in `nvs_runtime::terminal`, and neither that
    /// module's prompt half nor this module's names `Stream::In` or stdin at
    /// all. A `fgets(STDIN)`-shaped implementation would pass every
    /// behavioural test that could be written here and fail exactly this.
    #[test]
    fn a_prompt_reads_the_controlling_terminal_and_not_stdin() {
        let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Buffer(Vec::new()));
        assert!(!watched(&ctx));
        // A context with nothing scripted onto it, which is every context
        // outside a test: § 4's last paragraph adds an answer ahead of this
        // one, never a second way to reach the terminal.
        assert!(matches!(
            ask_terminal(&mut ctx, "Name? ", Echo::Shown),
            Answer::Ended
        ));
        assert!(matches!(
            not_interactive("ask"),
            Fault::Thrown(nvs_runtime::ThrownClass::CliNotInteractive, _)
        ));

        let terminal = include_str!("../../nvs-runtime/src/terminal.rs");
        let (_, prompts) = terminal
            .split_once(
                "// ------------------------------------------------------------------ the prompts",
            )
            .expect("nvs_runtime::terminal's prompt half is where the device is opened");
        assert!(prompts.contains("\"/dev/tty\""), "the Unix device, by name");
        assert!(prompts.contains("\"CONIN$\""), "the Windows console input");
        assert!(prompts.contains("\"CONOUT$\""), "and the screen beside it");
        assert!(
            !prompts.contains("std::io::stdin"),
            "the prompt half of nvs_runtime::terminal reads the program's standard input rather \
             than the terminal `rule:tooling/a-prompt-is-a-core-member` names"
        );
        // `Stream::In` *is* named there, and legitimately: `is_interactive`
        // asks whether standard input is a terminal, which is a question about
        // how the process was started and not a read of it. What the line
        // above forbids is the read.

        let here = include_str!("cli.rs");
        let (_, prompts) = here
            .split_once(
                "// ------------------------------------------------------------------ the prompts",
            )
            .expect("this module's prompt section");
        let (prompts, _) = prompts
            .split_once(
                "// ------------------------------------------------------------------ the carrier",
            )
            .expect("which ends where the carrier begins");
        for named in ["Stream::In", "std::io::stdin"] {
            assert!(
                !prompts.contains(named),
                "a prompt body in this module names `{named}`"
            );
        }
    }

    /// `rule:tooling/a-prompt-is-a-core-member`: a prompt **never blocks**, which is two rules — the
    /// terminal that is not there, and the terminal nobody is sitting at.
    ///
    /// The first is the other test's; this one is the second, and it is
    /// behavioural where it can be. `nvs_runtime::terminal::answer_within` is
    /// the whole of the bound, and a channel nobody sends on is an unattended
    /// terminal with no terminal and no person required — so the wait can be
    /// driven here at twenty milliseconds and asserted to end. The three
    /// silences are then distinguished, because the surface says a different
    /// sentence for each and a member that answered `TimedOut` where the device
    /// merely closed would name a deadline that never elapsed.
    ///
    /// The last two assertions are the ones a behavioural test cannot make: the
    /// deadline the real [`ask_terminal`] uses is finite and human-scaled, and
    /// no prompt takes a parameter that lengthens it. A `timeout` option would
    /// be a spelling for "wait longer", which `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s rule — the one § 4
    /// applies to this surface — exists to deny.
    #[test]
    fn no_prompt_blocks_without_a_deadline() {
        use nvs_runtime::terminal::{ANSWER_DEADLINE, answer_within};
        use std::time::{Duration, Instant};

        let (nobody, from_terminal) = std::sync::mpsc::channel::<Answer>();
        let started = Instant::now();
        assert!(matches!(
            answer_within(&from_terminal, Duration::from_millis(20)),
            Answer::TimedOut
        ));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the wait ended on its own clock rather than on the sender"
        );
        nobody
            .send(Answer::Line("ada".to_owned()))
            .expect("the receiver is still here");
        assert!(matches!(
            answer_within(&from_terminal, Duration::from_millis(20)),
            Answer::Line(line) if line == "ada"
        ));
        drop(nobody);
        assert!(matches!(
            answer_within(&from_terminal, Duration::from_millis(20)),
            Answer::Ended
        ));

        // Both silences are the same class — a program catching
        // `Core\Cli\NotInteractive` is asking whether the question could be
        // answered — and neither sentence is the other's.
        let said = |silence: Fault| match silence {
            Fault::Thrown(nvs_runtime::ThrownClass::CliNotInteractive, said) => said,
            other => panic!("a prompt's silence is `Core\\Cli\\NotInteractive`, not {other:?}"),
        };
        let (missing, late) = (said(not_interactive("ask")), said(timed_out("ask")));
        assert_ne!(missing, late);
        assert!(
            late.contains(&ANSWER_DEADLINE.as_secs().to_string()),
            "the sentence for a terminal nobody answered names the deadline it waited"
        );

        assert!(
            ANSWER_DEADLINE > Duration::ZERO && ANSWER_DEADLINE <= Duration::from_secs(600),
            "a prompt's deadline is finite and scaled to a person answering a question"
        );
        for prompt in ["ask", "confirm", "select", "multiSelect", "secret"] {
            let row = CLASS
                .methods
                .iter()
                .find(|method| method.name == prompt)
                .expect("§ 4's prompts are rows on this class");
            for named in row.names {
                assert!(
                    !["timeout", "deadline", "wait", "within"].contains(named),
                    "`Core\\Cli::{prompt}` takes a `{named}`, which is a spelling for waiting \
                     longer than `rule:tooling/a-prompt-is-a-core-member`'s deadline"
                );
            }
        }
    }

    /// `rule:tooling/a-prompt-is-a-core-member`: `multiSelect`'s answer is a **set**, which is the whole of
    /// what it adds to `select` — and the only part of it a conformance case
    /// cannot reach, since the parse runs after a line has been read from a
    /// terminal a piped case does not have.
    ///
    /// Four claims, and each is a decision the shipped body would still look
    /// right without. A choice named twice is chosen once and the order is the
    /// **menu's** rather than the typing's, because an answer paired against
    /// `$choices` at the call site is why § 4 answers values and not indices.
    /// An empty line is the empty set rather than a silence, which is why this
    /// member needs no `default` where the other four take one. And a token
    /// that is not a number on the menu is a refusal rather than a guess —
    /// `1-3` in particular, where reading a range would silently drop the
    /// option between its ends, and `0`, where the menu starts at one.
    #[test]
    fn multi_select_reads_a_set_and_refuses_what_it_cannot_read() {
        let chosen = |answer: &str| {
            chosen_of(answer, 3).map(|flags| {
                flags
                    .iter()
                    .enumerate()
                    .filter(|(_, taken)| **taken)
                    .map(|(at, _)| at + 1)
                    .collect::<Vec<_>>()
            })
        };

        // A set: named twice is once, and the order comes back the menu's.
        assert_eq!(chosen("3,1,3"), Some(vec![1, 3]));
        // Commas, spaces or both, since a person typing a list uses all three.
        for spelling in ["1,2", "1 2", "1, 2", " 1 ,2 ", "1,2,"] {
            assert_eq!(chosen(spelling), Some(vec![1, 2]), "`{spelling}`");
        }
        // The empty set is an answer, and it is the one no `default` is needed
        // for — § 4's table gives this member none.
        assert_eq!(chosen(""), Some(Vec::new()));
        assert!(
            !MULTI_SELECT_OPTIONS
                .iter()
                .any(|option| option.name == "default"),
            "an empty line already names the empty set"
        );

        // Off the menu at either end, and anything that is not a number at all,
        // is asked again rather than clamped or read as something near it.
        for refused in ["0", "4", "1-3", "all", "1,x", "-1", "1.0"] {
            assert_eq!(
                chosen(refused),
                None,
                "`{refused}` is not a choice on offer"
            );
        }
    }

    /// `nvs_runtime::value_to_string` renders slot 0 and nothing else, so what
    /// the layout owes it is a rendered `string` there — and the runs, which
    /// only a member that knows its stream can render, are the slot after it.
    #[test]
    fn the_carrier_renders_from_its_first_slot() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let body = Value::str(NvsStr::new("plain".as_bytes()));
        let text = nvs_runtime::call(nvs_core_cli_text_plain, &mut ctx, &[body])
            .expect("Text::plain answered");
        let carrier = text.obj_ptr().expect("a `Text` is an object");
        assert_eq!(
            crate::instance::slot(carrier, nvs_runtime::CARRIER_TEXT_SLOT).tag(),
            Some(Tag::Str),
            "the slot `value_to_string` renders holds a `string`"
        );
        assert_eq!(
            crate::instance::slot(carrier, TEXT_RUNS).tag(),
            Some(Tag::Array),
            "the runs sit beside that slot, where nothing in `nvs-runtime` reads them"
        );

        #[expect(unsafe_code, reason = "each value owns the reference it releases")]
        unsafe {
            body.release();
            text.release();
        }
    }

    /// `Core\Cli\Text::plain` substitutes its argument once and writes that one
    /// answer into both of a carrier's slots, as a run nothing styles.
    ///
    /// The second half is the one no `.nvst` case reaches. A case's output is
    /// captured, so it runs at [`ColorDepth::None`], where a run carrying a
    /// style and a run carrying none render alike — and a `plain` that had
    /// quietly given its run a style would pass every case in the suite. Asked
    /// for the terminal with every colour there is, the run still comes back
    /// bare, and the substituted escape is still substituted.
    // covers: Core\Cli\Text::plain
    #[test]
    fn plain_substitutes_into_both_slots_and_makes_a_run_no_terminal_colours() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let body = Value::str(NvsStr::new("cost \u{1B}[31m9\u{7}".as_bytes()));
        let text = nvs_runtime::call(nvs_core_cli_text_plain, &mut ctx, &[body])
            .expect("Text::plain answered");
        let carrier = text.obj_ptr().expect("a `Text` is an object");
        let want = "cost \u{241B}[31m9\u{2407}";

        assert_eq!(
            crate::instance::slot(carrier, nvs_runtime::CARRIER_TEXT_SLOT).as_text(),
            Some(want),
            "the rendering standard output takes is not the substituted text"
        );
        let runs = crate::instance::slot(carrier, TEXT_RUNS);
        for depth in [
            ColorDepth::None,
            ColorDepth::Ansi16,
            ColorDepth::Ansi256,
            ColorDepth::TrueColor,
        ] {
            assert_eq!(
                rendered_at(runs, depth).expect("a `Text` renders at every depth"),
                want,
                "a terminal at {depth:?} was sent something other than the text itself"
            );
        }

        #[expect(unsafe_code, reason = "each value owns the reference it releases")]
        unsafe {
            body.release();
            text.release();
        }
    }

    /// `Core\Cli\Text::styled` renders as the escape sequence its `Cli\Style`
    /// names, degraded to what the terminal at the other end can show, and
    /// never as anything its own body carried.
    ///
    /// This is the half no `.nvst` case reaches. A case's output is captured,
    /// so it runs at [`ColorDepth::None`], where every style renders as
    /// nothing — and a `styled` that had dropped its style on the floor would
    /// pass the whole suite. Asked for each terminal in turn, the bold red
    /// comes back as its two SGR parameters and one reset, and a 24-bit blue
    /// comes back as its three channels, as the nearest entry of the
    /// 256-colour palette, and as the nearest of the sixteen below that. The
    /// body's own escape stays substituted at every depth, so the two the
    /// style put there are the only ones a terminal is sent.
    // covers: Core\Cli\Text::styled
    #[test]
    fn styled_renders_its_style_at_every_depth_and_never_its_bodys_own_escape() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let style = warning(&mut ctx);
        let body = Value::str(NvsStr::new("cost \u{1B}[31m9".as_bytes()));
        let text = nvs_runtime::call(nvs_core_cli_text_styled, &mut ctx, &[body, style])
            .expect("Text::styled answered");
        let runs = crate::instance::slot(text.obj_ptr().expect("a `Text` is an object"), TEXT_RUNS);
        let want = "cost \u{241B}[31m9";

        assert_eq!(
            rendered_at(runs, ColorDepth::None).expect("a `Text` renders at every depth"),
            want,
            "a terminal with no colour was sent more than the text itself"
        );
        for depth in [
            ColorDepth::Ansi16,
            ColorDepth::Ansi256,
            ColorDepth::TrueColor,
        ] {
            let rendered = rendered_at(runs, depth).expect("a `Text` renders at every depth");
            assert_eq!(
                rendered,
                format!("\u{1B}[1;31m{want}\u{1B}[0m"),
                "a terminal at {depth:?} was not sent the bold red the style names"
            );
            assert_eq!(
                rendered.matches('\u{1B}').count(),
                2,
                "a terminal at {depth:?} was sent an escape the style did not put there"
            );
        }

        // A colour only a 24-bit terminal has, which is where the three depths
        // answer differently.
        let dodger = nvs_runtime::call(
            nvs_core_cli_color_rgb,
            &mut ctx,
            &[Value::uint(30), Value::uint(144), Value::uint(255)],
        )
        .expect("Color::rgb answered");
        let options = [
            dodger,
            Value::null(),
            Value::bool(false),
            Value::bool(false),
            Value::bool(false),
            Value::bool(false),
            Value::bool(false),
        ];
        let blue = nvs_runtime::call(nvs_core_cli_style_of, &mut ctx, &options)
            .expect("Style::of answered");
        let label = Value::str(NvsStr::new("done".as_bytes()));
        let shown = nvs_runtime::call(nvs_core_cli_text_styled, &mut ctx, &[label, blue])
            .expect("Text::styled answered");
        let blue_runs =
            crate::instance::slot(shown.obj_ptr().expect("a `Text` is an object"), TEXT_RUNS);

        for (depth, opening) in [
            (ColorDepth::TrueColor, "\u{1B}[38;2;30;144;255m"),
            (ColorDepth::Ansi256, "\u{1B}[38;5;75m"),
            (ColorDepth::Ansi16, "\u{1B}[96m"),
        ] {
            assert_eq!(
                rendered_at(blue_runs, depth).expect("a `Text` renders at every depth"),
                format!("{opening}done\u{1B}[0m"),
                "a terminal at {depth:?} was not sent the nearest colour it has"
            );
        }

        #[expect(unsafe_code, reason = "each value owns the reference it releases")]
        unsafe {
            body.release();
            style.release();
            text.release();
            dodger.release();
            blue.release();
            label.release();
            shown.release();
        }
    }

    /// `Core\Cli\Text::text` answers the runs' own bodies, and not the
    /// rendering the carrier slot holds for the stream `echo` writes.
    ///
    /// No `.nvst` case can tell those two apart. Every stream a case runs
    /// under is captured, so the process renders at [`ColorDepth::None`],
    /// where both slots hold the same bytes and a member answering the carrier
    /// would pass `cli-a-texts-text-is-its-runs-without-their-styling.nvst`
    /// line for line. The carrier here is given the rendering a colour
    /// terminal would be sent, which is the one thing a program cannot
    /// arrange, and the answer still comes back with no escape in it.
    // covers: Core\Cli\Text::text
    #[test]
    fn text_answers_the_runs_and_not_the_carriers_rendering_for_a_terminal() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let style = warning(&mut ctx);
        let body = Value::str(NvsStr::new("careful".as_bytes()));
        let styled = nvs_runtime::call(nvs_core_cli_text_styled, &mut ctx, &[body, style])
            .expect("Text::styled answered");
        let runs =
            crate::instance::slot(styled.obj_ptr().expect("a `Text` is an object"), TEXT_RUNS);
        let lit = rendered_at(runs, ColorDepth::Ansi16).expect("the runs render for a terminal");
        assert!(
            lit.contains('\u{1B}'),
            "the fixture is not a terminal's rendering at all"
        );

        // The carrier a process writing to a terminal builds: the same runs,
        // and the slot `echo` writes holding their escape sequences.
        #[expect(
            unsafe_code,
            reason = "the second carrier takes a reference of its own to the runs"
        )]
        unsafe {
            runs.retain();
        }
        let on_a_terminal =
            crate::instance::build(&TEXT, [Value::str(NvsStr::new(lit.as_bytes())), runs]);
        let read = nvs_runtime::call(nvs_core_cli_text_text, &mut ctx, &[on_a_terminal])
            .expect("Text::text answered");
        assert_eq!(
            read.as_text(),
            Some("careful"),
            "the answer is the carrier's rendering rather than what the text says"
        );

        #[expect(unsafe_code, reason = "each value owns the reference it releases")]
        unsafe {
            body.release();
            style.release();
            styled.release();
            on_a_terminal.release();
            read.release();
        }
    }

    /// `rule:tooling/terminal-output-is-a-sink`: terminal output substitutes a control sequence
    /// **visibly**, and it does so at the sink rather than at any caller's
    /// discretion.
    ///
    /// Asserted over the *pair*, because either half alone passes while the
    /// rule is broken. `echo` of a real CSI sequence is what an attacker
    /// reaches — the payload here is the shape of a title-report injection,
    /// `ESC ] 0 ; … BEL` — and every one of its command bytes has to come out
    /// as a glyph, with the human-readable text between them untouched. Then
    /// `Core\Cli::escape` is asked the same question and has to give the *same*
    /// answer, which is the property that fails silently when a launderer and
    /// its sink grow apart (`rule:security/launderers-are-sink-named`). The third assertion is the one a
    /// substitution written as a deletion would pass: nothing is dropped, so
    /// the neutralized form is *longer* than what arrived and the operator sees
    /// that something was there.
    #[test]
    fn terminal_output_substitutes_a_control_sequence_visibly() {
        let attack = "\u{1B}]0;rm -rf /\u{7}ok\r";
        let visible = "\u{241B}]0;rm -rf /\u{2407}ok\u{240D}";

        let mut ctx = nvs_runtime::Ctx::buffered();
        let written = Value::str(NvsStr::new(attack.as_bytes()));
        nvs_runtime::call(nvs_runtime::helpers::nvs_echo_str, &mut ctx, &[written])
            .expect("echo succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(visible.as_bytes()),
            "the terminal sink let a control sequence through"
        );

        let argument = Value::str(NvsStr::new(attack.as_bytes()));
        let laundered = nvs_runtime::call(nvs_core_cli_escape, &mut ctx, &[argument])
            .expect("escape succeeded");
        assert_eq!(
            laundered.as_text(),
            Some(visible),
            "Core\\Cli::escape and the sink it launders for disagree"
        );

        assert!(
            visible.chars().count() >= attack.chars().count(),
            "a substitution dropped a byte instead of showing it"
        );

        #[expect(unsafe_code, reason = "each value owns the reference it releases")]
        unsafe {
            written.release();
            argument.release();
            laundered.release();
        }
    }

    /// `rule:tooling/terminal-output-is-a-sink`'s *"there is exactly one raw path, `Cli\Text`"* — asserted
    /// as the **disagreement** it has to be, over one payload.
    ///
    /// The same control sequence is written three ways and the sink has to
    /// answer differently for the middle one: as a `string` it is substituted,
    /// inside a carrier it is written through byte for byte, and back through
    /// `Text::plain` it is substituted again. Either of the first two alone
    /// passes while the rule is broken — a sink that substituted everything
    /// would pass the first, and one that substituted nothing would pass the
    /// second — so what is pinned here is that the two differ *and* that the
    /// only constructor a program can reach lands on the substituted side.
    /// That third assertion is the one that fails if `plain` ever becomes the
    /// transfer [`built`] is.
    ///
    /// The carrier is built through `built` directly, which is
    /// `Core\Out::capture`'s route and the only way to get raw bytes into a
    /// `Text` until `styled` exists.
    #[test]
    fn the_sink_writes_a_carrier_raw_and_everything_else_substituted() {
        let raw = "\u{1B}[31mred";
        let visible = "\u{241B}[31mred";
        let mut ctx = nvs_runtime::Ctx::buffered();

        let loose = Value::str(NvsStr::new(raw.as_bytes()));
        nvs_runtime::call(nvs_runtime::helpers::nvs_echo_value, &mut ctx, &[loose])
            .expect("echo succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(visible.as_bytes()),
            "a loose `string` reached the terminal unsubstituted"
        );

        let carried = built(Value::str(NvsStr::new(raw.as_bytes())));
        nvs_runtime::call(nvs_runtime::helpers::nvs_echo_value, &mut ctx, &[carried])
            .expect("echo succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(raw.as_bytes()),
            "the sink substituted over its own carrier"
        );

        let argument = Value::str(NvsStr::new(raw.as_bytes()));
        let made = nvs_runtime::call(nvs_core_cli_text_plain, &mut ctx, &[argument])
            .expect("plain succeeded");
        nvs_runtime::call(nvs_runtime::helpers::nvs_echo_value, &mut ctx, &[made])
            .expect("echo succeeded");
        assert_eq!(
            ctx.take_buffered_output().as_deref(),
            Some(visible.as_bytes()),
            "`Text::plain` carried its argument's control bytes into the raw path"
        );

        assert!(
            nvs_runtime::helpers::is_carrier_value(carried)
                && !nvs_runtime::helpers::is_carrier_value(loose),
            "the raw path is keyed on something other than the carrier class"
        );

        #[expect(unsafe_code, reason = "each value owns the reference it releases")]
        unsafe {
            loose.release();
            carried.release();
            argument.release();
            made.release();
        }
    }

    /// `rule:tooling/the-terminal-profile-resolves-once`: the profile is resolved **once per process**, so every
    /// fact in it answers identically however many times it is asked.
    ///
    /// Asserted three ways, because the interesting failure is not "two reads
    /// differ" — a second read of a terminal nobody resized would agree by
    /// luck. It is that a second *resolution* could happen at all. So: the two
    /// reads are the same `&'static` profile, which is what proves the
    /// `OnceLock` filled once; every one of § 3's four facts agrees across
    /// them; and a freshly computed profile is **discarded** rather than
    /// installed, which is the property a caching bug breaks while leaving the
    /// first two intact.
    #[test]
    fn tty_colour_depth_and_width_resolve_once_per_process() {
        let first = nvs_runtime::terminal::profile();
        let second = nvs_runtime::terminal::profile();
        assert!(
            std::ptr::eq(first, second),
            "two reads of the profile resolved it twice"
        );
        for stream in [Stream::In, Stream::Out, Stream::Err] {
            assert_eq!(first.is_tty(stream), second.is_tty(stream));
        }
        assert_eq!(first.width(), second.width());
        assert_eq!(first.height(), second.height());
        assert_eq!(first.color_depth(), second.color_depth());

        // Resolving again answers a whole profile, and the members still read
        // the first one — which is the half a per-call implementation passes
        // the assertions above while failing.
        let _ = nvs_runtime::terminal::resolve();
        assert!(std::ptr::eq(nvs_runtime::terminal::profile(), first));
    }

    /// `cargo test` captures a child's output, so no standard stream here is a
    /// terminal — which is the case `rule:tooling/the-terminal-profile-resolves-once` names the fallback for, and
    /// the one that makes `myprog | grep` and a CI log plain. Both halves are
    /// asserted together: the pair `80`/`24`, and the `None` depth that has to
    /// accompany it.
    #[test]
    fn a_process_with_no_terminal_falls_back_and_shows_no_colour() {
        let profile = nvs_runtime::terminal::profile();
        if [Stream::In, Stream::Out, Stream::Err]
            .into_iter()
            .any(|stream| profile.is_tty(stream))
        {
            // Run with a terminal attached — `--nocapture` from a shell. The
            // fallback is not the thing under test then.
            return;
        }
        assert_eq!(profile.width(), nvs_runtime::terminal::FALLBACK_WIDTH);
        assert_eq!(profile.height(), nvs_runtime::terminal::FALLBACK_HEIGHT);
        assert_eq!(
            profile.color_depth(),
            ColorDepth::None,
            "colour was reported for a stream that is not a terminal"
        );
    }

    /// The registry's two enums and the runtime's are separate declarations in
    /// separate crates, and this is the one place they are held together: the
    /// ordinals a compiled `Core\Cli\Stream::Out` writes are what [`stream_of`]
    /// decodes, and the ones [`depth_ordinal`] answers are what a
    /// `== Core\Cli\ColorDepth::None` compares against.
    // covers: Core\Cli\ColorDepth
    #[test]
    fn the_two_enums_agree_with_the_runtimes_own() {
        for (ordinal, (name, declared)) in STREAM.cases.iter().enumerate() {
            let want = i64::try_from(ordinal).expect("three cases");
            assert_eq!(*declared, want, "`{name}` is not at its own index");
            assert_eq!(
                stream_of(&Value::int(want), "isTty").expect("a declared case"),
                match ordinal {
                    0 => Stream::In,
                    1 => Stream::Out,
                    _ => Stream::Err,
                }
            );
        }
        assert!(
            stream_of(&Value::int(3), "isTty").is_err(),
            "an ordinal past the roster has to be refused, not folded into a case"
        );

        let depths = [
            ColorDepth::None,
            ColorDepth::Ansi16,
            ColorDepth::Ansi256,
            ColorDepth::TrueColor,
        ];
        assert_eq!(COLOR_DEPTH.cases.len(), depths.len());
        for (depth, (name, declared)) in depths.into_iter().zip(COLOR_DEPTH.cases) {
            assert_eq!(
                depth_ordinal(depth),
                *declared,
                "`{name}` and `{depth:?}` disagree on their ordinal"
            );
        }
        assert!(
            COLOR_DEPTH
                .cases
                .windows(2)
                .all(|pair| pair[0].1 < pair[1].1),
            "the depths must ascend for the sink to degrade by comparison"
        );
    }

    /// `rule:tooling/commands-are-compiled` names four shells and nothing else, and a case is only
    /// reachable from source once [`crate::registry::ENUMS`] carries the enum.
    ///
    /// Both halves, because each fails on its own: a fifth case added ahead of
    /// the generator that would write its script fails the roster, and a
    /// declaration nobody registered compiles, documents itself, and then
    /// resolves nowhere — `Core\Cli\Shell::Bash` would be a name error with a
    /// card on disk describing it.
    // covers: Core\Cli\Shell
    #[test]
    fn the_shell_roster_is_the_four_adr_0086_names_and_is_registered() {
        let names: Vec<&str> = SHELL.cases.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["Bash", "Zsh", "Fish", "Pwsh"]);
        for (ordinal, (name, declared)) in SHELL.cases.iter().enumerate() {
            assert_eq!(
                *declared,
                i64::try_from(ordinal).expect("four cases"),
                "`{name}` is not at its own index"
            );
        }
        assert!(
            crate::registry::ENUMS
                .iter()
                .any(|declared| declared.name == SHELL_NAME),
            "`{SHELL_NAME}` is declared but not registered, so no source can name a case"
        );
    }

    /// `rule:tooling/styling-is-a-value-not-a-grammar`: **styling is a value type, never a grammar** — asserted as
    /// the three things that sentence means, because each half passes on its
    /// own while the rule is broken.
    ///
    /// First, the sixteen named colours are class constants *over the
    /// constructor*: every row is `Color::index(n)` at its own palette ordinal,
    /// so `Color::RED` is one allocation at the use site like any other object
    /// and there is no shared instance for a second isolate to reach.
    ///
    /// Second, neither rejected design does anything. The payload carries both
    /// of them — a `<red>` markup tag and a raw `ESC [ 31 m` — and comes back
    /// with the tag as literal text and the escape substituted, so the only
    /// control bytes in a `Text` are the ones the `Style` put there. That is
    /// § 2's *"a constructor that cannot produce an injected sequence"*, and it
    /// is asserted against the style this process's own terminal renders, so
    /// the case says the same thing on a developer's console and in CI.
    ///
    /// Third, the style is rendered for the terminal there is, over the whole
    /// `truecolor → 256 → 16 → none` ladder rather than at one depth — the
    /// assertion a renderer ignoring [`ColorDepth`] would pass at every single
    /// line — and `None` drops the attributes too, not only the colour.
    #[test]
    fn styling_is_a_value_type_and_never_a_grammar() {
        assert_eq!(
            COLOR.constants.len(),
            16,
            "`rule:tooling/styling-is-a-value-not-a-grammar` names sixteen colours as class constants"
        );
        for (ordinal, constant) in COLOR.constants.iter().enumerate() {
            let entry = u64::try_from(ordinal).expect("sixteen constants");
            assert!(
                matches!(constant.ty, CoreTy::Instance(class) if class == COLOR_NAME),
                "`{}` is a constant that is not an instance",
                constant.name
            );
            assert!(
                matches!(
                    constant.value,
                    Const::Built { symbol, args }
                        if symbol == INDEX_SYMBOL
                            && matches!(args, [Const::Uint(written)] if *written == entry)
                ),
                "`{}` is not `Color::index({ordinal})`, so a named colour and a \
                 constructed one are two different values",
                constant.name
            );
        }

        let mut ctx = nvs_runtime::Ctx::buffered();
        let red = nvs_runtime::call(nvs_core_cli_color_index, &mut ctx, &[Value::uint(1)])
            .expect("Color::index succeeded");
        let bold = [
            red,
            Value::null(),
            Value::bool(true),
            Value::bool(false),
            Value::bool(false),
            Value::bool(false),
            Value::bool(false),
        ];
        let style =
            nvs_runtime::call(nvs_core_cli_style_of, &mut ctx, &bold).expect("Style::of succeeded");

        let payload = Value::str(NvsStr::new("<red>\u{1B}[31m</red>".as_bytes()));
        let text = nvs_runtime::call(nvs_core_cli_text_styled, &mut ctx, &[payload, style])
            .expect("Text::styled succeeded");
        let carried = crate::instance::slot(
            text.obj_ptr().expect("a `Text` is an object"),
            nvs_runtime::CARRIER_TEXT_SLOT,
        );

        let opening = sgr(
            Some(Ink {
                kind: INK_INDEXED,
                value: 1,
            }),
            None,
            1,
            nvs_runtime::terminal::profile().color_depth(),
        );
        let reset = if opening.is_empty() { "" } else { "\u{1B}[0m" };
        let want = format!("{opening}<red>\u{241B}[31m</red>{reset}");
        assert_eq!(
            carried.as_text(),
            Some(want.as_str()),
            "a `Text` carried an escape sequence its `Style` did not put there"
        );

        let scarlet = Ink {
            kind: INK_RGB,
            value: 0x00FF_0000,
        };
        assert_eq!(
            sgr(Some(scarlet), None, 0, ColorDepth::TrueColor),
            "\u{1B}[38;2;255;0;0m"
        );
        assert_eq!(
            sgr(Some(scarlet), None, 0, ColorDepth::Ansi256),
            "\u{1B}[38;5;196m"
        );
        assert_eq!(
            sgr(Some(scarlet), None, 0, ColorDepth::Ansi16),
            "\u{1B}[91m"
        );
        assert_eq!(sgr(Some(scarlet), None, 0, ColorDepth::None), "");
        assert_eq!(sgr(None, None, 31, ColorDepth::Ansi16), "\u{1B}[1;2;3;4;9m");
        assert_eq!(
            sgr(None, None, 31, ColorDepth::None),
            "",
            "a terminal with no colour was still sent the attributes"
        );

        #[expect(unsafe_code, reason = "each value owns the reference it releases")]
        unsafe {
            payload.release();
            text.release();
            red.release();
            style.release();
        }
    }

    /// Which slot each of `Core\Cli\Style::of`'s seven options lands in, read
    /// off the value itself.
    ///
    /// `tests/conformance/core/cli-a-styles-seven-slots-are-independent.nvst`
    /// asks the half a program can see — that the seven are seven — by
    /// comparing whole renderings, and says in its own words that the flag word
    /// is this file's business. This is that half: each attribute alone is the
    /// one bit [`ATTRIBUTES`] pairs it with, so an option written into a
    /// neighbour's bit fails here while both still render as their own style.
    // covers: Core\Cli\Style::of
    #[test]
    fn style_of_writes_each_option_into_the_slot_and_the_bit_that_is_its_own() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let attributes = |bold, dim, italic, underline, strikethrough| {
            [
                Value::null(),
                Value::null(),
                Value::bool(bold),
                Value::bool(dim),
                Value::bool(italic),
                Value::bool(underline),
                Value::bool(strikethrough),
            ]
        };
        let built = |ctx: &mut nvs_runtime::Ctx, args: [Value; 7]| {
            nvs_runtime::call(nvs_core_cli_style_of, ctx, &args).expect("Style::of succeeded")
        };
        let read = |style: Value, slot| {
            crate::instance::slot(style.obj_ptr().expect("a `Style` is an object"), slot)
        };
        let spend = |style: Value| {
            #[expect(unsafe_code, reason = "this frame built the style it releases")]
            unsafe {
                style.release();
            }
        };

        // Nothing asked for: no colour in either slot, and no attribute set.
        let empty = built(&mut ctx, attributes(false, false, false, false, false));
        assert_eq!(read(empty, STYLE_FLAGS).as_int(), Some(0));
        assert!(
            read(empty, STYLE_COLOR).obj_ptr().is_none()
                && read(empty, STYLE_BACKGROUND).obj_ptr().is_none(),
            "a style nobody gave a colour carries one anyway"
        );
        spend(empty);

        // Each attribute alone, against the bit `sgr` reads it back out of.
        for (at, (bit, _)) in ATTRIBUTES.into_iter().enumerate() {
            let mut asked = attributes(false, false, false, false, false);
            asked[at + 2] = Value::bool(true);
            let style = built(&mut ctx, asked);
            assert_eq!(
                read(style, STYLE_FLAGS).as_int(),
                Some(bit),
                "option {} landed on another attribute's bit",
                STYLE_OPTIONS[at + 2].name
            );
            spend(style);
        }

        // All five at once are all five bits, which no single-attribute line
        // above can tell from a bag that keeps only the last one it read.
        let every = built(&mut ctx, attributes(true, true, true, true, true));
        assert_eq!(
            read(every, STYLE_FLAGS).as_int(),
            Some(ATTRIBUTES.iter().fold(0, |bits, (bit, _)| bits | bit)),
            "five attributes asked for together are not the five bits asked for alone"
        );
        spend(every);

        // The two colours are two slots. A style given a foreground has no
        // background, which is the pair most easily written into one place.
        let ink = nvs_runtime::call(nvs_core_cli_color_index, &mut ctx, &[Value::uint(160)])
            .expect("Color::index succeeded");
        let mut foreground = attributes(false, false, false, false, false);
        foreground[0] = ink;
        let front = built(&mut ctx, foreground);
        assert!(
            read(front, STYLE_COLOR).obj_ptr().is_some()
                && read(front, STYLE_BACKGROUND).obj_ptr().is_none(),
            "a foreground colour reached the background slot"
        );
        spend(front);

        let mut background = attributes(false, false, false, false, false);
        background[1] = ink;
        let behind = built(&mut ctx, background);
        assert!(
            read(behind, STYLE_BACKGROUND).obj_ptr().is_some()
                && read(behind, STYLE_COLOR).obj_ptr().is_none(),
            "a background colour reached the foreground slot"
        );
        spend(behind);
        spend(ink);
    }

    /// `Core\Cli\Color::index` is the palette's constructor, asserted as the
    /// three things one of its entries is.
    ///
    /// First the whole palette, counted rather than read off one line: each of
    /// the 256 entries carries [`INK_INDEXED`] in [`COLOR_KIND`] and its own
    /// number in [`COLOR_VALUE`], which a constructor writing the number into
    /// the other slot passes wherever the number is zero.
    ///
    /// Second the bound, on both sides — `255` is an entry and `256` is not —
    /// with the sentence the refusal carries. The class it is thrown as is
    /// `tests/conformance/core/cli-a-colour-outside-its-range-is-refused.nvst`'s,
    /// which catches `RuntimeError` by name, since this side reads the message
    /// alone.
    ///
    /// Third what an entry renders as at a colour depth this process does not
    /// have, which is the half no `.nvst` case can ask about: an entry is sent
    /// as itself wherever the terminal has a palette at all, so `Ansi256` and
    /// `TrueColor` agree on every one of them, the first sixteen are the
    /// `3x`/`9x` codes a terminal with no palette still knows, and `None` is
    /// sent nothing.
    // covers: Core\Cli\Color::index
    #[test]
    fn every_palette_entry_carries_its_own_number_and_stops_at_the_last_one() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let mut carried = 0_u32;
        for entry in 0..=255_u64 {
            let color =
                nvs_runtime::call(nvs_core_cli_color_index, &mut ctx, &[Value::uint(entry)])
                    .expect("an entry the palette has");
            let object = color.obj_ptr().expect("a `Color` is an object");
            if crate::instance::slot(object, COLOR_KIND).as_int() == Some(INK_INDEXED)
                && crate::instance::slot(object, COLOR_VALUE).as_uint() == Some(entry)
            {
                carried += 1;
            }
            #[expect(unsafe_code, reason = "the value owns the reference it releases")]
            unsafe {
                color.release();
            }
        }
        assert_eq!(
            carried, 256,
            "a palette entry does not carry its own number in its own slot"
        );

        for refused in [256_u64, u64::MAX] {
            nvs_runtime::call(nvs_core_cli_color_index, &mut ctx, &[Value::uint(refused)])
                .expect_err("a number no palette has an entry for was accepted");
            assert_eq!(
                ctx.take_pending().map(std::borrow::Cow::into_owned),
                Some(format!(
                    "Core\\Cli\\Color::index: {refused} is not a palette entry, which is 0 to 255"
                )),
                "the refusal does not say which number it read"
            );
        }

        for entry in 16..=255_u8 {
            let ink = Ink {
                kind: INK_INDEXED,
                value: u64::from(entry),
            };
            assert_eq!(
                sgr(Some(ink), None, 0, ColorDepth::Ansi256),
                format!("\u{1B}[38;5;{entry}m"),
                "palette entry {entry} is not sent as itself"
            );
            assert_eq!(
                sgr(Some(ink), None, 0, ColorDepth::TrueColor),
                sgr(Some(ink), None, 0, ColorDepth::Ansi256),
                "palette entry {entry} is sent one way to a 256-colour terminal and another to a \
                 true-colour one"
            );
            assert_eq!(
                sgr(None, Some(ink), 0, ColorDepth::Ansi256),
                format!("\u{1B}[48;5;{entry}m"),
                "palette entry {entry} is not the same colour behind the text as in front of it"
            );
            assert_eq!(
                sgr(Some(ink), None, 0, ColorDepth::None),
                "",
                "a terminal with no colour was sent palette entry {entry}"
            );
        }

        for entry in 0..8_u8 {
            let plain = Ink {
                kind: INK_INDEXED,
                value: u64::from(entry),
            };
            let bright = Ink {
                kind: INK_INDEXED,
                value: u64::from(entry) + 8,
            };
            assert_eq!(
                sgr(Some(plain), None, 0, ColorDepth::Ansi16),
                format!("\u{1B}[{}m", 30 + u32::from(entry)),
                "entry {entry} is not the code every terminal knows it by"
            );
            assert_eq!(
                sgr(Some(bright), None, 0, ColorDepth::Ansi16),
                format!("\u{1B}[{}m", 90 + u32::from(entry)),
                "entry {} is not the bright half of entry {entry}",
                entry + 8
            );
        }
    }

    /// `Core\Cli\Color::rgb` is the constructor for the sixteen million, and
    /// what it builds is asserted as the three things a 24-bit colour is.
    ///
    /// First the packing: slot 0 is [`INK_RGB`] and slot 1 is one number,
    /// `red << 16 | green << 8 | blue`, over a sweep of triples rather than
    /// one — a constructor that packed blue where red goes agrees with this on
    /// every grey.
    ///
    /// Second the bound, per channel and on both sides: `255` is a level and
    /// `256` is not, and each of the three names itself, which a check reading
    /// only its first argument fails while still refusing plausibly. The class
    /// the refusal is thrown as is
    /// `tests/conformance/core/cli-a-colour-outside-its-range-is-refused.nvst`'s,
    /// which catches `RuntimeError` by name.
    ///
    /// Third the degrading, at colour depths this process does not have. A
    /// terminal with true colour is sent the triple. One with 256 colours is
    /// sent a palette entry, and a grey is sent an entry of the grey ramp that
    /// rises with the level rather than one of the colour cube — the branch of
    /// [`entry_of_rgb`] a colour with three different channels never reaches.
    /// One with sixteen is sent one of the sixteen, and one with none is sent
    /// nothing.
    // covers: Core\Cli\Color::rgb
    #[test]
    fn a_24_bit_colour_packs_its_channels_and_degrades_to_what_the_terminal_has() {
        let mut ctx = nvs_runtime::Ctx::buffered();
        let triples = [
            (0_u64, 0_u64, 0_u64),
            (255, 255, 255),
            (30, 144, 255),
            (217, 70, 39),
            (1, 2, 3),
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
        ];
        let mut packed = 0_u32;
        for (red, green, blue) in triples {
            let color = nvs_runtime::call(
                nvs_core_cli_color_rgb,
                &mut ctx,
                &[Value::uint(red), Value::uint(green), Value::uint(blue)],
            )
            .expect("three levels a channel has");
            let object = color.obj_ptr().expect("a `Color` is an object");
            if crate::instance::slot(object, COLOR_KIND).as_int() == Some(INK_RGB)
                && crate::instance::slot(object, COLOR_VALUE).as_uint()
                    == Some(red << 16 | green << 8 | blue)
            {
                packed += 1;
            }
            #[expect(unsafe_code, reason = "the value owns the reference it releases")]
            unsafe {
                color.release();
            }
        }
        assert_eq!(
            packed,
            u32::try_from(triples.len()).expect("eight triples"),
            "a colour does not carry the three channels it was given"
        );

        for (at, channel) in ["red", "green", "blue"].into_iter().enumerate() {
            let mut levels = [Value::uint(255), Value::uint(255), Value::uint(255)];
            levels[at] = Value::uint(256);
            nvs_runtime::call(nvs_core_cli_color_rgb, &mut ctx, &levels)
                .expect_err("a level no channel has was accepted");
            assert_eq!(
                ctx.take_pending().map(std::borrow::Cow::into_owned),
                Some(format!(
                    "Core\\Cli\\Color::rgb: `{channel}` is 256, and a channel is 0 to 255"
                )),
                "the refusal does not name the channel it read"
            );
        }

        let dodger = Ink {
            kind: INK_RGB,
            value: 0x001E_90FF,
        };
        assert_eq!(
            sgr(Some(dodger), None, 0, ColorDepth::TrueColor),
            "\u{1B}[38;2;30;144;255m"
        );
        assert_eq!(
            sgr(None, Some(dodger), 0, ColorDepth::TrueColor),
            "\u{1B}[48;2;30;144;255m",
            "a colour behind the text is not the colour in front of it"
        );
        assert_eq!(
            sgr(Some(dodger), None, 0, ColorDepth::None),
            "",
            "a terminal with no colour was sent a colour"
        );

        let mut ramp = Vec::new();
        for level in 8..=248_u64 {
            let grey = Ink {
                kind: INK_RGB,
                value: level << 16 | level << 8 | level,
            };
            let rendered = sgr(Some(grey), None, 0, ColorDepth::Ansi256);
            let entry: u16 = rendered
                .trim_start_matches("\u{1B}[38;5;")
                .trim_end_matches('m')
                .parse()
                .unwrap_or_else(|_| panic!("`{rendered}` is not a palette entry"));
            assert!(
                (232..=255).contains(&entry),
                "grey {level} is sent as entry {entry}, which is in the colour cube and not the \
                 grey ramp"
            );
            assert!(
                ramp.last().is_none_or(|last| *last <= entry),
                "grey {level} is sent as a darker entry than the grey below it"
            );
            ramp.push(entry);

            let sixteen = sgr(Some(grey), None, 0, ColorDepth::Ansi16);
            let basic: u16 = sixteen
                .trim_start_matches("\u{1B}[")
                .trim_end_matches('m')
                .parse()
                .unwrap_or_else(|_| panic!("`{sixteen}` is not one of the sixteen"));
            assert!(
                (30..=37).contains(&basic) || (90..=97).contains(&basic),
                "grey {level} is sent to a sixteen-colour terminal as {basic}"
            );
        }
        assert_eq!(
            (ramp.first().copied(), ramp.last().copied()),
            (Some(232), Some(255)),
            "the grey ramp does not run from its first entry to its last"
        );
    }

    /// `Core\Cli\Live::set` is the whole of what a program may do to a region,
    /// asserted on a screen this test can read back — the alternative being a
    /// test that needs a terminal and a person in front of it.
    ///
    /// The frame a program hands it is the frame that lands, the **last** one
    /// included. A `set` the timer coalesces is painted by the region's end, so
    /// a program that draws its finished state and returns is not left showing
    /// the state before it. That is the one bug a coalescing region could have,
    /// and it is invisible to every case that reads stdout, since a region with
    /// no terminal renders nothing at all.
    ///
    /// A row arrives as the carrier's own text: an escape a program wrote into
    /// a string reaches the screen as the symbol `Core\Cli\Text::plain` put
    /// there, never as a command the terminal runs. A row wider than the
    /// terminal is cut, because a wrapped row leaves the region unable to find
    /// its own rows again.
    ///
    /// The refusal is the member's own — only the innermost open region paints
    /// — and the class it is thrown as is
    /// `tests/hostile/core/Cli/live/01-a-region-that-will-not-give-the-terminal-back.nvs`'s,
    /// which catches `LogicError` by name.
    // covers: Core\Cli\Live::set
    #[test]
    fn a_frame_lands_as_the_carrier_wrote_it_and_the_last_one_lands_too() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        /// A screen a test can read back.
        struct Screen(Arc<Mutex<Vec<u8>>>);

        impl Write for Screen {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0
                    .lock()
                    .expect("this screen is never poisoned")
                    .extend_from_slice(bytes);
                Ok(bytes.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let mut ctx = nvs_runtime::Ctx::buffered();
        let row = |ctx: &mut nvs_runtime::Ctx, text: &str| {
            nvs_runtime::call(
                nvs_core_cli_text_plain,
                ctx,
                &[Value::str(NvsStr::new(text.as_bytes()))],
            )
            .expect("a row of plain text")
        };
        let frame = |ctx: &mut nvs_runtime::Ctx, texts: &[&str]| {
            let mut rows = NvsArray::new();
            for text in texts {
                rows.append(row(ctx, text));
            }
            Value::array(rows)
        };

        let width = usize::try_from(nvs_runtime::terminal::profile().width()).unwrap_or(80);
        let wide = "w".repeat(width + 40);
        let screen = Arc::new(Mutex::new(Vec::new()));

        let open = Open::region();
        REGIONS.with(|regions| {
            regions.borrow_mut()[open.depth] =
                nvs_runtime::terminal::Region::painting_on(Box::new(Screen(Arc::clone(&screen))));
        });
        let handle = crate::instance::build(
            &LIVE,
            [Value::int(i64::try_from(open.depth).unwrap_or(i64::MAX))],
        );

        // The first frame lands at once, and the rest are coalesced: what the
        // screen owes is the last of them.
        let first = frame(&mut ctx, &["scanning one", &wide, "\u{1B}[2J"]);
        nvs_runtime::call(nvs_core_cli_live_set, &mut ctx, &[handle, first])
            .expect("the innermost region painted");
        let last = frame(&mut ctx, &["scanned three"]);
        nvs_runtime::call(nvs_core_cli_live_set, &mut ctx, &[handle, last])
            .expect("the innermost region painted");

        // An inner region owns the cursor from here, so the outer handle paints
        // nothing at all.
        let inner = Open::region();
        nvs_runtime::call(nvs_core_cli_live_set, &mut ctx, &[handle, last])
            .expect_err("an outer handle painted over the region inside it");
        let refusal = ctx
            .take_pending()
            .expect("the refusal says nothing at all")
            .into_owned();
        assert!(
            refusal.starts_with("Core\\Cli\\Live::set():")
                && refusal.ends_with("only the innermost open region paints"),
            "the refusal does not say which member would not paint, or why: {refusal:?}"
        );
        drop(inner);
        drop(open);

        let written = String::from_utf8(screen.lock().expect("the screen").clone())
            .expect("a region writes what it was handed, and that was text");
        assert!(
            written.contains("scanning one"),
            "the first frame never landed, and this run wrote {written:?}"
        );
        assert!(
            written.contains("scanned three"),
            "the last frame was coalesced away, so the region ended showing an older one"
        );
        assert!(
            written.contains('\u{241B}'),
            "an escape a program wrote is not on screen as the symbol the carrier put there"
        );
        assert!(
            !written.contains("\u{1B}[2J"),
            "a row cleared the screen, so a row reached the terminal as a command"
        );
        assert!(
            !written.contains(&"w".repeat(width + 1)),
            "a row wider than the {width} columns this terminal has was not cut"
        );

        #[expect(
            unsafe_code,
            reason = "the handle and the two frames are values this test built, and \
                      the call took its own reference to each argument"
        )]
        unsafe {
            first.release();
            last.release();
            handle.release();
        }
    }
}
