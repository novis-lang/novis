//! `Core\Password` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 16's three members, and the second of exactly two operations that take a
//! `secret` and answer something that is not one.
//!
//! [ADR 0051](../../../../docs/adr/0051-standard-library-tiers.md) § 3 places
//! this class in `Core` by tests 1 and 2 — "as a `Core\Crypto` primitive over a
//! `secret`" — rather than in
//! [ADR 0060](../../../../docs/adr/0060-application-security-protocols.md)'s
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
//! the calling task** ([AGENTS.md](../../../../AGENTS.md)'s priority ordering
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
//! # A stored hash that will not parse throws, where PHP answers `false`
//!
//! `password_verify` answers `false` for a malformed hash and
//! `password_needs_rehash` answers `true`, and both readings hide the same bug:
//! a storage layer handing back a column that is not a hash looks exactly like
//! every user suddenly typing the wrong password. Under
//! [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) failure throws
//! and absence is `?T`, and this is failure — the argument is not the thing the
//! parameter names — so both members throw `LogicError`. The refusal is the
//! same either way at the login screen; the difference is whether the operator
//! ever finds out.
//!
//! Neither message quotes the hash. A stored hash is credential-adjacent, and
//! ADR 0033's whole subject is values that must not reach a log — so the
//! message names the member and what was wrong with the shape, and never the
//! bytes.
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

use argon2::password_hash::phc::PasswordHash;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version};
use rand::Rng;

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual};

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

/// The salt length in bytes, `password_hash`'s own `RECOMMENDED_SALT_LEN`.
const SALT_LEN: usize = 16;

/// The parameters [`nvs_core_password_hash`] writes and
/// [`nvs_core_password_needs_rehash`] measures a stored hash against.
fn params() -> Params {
    // Every argument is a constant in range, so the builder cannot fail here —
    // and `expect` rather than `unwrap_or_default` because a default silently
    // weaker than the constants above is the one outcome this module must not
    // produce.
    Params::new(M_COST, T_COST, P_COST, None).expect("the module's own constants are in range")
}

/// Spec § 16's password hashing, and ADR 0033 § 3's second escape hatch.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Password",
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

/// `Core\Password::hash`'s reference card — ADR 0117.
const HASH_DOC: MethodDoc = MethodDoc {
    short: "Hashes `$password` for storage with Argon2id under parameters this library chooses, \
            answering the PHC string that carries the algorithm, the version, the cost and the \
            salt alongside the digest. There is no algorithm or cost argument: `needsRehash` is \
            how a stored hash learns it has fallen behind.",
    params: &[ParamDoc {
        name: "password",
        desc: "The password to hash. A `secret` is accepted here and the answer is not one — \
               storing the hash is the point.",
        shape: &[],
    }],
    ret: "The PHC string to store, as in `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<digest>`. Two \
          calls with the same password answer differently, because each draws its own salt.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "This process cannot spare the ~19 MiB the parameters ask for.",
    }],
};

/// `Core\Password::verify`'s reference card — ADR 0117.
const VERIFY_DOC: MethodDoc = MethodDoc {
    short: "Reports whether `$password` is the one `$hash` was made from, recomputing under the \
            parameters `$hash` itself carries so that a hash written under older settings still \
            verifies. The comparison is constant-time.",
    params: &[
        ParamDoc {
            name: "password",
            desc: "The password offered. A `secret` is accepted; the answer is a `bool` and \
                   carries nothing of it.",
            shape: &[],
        },
        ParamDoc {
            name: "hash",
            desc: "The stored PHC string, as `hash` answered it.",
            shape: &[],
        },
    ],
    ret: "`true` when `$password` produced `$hash`, `false` when it did not.",
    errors: &[
        ErrorDoc {
            error: "LogicError",
            desc: "`$hash` is not a stored hash this class wrote — it does not parse, or it \
                   names another algorithm, version or salt. A storage bug rather than a wrong \
                   password, which is why it is not `false`.",
        },
        ErrorDoc {
            error: "RuntimeError",
            desc: "`$hash` asks for more memory than any hash this class writes could need, or \
                   this process cannot spare what it asks for.",
        },
    ],
};

/// `Core\Password::needsRehash`'s reference card — ADR 0117.
const NEEDS_REHASH_DOC: MethodDoc = MethodDoc {
    short: "Reports whether `$hash` is weaker than what `hash` would write today — a different \
            algorithm or version, or a lower memory or time cost — so that a program can \
            rehash the password it has just verified.",
    params: &[ParamDoc {
        name: "hash",
        desc: "The stored PHC string to measure.",
        shape: &[],
    }],
    ret: "`true` when the stored hash has fallen behind, `false` when it is at or above the \
          current parameters. A hash *stronger* than the current ones answers `false`: \
          rehashing it would lower its cost.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$hash` is not a PHC string at all. A hash that parses but names another \
               algorithm answers `true` here rather than throwing — that is precisely the \
               question this member is asked.",
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

/// `stored` parsed, or the `LogicError` the module doc's *a stored hash that
/// will not parse throws* section specifies — which never quotes the bytes.
fn parsed(stored: &str, member: &str) -> Result<PasswordHash, Fault> {
    PasswordHash::new(stored).map_err(|_| {
        Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "Core\\Password::{member}(): $hash is not a stored password hash — the PHC \
                 string Core\\Password::hash() writes was expected. The value is not quoted \
                 here, because a stored hash does not belong in a log."
            ),
        )
    })
}

/// The memory the parameters ask for, in bytes, as [`nvs_runtime::affordable`]
/// wants it — saturating rather than wrapping, so an absurd `m` reaches the
/// policy seam as an absurd count instead of a small one.
fn footprint(m_cost: u32) -> usize {
    usize::try_from(m_cost)
        .unwrap_or(usize::MAX)
        .saturating_mul(1024)
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
    use super::*;

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

    /// ADR 0051 § 3 places this class as a `Core\Crypto` primitive, and the
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
