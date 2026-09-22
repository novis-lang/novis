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

use crate::registry::{CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc, Qual};

/// `rule:core-classes/secret-reveal`'s escape hatch, one row per qualifiable base.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Secret",
    doc: None,
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
    short: "Answers `$value` with the `secret` qualifier dropped, at the one call site where \
            handing the secret over is the point — the one named escape hatch, and the only \
            way a `secret string` reaches a sink that refuses one.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The secret to reveal. A plain `string` is accepted and revealing it is the \
                   identity.",
            shape: &[],
        },
        ParamDoc {
            name: "reason",
            desc: "Why this call site is allowed to see the value, written for the next reader. \
                   Nothing reads it at run time.",
            shape: &[],
        },
    ],
    ret: "The same text, unqualified — still `tainted` if `$value` was, since revealing a \
          secret says nothing about where it came from.",
    errors: &[],
};

/// `Core\Secret::revealBytes`'s reference card — `rule:core-api/reference-card`.
const REVEAL_BYTES_DOC: MethodDoc = MethodDoc {
    short: "`reveal` over `bytes`: answers `$value` with the `secret` qualifier dropped. A \
            separate name because a `Core` member has one signature, and answering \
            `string|bytes` would put a cast at every call site.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The secret bytes to reveal.",
            shape: &[],
        },
        ParamDoc {
            name: "reason",
            desc: "Why this call site is allowed to see the value, written for the next reader. \
                   Nothing reads it at run time.",
            shape: &[],
        },
    ],
    ret: "The same bytes, unqualified — still `tainted` if `$value` was.",
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
