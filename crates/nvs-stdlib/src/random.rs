//! `Core\Random` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 11's first table, which is a CSPRNG **always**.
//!
//! That section collapses PHP's `rand`, `mt_rand`, `random_int`, `lcg_value`,
//! `shuffle`, `str_shuffle` and `array_rand` into one class and keeps none of
//! the insecure generators under any name. So there is no fast-but-predictable
//! tier here to fall back to and none to add later: every member draws from a
//! cryptographic generator, and the only escape from that is [`SEEDED`], a
//! *different type* whose guarantee is reproducibility.
//!
//! # The generator, and why `rand`
//!
//! `rule:packaging/a-c-dependency-answers-two-questions` asks
//! two questions of a dependency, and this one answers the first: no
//! attacker-controlled data reaches it at all — a member here takes a count or
//! a pair of bounds and returns bytes — so it is accepted under ordinary
//! audit. It is pure Rust with no build script and no C, which is the wider
//! default it also meets.
//!
//! Every member outside a test runs on `rand::rng()`, the thread-local
//! `ThreadRng`: ChaCha12, seeded from the operating system's own generator and
//! reseeded from it every 64 KiB of output. (Inside one, a declared seed
//! selects the generator the last section describes, and nothing else can.)
//! Two properties are why this rather than reading the OS generator directly at
//! each call:
//!
//! * **It is a userspace generator.** `Core\Random::float()` in a loop is a
//!   ChaCha block every 64 draws rather than a syscall every one, which is
//!   AGENTS.md's priority 3 and the reason a *secure* generator can be the
//!   only generator without a program paying for the choice.
//! * **Bounded sampling and shuffling are already written and analysed there.**
//!   Drawing an integer in `[$min, $max]` without bias is the part of this
//!   class that is easy to get subtly wrong, and `rand`'s `unbiased` feature —
//!   enabled in the workspace's own dependency line — makes it exact rather
//!   than merely within 1-in-2^64 of exact. Hand-writing it beside a CSPRNG
//!   would be the one piece of security-relevant arithmetic in `Core` with no
//!   second reader.
//!
//! `getrandom` alone was the alternative: fewer crates, but a syscall per draw
//! *and* the rejection-sampling and Fisher-Yates code moved in here, which
//! trades priority 3 and priority 2 to buy priority 4. `rule:packaging/a-c-dependency-answers-two-questions` does not
//! ask a question that distinguishes them, so AGENTS.md's ordering does.
//!
//! # What it spends
//!
//! One `ThreadRng` per **thread**: ~136 bytes of ChaCha state plus its 256-byte
//! output block, allocated on the first draw a thread makes and never freed.
//! That is O(threads), not O(requests served) — nothing here is retained across
//! a call, so AGENTS.md's "attributable to a request and O(in-flight)" rule is
//! satisfied trivially: the generator belongs to the worker, not to the work.
//!
//! A [`SEEDED`] instance spends one object allocation of its own: a header and
//! a single 16-byte slot, which is the whole of that generator — SplitMix64's
//! state is one word, so a reproducible generator costs what any other
//! one-field object costs. It is charged to the request that wrote the `new`
//! and released with the value, so the cost is O(in-flight) by construction and
//! a program that writes none pays nothing.
//!
//! # Nothing forks, so no child inherits this state
//!
//! A `fork(2)` without an exec would leave the child drawing the parent's
//! stream, and nothing here reseeds. Novis does not fork:
//! `rule:core-classes/process-is-argv-only` makes a path and an argument array
//! the one way to run another program, so every child is an exec that replaces
//! this generator along with the rest of the image, and
//! `rule:packaging/a-service-is-one-stored-argv`'s `run` is the service
//! manager's own entry point into a fresh process — M7's plan writes a
//! `Type=notify` unit, which is the systemd type that does not daemonize.
//! Should a pre-fork model ever land, it owes `ThreadRng::reseed` in the child.
//!
//! # Reproducibility is a declared seed or a separate type, never this class
//!
//! A `#[Test(seed: …)]` isolate draws from [`SplitMix`] instead, so the test's
//! sequence reproduces
//! (`rule:testing/determinism-declared-on-the-test`). Every member reaches its generator through [`draw`], and that is what
//! makes the seed all-or-nothing: one that fixed `int` but not `shuffle` would
//! make a test's reproducibility depend on which members it happened to call.
//!
//! The selection is a field on `nvs_runtime::Ctx` that only the test runner
//! writes, so there is no spelling outside a `#[Test]` that reaches it — see
//! [`nvs_runtime::Ctx::random_state`], which owns that argument.
//!
//! [`SEEDED`] is the other way, and the one a program outside a test can write:
//! `new Core\Random\Seeded(42)` holds a [`SplitMix`] state of its own in a slot
//! and draws every member from it, so a seed fixes that one value's sequence
//! and nothing else's. The separation is a **type** rather than a mode this
//! class can be put into, which is spec § 11's own argument: a generator that
//! reproduces says so at every call site that takes one, and cannot be reached
//! for where an unpredictable one was meant. `srand` and `mt_srand` have no
//! equivalent, because seeding the global generator is the spelling this
//! separation removes.

use rand::seq::SliceRandom;
use rand::{Rng, RngExt};

use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, Tag, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc,
};

// ============================================================================
// Registration — this class's rows, and where its symbols live
// ============================================================================

/// The class's fully-qualified name, as the registry holds it and as every
/// message naming one of its members spells it.
pub(crate) const NAME: &str = r"Core\Random";

/// [`SEEDED`]'s name, the same way, and as [`CoreTy::Instance`] spells it.
pub(crate) const SEEDED_NAME: &str = r"Core\Random\Seeded";

/// `Core\Random`'s own card — `rule:core-api/reference-card`.
const RANDOM_CARD: ClassDoc = ClassDoc {
    short: "Random numbers, bytes, tokens and choices from a cryptographic generator, so a \
            value is safe to use for a key, a code or a password reset link. For a sequence \
            that repeats from a seed, use `Core\\Random\\Seeded`.",
};

/// `Core\Random`'s registry rows, in the spec's own order — all seven of
/// § 11's first table.
pub const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&RANDOM_CARD),
    methods: &[
        CoreMethod {
            name: "int",
            names: &["min", "max"],
            params: &[CoreTy::Int, CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_random_int",
            doc: Some(&INT_DOC),
        },
        CoreMethod {
            name: "float",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_random_float",
            doc: Some(&FLOAT_DOC),
        },
        CoreMethod {
            name: "bytes",
            names: &["count"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_random_bytes",
            doc: Some(&BYTES_DOC),
        },
        CoreMethod {
            name: "token",
            names: &["bytes"],
            params: &[CoreTy::Uint],
            defaults: &[Const::Uint(DEFAULT_TOKEN_BYTES)],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_random_token",
            doc: Some(&TOKEN_DOC),
        },
        CoreMethod {
            name: "pick",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "nvs_core_random_pick",
            doc: Some(&PICK_DOC),
        },
        CoreMethod {
            name: "sample",
            names: &["a", "count"],
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_random_sample",
            doc: Some(&SAMPLE_DOC),
        },
        CoreMethod {
            name: "shuffle",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_random_shuffle",
            doc: Some(&SHUFFLE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Random::int`'s reference card — `rule:core-api/reference-card`.
const INT_DOC: MethodDoc = MethodDoc {
    short: "Draws an integer uniformly from `[$min, $max]`, inclusive at both ends, from the \
            CSPRNG — as `random_int` does, replacing `rand` and `mt_rand` as well.",
    params: &[
        ParamDoc {
            name: "min",
            desc: "The lowest value the draw may answer.",
            shape: &[],
        },
        ParamDoc {
            name: "max",
            desc: "The highest value the draw may answer.",
            shape: &[],
        },
    ],
    ret: "An `int` in the range, without bias; `$min` itself when the two bounds are equal.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$min` is above `$max` — the range is empty, and the bounds are not swapped.",
    }],
};

/// `Core\Random::float`'s reference card — `rule:core-api/reference-card`.
const FLOAT_DOC: MethodDoc = MethodDoc {
    short: "Draws a float uniformly from the half-open interval `[0, 1)`, replacing `lcg_value` \
            and the `mt_rand() / mt_getrandmax()` idiom.",
    params: &[],
    ret: "A `float` with 53 random bits; `0.0` is drawable and `1.0` is not.",
    errors: &[],
};

/// `Core\Random::bytes`'s reference card — `rule:core-api/reference-card`.
const BYTES_DOC: MethodDoc = MethodDoc {
    short: "Draws `$count` bytes from the CSPRNG as a raw buffer — a key, a nonce or an IV — as \
            `random_bytes` does, replacing `openssl_random_pseudo_bytes` as well.",
    params: &[ParamDoc {
        name: "count",
        desc: "How many bytes to draw; at least one.",
        shape: &[],
    }],
    ret: "A `bytes` value of exactly `$count` octets, unrendered — `Core\\Random::token` is \
          the hex spelling.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$count` is `0`, as `random_bytes(0)` is a `ValueError`, or is larger than a \
               buffer this process can allocate.",
    }],
};

