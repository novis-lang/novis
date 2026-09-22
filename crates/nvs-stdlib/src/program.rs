//! `Core\Program` — `rule:programs/implementing`'s program enumeration beside § 6's program identity: one member the
//! compiler answers and one that is the only reason this module runs at all.
//!
//! # Why `implementing<T>()` never runs
//!
//! [`crate::attributes`] is the other class the compiler answers for, and the
//! reason is the same one stated
//! twice: § 3 says `implementing<T>()` "expands, at compile time, to an array
//! literal of `new` expressions — one per non-abstract class in the program
//! implementing `T`". The answer is decided by `nvs_hir::implementors` over a
//! graph the whole program is already in, so there is no lookup left for a
//! helper to perform and `nvs_types::program` is where the expansion happens.
//! The symbol below therefore names a body that exists only so
//! [`crate::symbols`] — `crate::address_of`, in fact — can hand the JIT an
//! address for every registered member without a second rule about which rows
//! are exempt. Reaching it is a compiler bug, and the body says so and aborts
//! rather than answering plausibly.
//!
//! # Why `implementing`'s row declares no parameter at all
//!
//! `implementing<T>()` takes nothing. Its whole input is the type argument,
//! which is why [`CoreTy::Written`] rather than [`crate::registry::CoreTy::Var`]
//! is the variant: a variable inferred from an argument needs a parameter
//! position holding the answer, and this member has none. That makes it the
//! only registered member whose `params` is empty *and* whose signature is
//! generic, so the call spelling a program writes is
//! `Core\Program::implementing<Module>()` and never the bare one — a written
//! type-argument list is not optional on a member that declares one
//! (`E_TYPE_ARGS_MISSING`).
//!
//! It is **not** one of [`crate::registry::WRITTEN_CLASS_MEMBERS`], which
//! `Core\Json::decodeAs` is: that roster puts the *written class's* descriptor
//! in argument slot 0 for a helper to hydrate against, and this member has no
//! helper to hand anything to. What `T` names here is an interface, which has
//! no descriptor at all.
//!
//! # Why `id()` is the one member here that cannot be folded
//!
//! § 6 states the circularity and this is the seam it lands on: folding the id
//! into a unit as a literal would change that unit's bytes, hence its content
//! hash, hence the id just folded. So the answer arrives from the outside.
//! `nvs_config::cache::program_id` combines every unit's content hash in
//! program order with the environment digest where both are in hand, the host
//! writes the result onto the context with
//! [`nvs_runtime::Ctx::set_program_id`] before any Novis code runs, and
//! [`nvs_core_program_id`] hands that string back and hashes nothing. The
//! member is therefore constant within a run by construction rather than by
//! memoization, and the 64 characters it returns are all 32 bytes: a caller
//! wanting eight can take eight, and one handed eight cannot get the rest.

use nvs_runtime::{Fault, NvsStr, ThrownClass, Value};

use crate::registry::{CoreClass, CoreMethod, CoreTy, ErrorDoc, MethodDoc, Qual};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Program";

