//! `Core\Secret` — `rule:core-classes/secret-reveal`'s one narrow escape hatch, and nothing else.
//!
//! Every refusal `rule:security/secret-sinks-refuse` states already tells the author to call
//! `Core\Secret::reveal(..., "reason")`; this module is the class that help
//! text names. The mark that makes it work — [`Qual::Reveal`] — has been
//! declared in [`crate::registry`] since the refusals landed, and these are the
//! rows that write it: it is the one mark that **admits** a `secret` argument,
//! and `nvs_types::expr::args` drops the qualifier from the answer because the
//! parameter is declared unqualified.
//!
//! # Why two members rather than one overload
//!
//! `rule:core-classes/secret-reveal` writes `reveal(secret string, string $reason): string` "and a
//! `bytes` overload". The registry has no overloading — one row per member name
//! — and the two alternatives to a second name are both worse. A union
//! parameter would have to answer `string|bytes`, so every call site would pay
//! a cast to get back the type it handed in, at the one call site the ADR wants
//! to read as a single conspicuous line. A type variable would bind to the
//! argument's *qualified* type and hand `secret string` straight back, which
//! compiles and reveals nothing — a silent no-op is the one failure mode this
//! surface cannot have. So `revealBytes` is a name, and it is the only cost.
//!
//! # The reason is checked as a `string` and read by nobody
//!
//! § 3's written reason exists to be read by a human at the call site and by
//! `grep`; nothing consumes it at run time, and the helper never touches the
//! slot. It is [`Qual::Neutral`] rather than unclassified because that is the
//! honest classification — no byte of it reaches the answer — and because a
//! `secret` reason is then refused by the ordinary rule, which is what a
//! program that wrote the credential into its own justification deserves.
//!
//! Deliberately **not** required to be a literal. `rule:core-classes/secret-reveal` asks for a
//! written reason and models the member on `Core\Taint::assertTrusted`, which
//! does not require one either; a rule refusing a `const` holding the reason
//! would buy no confidentiality, since the call is greppable by its own name.

use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc, Qual};

/// `Core\Secret`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "A `secret` value cannot be printed, logged or sent. `reveal()` and `revealBytes()` \
            return it as a plain `string` or `bytes`. Each call needs a reason, which says why \
            this line may use the secret.",
};

