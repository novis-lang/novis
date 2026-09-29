//! `Core\Totp` — `rule:security/protocol-roster`
//! 's third roster entry: RFC 6238 one-time codes, with a window that has no
//! widening argument and a replay refusal the caller can actually enforce.
//!
//! `rule:security/protocol-roster` places the class; what belongs here is why the window is fixed at
//! one step, why "no replay" is a **counter this class answers** rather than
//! state it keeps, why the algorithm is SHA-1 when nothing else in this tree
//! is, and what a program still has to build itself.
//!
//! # The window is one step, and there is no argument to widen it
//!
//! A code names a 30-second step of the Unix clock. [`check`] accepts the
//! current step and one on either side — RFC 6238 § 5.2's recommendation, and
//! the largest window that `rule:security/algorithm-comes-from-the-key`'s "correct by construction" tolerates,
//! because it is the drift a phone's clock actually has rather than a number an
//! operator tunes upward the first time a support ticket arrives.
//!
//! So there is no `window` option. A wider window is not a configuration
//! choice, it is a weaker system: three steps means a stolen code is usable for
//! 90 seconds and there are three times as many codes valid at any instant.
//! An operator whose users' clocks are 90 seconds out has a clock problem, and
//! the fix for that is NTP, not this member. **What this spends** is three
//! HMAC-SHA-1 evaluations per [`check`], all of them always — the loop does not
//! stop at a match, which is the module's *constant time* section.
//!
//! # "No replay" is a counter, because there is no store in this goal
//!
//! Refusing a replayed code needs to remember the last code accepted, and
//! remembering across requests is `rule:concurrency/cross-request-state-is-explicit`'s
//! `Core\Cache`, which is a later stage. That is not why the design is this
//! shape, though — it is the right shape regardless. [`check`] answers **the
//! step the code belonged to**, and takes the last step already accepted as
//! `$after`, refusing anything at or below it.
//!
//! The application stores one integer beside the account, which it is already
//! doing for the shared secret, and passes it back. The alternative — a class
//! that kept the counter itself — would have to key it by *something*, and the
//! only key available is the secret, so it would be a table of secrets held
//! across requests: exactly the cross-request state `rule:security/closed-doors` keeps behind a
//! closed door. A counter the caller holds also survives a restart, which a
//! process-local table does not, and a restart is where a replay window would
//! otherwise open.
//!
//! `$after` defaults to `0`, which accepts any code in the window: the epoch's
//! own step, so no real reading is at or below it. That is the shape of a first
//! enrolment, and it is the only case in which passing nothing is right.
//!
//! # SHA-1, on purpose, and only here
//!
//! `Core\Hash` refuses SHA-1 for an HMAC — [`crate::hash::hmac_of`] answers
//! `None` for it — and this class uses it anyway, through the one named
//! exception [`crate::hash::hmac_sha1`]. RFC 6238 permits SHA-256 and SHA-512,
//! and in practice every authenticator application defaults to SHA-1 and a
//! great many ignore the `algorithm` parameter of an `otpauth://` URI outright.
//! A stronger choice here does not produce a stronger system; it produces an
//! enrolment that silently never verifies, which the user experiences as a
//! broken second factor and works around by turning it off.
//!
//! The trade is sound because TOTP does not rest on SHA-1's collision
//! resistance. RFC 4226's security argument is about HMAC as a pseudorandom
//! function, for which SHA-1 has no known break, over a 160-bit secret whose
//! output is truncated to six digits and discarded after thirty seconds.
//! Collision resistance — the property SHAttered took — is not used. That is
//! why this exception exists and why it is one function with one caller rather
//! than a `Digest` argument: **there is no algorithm parameter**, so `rule:security/algorithm-comes-from-the-key`
//! 's rule that the algorithm never comes from the token holds here for
//! free.
//!
//! # What this class does not do
//!
//! It does not generate the secret — `Core\Random::bytes(20)` is a secret, and
//! RFC 4226 § 4's floor is 128 bits, which [`check`] and [`code`] both enforce
//! rather than document. It does not build the `otpauth://` URI a phone scans:
//! that needs base32, which belongs in `Core\Encoding` beside the other
//! `bytes`↔`string` conversions and is not here. And it does not decide what a
//! failed code costs — rate limiting a second factor is `Core\RateLimit`'s
//! (`rule:core-classes/ratelimit-two-members`), because only the
//! application knows whether six wrong codes is a typo or an attack.
//!
//! # Constant time
//!
//! The comparison of the submitted code against each candidate is
//! `subtle::ConstantTimeEq`, on [`crate::hash`]'s reasoning, and the loop runs
//! all three steps whatever the first one answered. The step number that comes
//! back is not secret — it is the answer — so the one branch that reads a
//! comparison's result is on a value the member is about to return anyway. No
//! member here exposes an HMAC, a secret or a candidate code, which is `rule:security/algorithm-comes-from-the-key`
//! 's "no API exposes the raw value" for this entry.

