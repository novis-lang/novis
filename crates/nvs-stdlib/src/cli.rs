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
//! One slot, holding the bytes as they came out of the sink. **No member at
//! all**, which is deliberate rather than unfinished: everything a program does
//! with a `Text` today it does by producing one (`Core\Out::capture`) or by
//! writing one out (`echo`, whose row is `nvs_runtime::value_to_string`'s
//! carrier arm). `Text::plain`, `Text::styled`, `Text + Text`, `Cli\Style` and
//! `Cli\Color` are ADR 0086 § 2's and are still owed. § 1's substitution, which
//! is what makes `Text::plain` a constructor that *cannot* produce an injected
//! escape sequence, has landed ahead of them and is at the sink.
//!
//! That ordering is why the carrier builder here substitutes nothing. A `Text`
//! this module builds holds bytes the sink already neutralized; applying § 1's
//! table to them here would be the second escape ADR 0088 § 5 exists to
//! prevent. The substitution belongs at the sink, on the way in, and
//! `nvs_runtime`'s `nvs_echo_str` is where it happens — that helper's own doc
//! comment owns it, including why the table's idempotence is what makes
//! `echo`ing a captured `Text` correct.
//!
//! [`nvs_core_cli_escape`] is the same table reached as a *value* rather than
//! as an effect — ADR 0024 § 3's named launderer for this sink — and it calls
//! `nvs_render::text::substitute` exactly as the sink does, so the two cannot
//! come to disagree.
//!
//! # Known gaps
//!
//! 1. **§ 3's `write` and `displayWidth` are not here.** `write` is a second
//!    spelling of the sink `echo` already is, so what it owes is a row and a
//!    stream argument rather than a rule; `displayWidth` is a question about
//!    how a renderer would lay a string out, belongs beside `Cli\Style`, and
//!    additionally owes a UAX #11 table this tree does not carry yet.
//! 2. **The rest of § 13 does not exist** — no `arguments`, no prompts (`ask`,
//!    `confirm`, `select<T>`, `secret`), no scoped `live<T>` or `progress<T>`,
//!    and no `Cli\Style` or `Cli\Color` beside the three enums below.
//!    `docs/spec/01-core-library.md` § 13 lists them and `docs/plan/m8.md` owns
//!    when.

use nvs_runtime::terminal::{ColorDepth, Stream};
use nvs_runtime::{Fault, NvsStr, Value};

use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, MethodDoc, ParamDoc, Qual,
};

/// `Core\Cli`'s fully-qualified name, in one place so the registry row and
/// every refusal that names the class cannot drift apart.
pub(crate) const CLASS_NAME: &str = r"Core\Cli";

/// ADR 0086 § 3's profile plus § 1's launderer, as registry rows. See
/// [`crate::registry::CLASSES`].
///
/// Five members and still no `write`: the module docs' gap 1 owns that split,
/// and [`nvs_core_cli_escape`] owns why the launderer could land ahead of it.
///
/// In the spec's own order (§ 15), which is why `escape` is first: `arguments`
/// and `write` come before it and are the two rows still owed.
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

/// Spec § 13's `Core\Cli\Text`, as much of it as ADR 0088 § 5 needs — see the
/// module docs for why that is a slot and no members.
pub(crate) const TEXT: CoreClass = CoreClass {
    name: NAME,
    methods: &[],
    instance: &[],
    slots: &["text"],
    constants: &[],
};

/// A `Core\Cli\Text` carrying `text`, which must be a `Tag::Str` value the
/// caller is transferring — the one producer, called by [`crate::out`].
pub(crate) fn built(text: nvs_runtime::Value) -> nvs_runtime::Value {
    crate::instance::build(&TEXT, [text])
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
}
