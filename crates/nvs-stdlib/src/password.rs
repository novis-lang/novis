//! `Core\Password` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 16's three members, and the second of exactly two operations that take a
//! `secret` and answer something that is not one.
//!
//! `rule:core-api/tier-roster` places
//! this class in `Core` by tests 1 and 2 — "as a `Core\Crypto` primitive over a
//! `secret`" — rather than in
//! `rule:security/protocol-roster`'s
//! closed protocol roster. What belongs here is the parameter choice, the
//! salt's provenance, what `needsRehash` compares, and the two places this
//! module refuses a stored hash the C `password_verify` would have answered
//! `false` for.
//!
//! # No algorithm argument, and no cost argument
//!
//! The spec's row and `examples/crypto.nvs`'s own header both write it as the
//! headline: `hash(secret string): string` takes the password and nothing
//! else. PHP's `password_hash($pw, PASSWORD_ARGON2ID, ['memory_cost' => …])`
//! puts three numbers at every call site, which means a program's cost is
//! whichever call site was copied last and an upgrade is a grep. Here the
//! parameters are the library's, they are [`ALGORITHM`], [`VERSION`] and
//! [`params`] below, and [`nvs_core_password_needs_rehash`] is how a *stored*
//! hash learns it has fallen behind — which is the whole reason a caller never
//! names a cost. A program that outgrows these numbers is asking for a
//! different library, not a fourth argument.
//!
//! # The parameters, and what they spend
//!
//! Argon2id, version 0x13, **m = 19 MiB, t = 2, p = 1** — OWASP's second
//! recommended configuration for Argon2id, the one that trades a second pass
//! for a third of the first configuration's footprint. Argon2id rather than
//! Argon2i or Argon2d because it is RFC 9106 § 4's own recommendation for
//! password hashing: the first pass is data-independent, so the side-channel
//! Argon2d is exposed to is closed, and the rest is data-dependent, so the
//! time-memory tradeoff Argon2i is exposed to is closed too.
//!
//! **What that spends is 19 MiB, transiently, per `hash` or `verify` call, on
//! the calling task** ([AGENTS.md](/AGENTS.md)'s priority ordering
//! asks for the number). It is allocated and released inside the one call, so
//! it is O(in-flight logins) and never O(logins served), and it goes through
//! this process's own allocator, which means `nvs_runtime::budget` charges it
//! to the request like any other buffer. That is memory spent to buy priority
//! 1, which is the direction the ordering allows; the alternative — a cheap
//! hash — is not a smaller version of this member, it is a different one.
//!
//! The allocation is **fallible on both sides**. `nvs_runtime::affordable` is
//! the policy seam every large count in `Core` passes through, and `argon2`'s
//! own block allocator answers `None` rather than aborting, so a machine that
//! cannot spare the table produces a throw the request can catch instead of an
//! abort that takes the worker with it.
//!
//! # The salt is drawn through `Core\Random`'s seam, not through `getrandom`
//!
//! `argon2`'s `getrandom` feature is off in `Cargo.toml` on purpose. Every draw
//! in `Core` goes through [`crate::random::draw`] — that function's own doc
//! comment is the home of why — so this tree has one CSPRNG, and a
//! `#[Test(seed: …)]` that armed the context reproduces this member's salt
//! along with every other draw the test made. Sixteen bytes, `password_hash`'s
//! own `RECOMMENDED_SALT_LEN`.
//!
//! # The read roster has two entries, and only one of them is ever written
//!
//! `rule:security/bcrypt-read-roster`
//! is the whole contract, and its § 1 is the roster: [`nvs_core_password_verify`]
//! reads the Argon2id PHC string [`nvs_core_password_hash`] writes, and a
//! bcrypt hash under `$2y$`, `$2a$` or `$2b$` — one algorithm under three tags,
//! verified identically. `$2x$` is not in it, because that tag exists to be
//! bug-compatible with `crypt_blowfish`'s sign-extension overflow and reading
//! it means reimplementing the bug. PHP's `PASSWORD_DEFAULT` has been bcrypt
//! since 5.5 and still is, so the column a migrating application arrives with
//! is the population rather than the exception; and a stored hash is not
//! invertible, so no offline tool can convert it. Only a presented password can
//! be rehashed, which is what makes the login-time loop *the* migration path:
//! `verify` proves the password, [`nvs_core_password_needs_rehash`] answers
//! `true` for every bcrypt row (§ 3), `hash` rewrites it under Argon2id, and the
//! bcrypt column converges to empty with no flag, no tool and no second code
//! path.
//!
//! **The write side is unchanged** (§ 2). `hash` writes Argon2id and nothing
//! else, and no member gains an algorithm argument — Novis never *produces* a
//! bcrypt hash, it only stopped refusing to read one.
//!
//! **What that spends is ~4 KiB, transiently, per `verify` of a legacy row**,
//! on the calling task (again [AGENTS.md](/AGENTS.md)'s ordering
//! asking for the number) — the eksblowfish key schedule, against Argon2id's
//! 19 MiB above. It is O(in-flight logins) for the same reason, and it shrinks
//! as § 3's loop upgrades the table.
//!
//! bcrypt's truncation is reproduced rather than corrected (§ 5): at most 72
//! bytes of the password are hashed and a NUL ends it. `verify` has to accept
//! exactly the passwords PHP accepted against the same row, so that is the
//! algorithm speaking and not a choice this module made — and it is one more
//! reason every such row is marked for rehash under one that has no such edge.
//!
//! # A stored hash that will not parse throws, where PHP answers `false`
//!
//! `password_verify` answers `false` for a malformed hash and
//! `password_needs_rehash` answers `true`, and both readings hide the same bug:
//! a storage layer handing back a column that is not a hash looks exactly like
//! every user suddenly typing the wrong password. Under
//! `rule:core-api/shape-rules` failure throws
//! and absence is `?T`, and this is failure — the argument is not the thing the
//! parameter names — so both members throw `LogicError`. The refusal is the
//! same either way at the login screen; the difference is whether the operator
//! ever finds out.
//!
//! Neither message quotes the hash. A stored hash is credential-adjacent, and
//! `rule:security/secret-qualifier`'s whole subject is values that must not reach a log — so the
//! message names the member and what was wrong with the shape, and never the
//! bytes.
//!
//! This section is the *mechanism*. Which stored values are inside the roster
//! and which are outside it is
//! `rule:security/an-unreadable-stored-hash-throws`
//! , and that is the one home of the boundary: the roster has two entries
//! instead of one, and the property that a wrong column wakes an operator is
//! unchanged by that.
//!
//! # `verify` bounds what the stored hash may ask for
//!
//! A PHC string carries its own `m` parameter and `verify` honours it, because
//! that is the only way a hash made under older parameters verifies at all.
//! That makes the stored column a lever on this process's memory: one row
//! reading `m=4194304` is a 4 GiB allocation requested by data. [`MAX_M_COST`]
//! is the ceiling — a hash asking for more is refused before anything is
//! allocated. It sits far above anything [`params`] will produce and far below
//! anything a machine notices, so it costs nothing to a real deployment and
//! closes a denial of service that would otherwise be one `UPDATE` away.
//!
//! The `t` parameter is the same lever on time: each pass walks the whole
//! table again, so a row reading `t=1000000` over this class's own 19 MiB is
//! over an hour of one core. [`MAX_WORK`] bounds `m` times `t`, the work
//! itself, and is refused before any pass runs.
//!
//! A bcrypt row is the same lever with a different unit: its cost is the base-2
//! log of the round count, so it buys CPU time where `m` bought memory, and it
//! is the same `UPDATE` away. [`MAX_BCRYPT_COST`] is that ceiling, `rule:security/bcrypt-cost-ceiling`'s
//! seventeen — far above the 10 to 13 real PHP deployments write, far below
//! 31's minutes of CPU — and it is read out of the stored string and refused
//! before a single round runs.