/// `Core\Random::token`'s reference card — `rule:core-api/reference-card`.
const TOKEN_DOC: MethodDoc = MethodDoc {
    short: "Draws `$bytes` bytes from the CSPRNG and renders them as lower-case hex — the \
            `bin2hex(random_bytes(…))` idiom, for a session identifier or a reset link.",
    params: &[ParamDoc {
        name: "bytes",
        desc: "How many bytes of entropy to draw, `32` by default — the answer is twice as many \
               characters.",
        shape: &[],
    }],
    ret: "A `string` of `2 * $bytes` hex digits, `0`–`9` and `a`–`f`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$bytes` is `0` — the empty string is a token every other empty token equals — \
               or the draw or its rendering is larger than a buffer this process can allocate.",
    }],
};

/// `Core\Random::pick`'s reference card — `rule:core-api/reference-card`.
const PICK_DOC: MethodDoc = MethodDoc {
    short: "Draws one entry of `$a` uniformly and answers its value, replacing `array_rand` in \
            its one-element spelling — the value, where `array_rand` answers the key.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to draw from.",
        shape: &[],
    }],
    ret: "One entry's value; `null` when `$a` is empty, which over an `array<?T>` is \
          indistinguishable from drawing a `null` entry.",
    errors: &[],
};

/// `Core\Random::sample`'s reference card — `rule:core-api/reference-card`.
const SAMPLE_DOC: MethodDoc = MethodDoc {
    short: "Draws `$count` distinct entries of `$a` uniformly, replacing `array_rand` with a \
            count — in random order, where `array_rand` keeps the subject's order.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to draw from.",
            shape: &[],
        },
        ParamDoc {
            name: "count",
            desc: "How many distinct entries to draw; at most the array's size.",
            shape: &[],
        },
    ],
    ret: "A fresh list of the drawn values under `0, 1, …` keys, in random order; the \
          subject's keys are discarded, and `$a` is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$count` is above the number of entries `$a` holds.",
    }],
};

/// `Core\Random::shuffle`'s reference card — `rule:core-api/reference-card`.
const SHUFFLE_DOC: MethodDoc = MethodDoc {
    short: "Answers every entry of `$a` in a uniformly random order, replacing `shuffle` and \
            `str_shuffle` — a fresh array rather than a reordering in place.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to reorder.",
        shape: &[],
    }],
    ret: "A fresh list of all the values under `0, 1, …` keys — the subject's keys are \
          discarded, as `shuffle` renumbers — and `$a` is unchanged.",
    errors: &[],
};

/// `Core\Random::token`'s default draw, which spec § 11 writes as
/// `token(uint $bytes = 32)`.
///
/// Stated once and used in the registry row so the signature the compiler
/// resolves and the number the helper documents cannot drift apart.
const DEFAULT_TOKEN_BYTES: u64 = 32;

/// The linker symbol `new Core\Random\Seeded(…)` lowers to — see
/// [`crate::registry::CONSTRUCTORS`], which is the roster `nvs-ir` reads.
pub(crate) const SEEDED_NEW_SYMBOL: &str = "nvs_core_random_seeded_new";

/// `new Core\Random\Seeded(int $seed)` — the constructor
/// [`crate::registry::CONSTRUCTORS`] registers.
///
/// A positional parameter rather than an options bag: spec § 11 writes
/// "constructed from an explicit seed", and a generator whose whole state is
/// that seed has nothing else to be told. There is no default, because a
/// *seeded* generator nobody gave a seed is the unpredictable one this class
/// exists to be distinguishable from.
pub(crate) const SEEDED_NEW: CoreMethod = CoreMethod {
    name: "constructor",
    names: &["seed"],
    params: &[CoreTy::Int],
    defaults: &[],
    return_ty: CoreTy::Instance(SEEDED_NAME),
    symbol: SEEDED_NEW_SYMBOL,
    doc: Some(&SEEDED_CONSTRUCTOR_DOC),
};

/// `Core\Random\Seeded`'s registry rows — [`CLASS`]'s seven members, in the
/// same order, over a sequence an explicit seed fixes.
///
/// **Every one of them is an instance member**, and the only static entry point
/// is the constructor, which is not a member at all
/// ([`crate::registry::CONSTRUCTORS`]). That is what makes the separation a
/// type: there is no spelling that draws a reproducible value without holding
/// one of these, and a parameter declaring it says which of the two generators
/// it will accept.
///
/// One slot, holding the generator's state as an `int`, because [`SplitMix`]'s
/// state is one word — the module docs' reason for writing that generator here
/// is also the reason this class is an ordinary object rather than a handle
/// into a table of native ones.
pub(crate) const SEEDED: CoreClass = CoreClass {
    name: SEEDED_NAME,
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "int",
            names: &["min", "max"],
            params: &[CoreTy::Int, CoreTy::Int],
            defaults: &[],
            return_ty: CoreTy::Int,
            symbol: "nvs_core_random_seeded_int",
            doc: Some(&SEEDED_INT_DOC),
        },
        CoreMethod {
            name: "float",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Float,
            symbol: "nvs_core_random_seeded_float",
            doc: Some(&SEEDED_FLOAT_DOC),
        },
        CoreMethod {
            name: "bytes",
            names: &["count"],
            params: &[CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_random_seeded_bytes",
            doc: Some(&SEEDED_BYTES_DOC),
        },
        CoreMethod {
            name: "token",
            names: &["bytes"],
            params: &[CoreTy::Uint],
            defaults: &[Const::Uint(DEFAULT_TOKEN_BYTES)],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_random_seeded_token",
            doc: Some(&SEEDED_TOKEN_DOC),
        },
        CoreMethod {
            name: "pick",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "nvs_core_random_seeded_pick",
            doc: Some(&SEEDED_PICK_DOC),
        },
        CoreMethod {
            name: "sample",
            names: &["a", "count"],
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Uint],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_random_seeded_sample",
            doc: Some(&SEEDED_SAMPLE_DOC),
        },
        CoreMethod {
            name: "shuffle",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_random_seeded_shuffle",
            doc: Some(&SEEDED_SHUFFLE_DOC),
        },
    ],
    slots: &["state"],
    constants: &[],
};

/// [`SEEDED`]'s state slot, by index — the layout its `slots` names.
const STATE_SLOT: usize = 0;

/// `new Core\Random\Seeded`'s reference card — `rule:core-api/reference-card`.
const SEEDED_CONSTRUCTOR_DOC: MethodDoc = MethodDoc {
    short: "Builds a generator whose sequence `$seed` fixes — the reproducible half of spec § 11, \
            replacing `srand` and `mt_srand` with a value rather than a mode.",
    params: &[ParamDoc {
        name: "seed",
        desc: "The seed the sequence is drawn from; every `int` is one, including `0`.",
        shape: &[],
    }],
    ret: "A generator drawing the same sequence for the same seed, and nothing about it is \
          unpredictable.",
    errors: &[],
};