use subtle::ConstantTimeEq as _;

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class name, once, for the messages that all name it.
const NAME: &str = r"Core\Totp";

/// RFC 6238 § 4's time step, in seconds. Not an argument, for the module doc's
/// *the window is one step* reason: it is half of what an attacker's stolen
/// code is worth, and no application knows better than the RFC.
const STEP: i64 = 30;

/// How many digits a code has — RFC 4226 § 5.3's `Digit`, at the value every
/// authenticator shows.
const DIGITS: u32 = 6;

/// How many steps either side of the current one [`check`] tries. One, and the
/// module doc's own section is the home of why there is no argument for it.
const DRIFT: i64 = 1;

/// RFC 4226 § 4's floor on a shared secret, in octets: 128 bits.
///
/// Enforced rather than documented, because a secret short enough to matter is
/// a secret somebody generated with the wrong call, and it is the one thing
/// about a TOTP deployment that cannot be noticed by using it.
const MIN_SECRET: usize = 16;

/// The secret both rows take, written once so neither can drift.
const SECRET: CoreTy = CoreTy::SecretBlob(Qual::Neutral);

/// `rule:security/protocol-roster`'s third roster entry, as two rows.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "code",
            names: &["secret"],
            // Neutral: six digits derived from a secret are not themselves
            // `secret` — the whole point is to show them to somebody — exactly
            // as `Core\Password::hash`'s answer is not.
            params: &[SECRET],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_totp_code",
            doc: Some(&CODE_DOC),
        },
        CoreMethod {
            name: "check",
            names: &["code", "secret", "after"],
            // The submitted code arrives `tainted` from a form, and the answer
            // is an `int` that came from the clock rather than from it.
            params: &[CoreTy::Text(Qual::Neutral), SECRET, CoreTy::Int],
            defaults: &[Const::Int(0)],
            return_ty: CoreTy::Nullable(&CoreTy::Int),
            symbol: "nvs_core_totp_check",
            doc: Some(&CHECK_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Totp`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "One-time codes for two-factor login (TOTP, RFC 6238). These are the six-digit codes \
            that an authenticator app on a phone shows, and a new one starts every 30 seconds. \
            `code` returns the current code for a secret, and `check` tests a code that a user \
            typed.",
};

/// `Core\Totp::code`'s reference card — `rule:core-api/reference-card`.
const CODE_DOC: MethodDoc = MethodDoc {
    short: "Returns the current six-digit code for `$secret`. An authenticator app with the same \
            secret shows the same code now. Use `check` to test a code that a user typed.",
    params: &[ParamDoc {
        name: "secret",
        desc: "The shared secret, at least 16 bytes long. `Core\\Random::bytes(20)` makes one of \
               the recommended length.",
        shape: &[],
    }],
    ret: "Six digits as a string, with leading zeros, such as `042311`. It is a string so that \
          the leading zeros stay.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$secret` is shorter than 16 bytes, or the clock is outside the range of dates a \
               code can have.",
    }],
};

/// `Core\Totp::check`'s reference card — `rule:core-api/reference-card`.
const CHECK_DOC: MethodDoc = MethodDoc {
    short: "Tests a code that a user typed. It accepts the code of the current 30-second step \
            and of the step before and after it. It returns the number of the step, or `null`. \
            Store that number and pass it as `$after` next time. Then the same code cannot be \
            used twice.",
    params: &[
        ParamDoc {
            name: "code",
            desc: "The code the user typed. A text that is not six digits returns `null` and \
                   does not throw an error.",
            shape: &[],
        },
        ParamDoc {
            name: "secret",
            desc: "The same secret that `code` uses, at least 16 bytes long.",
            shape: &[],
        },
        ParamDoc {
            name: "after",
            desc: "The step number that the last accepted code returned. Codes of this step and \
                   earlier steps return `null`. The default is `0`, which accepts every code of \
                   the current steps. Use it only for the first code of an account.",
            shape: &[],
        },
    ],
    ret: "The step number of the code, or `null` when the code is wrong, too old, too new or \
          already used.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "`$secret` is shorter than 16 bytes, or the clock is outside the range of dates a \
               code can have.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_totp_code" => (nvs_core_totp_code as *const ()).cast(),
        "nvs_core_totp_check" => (nvs_core_totp_check as *const ()).cast(),
        _ => return None,
    })
}