use argon2::password_hash::phc::PasswordHash;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version};
use rand::Rng;

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::{
    ClassDoc, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The variant every hash this module writes is made with — RFC 9106 § 4's
/// recommendation for password hashing, and the module doc's home for why.
const ALGORITHM: Algorithm = Algorithm::Argon2id;

/// Argon2's own version, 0x13. Present as a named constant because
/// [`nvs_core_password_needs_rehash`] compares against it: a hash carrying
/// 0x10 was made by a different function under the same name.
const VERSION: Version = Version::V0x13;

/// The memory cost, in KiB — 19 MiB, and the number the module doc's *what
/// they spend* paragraph is about.
const M_COST: u32 = 19 * 1024;

/// The number of passes.
const T_COST: u32 = 2;

/// The number of lanes. One, because the work is already sequential from the
/// caller's point of view and lanes buy latency only where there are cores
/// idle to spend — which, in a thread-per-core runtime serving other requests,
/// is precisely what there are not.
const P_COST: u32 = 1;

/// The largest `m` parameter [`nvs_core_password_verify`] will honour out of a
/// stored hash, in KiB — 1 GiB. The module doc's last section is why there is
/// a ceiling at all.
const MAX_M_COST: u32 = 1024 * 1024;

/// The most work [`nvs_core_password_verify`] will do for a stored hash, as
/// its memory cost in KiB times its passes — 4 GiB of passes, which is
/// libsodium's `SENSITIVE` preset (1 GiB over four passes) and the most
/// expensive configuration a real deployment writes. [`MAX_M_COST`] bounds
/// the table; this bounds how many times it is walked.
const MAX_WORK: u64 = 4 * 1024 * 1024;

/// The salt length in bytes, `password_hash`'s own `RECOMMENDED_SALT_LEN`.
const SALT_LEN: usize = 16;

/// `rule:security/bcrypt-read-roster`'s read roster, second entry: the three tags PHP writes a bcrypt
/// hash under, all one algorithm. `$2x$` is deliberately absent — the module
/// doc's *read roster* section is why — and the list grows by amending that
/// ADR section, never by accepting what a parser happens to read.
const BCRYPT_TAGS: [&str; 3] = ["$2y$", "$2a$", "$2b$"];

/// The largest bcrypt cost [`nvs_core_password_verify`] will run — `rule:security/bcrypt-cost-ceiling`
/// 's seventeen, and [`MAX_M_COST`]'s counterpart for the other entry in the
/// roster.
const MAX_BCRYPT_COST: u32 = 17;

/// Every bcrypt hash is exactly this many ASCII bytes: a four-byte tag, two
/// cost digits, a `$`, and 53 characters of bcrypt's own base64 carrying the
/// 16-byte salt and the 23-byte digest.
const BCRYPT_LEN: usize = 60;

/// The parameters [`nvs_core_password_hash`] writes and
/// [`nvs_core_password_needs_rehash`] measures a stored hash against.
fn params() -> Params {
    // Every argument is a constant in range, so the builder cannot fail here —
    // and `expect` rather than `unwrap_or_default` because a default silently
    // weaker than the constants above is the one outcome this module must not
    // produce.
    Params::new(M_COST, T_COST, P_COST, None).expect("the module's own constants are in range")
}

/// Spec § 16's password hashing, and `rule:core-classes/secret-reveal`'s second escape hatch.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Password",
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "hash",
            names: &["password"],
            params: &[CoreTy::Text(Qual::Reveal)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_password_hash",
            doc: Some(&HASH_DOC),
        },
        CoreMethod {
            name: "verify",
            names: &["password", "hash"],
            params: &[CoreTy::Text(Qual::Reveal), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_password_verify",
            doc: Some(&VERIFY_DOC),
        },
        CoreMethod {
            name: "needsRehash",
            names: &["hash"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_password_needs_rehash",
            doc: Some(&NEEDS_REHASH_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Password`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Stores and checks passwords safely. `hash` turns a password into a string you can \
            save, `verify` checks a password against that string, and `needsRehash` tells you \
            when a saved hash should be made again.",
};

/// `Core\Password::hash`'s reference card — `rule:core-api/reference-card`.
const HASH_DOC: MethodDoc = MethodDoc {
    short: "Turns `$password` into a hash you can store, using Argon2id. The library chooses the \
            settings, so there is no algorithm or cost argument. The result contains everything \
            `verify` needs later: the algorithm, the settings, a random salt and the hash itself.",
    params: &[ParamDoc {
        name: "password",
        desc: "The password to hash. It may be a `secret` string. The result is not secret, \
               because it is made to be stored.",
        shape: &[],
    }],
    ret: "A string such as `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`. Each call uses a new \
          random salt, so the same password gives a different string every time.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The request cannot use the 19 MiB of memory that one hash needs.",
    }],
};

/// `Core\Password::verify`'s reference card — `rule:core-api/reference-card`.
const VERIFY_DOC: MethodDoc = MethodDoc {
    short: "Checks whether `$password` is the password `$hash` was made from. It uses the \
            settings stored in `$hash`, so a hash made with older settings still works. It reads \
            the Argon2id strings `hash` writes, and bcrypt hashes that PHP wrote with `$2y$`, \
            `$2a$` or `$2b$`. The check takes the same time for every wrong password.",
    params: &[
        ParamDoc {
            name: "password",
            desc: "The password somebody typed. It may be a `secret` string.",
            shape: &[],
        },
        ParamDoc {
            name: "hash",
            desc: "The stored hash: a string from `hash`, or a bcrypt hash that PHP stored.",
            shape: &[],
        },
    ],
    ret: "`true` when `$password` matches `$hash`, `false` when it does not.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$hash` is not a password hash this class can read. It does not parse, it \
                   uses another algorithm, or it starts with `$2x$`. This usually means the \
                   stored value is broken, so it is an error and not `false`.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$hash` asks for too much work: more than 1 GiB of memory, too many passes \
                   over that memory, or a bcrypt cost above 17. It is also thrown when the \
                   request cannot use the memory the hash needs.",
        },
    ],
};

/// `Core\Password::needsRehash`'s reference card — `rule:core-api/reference-card`.
const NEEDS_REHASH_DOC: MethodDoc = MethodDoc {
    short: "Checks whether `$hash` is weaker than a hash `hash` makes today. Call it right after \
            `verify` returns `true`. If it returns `true`, hash the password again and store the \
            new hash.",
    params: &[ParamDoc {
        name: "hash",
        desc: "The stored hash to check.",
        shape: &[],
    }],
    ret: "`true` when `$hash` uses another algorithm or version, or less memory or fewer passes \
          than `hash` uses now. Every bcrypt hash returns `true`. `false` when `$hash` is as \
          strong as a new hash, or stronger.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$hash` is not a password hash at all. A hash that uses another algorithm does not \
               throw: it returns `true`.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_password_hash" => (nvs_core_password_hash as *const ()).cast(),
        "nvs_core_password_verify" => (nvs_core_password_verify as *const ()).cast(),
        "nvs_core_password_needs_rehash" => (nvs_core_password_needs_rehash as *const ()).cast(),
        _ => return None,
    })
}

/// The argument at `slot` as text.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member: `string` is the declared type of every
/// parameter this class has, so a value of another tag is a compiled-code bug
/// rather than anything a program can write.
fn text_of<'a>(args: &'a [Value], slot: usize, member: &str) -> Result<&'a str, Fault> {
    args[slot].as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Password::{member} expected a `string`, got tag {}",
            args[slot].tag_byte()
        ))
    })
}