/// `Core\Random\Seeded::int`'s reference card — `rule:core-api/reference-card`.
const SEEDED_INT_DOC: MethodDoc = MethodDoc {
    short: "Draws an integer uniformly from `[$min, $max]`, inclusive at both ends, from this \
            generator's own sequence — `Core\\Random::int`'s answer, reproducibly.",
    params: &[
        ParamDoc {
            name: "min",
            desc: "The lowest value the draw may answer.",
            shape: &[],
        },
        ParamDoc {
            name: "max",
            desc: "The highest value the draw may answer.",
            shape: &[],
        },
    ],
    ret: "An `int` in the range, without bias; `$min` itself when the two bounds are equal.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$min` is above `$max` — the range is empty, and the bounds are not swapped.",
    }],
};

/// `Core\Random\Seeded::float`'s reference card — `rule:core-api/reference-card`.
const SEEDED_FLOAT_DOC: MethodDoc = MethodDoc {
    short: "Draws a float uniformly from the half-open interval `[0, 1)` from this generator's own \
            sequence — `Core\\Random::float`'s answer, reproducibly.",
    params: &[],
    ret: "A `float` with 53 random bits; `0.0` is drawable and `1.0` is not.",
    errors: &[],
};

/// `Core\Random\Seeded::bytes`'s reference card — `rule:core-api/reference-card`.
const SEEDED_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Draws `$count` bytes from this generator's own sequence as a raw buffer — a fixture, \
            never a key, since a reproducible buffer is a secret anyone holding the seed has.",
    params: &[ParamDoc {
        name: "count",
        desc: "How many bytes to draw; at least one.",
        shape: &[],
    }],
    ret: "A `bytes` value of exactly `$count` octets, unrendered.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$count` is `0`, as `Core\\Random::bytes`'s own draw of nothing is, or is larger \
               than a buffer this process can allocate.",
    }],
};

/// `Core\Random\Seeded::token`'s reference card — `rule:core-api/reference-card`.
const SEEDED_TOKEN_DOC: MethodDoc = MethodDoc {
    short: "Draws `$bytes` bytes from this generator's own sequence and renders them as lower-case \
            hex — a stable identifier for a fixture, never a session token.",
    params: &[ParamDoc {
        name: "bytes",
        desc: "How many bytes of entropy to draw, `32` by default — the answer is twice as many \
               characters.",
        shape: &[],
    }],
    ret: "A `string` of `2 * $bytes` hex digits, `0`–`9` and `a`–`f`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$bytes` is `0`, or the draw or its rendering is larger than a buffer this process \
               can allocate.",
    }],
};

/// `Core\Random\Seeded::pick`'s reference card — `rule:core-api/reference-card`.
const SEEDED_PICK_DOC: MethodDoc = MethodDoc {
    short: "Draws one entry of `$a` uniformly from this generator's own sequence and answers its \
            value — `Core\\Random::pick`'s answer, reproducibly.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to draw from.",
        shape: &[],
    }],
    ret: "One entry's value; `null` when `$a` is empty, which over an `array<?T>` is \
          indistinguishable from drawing a `null` entry.",
    errors: &[],
};

/// `Core\Random\Seeded::sample`'s reference card — `rule:core-api/reference-card`.
const SEEDED_SAMPLE_DOC: MethodDoc = MethodDoc {
    short: "Draws `$count` distinct entries of `$a` uniformly from this generator's own sequence — \
            `Core\\Random::sample`'s answer, reproducibly.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to draw from.",
            shape: &[],
        },
        ParamDoc {
            name: "count",
            desc: "How many distinct entries to draw; at most the array's size.",
            shape: &[],
        },
    ],
    ret: "A fresh list of the drawn values under `0, 1, …` keys, in drawn order; the subject's \
          keys are discarded, and `$a` is unchanged.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$count` is above the number of entries `$a` holds.",
    }],
};

/// `Core\Random\Seeded::shuffle`'s reference card — `rule:core-api/reference-card`.
const SEEDED_SHUFFLE_DOC: MethodDoc = MethodDoc {
    short: "Answers every entry of `$a` in an order drawn from this generator's own sequence — \
            `Core\\Random::shuffle`'s answer, reproducibly.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to reorder.",
        shape: &[],
    }],
    ret: "A fresh list of all the values under `0, 1, …` keys — the subject's keys are discarded \
          — and `$a` is unchanged.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_random_int" => (nvs_core_random_int as *const ()).cast(),
        "nvs_core_random_float" => (nvs_core_random_float as *const ()).cast(),
        "nvs_core_random_bytes" => (nvs_core_random_bytes as *const ()).cast(),
        "nvs_core_random_token" => (nvs_core_random_token as *const ()).cast(),
        "nvs_core_random_pick" => (nvs_core_random_pick as *const ()).cast(),
        "nvs_core_random_sample" => (nvs_core_random_sample as *const ()).cast(),
        "nvs_core_random_shuffle" => (nvs_core_random_shuffle as *const ()).cast(),
        SEEDED_NEW_SYMBOL => (nvs_core_random_seeded_new as *const ()).cast(),
        "nvs_core_random_seeded_int" => (nvs_core_random_seeded_int as *const ()).cast(),
        "nvs_core_random_seeded_float" => (nvs_core_random_seeded_float as *const ()).cast(),
        "nvs_core_random_seeded_bytes" => (nvs_core_random_seeded_bytes as *const ()).cast(),
        "nvs_core_random_seeded_token" => (nvs_core_random_seeded_token as *const ()).cast(),
        "nvs_core_random_seeded_pick" => (nvs_core_random_seeded_pick as *const ()).cast(),
        "nvs_core_random_seeded_sample" => (nvs_core_random_seeded_sample as *const ()).cast(),
        "nvs_core_random_seeded_shuffle" => (nvs_core_random_seeded_shuffle as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// Argument decoding — the same shape as `crate::path`'s, naming this class
// ============================================================================

/// One `int` argument, reported against whichever of the two classes is asking.
fn integer(class: &str, value: &Value, member: &str, position: &str) -> Result<i64, Fault> {
    value.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "{class}::{member} expected {:?} for {position}, got tag {}",
            Tag::Int,
            value.tag_byte()
        ))
    })
}

/// One `uint` argument, as a `usize` — saturating, since a count larger than
/// this process could address is refused by the caller either way.
fn count(class: &str, value: &Value, member: &str, position: &str) -> Result<usize, Fault> {
    let raw = value.as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "{class}::{member} expected {:?} for {position}, got tag {}",
            Tag::Uint,
            value.tag_byte()
        ))
    })?;
    Ok(usize::try_from(raw).unwrap_or(usize::MAX))
}

/// The subject array of one of the three `array<T>` members, as the borrowed
/// handle [`crate::arr::borrowed`] owns the rules for — never
/// `NvsArray::from_raw`, which would release the caller's reference on drop.
///
/// `at` is the slot it arrived in, which is `0` for a static member and `1` for
/// an instance one, whose receiver is slot 0.
fn subject(
    class: &str,
    args: &[Value],
    at: usize,
    member: &str,
) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let array = args[at].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{class}::{member} expected {:?}, got tag {}",
            Tag::Array,
            args[at].tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(array))
}

/// Every live slot of a borrowed subject, in insertion order.
///
/// `sample` and `shuffle` draw over *slots* rather than over values, so a
/// value is retained only for the entries that end up in the answer. `pick`
/// needs one entry and does not build this list at all — `pick_from`.
fn slots(subject: &NvsArray) -> Vec<usize> {
    let mut out = Vec::new();
    let mut from = 0_usize;
    while let Some(slot) = subject.next_slot(from) {
        out.push(slot);
        from = slot + 1;
    }
    out
}

/// The value at `slot` as a fresh reference this frame owns — what a returned
/// value, or one stored into a fresh array, has to be.
///
/// `NvsArray::value_at` *borrows* from the subject, which belongs to the
/// caller, so every answer retains before it leaves. This is `crate::arr`'s
/// own rule, applied here for the same reason and stated there in full.
fn owned_value_at(subject: &NvsArray, slot: usize) -> Value {
    let value = subject
        .value_at(slot)
        .expect("next_slot only names live entries");
    #[expect(
        unsafe_code,
        reason = "the entry is owned by the subject array, which outlives this \
                  call, so the value handed back needs a reference of its own"
    )]
    unsafe {
        value.retain();
    }
    value
}

