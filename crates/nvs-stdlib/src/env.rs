//! `Core\Env` — `rule:statements/no-host-populated-variables`'s replacement
//! for `$_ENV` and `getenv()`.
//!
//! Four members — `get`, `all`, `mode` and `context` — and three
//! constants, `EOL`, `OS` and `VERSION`, whose one shared decision is on
//! [`CONSTANTS`]. `mode` and `context` read no environment variable; the
//! section on `mode` below says why they sit on this class anyway, and
//! `rule:core-classes/env-context` owns the second.
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
//! There is no way to set a variable, because a process-global mutation is
//! unsound across cores. That is what makes an unsynchronised read safe from a
//! core thread: nothing in this runtime writes the environment after start, so
//! two requests on two cores read the same bytes and neither can tear the
//! other's read.
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
//! run mode is set through `Core\Config` like every other directive, and no
//! environment variable is consulted for it. So the
//! member sits on this class for the reason a program asks the question —
//! "which deployment am I" is the same question as "what is `OS`" — and not
//! because of where the answer is kept. There is deliberately no `NVS_ENV` or
//! `APP_ENV`: a mode that could be selected two ways would be `rule:core-api/shape-rules`'s R6
//! twice over, and the one way is the file the mode's ceiling is also written
//! in (`rule:http-server/the-mode-ceiling-defaults-to-the-startup-mode`), which
//! is what makes a flip checkable at all.
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
    CaseDoc, ClassDoc, Const, CoreClass, CoreConst, CoreEnum, CoreMethod, CoreTy, EnumDoc,
    ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// `Core\Env`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Reads the environment the program runs in: its environment variables, whether it \
            runs in production or development, and facts about the system.",
};

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Env",
    doc: Some(&CARD),
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
        CoreMethod {
            name: "context",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Enum(CONTEXT_NAME),
            symbol: "nvs_core_env_context",
            doc: Some(&CONTEXT_MEMBER_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: CONSTANTS,
};

/// The three constants — `PHP_EOL`, `PHP_OS_FAMILY` and `PHP_VERSION`, each
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
        desc: "The line ending this platform writes: `\\r\\n` on Windows and `\\n` everywhere \
               else. Use it to write text in the platform's own format. `Core\\Str::lines` and \
               `Core\\IO::lines` do not use it, and split on all three line endings.",
    },
    CoreConst {
        name: "OS",
        ty: CoreTy::Enum(PLATFORM_NAME),
        value: Const::EnumCase(PLATFORM_NAME, OS),
        desc: "The operating system family, as a `Core\\Env::Platform` case. Compare it with a \
               case, such as `Core\\Env::OS == Core\\Env::Platform::Windows`.",
    },
    CoreConst {
        name: "VERSION",
        ty: CoreTy::Str,
        value: Const::Str(VERSION),
        desc: "This runtime's version: three numbers separated by dots, the same string \
               `nvs info` prints. To compare versions, split it with `Core\\Str::split` and \
               compare the numbers.",
    },
];

/// [`CONSTANTS`]' `EOL` — `PHP_EOL`'s own value.
const EOL: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// [`CONSTANTS`]' `OS`: the name of this build's [`PLATFORM`] case, one per
/// `PHP_OS_FAMILY` family.
///
/// The mapping from Rust's own `target_os`, which is finer: `macos` and `ios`
/// are one Darwin, the four BSDs are one `BSD`, and `android` is a Linux
/// because its kernel is the thing a program branching on this is asking
/// about. Anything not named is `Unknown` rather than its `target_os`, so the
/// enum stays closed at six cases.
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
    short: "Returns the value of the environment variable `$name`. The environment is read-only, \
            so a program cannot change a variable.",
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
    short: "Returns every environment variable, with the variable names as keys.",
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
    ret: "The mode as a `Core\\Env::Mode` case. `Production` for a host that configured nothing, \
          so a deployment is the restrictive one until an operator has said otherwise.",
    errors: &[],
};

/// `Core\Env::Mode`'s fully-qualified name, written once so the registry row and
/// every message quoting it cannot drift apart.
pub(crate) const MODE_NAME: &str = r"Core\Env::Mode";

/// `rule:config/a-mode-is-five-defaults`'s two run modes, valued by the ceiling's permissiveness order — the
/// same order `nvs_config::mode::rank` measures a flip against, so `Production`
/// is 0 and there is nothing below it.
///
/// Two cases and no third. A `Staging` would have to select a row of the
/// closed mode table that neither of these does; what a real staging host wants is
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

/// `Core\Env::context`'s reference card — `rule:core-api/reference-card`.
const CONTEXT_MEMBER_DOC: MethodDoc = MethodDoc {
    short: "Which kind of run the code is in: an HTTP request, a command-line program, a spawned \
            script, a scheduled script, a queued job, a test or a connection. Use it to choose \
            what a program does in each one.",
    params: &[],
    ret: "The kind of run as a `Core\\Env::Context` case. It is the same value for the whole run \
          and for every task the run starts.",
    errors: &[],
};