/// The `LogicError` the module doc's *a stored hash that will not parse throws*
/// section specifies — which never quotes the bytes.
///
/// One sentence for both entries of `rule:security/bcrypt-read-roster`'s roster: a value outside it
/// is the same storage bug whichever shape it failed to be, and two messages
/// would ask the operator reading one to work out which parser rejected it.
fn unreadable(member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "Core\\Password::{member}(): $hash is not a stored password hash — the PHC \
             string Core\\Password::hash() writes was expected. The value is not quoted \
             here, because a stored hash does not belong in a log."
        ),
    )
}

/// `stored` parsed as a PHC string, or [`unreadable`].
fn parsed(stored: &str, member: &str) -> Result<PasswordHash, Fault> {
    PasswordHash::new(stored).map_err(|_| unreadable(member))
}

/// The cost `stored` carries, for a stored value inside `rule:security/bcrypt-read-roster`'s bcrypt
/// half of the roster — and `None` for one that is not in it at all, which is
/// the PHC path's to read or to refuse.
///
/// The shape is checked here rather than left to the crate because the cost has
/// to be read *before* any work: it is the one number in a bcrypt string that is
/// data, exactly as `m` is in a PHC one. A value carrying a roster tag and
/// nothing else the shape needs answers `None` and reaches the PHC parser, which
/// refuses it — one refusal site for a stored value that is not a hash, rather
/// than a second sentence saying the same thing about a different parser.
fn bcrypt_cost(stored: &str) -> Option<u32> {
    if !BCRYPT_TAGS.iter().any(|tag| stored.starts_with(tag)) {
        return None;
    }
    let bytes = stored.as_bytes();
    if bytes.len() != BCRYPT_LEN || !stored.is_ascii() || bytes[6] != b'$' {
        return None;
    }
    stored[4..6].parse::<u32>().ok()
}