/// The values at `slots`, in that order, as a fresh `array<T>` under `0, 1, …`
/// keys.
///
/// Both `sample` and `shuffle` answer this way: their whole subject is which
/// entries and in what order, so the keys of the subject say nothing about the
/// answer and `NvsArray::append` assigns fresh ones — the same rule
/// `Core\Arr::values` states.
fn drawn(subject: &NvsArray, slots: &[usize]) -> Value {
    let mut out = NvsArray::new();
    for slot in slots {
        out.append(owned_value_at(subject, *slot));
    }
    Value::array(out)
}

// ============================================================================
// The draws — one per member, over whichever generator the caller selected
// ============================================================================
//
// Each is the whole of a member except its argument decoding and its refusals,
// which stay with the member so that the sentence a program is handed names the
// class it called. That is also why a refusal arrives here as a closure: the
// message belongs to `Core\Random::bytes` or to `Core\Random\Seeded::bytes`,
// and the arithmetic belongs to neither.

/// `int`'s draw: one integer from `[min, max]`, both ends included.
fn int_from(rng: &mut Generator<'_>, min: i64, max: i64) -> Value {
    Value::int(rng.random_range(min..=max))
}

/// `float`'s draw: 53 random bits, uniform in `[0, 1)`.
fn float_from(rng: &mut Generator<'_>) -> Value {
    Value::float(rng.random::<f64>())
}

/// How many octets [`octets_from`] and [`hex_from`] draw onto the stack at a
/// time before writing them into their result. A multiple of eight, so a
/// [`SplitMix`] sequence reads the same whether a draw is made in one pass or
/// in several.
const PASS: usize = 256;

/// `bytes`'s draw: `count` octets, written straight into the buffer the member
/// answers with.
///
/// One allocation, exactly `count` wide: the octets are drawn a [`PASS`] at a
/// time onto the stack and pushed, so no scratch buffer is built and copied.
/// [`NvsStr::try_build`] reserves it fallibly because `count` is off a call
/// site, and an allocator refusing a count is otherwise a process abort, which
/// in a server is every in-flight request paying for one argument.
///
/// # Errors
///
/// `refusal`, when the allocator will not hold `count` octets.
fn octets_from(
    rng: &mut Generator<'_>,
    count: usize,
    refusal: impl FnOnce() -> Fault,
) -> Result<NvsStr, Fault> {
    NvsStr::try_build(count, |out| {
        let mut pass = [0_u8; PASS];
        let mut left = count;
        while left > 0 {
            let width = left.min(PASS);
            rng.fill_bytes(&mut pass[..width]);
            out.push(&pass[..width]);
            left -= width;
        }
    })
    .ok_or_else(refusal)
}

/// `token`'s draw: `bytes` octets rendered as lower-case hex, written straight
/// into the `digits`-wide string the member answers with — one allocation, for
/// [`octets_from`]'s reasons.
///
/// # Errors
///
/// `refusal`, when the allocator will not hold `digits` characters.
fn hex_from(
    rng: &mut Generator<'_>,
    bytes: usize,
    digits: usize,
    refusal: impl FnOnce() -> Fault,
) -> Result<NvsStr, Fault> {
    NvsStr::try_build(digits, |out| {
        let mut pass = [0_u8; PASS];
        let mut hex = [0_u8; 2 * PASS];
        let mut left = bytes;
        while left > 0 {
            let width = left.min(PASS);
            rng.fill_bytes(&mut pass[..width]);
            for (pair, byte) in hex.chunks_exact_mut(2).zip(pass[..width].iter().copied()) {
                pair[0] = HEX_DIGITS[usize::from(byte >> 4)];
                pair[1] = HEX_DIGITS[usize::from(byte & 0x0f)];
            }
            out.push(&hex[..2 * width]);
            left -= width;
        }
    })
    .ok_or_else(refusal)
}

/// `pick`'s draw: one entry's value, or `null` over an empty subject.
///
/// It draws a slot and draws again when the slot is a hole, so every live
/// entry is equally likely and one pick costs the same over a million entries
/// as over three. [`NvsArray::slot_end`] states why that ends quickly. Over an
/// array with no holes it draws exactly once.
fn pick_from(rng: &mut Generator<'_>, subject: &NvsArray) -> Value {
    if subject.count() == 0 {
        return Value::null();
    }
    let end = subject.slot_end();
    loop {
        let slot = rng.random_range(0..end);
        if subject.value_at(slot).is_some() {
            return owned_value_at(subject, slot);
        }
    }
}

/// `sample`'s draw: `count` distinct entries, in the order they were drawn.
///
/// # Errors
///
/// `refusal`, handed the number of entries the subject holds, when `count` is
/// above it — there are not that many distinct entries to draw.
fn sample_from(
    rng: &mut Generator<'_>,
    subject: &NvsArray,
    count: usize,
    refusal: impl FnOnce(usize) -> Fault,
) -> Result<Value, Fault> {
    let mut slots = slots(subject);
    if count > slots.len() {
        return Err(refusal(slots.len()));
    }
    let (sampled, _) = slots.partial_shuffle(rng, count);
    Ok(drawn(subject, sampled))
}

/// `shuffle`'s draw: every entry, in a uniformly random order.
fn shuffle_from(rng: &mut Generator<'_>, subject: &NvsArray) -> Value {
    let mut slots = slots(subject);
    slots.shuffle(rng);
    drawn(subject, &slots)
}

/// `rule:testing/determinism-declared-on-the-test`'s seeded generator: SplitMix64, over the single `u64` of state
/// `nvs_runtime::Ctx` holds.
///
/// **Written here rather than reached for** because the requirement is unusual
/// and small: the whole state has to round-trip through one word of the
/// context, so that a draw is a pure function of "which draw is this" and a
/// test's sequence reproduces exactly. Every seedable generator in `rand` keeps
/// more state than that — `SmallRng` and `ChaCha12Rng` both — so storing one
/// would mean either a boxed generator in the crate every compiled unit links
/// or re-seeding and skipping `n` draws, which is quadratic. SplitMix64 is four
/// operations, is the standard seeding step for exactly this reason, and passes
/// the statistical batteries a test's dice rolls could possibly care about.
///
/// It is **not** a CSPRNG, and it does not need to be: see
/// [`nvs_runtime::Ctx::random_state`] for why nothing outside a `#[Test]` can
/// select it.
struct SplitMix(u64);

impl SplitMix {
    /// One SplitMix64 step: advance the state, then avalanche a copy of it.
    ///
    /// The state advances by an odd constant unconditionally, so the period is
    /// the full 2⁶⁴ and the output is a bijection of the counter — which is why
    /// this generator can be a single word and still be worth drawing from.
    fn step(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// `rand_core` 0.10 makes [`rand::Rng`] the infallible half of
/// [`rand::TryRng`], with a blanket impl over every `TryRng<Error =
/// Infallible>` — so an implementor writes the **fallible** three and gets
/// `Rng`, and with it [`rand::RngExt`]'s drawing methods, for free. Writing
/// `Rng` by hand instead collides with that blanket.
impl rand::TryRng for SplitMix {
    type Error = core::convert::Infallible;

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.step())
    }

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        // The high half, which is the better-mixed one in SplitMix64's final
        // avalanche and is what every 32-bit truncation of it takes. The shift
        // is what makes the cast lossless rather than merely intended, which is
        // why there is no `cast_possible_truncation` expectation here.
        Ok((self.step() >> 32) as u32)
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        for chunk in dst.chunks_mut(8) {
            let drawn = self.step().to_le_bytes();
            chunk.copy_from_slice(&drawn[..chunk.len()]);
        }
        Ok(())
    }
}