/// `Core\Env::Context`'s fully-qualified name, written once so the registry
/// row and every message quoting it cannot drift apart.
pub(crate) const CONTEXT_NAME: &str = r"Core\Env::Context";

/// `rule:core-classes/env-context`'s kinds of run, one case per row of
/// `rule:tooling/echo-always-has-a-sink`'s table. Each value is
/// [`nvs_runtime::RunKind`]'s discriminant for the case of the same name, so
/// [`nvs_core_env_context`] answers with the cast and nothing maps between
/// them; `context_cases_are_run_kinds_in_order` holds the two together.
pub(crate) const CONTEXT: CoreEnum = CoreEnum {
    name: CONTEXT_NAME,
    cases: &[
        ("Request", 0),
        ("Command", 1),
        ("Isolate", 2),
        ("Schedule", 3),
        ("Job", 4),
        ("Test", 5),
        ("Connection", 6),
    ],
    doc: Some(&CONTEXT_DOC),
};

/// `Core\Env::Platform`'s fully-qualified name, written once so the registry
/// row, the `OS` constant and every message quoting it cannot drift apart.
pub(crate) const PLATFORM_NAME: &str = r"Core\Env::Platform";

/// `Core\Env::OS`'s operating system families. The constant is folded to the
/// case's integer where a program names it, so comparing it is an integer
/// compare and a misspelled family does not compile.
pub(crate) const PLATFORM: CoreEnum = CoreEnum {
    name: PLATFORM_NAME,
    cases: &[
        ("Windows", 0),
        ("Darwin", 1),
        ("Linux", 2),
        ("BSD", 3),
        ("Solaris", 4),
        ("Unknown", 5),
    ],
    doc: Some(&PLATFORM_DOC),
};

/// [`PLATFORM`]'s reference card — `rule:core-api/reference-card`.
const PLATFORM_DOC: EnumDoc = EnumDoc {
    short: "The operating system family the program runs on. `Core\\Env::OS` is one of these \
            six cases.",
    cases: &[
        CaseDoc {
            name: "Windows",
            desc: "Microsoft Windows.",
        },
        CaseDoc {
            name: "Darwin",
            desc: "macOS and iOS.",
        },
        CaseDoc {
            name: "Linux",
            desc: "Linux, and Android.",
        },
        CaseDoc {
            name: "BSD",
            desc: "FreeBSD, OpenBSD, NetBSD and DragonFly BSD.",
        },
        CaseDoc {
            name: "Solaris",
            desc: "Solaris and illumos.",
        },
        CaseDoc {
            name: "Unknown",
            desc: "Any other operating system.",
        },
    ],
};

/// [`CONTEXT`]'s reference card — `rule:core-api/reference-card`.
const CONTEXT_DOC: EnumDoc = EnumDoc {
    short: "The kind of run the code is in. `Core\\Env::context()` returns one of these seven \
            cases.",
    cases: &[
        CaseDoc {
            name: "Request",
            desc: "The code answers an HTTP request, under `nvs serve` or inside \
                   `Core\\Test::request`. `echo` writes to the response body.",
        },
        CaseDoc {
            name: "Command",
            desc: "The code runs from the command line: a script started with `nvs run`, or a \
                   `#[Command]` method. `echo` writes to stdout.",
        },
        CaseDoc {
            name: "Isolate",
            desc: "The code runs in a script started with `spawn script`. A spawned script has no \
                   request, even when a request started it.",
        },
        CaseDoc {
            name: "Schedule",
            desc: "The code runs as a `[[schedule]]` entry that the server started at its time.",
        },
        CaseDoc {
            name: "Job",
            desc: "The code runs as a queued job, in a worker.",
        },
        CaseDoc {
            name: "Test",
            desc: "The code runs in a `#[Test]` method under `nvs test`.",
        },
        CaseDoc {
            name: "Connection",
            desc: "The code serves a connection that a request upgraded, a socket or an event \
                   stream. The request that upgraded it has already ended.",
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
        "nvs_core_env_context" => (nvs_core_env_context as *const ()).cast(),
        _ => return None,
    })
}