/// The memory the parameters ask for, in bytes, as [`nvs_runtime::affordable`]
/// wants it — saturating rather than wrapping, so an absurd `m` reaches the
/// policy seam as an absurd count instead of a small one.
fn footprint(m_cost: u32) -> usize {
    usize::try_from(m_cost)
        .unwrap_or(usize::MAX)
        .saturating_mul(1024)
}

/// The work a PHC hash's parameters ask for, in the unit [`MAX_WORK`] is
/// written in: KiB of table times passes over it. Widened first, so no pair of
/// `u32`s can wrap it.
fn work(m_cost: u32, t_cost: u32) -> u64 {
    u64::from(m_cost) * u64::from(t_cost)
}

nvs_runtime::nvs_helper! {
    /// `Core\Password::hash(secret string $password): string` — replacing
    /// `password_hash`, and with it `crypt`.
    ///
    /// The salt is sixteen bytes from [`crate::random::draw`], so the answer
    /// differs on every call for the same password; that is what makes two
    /// users who chose the same password indistinguishable in the store.
    fn nvs_core_password_hash(ctx, args: [1]) {
        let password = text_of(args, 0, "hash")?;

        let mut salt = [0_u8; SALT_LEN];
        crate::random::draw(ctx, |rng| rng.fill_bytes(&mut salt));

        nvs_runtime::affordable(Some(footprint(M_COST)), "Core\\Password::hash()")?;
        let hasher = Argon2::new(ALGORITHM, VERSION, params());
        let hash = hasher.hash_password_with_salt(password.as_bytes(), &salt).map_err(|_| {
            // Unreachable from source with no diagnostic to name: every input
            // this call validates is a constant of the module except the
            // password's own length, whose bound is `MAX_PWD_LEN` — four
            // gibibytes, which no `string` reaches under any memory cap a
            // request runs with. What is left is the block allocator refusing,
            // which is the world saying no rather than the program saying
            // anything, so it stays a throw rather than an `expect`.
            Fault::thrown(
                "Core\\Password::hash(): the password could not be hashed — this process \
                 could not spare the memory the parameters ask for"
            )
        })?;

        Ok(Value::str(NvsStr::new(hash.to_string().as_bytes())))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Password::verify(secret string $password, string $hash): bool` —
    /// replacing `password_verify`.
    ///
    /// The parameters come from `$hash` and not from [`params`], which is the
    /// whole reason a hash written years ago still verifies; the module doc's
    /// last section is why that is bounded rather than honoured unconditionally.
    fn nvs_core_password_verify(_ctx, args: [2]) {
        let password = text_of(args, 0, "verify")?;
        let stored = text_of(args, 1, "verify")?;

        // `rule:security/bcrypt-read-roster`'s second entry, read before the PHC parser sees the
        // string: a bcrypt hash is not a PHC one, and the tag is what says so.
        if let Some(cost) = bcrypt_cost(stored) {
            // § 4's ceiling, and the whole of "before any work" — the rounds
            // are `2^cost`, so this is the same denial of service `m` was, one
            // doubling at a time.
            if cost > MAX_BCRYPT_COST {
                return Err(Fault::thrown(format!(
                    "Core\\Password::verify(): $hash asks for a bcrypt cost of {cost}, past \
                     this class's {MAX_BCRYPT_COST} ceiling — no hash PHP writes needs that, \
                     and each step past it doubles the work"
                )));
            }
            return match bcrypt::verify(password.as_bytes(), stored) {
                Ok(matched) => Ok(Value::bool(matched)),
                // The shape passed `bcrypt_cost` and the crate still could not
                // read it — a salt or a digest that is not this alphabet's, or
                // a cost below the algorithm's own floor. Same class as any
                // other stored value that is not a hash, and the same sentence.
                Err(_) => Err(unreadable("verify")),
            };
        }

        let hash = parsed(stored, "verify")?;

        // Read before anything is allocated: `m` arrives inside the stored
        // string, so this is the one number in the class that is data rather
        // than a constant. Parameters this class cannot read are left to the
        // verification below to refuse — they allocate nothing on the way
        // there, and folding them in here would put two judgements behind one
        // message.
        if let Ok(asked) = Params::try_from(&hash) {
            if asked.m_cost() > MAX_M_COST {
                return Err(Fault::thrown(format!(
                    "Core\\Password::verify(): $hash asks for {} KiB of memory, past this \
                     class's {MAX_M_COST} KiB ceiling — no hash it writes needs that",
                    asked.m_cost()
                )));
            }
            // The passes are data too, and they buy time the way `m` buys
            // memory — so the product is bounded, not `t` alone, because a
            // cheap pass over a small table is not the work a costly one is.
            if work(asked.m_cost(), asked.t_cost()) > MAX_WORK {
                return Err(Fault::thrown(format!(
                    "Core\\Password::verify(): $hash asks for {} passes over {} KiB, past this \
                     class's ceiling of {MAX_WORK} KiB of passes — no hash it writes needs that",
                    asked.t_cost(),
                    asked.m_cost()
                )));
            }
            nvs_runtime::affordable(Some(footprint(asked.m_cost())), "Core\\Password::verify()")?;
        }

        let verifier = Argon2::new(ALGORITHM, VERSION, params());
        match verifier.verify_password(password.as_bytes(), &hash) {
            Ok(()) => Ok(Value::bool(true)),
            // Every wrong password lands here, so this arm may not distinguish
            // *why* beyond the one case below, which is not about the password
            // at all.
            Err(argon2::password_hash::Error::PasswordInvalid) => Ok(Value::bool(false)),
            // A PHC string that parsed but names something this class does not
            // compute — another algorithm, a version it does not know, or a
            // salt Argon2 refuses. Same class as an unparseable one, because it
            // is the same bug seen one layer in: what came out of the store is
            // not a hash `Core\Password::hash()` wrote.
            Err(_) => Err(Fault::thrown_as(
                ThrownClass::Logic,
                "Core\\Password::verify(): $hash parses as a PHC string but is not one \
                 Core\\Password::hash() wrote — the algorithm, the version or the salt is \
                 not this class's.",
            )),
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Password::needsRehash(string $hash): bool` — replacing
    /// `password_needs_rehash`.
    ///
    /// **Weaker, not different.** PHP compares the stored options against the
    /// current ones for equality, so a hash written under *stronger* settings
    /// than today's asks to be rehashed and is silently downgraded by a program
    /// doing what the member told it to. Here the question is whether the
    /// stored hash has fallen behind: a different algorithm or version, or a
    /// memory or time cost below [`M_COST`]/[`T_COST`]. Lanes are not compared
    /// — `p` divides the same work rather than changing how much of it there
    /// is, so a hash differing only there is neither weaker nor stronger.
    fn nvs_core_password_needs_rehash(_ctx, args: [1]) {
        let stored = text_of(args, 0, "needsRehash")?;

        // `rule:security/needs-rehash-answers-weaker`: a bcrypt row is read rather than thrown at, and every
        // one of them has fallen behind — a different algorithm is weaker by
        // this member's own rule, so the answer needs no comparison. No cost
        // ceiling here: this member runs no rounds, and a row `verify` will
        // refuse is still a row that wants rehashing.
        if bcrypt_cost(stored).is_some() {
            return Ok(Value::bool(true));
        }

        let hash = parsed(stored, "needsRehash")?;

        if hash.algorithm != ALGORITHM.ident() || hash.version != Some(VERSION as u32) {
            return Ok(Value::bool(true));
        }
        // Parameters this class cannot read are parameters it did not write,
        // which is exactly the answer `true` means here — so this one does not
        // throw where [`nvs_core_password_verify`]'s does.
        let Ok(asked) = Params::try_from(&hash) else {
            return Ok(Value::bool(true));
        };

        Ok(Value::bool(asked.m_cost() < M_COST || asked.t_cost() < T_COST))
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, call};

    use super::*;

    /// A bcrypt hash PHP itself wrote, under its own `PASSWORD_BCRYPT` at the
    /// cost it defaults to, for [`PASSWORD`] — frozen here because that is the
    /// point: these tests read a column this tree cannot write, and a fixture
    /// regenerated by our own code would only prove we agree with ourselves.
    const PHP_STORED: &str = "$2y$10$PE6UB/yJ1bk1dIwtgHee0es/SxguHDHJgvcdKauDX66xu84voItOi";

    /// The password [`PHP_STORED`] was made from.
    const PASSWORD: &[u8] = b"correct horse battery staple";

    /// The answer a member gave, or the sentence its throw carried — which is
    /// what a `catch` in a program reads, so it is what these assertions
    /// compare.
    fn answer(function: nvs_runtime::NvsFn, args: &[&str]) -> Result<bool, String> {
        let mut ctx = Ctx::buffered();
        let values: Vec<Value> = args
            .iter()
            .map(|text| Value::str(NvsStr::new(text.as_bytes())))
            .collect();
        match call(function, &mut ctx, &values) {
            Ok(value) => Ok(value.as_bool().expect("both members answer a `bool`")),
            Err(_) => Err(ctx
                .take_pending()
                .map(std::borrow::Cow::into_owned)
                .unwrap_or_default()),
        }
    }

    /// `rule:security/bcrypt-read-roster`'s second roster entry, end to end at the unit: the column a
    /// migrating application arrives with verifies, and the wrong password
    /// answers `false` rather than throwing — the whole point being that a
    /// legacy row is a *readable* hash and not a storage bug.
    // covers: Core\Password::verify
    #[test]
    fn a_php_stored_bcrypt_hash_verifies_and_the_wrong_password_does_not() {
        let offered = std::str::from_utf8(PASSWORD).expect("the fixture is text");
        assert_eq!(
            answer(nvs_core_password_verify, &[offered, PHP_STORED]),
            Ok(true),
            "a hash PHP wrote is one this class reads"
        );
        assert_eq!(
            answer(nvs_core_password_verify, &["hunter2", PHP_STORED]),
            Ok(false),
            "the wrong password is a `false`, not a refusal"
        );
    }

    /// `rule:security/needs-rehash-answers-weaker`: every tag in § 1's roster has fallen behind, because a
    /// different algorithm is weaker by this member's own rule. Asserted over
    /// the whole roster rather than one tag, so a member that grew a comparison
    /// for one spelling fails here.
    // covers: Core\Password::needsRehash
    #[test]
    fn needs_rehash_answers_true_for_every_bcrypt_tag() {
        for tag in BCRYPT_TAGS {
            let stored = format!("{tag}{}", &PHP_STORED[4..]);
            assert_eq!(
                answer(nvs_core_password_needs_rehash, &[&stored]),
                Ok(true),
                "{tag} is bcrypt, and bcrypt is not what `hash` writes"
            );
        }
        assert!(
            !BCRYPT_TAGS.contains(&"$2x$"),
            "the roster is three tags, and the sign-extension one is not among them"
        );
    }

    /// `rule:security/bcrypt-cost-ceiling`'s ceiling, in [`MAX_M_COST`]'s shape: the cost is data out
    /// of the store and `2^cost` rounds is the denial of service, so it is read
    /// and refused before a single round runs. Both sides of the bound are
    /// named — 17 is work this class does, 18 is work it will not.
    #[test]
    fn a_bcrypt_cost_past_the_ceiling_is_refused_before_any_work() {
        let ruinous = format!("$2y$31${}", &PHP_STORED[7..]);
        let refusal = answer(nvs_core_password_verify, &["hunter2", &ruinous])
            .expect_err("a cost of 31 is minutes of CPU one `UPDATE` away");
        assert!(
            refusal.contains("bcrypt cost of 31") && refusal.contains("17 ceiling"),
            "the sentence names what was asked and what is allowed: {refusal}"
        );
        assert!(
            !refusal.contains(&ruinous[7..]),
            "and never the stored bytes"
        );

        // The last accepted cost is answered rather than refused. It is real
        // work — 2^17 rounds — which is exactly why the ceiling sits here and
        // not higher.
        assert_eq!(
            bcrypt_cost(&format!("$2y$17${}", &PHP_STORED[7..])),
            Some(MAX_BCRYPT_COST),
            "17 is inside the bound this class reads"
        );
        assert!(
            matches!(
                answer(nvs_core_password_verify, &["hunter2", PHP_STORED]),
                Ok(false)
            ),
            "and an ordinary cost is not refused at all"
        );
    }

    /// `rule:security/an-unreadable-stored-hash-throws`: the roster has two entries and everything else still
    /// throws. `$2x$` is a bcrypt tag this class will not read — it exists to
    /// be bug-compatible with `crypt_blowfish`'s sign-extension overflow — and
    /// a PHC string naming another algorithm is the same storage bug one layer
    /// in. Both messages are checked for the one thing they must not carry.
    #[test]
    fn a_2x_hash_and_a_foreign_phc_string_still_throw() {
        let sign_extended = format!("$2x${}", &PHP_STORED[4..]);
        let foreign = "$pbkdf2$v=19$m=19456,t=2,p=1$jJ+hokoSJRsAzYsgfwhV6g\
                       $BGyWXoH11l0/GF4ezLqvtQgPY0T/4fv4DukErq9R0cI";

        for stored in [sign_extended.as_str(), foreign] {
            let refusal = answer(nvs_core_password_verify, &["hunter2", stored])
                .expect_err("outside the roster is a throw, not a `false`");
            assert!(
                refusal.starts_with("Core\\Password::verify(): $hash"),
                "the sentence names the member and the parameter: {refusal}"
            );
            assert!(
                !refusal.contains(&stored[4..]),
                "and never quotes the stored value: {refusal}"
            );
        }
        assert!(
            answer(nvs_core_password_needs_rehash, &[&sign_extended]).is_err(),
            "`$2x$` is outside the roster for the member that measures, too"
        );
    }

    /// A hash this module writes is one `verify` accepts and `needsRehash`
    /// leaves alone — the fixture's first two lines, asked of the bodies
    /// directly so that a parameter change that broke the round trip fails
    /// here rather than in a program leg.
    #[test]
    fn a_fresh_hash_verifies_and_is_not_stale() {
        let hasher = Argon2::new(ALGORITHM, VERSION, params());
        let stored = hasher
            .hash_password_with_salt(b"correct horse battery staple", &[7_u8; SALT_LEN])
            .expect("the module's own parameters hash")
            .to_string();

        let hash = PasswordHash::new(&stored).expect("what this module wrote, it can read");
        assert!(
            hasher
                .verify_password(b"correct horse battery staple", &hash)
                .is_ok()
        );
        assert!(hasher.verify_password(b"hunter2", &hash).is_err());

        let asked = Params::try_from(&hash).expect("the parameters round trip");
        assert!(asked.m_cost() >= M_COST && asked.t_cost() >= T_COST);
        assert_eq!(hash.algorithm, ALGORITHM.ident());
        assert_eq!(hash.version, Some(VERSION as u32));
    }

    /// `hash` called the way a program calls it: the same password twice gives
    /// two different strings, because each call draws its own salt, and both
    /// verify and are current.
    // covers: Core\Password::hash
    #[test]
    fn hash_draws_a_salt_per_call_and_each_answer_verifies() {
        let mut ctx = Ctx::buffered();
        let password = [Value::str(NvsStr::new(PASSWORD))];
        let stored: Vec<String> = (0..2)
            .map(|_| {
                let value = call(nvs_core_password_hash, &mut ctx, &password)
                    .expect("the module's own parameters hash");
                value
                    .as_text()
                    .expect("`hash` answers a `string`")
                    .to_owned()
            })
            .collect();

        assert_ne!(stored[0], stored[1], "each call draws its own salt");
        let offered = std::str::from_utf8(PASSWORD).expect("the fixture is text");
        for hash in &stored {
            assert!(
                hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
                "{hash}"
            );
            assert_eq!(answer(nvs_core_password_verify, &[offered, hash]), Ok(true));
            assert_eq!(answer(nvs_core_password_needs_rehash, &[hash]), Ok(false));
        }
    }

    /// [`MAX_WORK`] on both sides, and the refusal end to end: `t` is data out
    /// of the store like `m`, so a row asking for more passes than the ceiling
    /// allows is refused before one pass runs, and the sentence never quotes
    /// the row. The bound itself is libsodium's `SENSITIVE` preset.
    #[test]
    fn a_stored_pass_count_past_the_work_ceiling_is_refused_before_any_work() {
        assert_eq!(
            work(1024 * 1024, 4),
            MAX_WORK,
            "1 GiB over four passes is the ceiling"
        );
        assert!(
            work(M_COST, 215) <= MAX_WORK,
            "215 passes over 19 MiB is inside it"
        );
        assert!(
            work(M_COST, 216) > MAX_WORK,
            "216 is the first count past it"
        );
        assert!(
            work(u32::MAX, u32::MAX) > MAX_WORK,
            "and the widest pair does not wrap"
        );

        let endless = "$argon2id$v=19$m=19456,t=1000000,p=1$jJ+hokoSJRsAzYsgfwhV6g\
                       $BGyWXoH11l0/GF4ezLqvtQgPY0T/4fv4DukErq9R0cI";
        let refusal = answer(nvs_core_password_verify, &["hunter2", endless])
            .expect_err("a million passes is over an hour of one core");
        assert!(
            refusal.contains("1000000 passes over 19456 KiB"),
            "the sentence names what was asked: {refusal}"
        );
        assert!(
            !refusal.contains("jJ+hokoSJRsAzYsgfwhV6g"),
            "and never the stored bytes"
        );
    }

    /// `rule:core-api/tier-roster` places this class as a `Core\Crypto` primitive, and the
    /// primitive it is over is a *memory-hard* function: a parameter set that
    /// drifted below OWASP's floor would still round-trip above and still pass
    /// every conformance case, because nothing a program can observe says how
    /// expensive a hash was to make.
    #[test]
    fn the_parameters_are_at_owasps_floor() {
        let params = params();
        assert!(params.m_cost() >= 19 * 1024, "at least 19 MiB");
        assert!(params.t_cost() >= 2, "at least two passes");
        assert!(
            MAX_M_COST > params.m_cost(),
            "the verify ceiling admits every hash this class writes"
        );
    }
}