/// Runs `with` against the generator this context draws from: `rule:testing/determinism-declared-on-the-test`'s
/// seeded one where a `#[Test(seed: …)]` armed it, and the thread's CSPRNG
/// otherwise.
///
/// **Every draw in `Core` goes through here** — this module's members and
/// `Core\Uuid`'s two — because a seed that reproduced some of a test's draws
/// and not others would be worse than no seed at all: the sequence would depend
/// on which members the test happened to call. The advanced state is written
/// back before the value is handed on, so two draws in one test are two
/// different numbers and the *sequence* is what the seed fixes.
pub(crate) fn draw<T>(ctx: &mut Ctx, with: impl FnOnce(&mut Generator<'_>) -> T) -> T {
    let Some(state) = ctx.random_state() else {
        return with(&mut Generator(&mut rand::rng()));
    };
    let mut seeded = SplitMix(state);
    let drawn = with(&mut Generator(&mut seeded));
    ctx.set_random_state(seeded.0);
    drawn
}

/// The generator [`draw`] hands its closure — whichever of the two the context
/// selected, behind **one concrete type**.
///
/// The alternative that does not work is making [`draw`] generic over the
/// generator, which would need a generic *closure* — Rust has no spelling for
/// one. A trait object is the shape that does, and this newtype is what gives
/// it a `Sized` outside: [`rand::RngExt`]'s drawing methods are declared on a
/// sized receiver, so `dyn Rng` on its own would offer `next_u64` and nothing
/// a member here actually calls. One pointer indirection per draw.
pub(crate) struct Generator<'a>(&'a mut dyn rand::Rng);

impl rand::TryRng for Generator<'_> {
    type Error = core::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self.0.next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.0.next_u64())
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        self.0.fill_bytes(dst);
        Ok(())
    }
}

