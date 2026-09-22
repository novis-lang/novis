//! `Core\Taint` — `rule:security/launderers-are-sink-named`
//! 's one narrow escape hatch, and nothing else.
//!
//! The `tainted` twin of `crate::secret`, and deliberately the same shape: one
//! call, written where the judgement is made, carrying a reason for the next
//! reader. Every other launderer names one sink; this is the row that names
//! none, and [`crate::registry::Qual`]'s own doc comment is the home of why
//! that is a [`Qual::Launder`] row rather than a sixth mark. This module does
//! not restate that argument.
//!
//! # What it is for, and what it is not
//!
//! § 3's rule is that laundering is sink-named — a value safe for HTML text is
//! not safe for a shell argument, so there is no generic `sanitize()`. This
//! member is the case that rule cannot cover: the developer has validated the
//! value themselves and needs to say so. It is modeled on this project's own
//! `unsafe` policy — forbidden by default, rare, greppable, and never a silent
//! cast. `rule:core-classes/db-capabilities` is where it is
//! load-bearing rather than a fallback: `Settings.host` refuses `tainted` and
//! **has no launderer**, because no string check can establish that a hostname
//! is safe to send credentials to, and a malicious server answers any query
//! with a `LOCAL INFILE` request.
//!
//! # Why there is no `assertTrustedBytes`
//!
//! `crate::secret` has a row per qualifiable base and this class has one, which
//! reads like an omission and is not. § 3 spells a single signature, and the
//! registry holds exactly one `bytes` sink a second row could serve —
//! `Core\Serialize::decode`. That sink has no launderer on purpose: a payload
//! another host chose is precisely the input it exists to refuse, and there is
//! no check a program can perform on such bytes that makes decoding them safe.
//! So the missing row is a decision, and adding one would be an amendment to
//! § 3 rather than a spelling.
//!
//! # One axis, and the two hatches compose in one order
//!
//! [`Qual::Launder`] refuses a `secret` argument — `admits_secret_argument` is
//! `Qual::Reveal`'s alone, and `rule:security/sink-predicate`'s over-strictness there costs a refusal
//! rather than a leak. So a `secret tainted string` does not reach this member
//! at all: it passes `Core\Secret::reveal` first, whose answer is still
//! `tainted`, and this member second. The reverse order does not compile, and
//! that is the honest reading of two independent bits rather than an ordering
//! rule anything had to write down.
//!
//! # The reason is checked as a `string` and read by nobody
//!
//! Exactly as `crate::secret`'s is, for the same reasons and with the same
//! deliberate omission: [`Qual::Neutral`] because no byte of it reaches the
//! answer, and **not** required to be a literal, since a `const` holding the
//! reason is as greppable at the call site as the text would have been.

use crate::registry::{CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc, Qual};

/// `rule:security/launderers-are-sink-named`'s escape hatch, and the one launderer that names no single sink.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: r"Core\Taint",
    doc: None,
    methods: &[CoreMethod {
        name: "assertTrusted",
        names: &["value", "reason"],
        params: &[CoreTy::Text(Qual::Launder), CoreTy::Text(Qual::Neutral)],
        defaults: &[],
        return_ty: CoreTy::Str,
        symbol: "nvs_core_taint_assert_trusted",
        doc: Some(&ASSERT_TRUSTED_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Taint::assertTrusted`'s reference card — `rule:core-api/reference-card`.
const ASSERT_TRUSTED_DOC: MethodDoc = MethodDoc {
    short: "Answers `$value` with the `tainted` qualifier dropped, on the developer's own written \
            authority — the escape hatch for the case no sink-named launderer fits, forbidden by \
            default and greppable by its own name.",
    params: &[
        ParamDoc {
            name: "value",
            desc: "The value being asserted trustworthy. A plain `string` is accepted and \
                   asserting it is the identity.",
            shape: &[],
        },
        ParamDoc {
            name: "reason",
            desc: "What was checked, and why the value can be trusted, written for the next \
                   reader. Nothing reads it at run time.",
            shape: &[],
        },
    ],
    ret: "The same text, with `tainted` gone and nothing else changed. The other axis never arrives \
          here: a `secret` operand is refused outright, so a value carrying both passes \
          `Core\\Secret::reveal` first and this member second.",
    errors: &[],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See `crate::address_of`.
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_taint_assert_trusted" => (nvs_core_taint_assert_trusted as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Taint::assertTrusted(tainted string $value, string $reason): string`
    /// — the identity at run time.
    ///
    /// **The sink it launders for is all of them**, which is the obligation a
    /// `Qual::Launder` row carries and the one place it is answered this way:
    /// `rule:core-api/shape-rules` R11's four grammars, `Core\Db`'s query text, an outbound URL,
    /// a filesystem path, an environment variable's name, and — the case that
    /// earns the row — `Settings.host`, which `rule:core-classes/db-capabilities` leaves with no
    /// launderer of its own. `nvs_stdlib::registry`'s `Qual` doc comment
    /// argues once why naming all of them is allowed here and nowhere else.
    ///
    /// Like `Core\Secret::reveal`, the whole member is a compile-time
    /// judgement: `tainted` has no run-time representation at all
    /// (`nvs_types::ty::Ty` carries it and `nvs_ir::ty::Ty` does not), so
    /// there is nothing here to strip. What the call buys is the checker's
    /// admission, and what it costs is the line the author had to write.
    fn nvs_core_taint_assert_trusted(_ctx, args: [2]) {
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
