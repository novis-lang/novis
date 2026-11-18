//! Which `Core` members own their subject, and what an owning entry does with
//! the reference it is passed (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! A member owns its first parameter when that parameter is an `array<…>`, a
//! `string` or a `bytes`, and its result is the same kind, nullable or not.
//! The signature decides it, with no list and no opt-out, so the first test
//! reads every registry row and the helper table and names each member whose
//! entry disagrees. `nvs_runtime`'s `owned` module states the protocol the
//! second test holds: an owning entry that throws leaves the subject with its
//! caller, and one that succeeds consumes it.

use nvs_runtime::{Ctx, NvsStr, THROWN, Value, call};
use nvs_stdlib::registry::{CLASSES, CoreMethod, CoreTy};

/// The three kinds a member can own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Array,
    Str,
    Bytes,
}

/// The kind a written type has, through a nullable, an alias and a parameter
/// mark, or `None` for a type that is not one of the three.
fn kind(ty: &CoreTy) -> Option<Kind> {
    match ty {
        CoreTy::Nullable(inner)
        | CoreTy::Alias(_, inner)
        | CoreTy::Keys(_, inner)
        | CoreTy::Dispatched(inner)
        | CoreTy::Bounded(_, inner) => kind(inner),
        CoreTy::Array(_) => Some(Kind::Array),
        CoreTy::Str
        | CoreTy::Text(_)
        | CoreTy::Path(_)
        | CoreTy::Program(_)
        | CoreTy::ClassName(_)
        | CoreTy::StringHoles(..)
        | CoreTy::SecretStr
        | CoreTy::SecretText(_)
        | CoreTy::TaintedStr
        | CoreTy::SecretTaintedStr => Some(Kind::Str),
        CoreTy::Bytes
        | CoreTy::Blob(_)
        | CoreTy::SecretBytes
        | CoreTy::SecretBlob(_)
        | CoreTy::TaintedBytes
        | CoreTy::SecretTaintedBytes => Some(Kind::Bytes),
        _ => None,
    }
}

/// Whether the signature rule makes `method` own its first parameter. The
/// result may be nullable and the parameter may not, because an owning entry
/// takes a subject of its declared kind and a `null` is none of the three.
fn signature_owns(method: &CoreMethod) -> bool {
    method
        .params
        .first()
        .filter(|param| !matches!(param, CoreTy::Nullable(_)))
        .and_then(kind)
        .is_some_and(|subject| kind(&method.return_ty) == Some(subject))
}

#[test]
fn every_member_owns_exactly_what_its_signature_says() {
    let mut wrong = Vec::new();
    for class in CLASSES {
        // An instance member's receiver is argument slot 0, so its first
        // parameter is slot 1.
        for (members, slot) in [(class.methods, 0), (class.instance, 1)] {
            for method in members {
                let owns = signature_owns(method);
                match (owns, nvs_stdlib::owning_entry(method.symbol)) {
                    (false, None) => {}
                    (true, Some(entry)) if entry.slot == slot => {}
                    (true, Some(entry)) => wrong.push(format!(
                        "{}::{} takes its subject from slot {}, and its first parameter is slot {slot}",
                        class.name, method.name, entry.slot
                    )),
                    (true, None) => wrong.push(format!(
                        "{}::{} owns its subject by its signature, and `{}` has no owning entry",
                        class.name, method.name, method.symbol
                    )),
                    (false, Some(_)) => wrong.push(format!(
                        "{}::{} has an owning entry, and its signature does not own its subject",
                        class.name, method.name
                    )),
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} member(s) disagree with the signature rule:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The owning entry of `symbol`, as the JIT is handed it.
fn owned_entry(symbol: &str) -> nvs_runtime::NvsFn {
    let owned = nvs_stdlib::owning_entry(symbol)
        .unwrap_or_else(|| panic!("`{symbol}` has an owning entry"));
    assert!(
        nvs_stdlib::symbols()
            .iter()
            .any(|(name, address)| *name == owned.symbol && *address == owned.address),
        "the JIT's symbol table lists `{}`",
        owned.symbol
    );
    #[expect(
        unsafe_code,
        reason = "an owning entry is a helper compiled with exactly this signature"
    )]
    unsafe {
        std::mem::transmute::<*const u8, nvs_runtime::NvsFn>(owned.address)
    }
}

#[test]
#[expect(
    clippy::disallowed_methods,
    reason = "the test watches one string handle's count, and its bytes are an ASCII literal"
)]
fn an_owning_helper_leaves_a_refused_subject_with_its_caller() {
    let repeat = owned_entry("nvs_core_str_repeat");
    let mut ctx = Ctx::buffered();
    let kept = NvsStr::new(b"ab");

    // A count no string can hold throws in `check`. The reference passed in
    // is still the caller's, so the count is unchanged and the text is intact.
    let passed = Value::str(kept.clone());
    let refused = call(repeat, &mut ctx, &[passed, Value::uint(u64::MAX)]);
    assert_eq!(refused.err(), Some(THROWN));
    drop(ctx.take_pending());
    assert_eq!(kept.refcount(), 2, "a refused call leaves the subject with its caller");
    assert_eq!(passed.as_text(), Some("ab"));
    #[expect(
        unsafe_code,
        reason = "the refused call left this reference with the caller, which releases it once"
    )]
    unsafe {
        passed.release();
    }
    assert_eq!(kept.refcount(), 1);

    // A call that succeeds consumes the reference it was passed.
    let passed = Value::str(kept.clone());
    let result = call(repeat, &mut ctx, &[passed, Value::uint(2)]);
    let result = result.unwrap_or_else(|status| panic!("repeat returned status {status}"));
    assert_eq!(result.as_text(), Some("abab"));
    assert_eq!(kept.refcount(), 1, "a call that succeeds consumes the subject it was passed");
    #[expect(unsafe_code, reason = "the result carries one fresh reference the caller owns")]
    unsafe {
        result.release();
    }
}
