//! `Core\Env` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 15's environment half, which is
//! `rule:statements/no-host-populated-variables`'s replacement
//! for `$_ENV` and `getenv()`.
//!
//! All three members — `get`, `all` and `mode` — and all three of § 15's
//! constants, `EOL`, `OS` and `VERSION`, whose one shared decision is on
//! [`CONSTANTS`]. The class is complete.
//!
//! # It is not a capability door, and that is decided rather than skipped
//!
//! `rule:security/capability-check-at-the-door`
//! puts a check at the door to an effect, and reading the environment is
//! not one: `rule:security/request-state-throws-in-an-isolate` says outright that `Core\Env` and `Core\Cli` are
//! "process-wide facts already governed by the existing capability/config-overlay
//! machinery", not per-request secrets to wall off. The operator who launched
//! the process chose its environment in the same breath as its `nvs.toml`, so a
//! grant here would gate one half of a decision the other half is already
//! trusted with. `nvs_runtime::terminal`'s module doc records the same
//! conclusion from the other side, for the four variables `Core\Cli` reads and
//! never hands back.
//!
//! What does the protecting is `rule:security/tainted-qualifier`
//! 's qualifier: every value here is `tainted`, because the environment is
//! outside the program's own text, so a variable holding a URL still has to
//! reach `Core\Http::allowUrl` and one holding a table name still has to reach
//! `Core\Db::quoteIdentifier`.
//!
//! The **name** is a [`Qual::Sink`] and so refuses a `tainted` argument, on
//! `rule:security/sink-predicate`'s predicate: its content is an instruction naming what the
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
//! one here — `rule:security/capability-check-at-the-door` puts every effect behind a door in `nvs_runtime`,
//! and that module's doc owns why this particular door asks nothing.
//!
//! # A value that is not text throws, and `all` skips it instead
//!
//! `rule:types/bytes` makes a `string`
//! UTF-8, and an environment variable is bytes on every platform this runs on,
//! so the two do not always meet. The split:
//!
//! * `get` **throws**. The caller named one variable, so the honest answer to
//!   "what is `X`" is not `null` — that would report an unreadable value as an
//!   absent one, and `rule:core-api/shape-rules`'s
//!   `?T` means absence and nothing else. Repairing it lossily is
//!   `rule:errors/ambiguous-input-refused`'s
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
//! # `mode` is the one member here that reads no environment at all
//!
//! `rule:config/two-modes-and-the-default-is-production`'s
//! run mode is set through `Core\Config` like every other directive, and spec
//! § 15 says outright that no environment variable is consulted for it. So the
//! member sits on this class for the reason a program asks the question —
//! "which deployment am I" is the same question as "what is `OS`" — and not
//! because of where the answer is kept. There is deliberately no `NVS_ENV` or
//! `APP_ENV`: a mode that could be selected two ways would be `rule:core-api/shape-rules`'s R6
//! twice over, and the one way is the file the ceiling in § 5 is also written
//! in, which is what makes a flip checkable at all.
//!
//! Where the value comes from is `nvs_config::Request::mode`, whose own doc
//! owns the order — this module reads it and turns it into an ordinal, and
//! knows nothing else about run modes. What is settled *here* is the two edges
//! that reader does not have an opinion about: a context nobody configured, and
//! a name that is neither mode. Both are `Production`, on [`mode_ordinal`]'s
//! reasoning.
//!

use std::collections::BTreeMap;

