//! `Core\Env` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 15's environment half, which is
//! [ADR 0012](../../../../docs/adr/0012-no-superglobals.md) § 1's replacement
//! for `$_ENV` and `getenv()`.
//!
//! Two members so far, `get` and `all`, and all three of § 15's constants —
//! `EOL`, `OS` and `VERSION`, whose one shared decision is on [`CONSTANTS`].
//! `mode()` is gap 1 below and the last of the row.
//!
//! # It is not a capability door, and that is decided rather than skipped
//!
//! [ADR 0118](../../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md)
//! § 2 puts a check at the door to an effect, and reading the environment is
//! not one: ADR 0012 § 7 says outright that `Core\Env` and `Core\Cli` are
//! "process-wide facts already governed by the existing capability/config-overlay
//! machinery", not per-request secrets to wall off. The operator who launched
//! the process chose its environment in the same breath as its `nvs.toml`, so a
//! grant here would gate one half of a decision the other half is already
//! trusted with. `nvs_runtime::terminal`'s module doc records the same
//! conclusion from the other side, for the four variables `Core\Cli` reads and
//! never hands back.
//!
//! What does the protecting is [ADR 0024](../../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)
//! § 1's qualifier: every value here is `tainted`, because the environment is
//! outside the program's own text, so a variable holding a URL still has to
//! reach `Core\Http::allowUrl` and one holding a table name still has to reach
//! `Core\Db::quoteIdentifier`.
//!
//! The **name** is a [`Qual::Sink`] and so refuses a `tainted` argument, on
//! ADR 0088 § 1's predicate: its content is an instruction naming what the
//! runtime must hand back, and a program that let a request choose would hand
//! out the whole environment one query string at a time — the deployment's
//! database password included. It is `Core\IO`'s rule for a path, reached by
//! the same reasoning and for a store that is usually more sensitive than the
//! filesystem. Not [`Qual::Contagious`], which the classification rule beside
//! that enum would otherwise give it — the answer is already `tainted`
//! unconditionally, so contagion would add nothing and admit everything.
//!
//! # Read-only, so there is nothing process-global to synchronise
//!
//! There is no `putenv` — `docs/spec/02-php-migration.md` drops it because a
//! process-global mutation is unsound across cores. That is what makes an
//! unsynchronised read safe from a core thread: nothing in this runtime writes
//! the environment after start, so two requests on two cores read the same
//! bytes and neither can tear the other's read.
//!
//! The read itself is [`nvs_runtime::environment`] and not this crate's own
//! spelling, because `nvs_stdlib_reaches_the_os_only_through_the_gate` forbids
//! one here — ADR 0118 § 2 puts every effect behind a door in `nvs_runtime`,
//! and that module's doc owns why this particular door asks nothing.
//!
//! # A value that is not text throws, and `all` skips it instead
//!
//! [ADR 0009](../../../../docs/adr/0009-string-and-bytes.md) makes a `string`
//! UTF-8, and an environment variable is bytes on every platform this runs on,
//! so the two do not always meet. The split:
//!
//! * `get` **throws**. The caller named one variable, so the honest answer to
//!   "what is `X`" is not `null` — that would report an unreadable value as an
//!   absent one, and [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md)'s
//!   `?T` means absence and nothing else. Repairing it lossily is
//!   [ADR 0095](../../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md)'s
//!   refusal.
//! * `all` **omits** it. That member enumerates, and one variable set by
//!   something else on the machine must not be able to make a program's own
//!   sweep of the environment fail. The omission is discoverable at full
//!   fidelity, because `get` on the same name says so.
//!
//! # Name order, so the answer is the same twice
//!
//! `std::env::vars_os` yields the platform's own order — `environ`'s on Unix
//! and a sorted block on Windows — so `all` sorts by name before building the
//! array. Two calls in one run then agree by construction and two runs on two
//! platforms agree as well, which is the property that makes the result
//! printable in a `.nvst` case at all. It is the same rule `Core\Config::all`
//! states, reached differently: that member's sources are ordered maps already.
//!
//! # Known gaps
//!
//! 1. **`mode()`.** It reads
//!    [ADR 0091](../../../../docs/adr/0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)'s
//!    run mode through `Core\Config` and needs its `Env\Mode` enum first —
//!    `Production` and `Development`, and no third value. Spec § 15 is explicit
//!    that no environment variable is consulted for it, so it is this class's
//!    one member that does not read the environment at all.

