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
//! What is left here is the surface: four rows, two enums, and the mapping
//! between the runtime's Rust `ColorDepth` and the ordinals
//! [`crate::registry::ENUMS`] gives `Core\Cli\ColorDepth`. The mapping is the
//! one thing this file can get wrong on its own, so
//! [`tests::the_two_enums_agree_with_the_runtimes_own`] holds the two rosters
//! together.
//!
//! # What a `Text` is, and what it is not
//!
//! One slot, holding the bytes as they came out of the sink. **No member at
//! all**, which is deliberate rather than unfinished: everything a program does
//! with a `Text` today it does by producing one (`Core\Out::capture`) or by
//! writing one out (`echo`, whose row is `nvs_runtime::value_to_string`'s
//! carrier arm). `Text::plain`, `Text::styled`, `Text + Text`, `Cli\Style` and
//! `Cli\Color` are ADR 0086 § 2's and land with the sink's own substitution
//! table — the one from § 1 that makes `Text::plain` a constructor which
//! *cannot* produce an injected escape sequence.
//!
//! That ordering is why nothing here substitutes anything. A `Text` this module
//! builds holds bytes the sink already wrote; applying § 1's table to them here
//! would be the second escape ADR 0088 § 5 exists to prevent. The substitution
//! belongs at the sink, on the way in, and `echo` does not perform it yet —
//! which is a gap in the *sink*, not in the carrier, and is stated as gap 1
//! below.
//!
//! # Known gaps
//!
//! 1. **`echo` does not neutralize control bytes yet.** ADR 0086 § 1's table is
//!    unbuilt, so the terminal sink today writes what it is given. When it
//!    lands, nothing in this module changes: a captured `Text` will simply
//!    already hold the neutralized form.
//! 2. **§ 3's `write` and `displayWidth` are not here, and they are the sink's
//!    half rather than the profile's.** `write` is the entry point § 1's
//!    substitution table sits on, and `displayWidth` is a question about how a
//!    renderer would lay a string out; both belong beside `Cli\Text`'s own
//!    members and `Cli\Style`, and `displayWidth` additionally owes a UAX #11
//!    table this tree does not carry yet. What is here is exactly the set § 3
//!    calls *the profile*, which is the set that had to be cached.
//! 3. **The rest of § 13 does not exist** — no `arguments`, no `escape`, no
//!    prompts (`ask`, `confirm`, `select<T>`, `secret`), no scoped `live<T>` or
//!    `progress<T>`, and no `Cli\Style`, `Cli\Color` or `Cli\Shell` beside the
//!    two enums below. `docs/spec/01-core-library.md` § 13 lists them and
//!    `docs/plan/m8.md` owns when.

use nvs_runtime::terminal::{ColorDepth, Stream};
use nvs_runtime::{Fault, Value};

use crate::registry::{
    CaseDoc, CoreClass, CoreEnum, CoreMethod, CoreTy, EnumDoc, MethodDoc, ParamDoc,
};

/// `Core\Cli`'s fully-qualified name, in one place so the registry row and
/// every refusal that names the class cannot drift apart.
pub(crate) const CLASS_NAME: &str = r"Core\Cli";

/// ADR 0086 § 3's profile, as registry rows. See [`crate::registry::CLASSES`].
///
/// Four members and no `write`: the module docs' gap 2 owns that split.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: CLASS_NAME,
    methods: &[
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

// ------------------------------------------------------------------ the members

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
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
}