/// The [`MODE`] ordinal for a mode spelled as `nvs_config::mode` spells one.
///
/// Written out rather than taken from `nvs_config::mode::rank`, on the rule
/// [`crate::cli`]'s `depth_ordinal` states: the two rosters are declared in
/// different crates for different readers, and a rank is the ceiling's
/// *permissiveness order*, which is free to gain a value that is not a case here.
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
    /// `Core\Env::mode(): Core\Env::Mode` — `rule:config/two-modes-and-the-default-is-production`'s run mode, and the one
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
        let ordinal = match ctx.config() {
            Some(config) => mode_ordinal(&config.mode()),
            None => mode_ordinal(nvs_config::mode::PRODUCTION),
        };
        Ok(Value::int(ordinal))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Env::context(): Core\Env::Context` — `rule:core-classes/env-context`'s
    /// kind of run, one read of [`nvs_runtime::Ctx::run_kind`].
    ///
    /// The ordinal is the discriminant, because [`CONTEXT`] lists its cases in
    /// [`nvs_runtime::RunKind`]'s order. It never throws and asks no
    /// capability: like `mode`, it is a fact about the run and not an effect.
    fn nvs_core_env_context(ctx, _args: [0]) {
        Ok(Value::int(i64::from(ctx.run_kind() as u8)))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nvs_runtime::{Ctx, NvsArray, NvsStr, Value, call};

    use super::{mode_ordinal, nvs_core_env_all, nvs_core_env_get, nvs_core_env_mode};

    /// `Core\Env::all`'s answer, in the order the member reported it.
    fn all(ctx: &mut Ctx) -> Vec<(String, String)> {
        let answered = call(nvs_core_env_all, ctx, &[]).expect("`all` refuses nothing");
        assert!(ctx.take_pending().is_none(), "`all` left a refusal behind");
        #[expect(
            unsafe_code,
            reason = "this frame owns the array the member built, and the \
                      handle releases it"
        )]
        let array =
            unsafe { NvsArray::from_raw(answered.array_ptr().expect("`all` answers an array")) };
        array
            .keys()
            .iter()
            .map(|key| {
                let value = array.get(key).expect("a key `all` has just reported");
                let text = String::from_utf8(
                    value
                        .as_str_bytes()
                        .expect("`all` answers one `string` per entry")
                        .to_vec(),
                )
                .expect("`rule:types/bytes` guarantees a `string` is UTF-8");
                (
                    String::from_utf8(key.clone()).expect("`all` keys by a UTF-8 name"),
                    text,
                )
            })
            .collect()
    }

    /// `Core\Env::get`'s answer for `name`, or `None` where it answered `null`.
    fn get(ctx: &mut Ctx, name: &str) -> Option<String> {
        let argument = Value::str(NvsStr::new(name.as_bytes()));
        let answered = call(nvs_core_env_get, ctx, &[argument]).expect("`get` answers");
        assert!(
            ctx.take_pending().is_none(),
            "`get` refused {name:?}, which only a value that is not text may make it do"
        );
        let out = answered.as_str_bytes().map(|bytes| {
            String::from_utf8(bytes.to_vec())
                .expect("`rule:types/bytes` guarantees a `string` is UTF-8")
        });
        #[expect(
            unsafe_code,
            reason = "this frame owns the argument it built and the answer the \
                      member built, and the member borrowed rather than consumed \
                      the first"
        )]
        unsafe {
            answered.release();
            argument.release();
        }
        out
    }

    /// `all` is the environment's text half in name order, and nothing else:
    /// every name the platform reports with a UTF-8 name and value is in it,
    /// every entry it reports is the platform's own value for that name, and
    /// each name sorts after the one before it.
    ///
    /// The `.nvst` cases can only pin the few variables they set themselves,
    /// because the rest belong to the machine. This side asks about the whole
    /// environment the test binary runs in, so a member that dropped, reordered
    /// or rewrote an entry nobody named still fails here.
    // covers: Core\Env::all
    #[test]
    fn all_is_the_whole_text_environment_in_name_order() {
        let mut ctx = Ctx::buffered();
        let reported = all(&mut ctx);

        for pair in reported.windows(2) {
            assert!(
                pair[0].0.as_bytes() < pair[1].0.as_bytes(),
                "{:?} is reported before {:?}",
                pair[0].0,
                pair[1].0
            );
        }
        for (name, value) in &reported {
            let platform = nvs_runtime::environment::var(name)
                .unwrap_or_else(|| panic!("`all` reported {name:?}, which the platform does not"));
            assert_eq!(platform.to_str(), Some(value.as_str()), "{name:?}");
        }
        for (name, value) in nvs_runtime::environment::vars() {
            if let (Some(name), Some(_)) = (name.to_str(), value.to_str()) {
                assert!(
                    reported.iter().any(|(reported, _)| reported == name),
                    "`all` left out {name:?}, whose name and value are both text"
                );
            }
        }

        assert_eq!(
            all(&mut ctx),
            reported,
            "two calls in one run answer differently"
        );
    }

    /// `get` agrees with `all` on every name `all` reports, and answers `null`
    /// without refusing for the two kinds of name nothing can have set: one the
    /// platform cannot hold at all, and one nobody exported.
    ///
    /// The three unholdable names are the door's own answer and not a throw,
    /// because a throw is reserved for a value that is not text.
    // covers: Core\Env::get
    #[test]
    fn get_agrees_with_all_and_answers_null_for_a_name_nothing_set() {
        let mut ctx = Ctx::buffered();
        let reported = all(&mut ctx);
        let mut agreed = 0_usize;
        for (name, value) in &reported {
            assert_eq!(
                get(&mut ctx, name).as_deref(),
                Some(value.as_str()),
                "{name:?}"
            );
            agreed += 1;
        }
        assert_eq!(agreed, reported.len());

        for name in ["", "NVS=SPLIT", "NVS\0NUL", "NVS_ENV_A_NAME_NOBODY_EXPORTS"] {
            assert_eq!(get(&mut ctx, name), None, "{name:?}");
        }
    }

    /// `mode` answers `Production` until a configuration says otherwise, and
    /// reads the configuration and never a variable.
    ///
    /// A context nobody configured, a configuration that names no mode and one
    /// that misspells it all answer `Production`, the restrictive end. Only a
    /// written `development` answers `Development`.
    // covers: Core\Env::mode
    #[test]
    fn mode_is_production_until_the_configuration_says_development() {
        let asked = |ctx: &mut Ctx| {
            let answered = call(nvs_core_env_mode, ctx, &[]).expect("`mode` refuses nothing");
            assert!(ctx.take_pending().is_none(), "`mode` left a refusal behind");
            answered.as_int().expect("an enum answers as its ordinal")
        };
        let configured = |written: &str| {
            let table: toml::Table = written.parse().expect("the fixture is valid TOML");
            let config = toml::Value::Table(table.clone())
                .try_into()
                .expect("the fixture deserializes into the tree it is written for");
            let mut ctx = Ctx::buffered();
            ctx.set_config(Arc::new(nvs_config::Snapshot {
                config,
                table,
                ..nvs_config::Snapshot::default()
            }));
            ctx
        };

        assert_eq!(
            asked(&mut Ctx::buffered()),
            0,
            "nobody configured this context"
        );
        assert_eq!(asked(&mut configured("")), 0, "the file names no mode");
        assert_eq!(
            asked(&mut configured("[mode]\ndefault = \"development\"\n")),
            1
        );
        assert_eq!(
            asked(&mut configured("[mode]\ndefault = \"production\"\n")),
            0
        );
        assert_eq!(
            mode_ordinal("staging"),
            0,
            "a misspelled mode loosens nothing"
        );
    }

    /// [`super::CONTEXT`]'s values are `RunKind`'s discriminants, so the
    /// member's cast and the enum a program compares against name one case.
    // covers: Core\Env::Context
    #[test]
    fn context_cases_are_run_kinds_in_order() {
        use nvs_runtime::RunKind;
        let kinds = [
            ("Request", RunKind::Request),
            ("Command", RunKind::Command),
            ("Isolate", RunKind::Isolate),
            ("Schedule", RunKind::Schedule),
            ("Job", RunKind::Job),
            ("Test", RunKind::Test),
            ("Connection", RunKind::Connection),
        ];
        let cases: Vec<(&str, i64)> = super::CONTEXT.cases.to_vec();
        let expected: Vec<(&str, i64)> = kinds
            .iter()
            .map(|(name, kind)| (*name, i64::from(*kind as u8)))
            .collect();
        assert_eq!(cases, expected);
    }

    /// `Core\Env::OS` is the `Core\Env::Platform` case of the platform this
    /// test was built for, written out per `cfg` so a wrong mapping fails on
    /// the platform it is wrong for.
    // covers: Core\Env::Platform
    #[test]
    fn the_os_constant_is_the_platform_case_of_this_build() {
        use crate::registry::{Const, CoreTy};
        let expected = if cfg!(windows) {
            "Windows"
        } else if cfg!(target_os = "macos") {
            "Darwin"
        } else if cfg!(target_os = "linux") {
            "Linux"
        } else {
            return;
        };
        let os = super::CONSTANTS
            .iter()
            .find(|found| found.name == "OS")
            .expect("Core\\Env registers OS");
        assert!(matches!(os.ty, CoreTy::Enum(super::PLATFORM_NAME)));
        assert!(
            matches!(os.value, Const::EnumCase(super::PLATFORM_NAME, case) if case == expected),
            "OS is {:?}, want {expected}",
            os.value
        );
    }

    /// A context nobody built for anything else is a command.
    // covers: Core\Env::context
    #[test]
    fn context_answers_command_for_a_new_context() {
        let answered =
            call(super::nvs_core_env_context, &mut Ctx::buffered(), &[]).expect("never throws");
        assert_eq!(answered.as_int(), Some(1));
    }
}