use nvs_runtime::{Fault, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    CaseDoc, Const, CoreClass, CoreConst, CoreEnum, CoreMethod, CoreTy, EnumDoc, ErrorDoc,
    MethodDoc, ParamDoc, Qual,
};

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Env",
    doc: None,
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
        CoreMethod {
            name: "mode",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(MODE_NAME),
            symbol: "nvs_core_env_mode",
            doc: Some(&MODE_MEMBER_DOC),
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
/// `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s
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

/// `Core\Env::get`'s reference card — `rule:core-api/reference-card`.
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

/// `Core\Env::all`'s reference card — `rule:core-api/reference-card`.
const ALL_DOC: MethodDoc = MethodDoc {
    short: "Every environment variable, keyed by name — the whole of `$_ENV`, and `getenv` with \
            no argument.",
    params: &[],
    ret: "An `array<string, tainted string>` in name order, so two calls in one run and two runs \
          on two platforms agree. A variable whose name or value is not valid UTF-8 is omitted \
          rather than allowed to fail the sweep; `get` on that name reports it.",
    errors: &[],
};

/// `Core\Env::mode`'s reference card — `rule:core-api/reference-card`.
const MODE_MEMBER_DOC: MethodDoc = MethodDoc {
    short: "Which deployment this program is running in. The mode is written in `nvs.toml` and \
            read back through `Core\\Config` like every other directive: **no environment \
            variable is consulted for it**, so there is no `APP_ENV` convention to get wrong and \
            no way for a request to select one.",
    params: &[],
    ret: "The mode as a `Core\\Env\\Mode` case. `Production` for a host that configured nothing, \
          so a deployment is the restrictive one until an operator has said otherwise.",
    errors: &[],
};

/// `Core\Env\Mode`'s fully-qualified name, written once so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const MODE_NAME: &str = r"Core\Env\Mode";

/// `rule:config/a-mode-is-five-defaults`'s two run modes, valued by § 5's permissiveness order — the
/// same order `nvs_config::mode::rank` measures a flip against, so `Production`
/// is 0 and there is nothing below it.
///
/// Two cases and no third. A `Staging` would have to select a row of § 3's
/// closed table that neither of these does; what a real staging host wants is
/// `Production` with the two or three directives it differs on written out,
/// which is exactly what leaving them individually settable is for.
pub(crate) const MODE: CoreEnum = CoreEnum {
    name: MODE_NAME,
    cases: &[("Production", 0), ("Development", 1)],
    doc: Some(&MODE_DOC),
};

/// [`MODE`]'s reference card — `rule:core-api/reference-card`.
const MODE_DOC: EnumDoc = EnumDoc {
    short: "Which deployment a program is running in — two modes, and there is no third. A mode \
            is a shorthand for the defaults of five directives, each of which stays settable on \
            its own.",
    cases: &[
        CaseDoc {
            name: "Production",
            desc: "The restrictive end of the two, and what a host that wrote no configuration is \
                   in — a deployment is the safe one before anyone has said so. Which defaults it \
                   selects is fixed by the mode, and is not a table a program should re-derive.",
        },
        CaseDoc {
            name: "Development",
            desc: "The permissive end, selected in `nvs.toml` and never inferred from a variable, \
                   a hostname or a build. A request may flip into it with `Core\\Config::set` only \
                   where `[mode] ceiling` reaches this far.",
        },
    ],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_env_get" => (nvs_core_env_get as *const ()).cast(),
        "nvs_core_env_all" => (nvs_core_env_all as *const ()).cast(),
        "nvs_core_env_mode" => (nvs_core_env_mode as *const ()).cast(),
        _ => return None,
    })
}

/// The [`MODE`] ordinal for a mode spelled as `nvs_config::mode` spells one.
///
/// Written out rather than taken from `nvs_config::mode::rank`, on the rule
/// [`crate::cli`]'s `depth_ordinal` states: the two rosters are declared in
/// different crates for different readers, and a rank is § 5's *permissiveness
/// order*, which is free to gain a value that is not a case here.
///
/// A name that is neither mode answers `Production`, the restrictive end. It is
/// reachable only from a `mode.default` an operator misspelled in a root-owned
/// file — `Core\Config::set` refuses one, because `nvs_config::mode::within`
/// ranks no third name — and every flip is already refused in that state, so
/// answering `Development` would be the one place a typo loosened something.
/// This member reports rather than admitting anything, and a throw would turn
/// one bad line into a failure on every request that asks a question.
fn mode_ordinal(mode: &str) -> i64 {
    match mode {
        nvs_config::mode::DEVELOPMENT => 1,
        // `PRODUCTION`, and every name that is neither — see above.
        _ => 0,
    }
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

nvs_runtime::nvs_helper! {
    /// `Core\Env::mode(): Core\Env\Mode` — `rule:config/two-modes-and-the-default-is-production`'s run mode, and the one
    /// member of this class that reads no environment variable.
    ///
    /// An enum answers as its ordinal, exactly as a user-declared enum does
    /// (`rule:enums/closed-integer-type`).
    ///
    /// The four-step order behind the answer — a flip this request made, the
    /// application's own mode, the global `mode.default`, then `production` —
    /// is `nvs_config::Request::mode`'s and is stated there. A context nobody
    /// configured is that last row reached one step earlier: an embedder with
    /// no snapshot is a host that wrote nothing, which `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 3
    /// makes a valid host rather than an error, so this is `Production` and not
    /// a throw.
    fn nvs_core_env_mode(ctx, _args: [0]) {
        let mode = match ctx.config() {
            Some(config) => config.mode(),
            None => nvs_config::mode::PRODUCTION.to_string(),
        };
        Ok(Value::int(mode_ordinal(&mode)))
    }
}