/// Runs `with` against the generator the receiver in `args[0]` holds, writing
/// the advanced state back into its slot.
///
/// Read, draw, write back — which is what makes a seed fix a *sequence* rather
/// than one number, and it is per instance: two generators at one seed are two
/// states that happen to be equal, so a draw from one says nothing about the
/// other. [`draw`] is the same three steps against `nvs_runtime::Ctx`'s state,
/// and the two are deliberately not one function: a member here has a receiver
/// to read and never consults the context, so a test's declared seed and a
/// value a program is holding cannot reach into each other.
///
/// The state is written back after the draw and not before, so a member that
/// refused its arguments leaves the sequence exactly where it found it.
///
/// # Errors
///
/// The [`crate::instance::receiver`] fault a wrongly-tagged receiver is, which
/// compiled code cannot produce.
fn sequence<T>(
    args: &[Value],
    member: &str,
    with: impl FnOnce(&mut Generator<'_>) -> T,
) -> Result<T, Fault> {
    let receiver = crate::instance::receiver(args[0], &SEEDED, member)?;
    // The constructor wrote an `int` into this slot and nothing else ever
    // writes it, so the fallback is a shape this crate cannot produce rather
    // than a default anything relies on.
    let held = crate::instance::slot(receiver, STATE_SLOT)
        .as_int()
        .unwrap_or(0);
    let mut state = SplitMix(held.cast_unsigned());
    let drawn = with(&mut Generator(&mut state));
    crate::instance::set_slot(receiver, STATE_SLOT, Value::int(state.0.cast_signed()));
    Ok(drawn)
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `Core\Random::int(int $min, int $max): int` — replacing PHP's `rand`,
    /// `mt_rand` and `random_int` with the last one's semantics and the last
    /// one's generator.
    ///
    /// **Inclusive at both ends**, so `int(1, 6)` is a die and `int($n, $n)` is
    /// `$n`. `$min > $max` names an empty range, which has no answer to invent,
    /// so it throws (`rule:core-api/shape-rules` R4) rather than swapping the bounds — a swap
    /// would turn a computed-bounds bug into a plausible-looking result.
    fn nvs_core_random_int(ctx, args: [2]) {
        let min = integer(NAME, &args[0], "int", "the lower bound")?;
        let max = integer(NAME, &args[1], "int", "the upper bound")?;

        if min > max {
            return Err(Fault::thrown(format!(
                "Core\\Random::int(): the range is empty — the lower bound {min} is above the \
                 upper bound {max}, and both ends are inclusive"
            )));
        }
        Ok(draw(ctx, |rng| int_from(rng, min, max)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random::float(): float` — replacing PHP's `lcg_value` and the
    /// `mt_rand() / mt_getrandmax()` idiom.
    ///
    /// Uniform in `[0, 1)`: `0.0` is drawable and `1.0` is not, which is the
    /// half-open interval every "scale it into my own range" use expects, and
    /// the one the spec row writes. 53 random bits, the whole significand of an
    /// `f64`.
    fn nvs_core_random_float(ctx, args: [0]) {
        let _ = args;
        Ok(draw(ctx, float_from))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random::bytes(uint $count): bytes` — replacing `random_bytes` and
    /// `openssl_random_pseudo_bytes`, both of which this class's one generator
    /// already answers with the stronger of their two guarantees.
    ///
    /// **A raw buffer, not hex.** This is the row a key, a nonce or an IV comes
    /// from, where the consumer wants octets and any rendering is that
    /// consumer's own step — [`nvs_core_random_token`] is the rendered
    /// spelling, and the two exist side by side rather than one being the
    /// other's `2 * n` special case. Nothing here can be asserted by value, so
    /// a case pins `Core\Bytes::length` of the answer and that two draws
    /// differ; `rule:types/conversion` is why it renders through `Core\Encoding::toHex`
    /// to compare them rather than reading the buffer as a `string`.
    ///
    /// **Zero bytes throws** (`rule:core-api/shape-rules` R4), for `token`'s reason applied one
    /// level down: an empty buffer used as a key or an IV is a key every other
    /// empty draw matches, and PHP's own `random_bytes(0)` is a `ValueError`
    /// rather than `""`.
    ///
    /// A count larger than this process can hold is an ordinary throw too, and
    /// it is decided by the allocator rather than by an arithmetic bound: there
    /// is no doubling here to overflow the way [`nvs_core_random_token`]'s
    /// does, so the only question left is whether the allocator has the buffer,
    /// and asking it is both exact and the difference between a throw and an
    /// abort.
    fn nvs_core_random_bytes(ctx, args: [1]) {
        let count = count(NAME, &args[0], "bytes", "the byte count")?;

        if count == 0 {
            return Err(Fault::thrown(
                "Core\\Random::bytes(): a draw of zero bytes is the empty buffer, which is not a \
                 secret — draw at least one byte",
            ));
        }

        // Two checks, and they answer different questions: `affordable` is the
        // policy seam every count-shaped argument passes through, and
        // `octets_from`'s fallible build is the allocator actually refusing. Keep both —
        // the first is where `[limits.hard]` will attach, the second is what
        // catches a draw the machine cannot satisfy today.
        nvs_runtime::affordable(Some(count), "Core\\Random::bytes()")?;
        let refusal = || {
            Fault::thrown(
                "Core\\Random::bytes(): the requested draw is larger than any buffer this process \
                 could hold",
            )
        };
        Ok(Value::bytes(draw(ctx, |rng| octets_from(rng, count, refusal))?))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random::token(uint $bytes = 32): string` — replacing the
    /// `bin2hex(random_bytes(…))` idiom, which is what PHP code actually
    /// writes when it wants a session identifier or a reset link.
    ///
    /// **Hex, and therefore a `string`.** [`nvs_core_random_bytes`] is the same
    /// draw unrendered, and this row exists beside it because a token is
    /// written into a URL, a cookie or a database column, so the hex is what a
    /// program wanted in every case anyway. `$bytes` counts the
    /// *entropy* drawn, never the characters produced — the answer is twice as
    /// long as the count, and reading it as a length would silently halve the
    /// strength of every token in a program that guessed wrong.
    ///
    /// **Zero bytes throws** (`rule:core-api/shape-rules` R4). The empty string is a token that
    /// compares equal to every other empty token, so answering with it would
    /// turn an arithmetic slip into an authentication bypass; there is no
    /// reading of `token(0)` worth being total for.
    ///
    /// **A count too large refuses in two places, for the same reason
    /// [`nvs_core_random_bytes`] does.** The arithmetic bound is this member's
    /// own — a token is twice as long as its draw, so `2 * $bytes` is what the
    /// seam is asked about, and the answer is that this member's bound sits at
    /// exactly half of `bytes`'s. Everything the seam allows is then asked of
    /// the allocator rather than assumed, through the same fallible build
    /// `bytes` uses, and a refusal reports the same sentence `bytes` reports.
    fn nvs_core_random_token(ctx, args: [1]) {
        let bytes = count(NAME, &args[0], "token", "the byte count")?;

        if bytes == 0 {
            return Err(Fault::thrown(
                "Core\\Random::token(): a token of zero bytes is the empty string, which is not a \
                 token — draw at least one byte",
            ));
        }
        // Two checks, as in `bytes`, and they answer different questions. The
        // seam is asked about the *answer's* width, since that is the larger of
        // the two and the only one that can overflow — `Core\Str::repeat` takes
        // the same shape for the same reason.
        let digits = nvs_runtime::affordable(bytes.checked_mul(2), "Core\\Random::token()")?;

        // The allocator is then asked rather than assumed, since a count the
        // seam allows can still be a draw this machine cannot serve.
        let refusal = || {
            Fault::thrown(
                "Core\\Random::token(): the requested draw is larger than any buffer this \
                 process could hold",
            )
        };
        Ok(Value::str(draw(ctx, |rng| {
            hex_from(rng, bytes, digits, refusal)
        })?))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random::pick(array<T> $a): ?T` — one entry's value, uniformly,
    /// replacing PHP's `array_rand` in its one-element spelling.
    ///
    /// **`?T` over an empty array rather than a throw** (`rule:core-api/shape-rules` R5).
    /// `Core\Arr::first`'s own docs own that rule and the one thing it costs:
    /// over an `array<?T>` the answer cannot tell "the array was empty" from
    /// "the entry drawn was `null`". R4's throw is for a *failure*, and asking
    /// a possibly-empty array for an element is not one.
    ///
    /// The **value**, never the key — PHP's `array_rand` answers with a key,
    /// which is the shape that makes `$a[array_rand($a)]` the idiom. `rule:types/arrays` stores every key as a string, so answering with one would hand back
    /// a `string` for an `array<T>` and lose the type the caller had.
    fn nvs_core_random_pick(ctx, args: [1]) {
        let subject = subject(NAME, args, 0, "pick")?;

        Ok(draw(ctx, |rng| pick_from(rng, &subject)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random::sample(array<T> $a, uint $count): array<T>` — `$count`
    /// *distinct* entries, uniformly, replacing PHP's `array_rand` with a
    /// count.
    ///
    /// **The answer is in random order**, not the subject's. PHP's
    /// `array_rand` preserves the subject's order, which quietly leaks it into
    /// every sample — a program drawing three winners from a list sorted by
    /// signup date gets them back in signup order. This member is a draw, so
    /// its order is drawn too; a caller who wants the subject's order back
    /// sorts, and one who wants the whole array reordered has `shuffle` beside
    /// it.
    ///
    /// **A count above the array's size throws** (`rule:core-api/shape-rules` R4). There are not
    /// that many distinct entries to draw, so the alternatives are inventing a
    /// duplicate or silently answering short — a failure either way, and only
    /// the throw says so.
    fn nvs_core_random_sample(ctx, args: [2]) {
        let subject = subject(NAME, args, 0, "sample")?;
        let count = count(NAME, &args[1], "sample", "the sample size")?;

        draw(ctx, |rng| {
            sample_from(rng, &subject, count, |held| {
                Fault::thrown(format!(
                    "Core\\Random::sample(): asked for {count} distinct entries from an array \
                     that holds {held}"
                ))
            })
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random::shuffle(array<T> $a): array<T>` — every entry, in a
    /// uniformly random order, replacing PHP's `shuffle` and `str_shuffle`.
    ///
    /// **A fresh array, not a reordering in place.** PHP's `shuffle` takes its
    /// subject by reference and answers `bool`; an Novis array is a copy-on-write
    /// *value*, so there is nothing to mutate and the spec's signature returns
    /// the new order instead.
    ///
    /// **Keys are discarded**, exactly as PHP's own `shuffle` renumbers: a
    /// shuffle is about position, and a key that survived it would name an
    /// entry that is no longer where the caller left it.
    fn nvs_core_random_shuffle(ctx, args: [1]) {
        let subject = subject(NAME, args, 0, "shuffle")?;

        Ok(draw(ctx, |rng| shuffle_from(rng, &subject)))
    }
}

// ============================================================================
// `Core\Random\Seeded` — the same seven, over a sequence a seed fixes
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `new Core\Random\Seeded(int $seed)` — a generator whose sequence
    /// `$seed` fixes.
    ///
    /// Reached as a symbol rather than as a registered `constructor` member: a
    /// `Core` class has no constructor a program could resolve, so `nvs-ir`
    /// lowers `new` on one straight to this helper ([`crate::instance`]'s
    /// module docs), and [`SEEDED_NEW`] is the signature the checker holds the
    /// call to.
    ///
    /// **The seed is the state**, which is what makes
    /// `new Core\Random\Seeded(42)` draw the sequence a `#[Test(seed: 42)]`
    /// isolate draws from `Core\Random`: the test runner seeds
    /// `nvs_runtime::Ctx::random_state` with the same `int`, and both ends
    /// build the same [`SplitMix`] over it. Every `int` is a seed, `0`
    /// included, so there is nothing here to refuse.
    fn nvs_core_random_seeded_new(_ctx, args: [1]) {
        let seed = integer(SEEDED_NAME, &args[0], "constructor", "the seed")?;
        Ok(crate::instance::build(&SEEDED, [Value::int(seed)]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random\Seeded::int(int $min, int $max): int` — the draw
    /// [`nvs_core_random_int`] makes, from this instance's own sequence.
    ///
    /// The closed bounds and the empty-range throw are that member's, argued
    /// there; what differs is only where the bits come from.
    fn nvs_core_random_seeded_int(_ctx, args: [3]) {
        let min = integer(SEEDED_NAME, &args[1], "int", "the lower bound")?;
        let max = integer(SEEDED_NAME, &args[2], "int", "the upper bound")?;

        if min > max {
            return Err(Fault::thrown(format!(
                "Core\\Random\\Seeded::int(): the range is empty — the lower bound {min} is \
                 above the upper bound {max}, and both ends are inclusive"
            )));
        }
        sequence(args, "int", |rng| int_from(rng, min, max))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random\Seeded::float(): float` — the draw
    /// [`nvs_core_random_float`] makes, from this instance's own sequence.
    fn nvs_core_random_seeded_float(_ctx, args: [1]) {
        sequence(args, "float", float_from)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random\Seeded::bytes(uint $count): bytes` — the draw
    /// [`nvs_core_random_bytes`] makes, from this instance's own sequence.
    ///
    /// **A fixture, not a key.** The buffer is exactly as unpredictable as the
    /// seed that produced it, so the guarantee `Core\Random::bytes` carries is
    /// not this member's; the two are different types precisely so that a
    /// program cannot reach one where it meant the other.
    ///
    /// Zero bytes throws and an oversized draw is refused twice over, both for
    /// the reasons [`nvs_core_random_bytes`] gives.
    fn nvs_core_random_seeded_bytes(_ctx, args: [2]) {
        let count = count(SEEDED_NAME, &args[1], "bytes", "the byte count")?;

        if count == 0 {
            return Err(Fault::thrown(
                "Core\\Random\\Seeded::bytes(): a draw of zero bytes is the empty buffer, which \
                 is no sequence at all — draw at least one byte",
            ));
        }
        nvs_runtime::affordable(Some(count), "Core\\Random\\Seeded::bytes()")?;
        let refusal = || {
            Fault::thrown(
                "Core\\Random\\Seeded::bytes(): the requested draw is larger than any buffer \
                 this process could hold",
            )
        };
        Ok(Value::bytes(sequence(args, "bytes", |rng| {
            octets_from(rng, count, refusal)
        })??))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random\Seeded::token(uint $bytes = 32): string` — the draw
    /// [`nvs_core_random_token`] makes, from this instance's own sequence, and
    /// rendered the same way.
    ///
    /// A stable identifier for a fixture rather than a session token, which is
    /// [`nvs_core_random_seeded_bytes`]'s distinction one rendering further on.
    /// `$bytes` counts the entropy drawn and never the characters produced, as
    /// it does there.
    fn nvs_core_random_seeded_token(_ctx, args: [2]) {
        let bytes = count(SEEDED_NAME, &args[1], "token", "the byte count")?;

        if bytes == 0 {
            return Err(Fault::thrown(
                "Core\\Random\\Seeded::token(): a token of zero bytes is the empty string, which \
                 is no sequence at all — draw at least one byte",
            ));
        }
        let digits =
            nvs_runtime::affordable(bytes.checked_mul(2), "Core\\Random\\Seeded::token()")?;
        let refusal = || {
            Fault::thrown(
                "Core\\Random\\Seeded::token(): the requested draw is larger than any buffer \
                 this process could hold",
            )
        };
        Ok(Value::str(sequence(args, "token", |rng| {
            hex_from(rng, bytes, digits, refusal)
        })??))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random\Seeded::pick(array<T> $a): ?T` — the draw
    /// [`nvs_core_random_pick`] makes, from this instance's own sequence.
    fn nvs_core_random_seeded_pick(_ctx, args: [2]) {
        let subject = subject(SEEDED_NAME, args, 1, "pick")?;

        sequence(args, "pick", |rng| pick_from(rng, &subject))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random\Seeded::sample(array<T> $a, uint $count): array<T>` — the
    /// draw [`nvs_core_random_sample`] makes, from this instance's own
    /// sequence, in the order it drew them.
    fn nvs_core_random_seeded_sample(_ctx, args: [3]) {
        let subject = subject(SEEDED_NAME, args, 1, "sample")?;
        let count = count(SEEDED_NAME, &args[2], "sample", "the sample size")?;

        sequence(args, "sample", |rng| {
            sample_from(rng, &subject, count, |held| {
                Fault::thrown(format!(
                    "Core\\Random\\Seeded::sample(): asked for {count} distinct entries from an \
                     array that holds {held}"
                ))
            })
        })?
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Random\Seeded::shuffle(array<T> $a): array<T>` — the draw
    /// [`nvs_core_random_shuffle`] makes, from this instance's own sequence.
    fn nvs_core_random_seeded_shuffle(_ctx, args: [2]) {
        let subject = subject(SEEDED_NAME, args, 1, "shuffle")?;

        sequence(args, "shuffle", |rng| shuffle_from(rng, &subject))
    }
}

/// Lower-case hex, which is what `bin2hex` emits and what every consumer of a
/// token compares against.
const HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsArray, OutputSink, Value, call};

    /// Runs one member through the `rule:errors/propagation` boundary compiled code reaches it
    /// at.
    ///
    /// It releases nothing: a helper *borrows* its arguments, so a test that
    /// builds a heap one owns it afterwards and says so at its own end — which
    /// is `crate::arr`'s convention and the only one under which a refcount
    /// assertion means anything.
    fn run(
        member: unsafe extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32,
        args: &[Value],
    ) -> Result<Value, i32> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        call(member, &mut ctx, args)
    }

    /// The produced `string`, as text, releasing the reference this test now
    /// owns.
    fn taken(value: Value) -> String {
        let text = String::from_utf8(
            value
                .as_str_bytes()
                .expect("the member answered with a `string`")
                .to_vec(),
        )
        .expect("`rule:types/bytes` guarantees a `string` is UTF-8");
        #[expect(
            unsafe_code,
            reason = "the helper handed back the one reference it built, so \
                      this test owns it"
        )]
        unsafe {
            value.release();
        }
        text
    }

    // covers: Core\Random::int
    #[test]
    fn a_drawn_int_is_inside_its_inclusive_bounds() {
        for _ in 0..256 {
            let drawn = run(super::nvs_core_random_int, &[Value::int(-3), Value::int(4)])
                .expect("a non-empty range always has an answer")
                .as_int()
                .expect("`int` answers with an `int`");
            assert!((-3..=4).contains(&drawn), "{drawn} is outside [-3, 4]");
        }
    }

    /// The one range whose answer is deterministic, which is what the
    /// conformance case pins too.
    #[test]
    fn a_degenerate_range_is_its_own_bound() {
        let drawn = run(super::nvs_core_random_int, &[Value::int(5), Value::int(5)])
            .expect("[5, 5] holds exactly one integer")
            .as_int();
        assert_eq!(drawn, Some(5));
    }

    #[test]
    fn the_widest_range_still_draws() {
        run(
            super::nvs_core_random_int,
            &[Value::int(i64::MIN), Value::int(i64::MAX)],
        )
        .expect("every `int` is in range")
        .as_int()
        .expect("`int` answers with an `int`");
    }

    #[test]
    fn an_empty_range_throws() {
        let status = run(super::nvs_core_random_int, &[Value::int(4), Value::int(3)])
            .expect_err("no integer is both at least 4 and at most 3");
        assert_eq!(status, nvs_runtime::THROWN);
    }

    // covers: Core\Random::float
    #[test]
    fn a_drawn_float_is_in_the_half_open_unit_interval() {
        for _ in 0..256 {
            let drawn = run(super::nvs_core_random_float, &[])
                .expect("`float` never fails")
                .as_float()
                .expect("`float` answers with a `float`");
            assert!((0.0..1.0).contains(&drawn), "{drawn} is outside [0, 1)");
        }
    }

    /// The octets drawn, releasing the reference this test now owns.
    fn octets(value: Value) -> Vec<u8> {
        let drawn = value
            .as_bytes()
            .expect("`bytes` answers with a `bytes`")
            .to_vec();
        #[expect(
            unsafe_code,
            reason = "the helper handed back the one reference it built, so \
                      this test owns it"
        )]
        unsafe {
            value.release();
        }
        drawn
    }

    /// A draw is exactly its count long, two draws differ, and the empty draw
    /// and the one no allocator could hold both throw rather than answer.
    // covers: Core\Random::bytes
    #[test]
    fn a_byte_draw_is_its_count_long_and_zero_or_too_many_throws() {
        let first = octets(
            run(super::nvs_core_random_bytes, &[Value::uint(32)]).expect("32 bytes is drawable"),
        );
        let second = octets(
            run(super::nvs_core_random_bytes, &[Value::uint(32)]).expect("32 bytes is drawable"),
        );
        assert_eq!(first.len(), 32);
        assert_eq!(second.len(), 32);
        assert_ne!(first, second);

        for refused in [0, u64::MAX] {
            let status = run(super::nvs_core_random_bytes, &[Value::uint(refused)])
                .expect_err("neither the empty buffer nor an unallocatable one is a draw");
            assert_eq!(status, nvs_runtime::THROWN, "a draw of {refused} bytes");
        }
    }

    // covers: Core\Random::token
    #[test]
    fn a_token_is_twice_its_byte_count_in_lower_case_hex() {
        let token = taken(
            run(super::nvs_core_random_token, &[Value::uint(8)]).expect("8 bytes is drawable"),
        );
        assert_eq!(token.len(), 16);
        assert!(
            token
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "`{token}` is not lower-case hex"
        );
    }

    /// Two draws differing is not a proof of anything on its own, but a member
    /// that answered with a constant would fail it every time.
    #[test]
    fn two_tokens_differ() {
        let first = taken(
            run(super::nvs_core_random_token, &[Value::uint(32)]).expect("32 bytes is drawable"),
        );
        let second = taken(
            run(super::nvs_core_random_token, &[Value::uint(32)]).expect("32 bytes is drawable"),
        );
        assert_eq!(first.len(), 64);
        assert_ne!(first, second);
    }

    #[test]
    fn a_zero_byte_token_throws() {
        let status = run(super::nvs_core_random_token, &[Value::uint(0)])
            .expect_err("the empty string is not a token");
        assert_eq!(status, nvs_runtime::THROWN);
    }

    #[test]
    fn an_unrepresentable_token_throws() {
        let status = run(super::nvs_core_random_token, &[Value::uint(u64::MAX)])
            .expect_err("no string that long can exist");
        assert_eq!(status, nvs_runtime::THROWN);
    }

    /// A subject `array<int>` holding `0, 1, …, len - 1` as a list.
    fn ints(len: i64) -> Value {
        let mut array = NvsArray::new();
        for n in 0..len {
            array.append(Value::int(n));
        }
        Value::array(array)
    }

    /// Every `int` an answered `array<int>` holds, in its own order,
    /// releasing the reference this test now owns.
    fn taken_ints(value: Value) -> Vec<i64> {
        let answer = crate::arr::borrowed(
            value
                .array_ptr()
                .expect("the member answered with an `array`"),
        );
        let mut out = Vec::new();
        let mut from = 0_usize;
        while let Some(slot) = answer.next_slot(from) {
            out.push(
                answer
                    .value_at(slot)
                    .expect("next_slot only names live entries")
                    .as_int()
                    .expect("the subject held `int`s"),
            );
            from = slot + 1;
        }
        #[expect(
            unsafe_code,
            reason = "the helper handed back the one reference it built, so \
                      this test owns it"
        )]
        unsafe {
            value.release();
        }
        out
    }

    /// Releases a subject this test built, which the helper only borrowed.
    fn release(subject: Value) {
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built, and the \
                      helper borrowed rather than consumed it"
        )]
        unsafe {
            subject.release();
        }
    }

    #[test]
    fn picking_from_an_empty_array_is_null() {
        let subject = ints(0);
        let answer =
            run(super::nvs_core_random_pick, &[subject]).expect("an empty array is `null`");
        assert_eq!(answer.tag(), Some(nvs_runtime::Tag::Null));
        release(subject);
    }

    #[test]
    fn picking_from_one_element_is_that_element() {
        let subject = ints(1);
        let answer = run(super::nvs_core_random_pick, &[subject]).expect("one element is drawable");
        assert_eq!(answer.as_int(), Some(0));
        release(subject);
    }

    // covers: Core\Random::pick
    #[test]
    fn every_pick_is_an_element_of_the_subject() {
        let subject = ints(6);
        for _ in 0..256 {
            let drawn = run(super::nvs_core_random_pick, &[subject])
                .expect("a non-empty array is drawable")
                .as_int()
                .expect("the subject held `int`s");
            assert!((0..6).contains(&drawn), "{drawn} is not an element");
        }
        release(subject);
    }

    // covers: Core\Random::sample
    #[test]
    fn a_sample_is_that_many_distinct_entries() {
        let subject = ints(10);
        for _ in 0..64 {
            let mut sample = taken_ints(
                run(super::nvs_core_random_sample, &[subject, Value::uint(4)])
                    .expect("4 of 10 is drawable"),
            );
            assert_eq!(sample.len(), 4);
            sample.sort_unstable();
            sample.dedup();
            assert_eq!(sample.len(), 4, "a sample repeated an entry");
        }
        release(subject);
    }

    /// A sample the size of its subject is a permutation of it, which is what
    /// makes `sample` and `shuffle` one algorithm with two stopping points.
    #[test]
    fn a_full_sample_is_a_permutation() {
        let subject = ints(8);
        let mut sample = taken_ints(
            run(super::nvs_core_random_sample, &[subject, Value::uint(8)])
                .expect("8 of 8 is drawable"),
        );
        sample.sort_unstable();
        assert_eq!(sample, (0..8).collect::<Vec<_>>());
        release(subject);
    }

    #[test]
    fn a_sample_larger_than_its_subject_throws() {
        let subject = ints(3);
        let status = run(super::nvs_core_random_sample, &[subject, Value::uint(4)])
            .expect_err("3 entries hold no 4 distinct ones");
        assert_eq!(status, nvs_runtime::THROWN);
        release(subject);
    }

    // covers: Core\Random::shuffle
    #[test]
    fn a_shuffle_keeps_every_element_and_renumbers() {
        let subject = ints(16);
        let mut shuffled = taken_ints(
            run(super::nvs_core_random_shuffle, &[subject]).expect("shuffling never fails"),
        );
        assert_eq!(shuffled.len(), 16);
        shuffled.sort_unstable();
        assert_eq!(shuffled, (0..16).collect::<Vec<_>>());
        release(subject);
    }

    /// The borrowed handle every `array<T>` member reads through must leave
    /// the caller's reference count exactly as it found it — a release there
    /// is a double free and a retain there leaks one reference per call.
    #[test]
    fn drawing_leaves_the_subjects_refcount_alone() {
        let mut array = NvsArray::new();
        array.append(Value::int(1));
        array.append(Value::int(2));
        let before = array.refcount();
        let subject = Value::array(array);

        taken_ints(run(super::nvs_core_random_shuffle, &[subject]).expect("shuffling never fails"));
        run(super::nvs_core_random_pick, &[subject]).expect("two elements are drawable");

        // The handle takes over the one reference this test owns and drops at
        // the end of the statement, which is also this test's release.
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built above, and \
                      each helper borrowed rather than consumed it"
        )]
        let after =
            unsafe { NvsArray::from_raw(subject.array_ptr().expect("an array")) }.refcount();
        assert_eq!(after, before);
    }

    #[test]
    fn a_non_array_subject_is_a_contained_fault() {
        let status =
            run(super::nvs_core_random_pick, &[Value::int(7)]).expect_err("an int is not an array");
        assert_eq!(status, nvs_runtime::FATAL);
    }

    #[test]
    fn a_non_int_bound_is_a_contained_fault() {
        let status = run(super::nvs_core_random_int, &[Value::uint(1), Value::int(6)])
            .expect_err("a `uint` is not an `int`");
        assert_eq!(status, nvs_runtime::FATAL);
    }

    /// A generator at `seed`, as `new Core\Random\Seeded($seed)` builds one.
    fn seeded(seed: i64) -> Value {
        run(super::nvs_core_random_seeded_new, &[Value::int(seed)]).expect("every `int` is a seed")
    }

    /// `count` draws over the whole non-negative range, which is wide enough
    /// that two sequences agreeing by chance is not a thing that happens.
    fn sequence(generator: Value, count: usize) -> Vec<i64> {
        let mut drawn = Vec::new();
        for _ in 0..count {
            drawn.push(
                run(
                    super::nvs_core_random_seeded_int,
                    &[generator, Value::int(0), Value::int(i64::MAX)],
                )
                .expect("a non-empty range always has an answer")
                .as_int()
                .expect("`int` answers with an `int`"),
            );
        }
        drawn
    }

    /// The class's whole promise: one seed is one sequence, and another seed is
    /// another one.
    #[test]
    fn core_random_seeded_gives_the_same_sequence_for_the_same_seed() {
        let left = seeded(42);
        let right = seeded(42);
        let apart = seeded(43);

        let replayed = sequence(left, 16);
        assert_eq!(replayed, sequence(right, 16));
        assert_ne!(replayed, sequence(apart, 16));

        release(left);
        release(right);
        release(apart);
    }

    /// The advanced state reaches the instance's slot, so the seed fixes a
    /// sequence rather than a number — a generator that kept its seed unchanged
    /// would answer one value forever and still pass the test above.
    #[test]
    fn a_seeded_generator_advances_between_draws() {
        let generator = seeded(7);
        let drawn = sequence(generator, 8);

        let mut distinct = drawn.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), drawn.len(), "a draw repeated: {drawn:?}");
        release(generator);
    }

    /// A refused draw takes nothing from the sequence: the state is written
    /// back after the draw and not before it, so the next draw is the one the
    /// refusal was asked instead of.
    #[test]
    fn a_refused_seeded_draw_leaves_the_sequence_where_it_was() {
        let bumped = seeded(11);
        let status = run(
            super::nvs_core_random_seeded_int,
            &[bumped, Value::int(4), Value::int(3)],
        )
        .expect_err("no integer is both at least 4 and at most 3");
        assert_eq!(status, nvs_runtime::THROWN);

        let clean = seeded(11);
        assert_eq!(sequence(bumped, 4), sequence(clean, 4));
        release(bumped);
        release(clean);
    }
}
