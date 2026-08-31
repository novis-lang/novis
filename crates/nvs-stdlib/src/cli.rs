//! `Core\Cli` — the terminal facts a program is allowed to ask for, and
//! `Core\Cli\Text`, the carrier of the terminal sink.
//!
//! [ADR 0086](../../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 3 is
//! this module's half of that ADR: which of the three standard streams is a
//! terminal, how wide and how tall it is, and how much colour it can show.
//! [ADR 0088](../../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
//! § 3's table pairs every context with a sink and every sink with a *carrier*:
//! `Core\Html\Markup` under an HTTP request, `Core\Cli\Text` everywhere else,
//! and § 5 makes that carrier the return of `Core\Out::capture`. So the carrier
//! had to exist before [`crate::out`] could, which is why it is in this file
//! and was in it alone for a while.
//!
//! # Every member here is one read of a profile resolved once
//!
//! ADR 0086 § 3 says the terminal profile is resolved **once per process, not
//! per call**, so two reads of `Core\Cli::width()` are the same number by
//! construction rather than by luck. That resolution is
//! [`nvs_runtime::terminal`] and not this module: it reaches the operating
//! system, and [ADR 0118](../../../../docs/adr/0118-capabilities-are-configured-not-requested.md)
//! § 2 says a `Core` member may not — `tests/capability.rs`'s
//! `nvs_stdlib_reaches_the_os_only_through_the_gate` holds that shut by name.
//! That module's own docs own the caching, what it spends, and why no
//! capability gates it.
//!
//! What is left here is the surface: five rows, three enums, and the mapping
//! between the runtime's Rust `ColorDepth` and the ordinals
//! [`crate::registry::ENUMS`] gives `Core\Cli\ColorDepth`. The mapping is the
//! one thing this file can get wrong on its own, so
//! [`tests::the_two_enums_agree_with_the_runtimes_own`] holds the two rosters
//! together.
//!
//! # Why `Core\Cli\Shell` is here, taken by nothing in this file
//!
//! [`SHELL`] is the third enum and no member of `Core\Cli` reads it: ADR 0086
//! § 6's `Core\Command::completions` is the one thing that does, and
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
//! ADR 0086 § 1's *"there is exactly one raw path, `Cli\Text`"* asks for and
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
//! ADR 0086 § 2's other half is here too: [`STYLE`] and [`COLOR`], the two
//! value types that exist so the carrier has something to wear that is not a
//! grammar. A colour is a class *constant that is an instance* —
//! `Color::RED` is `Color::index(1)` inlined at the use site, which
//! [`crate::registry::Const::Built`] has expressed since `Core\Time\Zone::UTC`
//! — so the sixteen names cost one allocation each where they are written and
//! nothing is shared between isolates.
//!
//! [`nvs_core_cli_text_styled`] renders that style **when the `Text` is built**
//! rather than when it is written, against the profile § 3 resolves once per
//! process. Its own doc comment owns why the two are the same bytes today and
//! what the difference would be; § 2's body records the decision.
//!
//! `Text + Text` is § 2's and is still owed: `+` over two objects needs a row
//! in `nvs_types`' operator table before this file can express it.
//!
//! [`nvs_core_cli_escape`] is the same table reached as a *value* rather than
//! as an effect — ADR 0024 § 3's named launderer for this sink — and it calls
//! `nvs_render::text::substitute` exactly as the sink does, so the two cannot
//! come to disagree.
//!
//! # The prompts, and the two questions each one asks first
//!
//! ADR 0086 § 4's `ask`, `confirm`, `select<T>` and `secret` are here, and
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
//! That pair is also what makes the prompts *testable*: a `.nvst` case runs as
//! a child with its output piped, so every prompt in one takes the
//! not-interactive path by construction rather than by luck, and a case can
//! freeze what it answers without a terminal or a person anywhere near it.
//!
//! # Known gaps
//!
//! 1. **§ 3's `write` and `displayWidth` are not here.** `write` is a second
//!    spelling of the sink `echo` already is, so what it owes is a row and a
//!    stream argument rather than a rule; `displayWidth` is a question about
//!    how a renderer would lay a string out, belongs beside `Cli\Style`, and
//!    additionally owes a UAX #11 table this tree does not carry yet.
//! 2. **The rest of § 15 does not exist** — no `arguments`, no `multiSelect`,
//!    and no scoped `live<T>` or `progress<T>`.
//!    `docs/spec/01-core-library.md` § 15 lists them and `docs/plan/m8.md`
//!    owns when. ADR 0086 § 4's last paragraph is owed with them: under
//!    `nvs test` a prompt should drain a scripted answer queue rather than
//!    read a terminal, and today it takes the not-interactive path there
//!    instead — a test's output is a buffer, so [`watched`] answers `false` —
//!    which makes an interactive flow assertable only through its defaults.
//! 3. **A `Text` cannot be plain on one stream and styled on another in the
//!    same run.** It holds bytes, and the styling is rendered into them once —
//!    so a program writing the same `Text` to a terminal standard output and a
//!    redirected standard error sends both the same thing. Nothing on disk can
//!    observe it: `echo` is the only sink, it writes standard output, and § 3's
//!    colour depth is a process answer (`Cli::colorDepth` takes no stream).
//!    What closes it is the `Cli\Text` of runs § 2's body names as the shape a
//!    per-stream `Cli::write` would need.