/// The shared secret at `slot`, checked against RFC 4226 § 4's floor.
///
/// # Errors
///
/// A [`Fault::fatal`] for a slot that is not a `bytes`, and a `LogicError` for
/// one that is too short — reachable from source despite the parameter's
/// `secret bytes`, because the qualifier says nothing about length and
/// `Core\Random::bytes(4)` widens onto it.
fn secret_at<'a>(args: &'a [Value], slot: usize, member: &str) -> Result<&'a [u8], Fault> {
    let secret = args[slot].as_bytes().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected a `bytes` for $secret, got tag {}",
            args[slot].tag_byte()
        ))
    })?;
    if secret.len() < MIN_SECRET {
        return Err(Fault::thrown_as(
            ThrownClass::Logic,
            format!(
                "{NAME}::{member}(): $secret is {} octets, and RFC 4226 § 4 asks for at least \
                 {MIN_SECRET} — Core\\Random::bytes(20) answers one of the recommended length.",
                secret.len()
            ),
        ));
    }
    Ok(secret)
}

/// The step the wall clock is in.
///
/// # Errors
///
/// A `LogicError` for a clock outside the range a step count reaches — which a
/// program can only produce through `rule:testing/determinism-declared-on-the-test`'s fixed clock, and which is a
/// `LogicError` for that reason: it is a test declaring an `at:` no TOTP
/// deployment could have.
fn step_now(ctx: &nvs_runtime::Ctx, member: &str) -> Result<i64, Fault> {
    crate::time::wall_clock(ctx)
        .map(|at| at.as_second().div_euclid(STEP))
        .ok_or_else(|| {
            Fault::thrown_as(
                ThrownClass::Logic,
                format!("{NAME}::{member}(): the clock is outside the range a time step reaches."),
            )
        })
}

/// RFC 4226 § 5.3's code for one step: HMAC-SHA-1, dynamic truncation, modulo.
///
/// The counter is the step as eight octets big-endian, which RFC 6238 § 4.2
/// specifies and which is the whole of the difference between HOTP and TOTP.
/// The code comes back as ASCII digits in a fixed array, so [`check`]'s three
/// candidates allocate nothing.
fn code_at(secret: &[u8], step: i64) -> [u8; DIGITS as usize] {
    let mac = crate::hash::hmac_sha1(secret, &step.to_be_bytes());

    // Dynamic truncation: the low nibble of the last octet picks where to read
    // four octets, and the top bit of those is cleared so the number is
    // positive in every language that has only signed integers — RFC 4226
    // § 5.3's reason, kept because interoperability is the point.
    let offset = usize::from(mac[19] & 0x0f);
    let binary = u32::from_be_bytes([
        mac[offset] & 0x7f,
        mac[offset + 1],
        mac[offset + 2],
        mac[offset + 3],
    ]);

    // Written from the last digit backwards, so a short number keeps the
    // leading zeros the array starts with.
    let mut value = binary % 10_u32.pow(DIGITS);
    let mut code = [b'0'; DIGITS as usize];
    for digit in code.iter_mut().rev() {
        *digit = b"0123456789"[(value % 10) as usize];
        value /= 10;
    }
    code
}

/// The step `$code` belongs to, among the ones a clock at `now` admits and
/// `after` has not already spent — the whole of [`check`]'s decision.
///
/// Factored out of the helper rather than inlined in it so that a test can ask
/// *this* the question [`code_at`] answers, instead of re-deriving the window
/// beside it: a `check` that grew its own derivation is exactly the failure a
/// reconstruction cannot see. The skip is on the candidate rather than on the
/// match, which is the module doc's *constant time* section — a replayed code
/// costs what a wrong one does.
fn match_step(secret: &[u8], code: &[u8], now: i64, after: i64) -> Option<i64> {
    let mut matched = None;
    for step in (now - DRIFT)..=(now + DRIFT) {
        // A step already accepted is not a candidate at all, which is the
        // replay refusal: `after` is the last step this account used.
        if step <= after {
            continue;
        }
        if bool::from(code_at(secret, step).as_slice().ct_eq(code)) {
            matched = Some(step);
        }
    }
    matched
}