/// `T` — the interface the enumeration is asked for, written at the call site.
/// [`CoreTy::Written`] owns why a variable appearing in no parameter position
/// has to be supplied there.
const T: CoreTy = CoreTy::Written("T");

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[
        CoreMethod {
            name: "implementing",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&T),
            symbol: "nvs_core_program_implementing",
            doc: Some(&IMPLEMENTING_DOC),
        },
        CoreMethod {
            name: "id",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Text(Qual::Neutral),
            symbol: "nvs_core_program_id",
            doc: Some(&ID_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Program::implementing`'s reference card — `rule:core-api/reference-card`.
const IMPLEMENTING_DOC: MethodDoc = MethodDoc {
    short: "Expands, at compile time, to an array literal of `new` expressions — one per \
            non-abstract class in the program implementing the interface `T` written as the \
            type argument. Nothing runs at run time, and the type argument is never optional: \
            the call is always `Core\\Program::implementing<T>()`.",
    params: &[],
    ret: "One fresh instance per implementing class, as an `array<T>`; an empty array when no \
          class implements `T`.",
    errors: &[],
};

/// `Core\Program::id`'s reference card — `rule:core-api/reference-card`.
const ID_DOC: MethodDoc = MethodDoc {
    short: "This program's identity: `BLAKE3` over every compiled unit's content hash, in program \
            order, combined with the digest of the environment they were compiled for. The same \
            code on the same host answers the same thing on every run, and any change to either \
            answers something else.",
    params: &[],
    ret: "All 32 bytes as 64 lowercase hex characters, never truncated — take a prefix if a \
          shorter one is wanted. It is safe to echo: it is a digest, so it reveals no source, \
          though a reader who watches it can tell when a deployment last changed.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "No host wrote an identity onto this context, so there is nothing to answer with \
               and an invented value would be worse than none — callers key caches on this.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_program_implementing" => (expanded_at_compile_time as *const ()).cast(),
        "nvs_core_program_id" => (nvs_core_program_id as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Program::id(): string` — `rule:programs/program-id`'s identity, read back off the
    /// context the host wrote it onto.
    ///
    /// Every interesting decision is upstream of here, in the module doc: the
    /// formula is `nvs_config::cache::program_id`'s, the moment is program
    /// resolution, and this body neither hashes nor caches nor shortens
    /// anything. What it does decide is the empty case. A context nobody wrote
    /// an id onto throws rather than answering `""`, which is the call
    /// `Core\Command::completions` makes for a context with no program name and
    /// for the same reason: the value's whole use is as a key, and a key that
    /// silently collides across every such context is worse than a refusal that
    /// names what is missing.
    fn nvs_core_program_id(ctx, _args: [0]) {
        let id = ctx.program_id().to_owned();
        if id.is_empty() {
            // no case can reach this: every `nvs run` writes the id onto the
            // context before the program starts, so a `.nvst` case — which is
            // a program with a `nvs run` in front of it — has no way to be
            // handed a context without one. `id_refuses_a_context_no_host_wrote_an_identity_onto`
            // asserts it instead, over a `Ctx` built by hand.
            return Err(Fault::thrown_as(
                ThrownClass::Logic,
                "`Core\\Program::id` has no identity to answer with: no host wrote one onto this \
                 context before the program started"
                    .to_owned(),
            ));
        }
        Ok(Value::str(NvsStr::new(id.as_bytes())))
    }
}

/// The body the row names, and the one this crate hopes is never entered:
/// `rule:programs/implementing` expands the call in `nvs check`, so a call reaching a helper
/// means `nvs-ir` lowered one it should have replaced.
extern "C" fn expanded_at_compile_time() {
    // Written through the handle rather than with `eprintln!`, which this
    // crate's lints refuse — and this is not a `Core` member's output anyway,
    // it is the last thing a process does before aborting.
    use std::io::Write as _;
    let _ = std::io::stderr().write_all(
        b"nvs: `Core\\Program::implementing` reached a runtime helper - `rule:programs/no-runtime-autoload` \xc2\xa7 3 \
          expands every one of them in `nvs check`, so this is a bug in `nvs-ir`'s lowering \
          rather than in the program\n",
    );
    std::process::abort();
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{Ctx, OutputSink, call};

    /// The one path in this module the `.nvst` corpus cannot reach, and the
    /// reason it cannot is the feature working: a `nvs run` writes the identity
    /// onto the context before the program starts, so a case would have to be
    /// run by a host that forgot to. `Ctx::new` is that host.
    ///
    /// What is asserted is the refusal *and* its sentence, because the sentence
    /// is the whole value of refusing: a member that answered `""` here would
    /// hand every such context the same cache key.
    #[test]
    fn id_refuses_a_context_no_host_wrote_an_identity_onto() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        assert!(
            ctx.program_id().is_empty(),
            "a context nobody wrote an identity onto is the fixture"
        );
        assert_eq!(
            call(super::nvs_core_program_id, &mut ctx, &[])
                .expect_err("a context with no identity has nothing to answer with"),
            nvs_runtime::THROWN
        );
        assert_eq!(
            ctx.take_pending().as_deref(),
            Some(
                "`Core\\Program::id` has no identity to answer with: no host wrote one onto this \
                 context before the program started"
            ),
            "the sentence names what is missing rather than what was asked for"
        );
    }

    /// The other half, over the same fixture: an identity a host *did* write is
    /// handed back whole, byte for byte and with nothing removed. 64 characters
    /// go in and 64 come out — this module truncates nothing, which is the one
    /// thing `rule:programs/program-id` forbids it to do.
    #[test]
    fn id_answers_the_identity_the_host_wrote_and_shortens_nothing() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let written = "0".repeat(32) + &"f".repeat(32);
        ctx.set_program_id(written.clone());
        let answer = call(super::nvs_core_program_id, &mut ctx, &[])
            .expect("a context carrying an identity answers it");
        assert_eq!(
            answer.as_str_bytes().expect("`id` answers text"),
            written.as_bytes(),
            "the member hands back what the host wrote, unshortened"
        );

        #[expect(
            unsafe_code,
            reason = "this test owns the one reference `id` answered with, and no \
                      callee has seen it"
        )]
        unsafe {
            answer.release();
        }
    }
}