use std::collections::BTreeMap;

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    Const, CoreClass, CoreConst, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Env",
    methods: &[
        CoreMethod {
            name: "get",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Sink)],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::TaintedStr),
            symbol: "nvs_core_env_get",
            doc: Some(&GET_DOC),
        },
        CoreMethod {
            name: "all",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::TaintedStr),
            symbol: "nvs_core_env_all",
            doc: Some(&ALL_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: CONSTANTS,
};

/// § 15's three constants — `PHP_EOL`, `PHP_OS_FAMILY` and `PHP_VERSION`, each
/// with the one thing about it that is decided rather than transcribed.
///
/// A `CoreConst` is folded into the program at the site that names it, so all
/// three are fixed when the program is **compiled**, where PHP's are fixed when
/// it runs. That is the same machine here: Novis compiles the program it is
/// about to run, and `Core\Path::SEPARATOR` — the platform-dependent constant
/// that landed first — already rests on exactly this reading. Whether a cached
/// artifact may ever be replayed on another machine is
/// [ADR 0042](../../../../docs/adr/0042-on-disk-artifact-cache-format.md)'s
/// question about that cache's identity and not this module's, and the day it
/// is answered these constants and `SEPARATOR` are answered together.
const CONSTANTS: &[CoreConst] = &[
    CoreConst {
        name: "EOL",
        ty: CoreTy::Str,
        value: Const::Str(EOL),
        desc: "The line ending this platform writes — `\\r\\n` on Windows and `\\n` everywhere \
               else, as `PHP_EOL` is. It is for *emitting* platform-native text and nothing \
               reads it: `Core\\Str::lines` and `Core\\IO::lines` split on all three terminators \
               and never consult it, which is spec § 1's own note.",
    },
    CoreConst {
        name: "OS",
        ty: CoreTy::Str,
        value: Const::Str(OS),
        desc: "The operating system **family**, spelled as `PHP_OS_FAMILY` spells it — \
               `Windows`, `Darwin`, `Linux`, `BSD`, `Solaris`, or `Unknown` for anything else. \
               A closed set a program can compare against, and never `uname`'s free text, \
               which is what PHP's other spelling `PHP_OS` hands over.",
    },
    CoreConst {
        name: "VERSION",
        ty: CoreTy::Str,
        value: Const::Str(VERSION),
        desc: "This runtime's version, replacing `PHP_VERSION` — three dot-separated numbers, \
               and the same string `nvs info` reports. There is no `PHP_VERSION_ID` beside it: \
               a second spelling of one fact is what R6 closes, and comparing versions is \
               `Core\\Str::split` plus arithmetic on what this already says.",
    },
];

/// [`CONSTANTS`]' `EOL` — `PHP_EOL`'s own value.
const EOL: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// [`CONSTANTS`]' `OS`, as `PHP_OS_FAMILY` spells a family.
///
/// The mapping from Rust's own `target_os`, which is finer: `macos` and `ios`
/// are one Darwin, the four BSDs are one `BSD`, and `android` is a Linux
/// because its kernel is the thing a program branching on this is asking
/// about. Anything not named is `Unknown` rather than its `target_os`, so the
/// set a program compares against stays closed.
const OS: &str = if cfg!(windows) {
    "Windows"
} else if cfg!(any(target_os = "macos", target_os = "ios")) {
    "Darwin"
} else if cfg!(any(target_os = "linux", target_os = "android")) {
    "Linux"
} else if cfg!(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
)) {
    "BSD"
} else if cfg!(any(target_os = "solaris", target_os = "illumos")) {
    "Solaris"
} else {
    "Unknown"
};