use nvs_runtime::terminal::{ColorDepth, Echo, Stream};
use nvs_runtime::{Fault, NvsStr, Tag, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreConst, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc,
    ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// `Core\Cli`'s fully-qualified name, in one place so the registry row and
/// every refusal that names the class cannot drift apart.
pub(crate) const CLASS_NAME: &str = r"Core\Cli";

/// ADR 0086 § 3's profile, § 1's launderer and § 4's prompts, as registry
/// rows. See [`crate::registry::CLASSES`].
///
/// Nine members and still no `write`: the module docs' gap 1 owns that split,
/// and [`nvs_core_cli_escape`] owns why the launderer could land ahead of it.
///
/// In the spec's own order (§ 15), which is why `escape` is first: `arguments`
/// and `write` come before it and are the two rows still owed, and § 4's
/// `multiSelect` is the third — it is the one prompt whose answer is a set
/// rather than a value, so it owes a second reading loop rather than another
/// row of the shape below.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: CLASS_NAME,
    methods: &[
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
            // `choices` rather than § 4's own `$options`: ADR 0063 R2 reserves
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
            name: "secret",
            names: &["question"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::SecretTaintedStr,
            symbol: "nvs_core_cli_secret",
            doc: Some(&SECRET_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Cli::escape`'s reference card — ADR 0117.
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

/// `Core\Cli::isTty`'s reference card — ADR 0117.
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

/// `Core\Cli::width`'s reference card — ADR 0117.
const WIDTH_DOC: MethodDoc = MethodDoc {
    short: "The controlling terminal's width in columns — `tput cols`, without a child process. \
            Resolved once for the process.",
    params: &[],
    ret: "The column count, or `80` when no standard stream is a terminal. Never `0`, so a \
          caller may subtract a margin from it without checking.",
    errors: &[],
};

/// `Core\Cli::height`'s reference card — ADR 0117.
const HEIGHT_DOC: MethodDoc = MethodDoc {
    short: "The controlling terminal's height in rows — `tput lines`. Resolved once for the \
            process, alongside the width it was read with.",
    params: &[],
    ret: "The row count, or `24` when no standard stream is a terminal. Never `0`, for \
          `width`'s reason.",
    errors: &[],
};

/// `Core\Cli::colorDepth`'s reference card — ADR 0117.
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

/// `Core\Cli::ask`'s trailing options — ADR 0086 § 4's
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

/// `Core\Cli::ask`'s reference card — ADR 0117.
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
        desc: "There is no controlling terminal to ask, or its input ended, and the call named no \
               `default`.",
    }],
};

/// `Core\Cli::confirm`'s reference card — ADR 0117.
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
        desc: "There is no controlling terminal to ask, or its input ended, and the call named no \
               `default`.",
    }],
};

/// `Core\Cli::select`'s reference card — ADR 0117.
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
            desc: "There is no controlling terminal to ask, or its input ended, and the call \
                   named no `default`.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`$choices` is empty, so there is nothing that could be chosen.",
        },
    ],
};

/// `Core\Cli::secret`'s reference card — ADR 0117.
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
        desc: "There is no controlling terminal to ask, or its input ended. `secret` takes no \
               `default`, because a password nobody typed is not a password.",
    }],
};

/// `Core\Cli\Stream`'s fully-qualified name, written once — [`STREAM`] declares
/// it and the [`CoreTy::Enum`] naming it resolves against
/// [`crate::registry::ENUMS`], so the two cannot drift apart.
pub(crate) const STREAM_NAME: &str = r"Core\Cli\Stream";

/// ADR 0086 § 3's `Cli\Stream` — which standard stream a question is about.
///
/// The ordinals are [`nvs_runtime::terminal::Stream`]'s own declaration order,
/// which is what [`stream_of`] converts between.
pub(crate) const STREAM: CoreEnum = CoreEnum {
    name: STREAM_NAME,
    cases: &[("In", 0), ("Out", 1), ("Err", 2)],
    doc: Some(&STREAM_DOC),
};

/// [`STREAM`]'s reference card — ADR 0117.
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

/// ADR 0086 § 3's `Cli\ColorDepth` — how much colour a terminal can show.
///
/// Ordered from least to most, so the sink's `truecolor → 256 → 16 → none`
/// degradation is a comparison on the ordinal rather than a table.
pub(crate) const COLOR_DEPTH: CoreEnum = CoreEnum {
    name: COLOR_DEPTH_NAME,
    cases: &[("None", 0), ("Ansi16", 1), ("Ansi256", 2), ("TrueColor", 3)],
    doc: Some(&COLOR_DEPTH_DOC),
};

/// [`COLOR_DEPTH`]'s reference card — ADR 0117.
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

/// ADR 0086 § 6's `Cli\Shell` — the shell `Core\Command::completions` writes a
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

/// [`SHELL`]'s reference card — ADR 0117.
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
        "nvs_core_cli_escape" => (nvs_core_cli_escape as *const ()).cast(),
        "nvs_core_cli_is_tty" => (nvs_core_cli_is_tty as *const ()).cast(),
        "nvs_core_cli_width" => (nvs_core_cli_width as *const ()).cast(),
        "nvs_core_cli_height" => (nvs_core_cli_height as *const ()).cast(),
        "nvs_core_cli_color_depth" => (nvs_core_cli_color_depth as *const ()).cast(),
        "nvs_core_cli_text_plain" => (nvs_core_cli_text_plain as *const ()).cast(),
        "nvs_core_cli_text_styled" => (nvs_core_cli_text_styled as *const ()).cast(),
        "nvs_core_cli_color_index" => (nvs_core_cli_color_index as *const ()).cast(),
        "nvs_core_cli_color_rgb" => (nvs_core_cli_color_rgb as *const ()).cast(),
        "nvs_core_cli_style_of" => (nvs_core_cli_style_of as *const ()).cast(),
        "nvs_core_cli_ask" => (nvs_core_cli_ask as *const ()).cast(),
        "nvs_core_cli_confirm" => (nvs_core_cli_confirm as *const ()).cast(),
        "nvs_core_cli_select" => (nvs_core_cli_select as *const ()).cast(),
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
fn stream_of(value: &Value) -> Result<Stream, Fault> {
    match value.as_int() {
        Some(0) => Ok(Stream::In),
        Some(1) => Ok(Stream::Out),
        Some(2) => Ok(Stream::Err),
        // This is unreachable from source: `isTty`'s parameter is
        // `CoreTy::Enum(STREAM_NAME)`, so `E0401` refuses anything that is not
        // one of the three cases before a single instruction of this body runs,
        // and compiled code writes the ordinal itself.
        _ => Err(Fault::fatal(format!(
            "Core\\Cli::isTty expected a `Core\\Cli\\Stream` case, got tag {} value {:?}",
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
    /// `Core\Cli::escape(tainted string $text): string` — ADR 0086 § 1's named
    /// launderer for the terminal sink, and PHP's missing counterpart to
    /// `htmlspecialchars`.
    ///
    /// # Why this exists when `echo` already substitutes
    ///
    /// ADR 0086 § 1 puts the substitution at the sink and states outright that
    /// *ordinary output does not need this member*. What needs it is a program
    /// that wants the neutralized text **as a value** — to interpolate into a
    /// `Core\Str::format` template, to measure, or to compare — and, under
    /// [ADR 0024](../../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)
    /// § 3, to hand a `tainted string` to something else that refuses one. That
    /// is the whole of its job, which is why it could land before `write` did:
    /// the sink is `echo`, `write` is a second spelling of it, and neither is
    /// what this member answers.
    ///
    /// # One table, one implementation
    ///
    /// [`nvs_render::text::substitute`] is called rather than restated, so this
    /// member and [`nvs_runtime`]'s `echo` cannot answer differently — which is
    /// the property that would otherwise fail silently, since a launderer that
    /// neutralizes *less* than its sink is exactly the false confidence ADR 0024
    /// § 3 refuses a generic `sanitize()` over. That module owns the table and
    /// its rows' reasoning; [ADR 0087](../../../../docs/adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)
    /// owns the bidi row's predicate.
    ///
    /// # Why the qualifier comes off
    ///
    /// [`Qual::Launder`] on the parameter, per ADR 0024 § 3's rule that a
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
        let stream = stream_of(&args[0])?;
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
    /// `Core\Cli::colorDepth(): Cli\ColorDepth` — a question PHP has no
    /// spelling for at all, which is why every CLI package ships its own
    /// heuristic.
    ///
    /// An enum answers as its ordinal, exactly as a user-declared enum does
    /// (ADR 0010).
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
/// ADR 0086 § 1's substitution over the question and not only over the answer:
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

/// Whether this call has anyone to ask — ADR 0086 § 4's "with no controlling
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

/// One line off the controlling terminal, or `None` when there is nobody to
/// ask or the terminal's input ended.
fn ask_terminal(ctx: &nvs_runtime::Ctx, question: &str, echo: Echo) -> Option<String> {
    if !watched(ctx) {
        return None;
    }
    nvs_runtime::terminal::prompt(question, echo)
}

/// What a prompt with nowhere to read and nothing to fall back on throws —
/// ADR 0086 § 4's `Core\Cli\NotInteractive`, which is in
/// `nvs_hir::errors::TREE` so that a program can `catch` it by name.
///
/// One function for all four prompts, so the sentence a program sees is the
/// same wherever it came from; `tests/conformance/core/cli-prompts-are-not-interactive-without-a-terminal.nvst`
/// is what freezes it.
fn not_interactive(member: &str) -> Fault {
    Fault::thrown_as(
        nvs_runtime::ThrownClass::CliNotInteractive,
        format!("no controlling terminal to answer Core\\Cli::{member}"),
    )
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
    /// — ADR 0086 § 4's first prompt, replacing `readline` and the
    /// `fgets(STDIN)` every PHP script writes instead of it.
    ///
    /// # It is not `fgets(STDIN)`, and that is the point
    ///
    /// § 4: *prompts read the controlling terminal, not standard input*, so
    /// `cat data.csv | myprog` can still ask a question. `nvs_runtime::terminal`
    /// is where the device is opened by name; nothing in this module reads the
    /// standard input stream at all, which
    /// [`tests::a_prompt_reads_the_controlling_terminal_and_not_stdin`] holds
    /// shut over this file's own source.
    ///
    /// # Why the answer is `tainted` and the question is `Qual::Neutral`
    ///
    /// The answer came from outside the program, so it is
    /// [`CoreTy::TaintedStr`] — a promise about the *value*, unconditional,
    /// exactly as ADR 0060 § 5's verified claims are. The question's own
    /// classification is [`Qual::Neutral`] rather than [`Qual::Sink`] because
    /// the terminal substitutes instead of refusing (§ 1) and because the
    /// answer's qualifier does not depend on the question's: a prompt built
    /// from a literal and a prompt built from a request parameter both answer
    /// something untrusted.
    fn nvs_core_cli_ask(ctx, args: [3]) {
        let question = question_of(&args[0], "ask")?;
        let (fallback, validate) = (args[1], args[2]);
        loop {
            let Some(answer) = ask_terminal(ctx, &question, Echo::Shown) else {
                return if given(fallback) {
                    Ok(handed_back(fallback))
                } else {
                    Err(not_interactive("ask"))
                };
            };
            if answer.is_empty() && given(fallback) {
                return Ok(handed_back(fallback));
            }
            let value = Value::str(NvsStr::new(answer.as_bytes()));
            if !given(validate) {
                return Ok(value);
            }
            let verdict = nvs_runtime::call_closure(ctx, validate, &[value]);
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
    /// ADR 0024 § 2 launders a checked conversion, and a closed two-case
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
            let Some(answer) = ask_terminal(ctx, &question, Echo::Shown) else {
                return if given(fallback) {
                    Ok(handed_back(fallback))
                } else {
                    Err(not_interactive("confirm"))
                };
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
    /// [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R4 and R5
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
        // where it can be read, so an unattended run calls no `labels`
        // callback and renders no line it would then discard.
        if !watched(ctx) {
            return if given(fallback) {
                Ok(handed_back(fallback))
            } else {
                Err(not_interactive("select"))
            };
        }

        let mut menu = String::new();
        for (at, option) in options.iter().enumerate() {
            menu.push_str(&format!("{}) {}\n", at + 1, label_of(ctx, labels, *option)?));
        }
        menu.push_str(&question);
        menu.push(' ');

        loop {
            let Some(answer) = ask_terminal(ctx, &menu, Echo::Shown) else {
                return if given(fallback) {
                    Ok(handed_back(fallback))
                } else {
                    Err(not_interactive("select"))
                };
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

nvs_runtime::nvs_helper! {
    /// `Core\Cli::secret(string $question): secret tainted string` — § 4's
    /// fourth prompt, and the clearest demonstration of why
    /// [ADR 0033](../../../../docs/adr/0033-secret-qualifier-for-confidential-values.md)'s
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
        let Some(answer) = ask_terminal(ctx, &question, Echo::Hidden) else {
            return Err(not_interactive("secret"));
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
/// object option renders through ADR 0028 § 1's `toString` — a menu of
/// `Core\Time\Zone`s should read as its zones, and a class with no renderer
/// throws here rather than printing a placeholder nobody can choose between.
fn label_of(ctx: &mut nvs_runtime::Ctx, labels: Value, option: Value) -> Result<String, Fault> {
    let rendered = if given(labels) {
        let answered = nvs_runtime::call_closure(ctx, labels, &[option])?;
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
/// [`CoreTy::Instance`](crate::registry::CoreTy::Instance) spells it.
///
/// Taken from `nvs_runtime::CARRIER_CLI_TEXT` rather than written again here:
/// the *sink* decides what its carrier is (ADR 0088 § 3), the sink lives in
/// `nvs-runtime`, and `nvs_runtime::value_to_string` renders whatever that
/// constant names. Two spellings could disagree and the render would silently
/// stop happening.
pub(crate) const NAME: &str = nvs_runtime::CARRIER_CLI_TEXT;

/// Spec § 13's `Core\Cli\Text` — ADR 0088 § 5's slot, and ADR 0086 § 2's first
/// constructor over it. See the module docs for what is still owed.
pub(crate) const TEXT: CoreClass = CoreClass {
    name: NAME,
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
    instance: &[],
    slots: &["text"],
    constants: &[],
};

/// `Core\Cli\Text::plain`'s reference card — ADR 0117.
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

/// `Core\Cli\Text::styled`'s reference card — ADR 0117.
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

/// A `Core\Cli\Text` carrying `text`, which must be a `Tag::Str` value the
/// caller is transferring.
///
/// **This transfers bytes; it does not neutralize them.** Every caller owes
/// that itself, and there are two: [`crate::out`]'s `capture`, whose bytes came
/// out of the sink already, and [`nvs_core_cli_text_plain`], which substitutes
/// over its argument first. That is the whole of what keeps ADR 0086 § 1's raw
/// path closed — `nvs_runtime::helpers::is_carrier_value` owns why the sink
/// trusts the class rather than the bytes.
pub(crate) fn built(text: nvs_runtime::Value) -> nvs_runtime::Value {
    crate::instance::build(&TEXT, [text])
}

nvs_runtime::nvs_helper! {
    /// `Core\Cli\Text::plain(string $text): Cli\Text` — ADR 0086 § 2's first
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

/// ADR 0086 § 2's `Cli\Color` — **a value type, not an enum**.
///
/// [ADR 0010](../../../../docs/adr/0010-enums-are-a-value-type.md)'s closed
/// named integer type does not fit a set with sixteen million members, so the
/// sixteen the terminal names are class constants and the rest is constructed.
/// Two slots rather than one packed integer because the two colour spaces are
/// genuinely different questions — a palette entry is resolved by the
/// terminal's own theme and a triple is not — and [`sgr_ink`] degrades between
/// them, which a single encoded number would make an arithmetic puzzle.
pub(crate) const COLOR: CoreClass = CoreClass {
    name: COLOR_NAME,
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

/// `Core\Cli\Color::index`'s reference card — ADR 0117.
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

/// `Core\Cli\Color::rgb`'s reference card — ADR 0117.
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

/// ADR 0086 § 2's *"the sixteen named colours are class constants"*, as the
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

/// ADR 0086 § 2's `Cli\Style::of` options — R2's one trailing shape, and the
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

/// ADR 0086 § 2's `Cli\Style` — what a `Text` wears, as a value.
///
/// One member, because a style is constructed and then read: ADR 0063 R5's
/// `of` for the canonical construction, R2's one trailing shape for the
/// options, R20's immutability for everything after.
pub(crate) const STYLE: CoreClass = CoreClass {
    name: STYLE_NAME,
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

/// `Core\Cli\Style::of`'s reference card — ADR 0117.
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
/// has — ADR 0086 § 3's `truecolor → 256 → 16`, which is a comparison on
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
/// [`ColorDepth::None`], which is ADR 0086 § 3's *"when the stream is not a
/// terminal, styling is dropped entirely"*.
///
/// A pure function of the style and the depth, so
/// [`tests::styling_is_a_value_type_and_never_a_grammar`] can ask it about a
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
    /// `Core\Cli\Color::index(uint $index): Cli\Color` — ADR 0086 § 2's
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
    /// underline?, strikethrough?}): Cli\Style` — ADR 0086 § 2's style, as a
    /// value rather than as a fifth grammar (ADR 0063 R11 fixes the count at
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
    /// # Where the degradation happens, and the one thing it cannot see
    ///
    /// ADR 0086 § 3's profile is resolved once per process, so rendering the
    /// style here gives byte-identical output to rendering it at the moment of
    /// the write — with one exception, which § 2's body records: a `Text` holds
    /// bytes, so it cannot be written *plain to a redirected stderr and styled
    /// to a terminal stdout* in the same run. The colour depth is a process
    /// answer (`Cli::colorDepth` takes no stream), and `echo` — the only sink
    /// that exists — writes standard output, so nothing on disk can observe
    /// the difference today.
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
        let style = crate::instance::receiver(args[1], &STYLE, "styled")?;
        let color = ink_of(crate::instance::slot(style, STYLE_COLOR), "color")?;
        let background = ink_of(crate::instance::slot(style, STYLE_BACKGROUND), "background")?;
        let flags = crate::instance::slot(style, STYLE_FLAGS).as_int().unwrap_or(0);
        let opening = sgr(
            color,
            background,
            flags,
            nvs_runtime::terminal::profile().color_depth(),
        );
        let body = nvs_render::text::substitute(text);
        let carried = if opening.is_empty() {
            body.into_owned()
        } else {
            // One reset closes everything the opening sequence set, so a `Text`
            // never leaks its own styling into what is written after it.
            format!("{opening}{body}\u{1B}[0m")
        };
        Ok(built(Value::str(NvsStr::new(carried.as_bytes()))))
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
        assert!(nvs_runtime::is_carrier(NAME));
    }

    /// ADR 0086 § 4: a prompt reads the **controlling terminal**, so
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
        let ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Buffer(Vec::new()));
        assert!(!watched(&ctx));
        assert!(ask_terminal(&ctx, "Name? ", Echo::Shown).is_none());
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
             than the terminal ADR 0086 § 4 names"
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

    /// The carrier holds exactly one slot: `nvs_runtime::value_to_string`
    /// renders slot 0 and nothing else, so a second one would be invisible to
    /// the only consumer there is.
    #[test]
    fn the_carrier_holds_one_slot() {
        assert_eq!(TEXT.slots.len(), 1);
    }

    /// ADR 0086 § 1: terminal output substitutes a control sequence
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
    /// its sink grow apart (ADR 0024 § 3). The third assertion is the one a
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

    /// ADR 0086 § 1's *"there is exactly one raw path, `Cli\Text`"* — asserted
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

    /// ADR 0086 § 3: the profile is resolved **once per process**, so every
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
    /// terminal — which is the case ADR 0086 § 3 names the fallback for, and
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
    #[test]
    fn the_two_enums_agree_with_the_runtimes_own() {
        for (ordinal, (name, declared)) in STREAM.cases.iter().enumerate() {
            let want = i64::try_from(ordinal).expect("three cases");
            assert_eq!(*declared, want, "`{name}` is not at its own index");
            assert_eq!(
                stream_of(&Value::int(want)).expect("a declared case"),
                match ordinal {
                    0 => Stream::In,
                    1 => Stream::Out,
                    _ => Stream::Err,
                }
            );
        }
        assert!(
            stream_of(&Value::int(3)).is_err(),
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

    /// ADR 0086 § 6 names four shells and nothing else, and a case is only
    /// reachable from source once [`crate::registry::ENUMS`] carries the enum.
    ///
    /// Both halves, because each fails on its own: a fifth case added ahead of
    /// the generator that would write its script fails the roster, and a
    /// declaration nobody registered compiles, documents itself, and then
    /// resolves nowhere — `Core\Cli\Shell::Bash` would be a name error with a
    /// card on disk describing it.
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

    /// ADR 0086 § 2: **styling is a value type, never a grammar** — asserted as
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
            "ADR 0086 § 2 names sixteen colours as class constants"
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
}