/// `rule:core-classes/secret-reveal`'s escape hatch, one row per qualifiable base.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Secret",
    doc: Some(&CARD),
    methods: &[
        CoreMethod {
            name: "reveal",
            names: &["value", "reason"],
            params: &[CoreTy::Text(Qual::Reveal), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Str,
            symbol: "nvs_core_secret_reveal",
            doc: Some(&REVEAL_DOC),
        },
        CoreMethod {
            name: "revealBytes",
            names: &["value", "reason"],
            params: &[CoreTy::Blob(Qual::Reveal), CoreTy::Text(Qual::Neutral)],
            defaults: &[],
            return_ty: CoreTy::Bytes,
            symbol: "nvs_core_secret_reveal_bytes",
            doc: Some(&REVEAL_BYTES_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Secret::reveal`'s reference card — `rule:core-api/reference-card`.
const REVEAL_DOC: MethodDoc = MethodDoc {
    short: "Returns a `secret string` as a plain `string`. Use it only on the line that must \
            print, send or store the secret.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The secret text. A plain `string` is also allowed, and is returned unchanged.",
            shape: &[],
        },
        ParamDoc {
            name: "reason",
            desc: "Why this line needs the secret. It is for the people who read the code, and \
                   the program never reads it. A `secret` value is not allowed here.",
            shape: &[],
        },
    ],
    ret: "The same text, as a `string`. If `$value` was also `tainted`, the result is still \
          `tainted`.",
    errors: &[],
};

/// `Core\Secret::revealBytes`'s reference card — `rule:core-api/reference-card`.
const REVEAL_BYTES_DOC: MethodDoc = MethodDoc {
    short: "Returns `secret bytes` as plain `bytes`, such as a key the program must save. It \
            works like `reveal()`, for `bytes`.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The secret bytes. Plain `bytes` are also allowed, and are returned unchanged.",
            shape: &[],
        },
        ParamDoc {
            name: "reason",
            desc: "Why this line needs the secret. It is for the people who read the code, and \
                   the program never reads it. A `secret` value is not allowed here.",
            shape: &[],
        },
    ],
    ret: "The same bytes, as `bytes`. If `$value` was also `tainted`, the result is still \
          `tainted`.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_secret_reveal" => (nvs_core_secret_reveal as *const ()).cast(),
        "nvs_core_secret_reveal_bytes" => (nvs_core_secret_reveal_bytes as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Secret::reveal(string $value, string $reason): string` — the
    /// identity at run time.
    ///
    /// The whole member is a compile-time judgement: `secret` has no run-time
    /// representation at all (`nvs_types::ty::Ty` carries it and
    /// `nvs_ir::ty::Ty` does not), so there is nothing here to strip. What the
    /// call buys is the checker's admission, and what it costs is the line the
    /// author had to write.
    fn nvs_core_secret_reveal(_ctx, args: [2]) {
        // The answer is the argument itself, and the slot's own reference
        // belongs to the caller for the length of the call — so the reference
        // handed back is a new one.
        #[expect(
            unsafe_code,
            reason = "the argument slot holds a live reference for the length of \
                      the call, which is `Value::retain`'s whole obligation"
        )]
        unsafe {
            args[0].retain();
        }
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Secret::revealBytes(bytes $value, string $reason): bytes` — the
    /// identity at run time, for [`nvs_core_secret_reveal`]'s reason.
    fn nvs_core_secret_reveal_bytes(_ctx, args: [2]) {
        #[expect(
            unsafe_code,
            reason = "the argument slot holds a live reference for the length of \
                      the call, which is `Value::retain`'s whole obligation"
        )]
        unsafe {
            args[0].retain();
        }
        Ok(args[0])
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, NvsFn, NvsStr, StrHeader, Value};

    /// Calls `member` over `value` and a reason, and answers whether the same
    /// allocation came back, with how many owners it had before the call,
    /// after it, and after the caller released the answer. Releases
    /// everything it was handed.
    fn identity_of(member: NvsFn, value: Value, reason: Option<&str>) -> (bool, [usize; 3]) {
        let header: *const StrHeader = value
            .buffer_ptr()
            .expect("the value revealed is a `string` or a `bytes`");
        let reason =
            reason.map_or_else(Value::null, |text| Value::str(NvsStr::new(text.as_bytes())));
        let mut ctx = Ctx::buffered();
        #[expect(
            unsafe_code,
            reason = "`value` owns one reference to `header` until its release below"
        )]
        let before = unsafe { NvsStr::refcount_of(header) };
        let answer = nvs_runtime::call(member, &mut ctx, &[value, reason])
            .expect("revealing a value never throws");
        let same = answer.bits() == value.bits() && answer.tag() == value.tag();
        #[expect(
            unsafe_code,
            reason = "`value` keeps `header` live, and the caller owns the one reference \
                      the member handed back"
        )]
        let (during, after) = unsafe {
            let during = NvsStr::refcount_of(header);
            answer.release();
            (during, NvsStr::refcount_of(header))
        };
        #[expect(
            unsafe_code,
            reason = "the caller built both arguments, and each is released once"
        )]
        unsafe {
            value.release();
            reason.release();
        }
        (same, [before, during, after])
    }

    /// `reveal` is the identity at run time: the answer is the argument's own
    /// allocation with one more owner, and the reason is never read, so a
    /// `null` in its slot is answered the same as a text.
    // covers: Core\Secret::reveal
    #[test]
    fn reveal_hands_back_the_argument_itself_and_reads_no_reason() {
        for reason in [Some("the print-token command shows it once"), None] {
            let value = Value::str(NvsStr::new("hunter2 — ключ 🔑".as_bytes()));
            assert_eq!(
                identity_of(super::nvs_core_secret_reveal, value, reason),
                (true, [1, 2, 1]),
                "`reveal` answers its argument, retained once, with reason {reason:?}"
            );
        }
    }

    /// `revealBytes` is the same identity over a buffer of every octet,
    /// half of which no `string` can carry.
    // covers: Core\Secret::revealBytes
    #[test]
    fn reveal_bytes_hands_back_every_octet_unchanged() {
        let every: Vec<u8> = (0..=255).collect();
        let value = Value::bytes(NvsStr::new(&every));
        assert_eq!(
            value.as_bytes(),
            Some(every.as_slice()),
            "the fixture is the 256 octets"
        );
        let reason = Some("the signer takes the key as bytes");
        assert_eq!(
            identity_of(super::nvs_core_secret_reveal_bytes, value, reason),
            (true, [1, 2, 1]),
            "`revealBytes` answers its argument, retained once"
        );
    }
}