/// [`CONSTANTS`]' `VERSION` — this workspace's own, which is what `nvs info`
/// prints and what `nvs_config`'s artifact cache is keyed on.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `Core\Env::get`'s reference card — ADR 0117.
const GET_DOC: MethodDoc = MethodDoc {
    short: "The environment variable `$name`, replacing `getenv` and `$_ENV`. The environment is \
            read-only: there is no `putenv`, because a process-global mutation is unsound across \
            cores.",
    params: &[ParamDoc {
        name: "name",
        desc: "The variable's name, exactly as the operator spelled it. A sink, so it may not be \
               `tainted`: letting a request choose which variable to read hands out the whole \
               environment one query at a time.",
        shape: &[],
    }],
    ret: "Its value as a `tainted string` — the environment is outside the program's own text, so \
          every sink still has to be passed through its own launderer — or `null` where nothing \
          set the variable.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The variable is set to bytes that are not valid UTF-8, so its value is not a \
               `string`. `null` is reserved for a variable nothing set, and a lossy repair would \
               hand back text the operator did not write.",
    }],
};

/// `Core\Env::all`'s reference card — ADR 0117.
const ALL_DOC: MethodDoc = MethodDoc {
    short: "Every environment variable, keyed by name — the whole of `$_ENV`, and `getenv` with \
            no argument.",
    params: &[],
    ret: "An `array<string, tainted string>` in name order, so two calls in one run and two runs \
          on two platforms agree. A variable whose name or value is not valid UTF-8 is omitted \
          rather than allowed to fail the sweep; `get` on that name reports it.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_env_get" => (nvs_core_env_get as *const ()).cast(),
        "nvs_core_env_all" => (nvs_core_env_all as *const ()).cast(),
        _ => return None,
    })
}

/// One `string` argument, or the engine fault a wrongly-tagged one is: the
/// signature is checked at compile time, so a bad tag here is a lowering bug
/// and not something a program can provoke.
fn text<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Env::{member} expected {:?} for its name, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Env::get(string $name): ?tainted string` — replacing `getenv` and
    /// `$_ENV[…]`.
    ///
    /// A name the platform cannot hold at all — empty, or carrying `=` or a NUL
    /// — is absence rather than an error, which is the door's own answer and the
    /// same one a lookup of a name nobody set gives. There is no third state to
    /// report: such a variable cannot have been set either.
    ///
    /// A *value* has no unrepresentable spelling and no syntax. An `=`, a `;`,
    /// a pair of quotes or leading padding are bytes an operator put there, and
    /// this member hands back the whole of what the platform holds rather than
    /// the reading a shell would give it — nothing between the two has an
    /// opinion about what a value means. The empty string is a value on those
    /// same terms and not the absence above, so `null` reports exactly one
    /// thing, and `?? ""` is what tells an unset name from one set to nothing.
    fn nvs_core_env_get(_ctx, args: [1]) {
        let name = text(&args[0], "get")?;
        match nvs_runtime::environment::var(name) {
            None => Ok(Value::null()),
            Some(value) => match value.to_str() {
                Some(text) => Ok(Value::str(NvsStr::new(text.as_bytes()))),
                // Unreachable from source in `Core\Command`'s `usage` sense
                // rather than a diagnostic's: no program can put anything in
                // the environment, because there is no `putenv` and nothing in
                // this runtime writes one after start. Reaching it takes an
                // operator who exported a value that is not text — the world
                // saying no, which is why it is a throw and not a fatal.
                None => Err(Fault::thrown(format!(
                    "Core\\Env::get: `{name}` is set to bytes that are not valid UTF-8, so its \
                     value is not a `string`"
                ))),
            },
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Env::all(): array<string, tainted string>` — replacing `$_ENV` and
    /// the no-argument `getenv`.
    ///
    /// Sorted, and skipping what is not text, for the two reasons this module's
    /// own doc gives.
    fn nvs_core_env_all(_ctx, _args: [0]) {
        let sorted: BTreeMap<String, String> = nvs_runtime::environment::vars()
            .into_iter()
            .filter_map(|(name, value)| {
                Some((name.to_str()?.to_owned(), value.to_str()?.to_owned()))
            })
            .collect();
        let mut out = NvsArray::new();
        for (name, value) in sorted {
            out.set(
                NvsStr::new(name.as_bytes()),
                Value::str(NvsStr::new(value.as_bytes())),
            );
        }
        Ok(Value::array(out))
    }
}