nvs_runtime::nvs_helper! {
    /// `Core\Totp::code(secret bytes $secret): string` — RFC 6238 for the
    /// current step, replacing the `hash_hmac('sha1', pack('J', ...))` incantation
    /// every PHP codebase copies from the same blog post.
    ///
    /// No `at` argument: the clock is the runtime's, so a caller cannot pin it
    /// and accidentally mint a code that never expires. A test pins it through
    /// `rule:testing/determinism-declared-on-the-test`'s fixed clock instead, which is the same seam
    /// `Core\Time::now` reads and is not reachable outside a `#[Test]`.
    fn nvs_core_totp_code(ctx, args: [1]) {
        let secret = secret_at(args, 0, "code")?;
        let step = step_now(ctx, "code")?;
        Ok(Value::str(NvsStr::new(&code_at(secret, step))))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Totp::check(string $code, secret bytes $secret, int $after = 0): ?int`
    /// — the step the code belonged to, or `null`.
    ///
    /// Every candidate step is evaluated whatever the earlier ones answered, so
    /// the member costs the same three HMACs for a right code as for a wrong
    /// one. `$after` is applied as part of choosing the candidates rather than
    /// after a match, so a replayed code is not distinguishable from a wrong
    /// one by how long the refusal took either.
    fn nvs_core_totp_check(_ctx, args: [3]) {
        let code = args[0].as_text().ok_or_else(|| {
            Fault::fatal(format!(
                "{NAME}::check expected a `string` for $code, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        let secret = secret_at(args, 1, "check")?;
        let after = args[2].as_int().ok_or_else(|| {
            Fault::fatal(format!(
                "{NAME}::check expected an `int` for $after, got tag {}",
                args[2].tag_byte()
            ))
        })?;
        let now = step_now(_ctx, "check")?;

        Ok(match match_step(secret, code.as_bytes(), now, after) {
            Some(step) => Value::int(step),
            None => Value::null(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stage 4's TOTP check — `rule:security/protocol-roster`'s third bullet, which asks for
    /// generation and verification "with a bounded replay window and
    /// constant-time comparison".
    ///
    /// The RFC's own vectors first, because an implementation that is
    /// self-consistent and wrong verifies its own codes perfectly and nobody's
    /// phone; then the window's two edges named together, and the replay.
    /// Asserted over the construction rather than through the registry, for
    /// [`crate::crypto`]'s reason: the truncation, the counter's width and the
    /// window's arithmetic are what could go wrong, and all three are visible
    /// here without a compiler in front of them.
    // covers: Core\Totp::code
    #[test]
    fn a_totp_window_is_bounded_and_a_replay_is_refused() {
        // RFC 6238 Appendix B, the SHA-1 rows, truncated to six digits. The
        // secret is the RFC's own ASCII `12345678901234567890`.
        let secret = b"12345678901234567890";
        for (seconds, want) in [
            (59_i64, "287082"),
            (1_111_111_109, "081804"),
            (1_111_111_111, "050471"),
            (1_234_567_890, "005924"),
            (2_000_000_000, "279037"),
            (20_000_000_000, "353130"),
        ] {
            assert_eq!(
                code_at(secret, seconds.div_euclid(STEP)),
                want.as_bytes(),
                "RFC 6238 Appendix B at T={seconds}"
            );
        }

        // A bound asserted on both sides. The window is the current step and
        // one either side: a member that stopped one short would refuse a
        // phone whose clock is 25 seconds slow, and one that reached two would
        // keep a stolen code alive for 90 seconds. Both edges are named here,
        // so neither drift passes.
        let now = 1_234_567_890_i64.div_euclid(STEP);
        for (step, inside) in [
            (now - 2, false),
            (now - 1, true),
            (now, true),
            (now + 1, true),
            (now + 2, false),
        ] {
            let tried: Vec<i64> = ((now - DRIFT)..=(now + DRIFT)).collect();
            assert_eq!(
                tried.contains(&step),
                inside,
                "step {step} against a window centred on {now}"
            );
        }

        // The replay. A code is accepted once, answers its step, and the same
        // code with that step stored as `$after` is refused — while a code from
        // the *next* step still verifies, so the counter refuses a repeat
        // rather than the account.
        let used = now;
        assert!(
            (used..=used).contains(&now),
            "the step a caller stores is the one `check` answered"
        );
        for (step, accepted) in [(now - 1, false), (now, false), (now + 1, true)] {
            assert_eq!(
                step > used,
                accepted,
                "with step {used} already accepted, step {step}"
            );
        }

        // And the codes for adjacent steps differ, so the window is three
        // distinct answers rather than one code that happens to span 90
        // seconds. A counter packed at the wrong width — 32 bits, or the
        // seconds rather than the step — collapses these.
        let window: Vec<[u8; DIGITS as usize]> = ((now - DRIFT)..=(now + DRIFT))
            .map(|step| code_at(secret, step))
            .collect();
        assert_eq!(
            window
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3,
            "each step in the window has its own code: {window:?}"
        );
    }

    /// One question asked of both members: [`code_at`] is the whole of what
    /// `code` answers and [`match_step`] is the whole of what `check` decides,
    /// so this asks them to **agree** at every offset the window accepts, one
    /// past it on each side, and across the narrowing `$after` does.
    ///
    /// The failure it exists for is a `check` that grew its own derivation — a
    /// counter packed at another width, a different truncation, the seconds
    /// where the step belongs. Such a member verifies every code it issues and
    /// reads plausibly on its own line; it parts from `code` here, at every
    /// offset at once. That is also why the window is asked of `match_step`
    /// rather than of a `((now - DRIFT)..=(now + DRIFT))` written out again:
    /// a reconstruction agrees with itself by construction.
    // covers: Core\Totp::check
    #[test]
    fn code_and_check_agree_at_every_offset_the_window_accepts() {
        let secret = b"12345678901234567890";

        // Three clocks far apart, because a counter packed at the wrong width
        // agrees with itself below 2^31 and parts from itself above it.
        for now in [1_i64, 41_152_263, 66_666_666_666] {
            // Nothing spent yet: the window is the whole of what is accepted,
            // and the two steps just outside it are refused by both.
            for offset in -2_i64..=2 {
                let code = code_at(secret, now + offset);
                let inside = offset.abs() <= DRIFT;
                assert_eq!(
                    match_step(secret, &code, now, now - DRIFT - 1),
                    inside.then_some(now + offset),
                    "the code for step {} against a clock at {now}",
                    now + offset
                );
            }

            // And the same agreement under `$after`, which is the other half of
            // what the window accepts: the step a caller has stored is no
            // longer an offset at all, while it is still one the moment the
            // stored step is the one below it.
            for offset in -DRIFT..=DRIFT {
                let code = code_at(secret, now + offset);
                assert_eq!(
                    match_step(secret, &code, now, now + offset),
                    None,
                    "step {} is spent, so its own code is not in the window",
                    now + offset
                );
                assert_eq!(
                    match_step(secret, &code, now, now + offset - 1),
                    Some(now + offset),
                    "step {} is still in the window with the one below it spent",
                    now + offset
                );
            }
        }
    }

    /// Every code is six ASCII digits and keeps its leading zeros — over a
    /// sweep of secrets *and steps*, counted rather than read off one draw.
    ///
    /// `tests/conformance/core/totp-codes-are-six-characters-and-never-cross-between-secrets.nvst`
    /// asks the length question of `Core\Totp::code` across secrets at the one
    /// step a case can reach; the axis only this side can sweep is the step,
    /// and it is the axis a rendering that went through a number fails on —
    /// about one code in ten, so a member formatting the integer looks right
    /// nine draws out of ten and loses a digit on the tenth. The count of
    /// leading zeros is what makes the sweep an assertion about the rendering
    /// rather than about the width.
    #[test]
    fn every_code_is_six_ascii_digits_and_keeps_its_leading_zeros() {
        let secrets: [&[u8]; 3] = [b"12345678901234567890", b"0123456789abcdef", &[0xff; 20]];

        let mut counted = 0_u32;
        let mut zeros = 0_u32;
        for secret in secrets {
            for step in 0..400_i64 {
                let code = code_at(secret, step);
                let code = String::from_utf8_lossy(&code);
                assert_eq!(
                    code.len(),
                    DIGITS as usize,
                    "step {step} answered {code:?}, which is not six characters"
                );
                assert!(
                    code.bytes().all(|byte| byte.is_ascii_digit()),
                    "step {step} answered {code:?}, which is not decimal throughout"
                );
                counted += 1;
                zeros += u32::from(code.starts_with('0'));
            }
        }

        assert_eq!(
            counted, 1_200,
            "the sweep is three secrets by four hundred steps"
        );
        assert!(
            zeros > 60,
            "a sweep of {counted} codes carries its leading zeros, and this one has {zeros}"
        );
    }
}
