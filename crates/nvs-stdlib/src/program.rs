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
//! helper to hand anything to. What `T` names here is an interface or a class,
//! and the expansion writes its `new`s without either one's descriptor.
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

use crate::registry::{
    ClassDoc, Const, CoreClass, CoreField, CoreMethod, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// This class's fully-qualified name, in one place so the registry row and
/// every consumer that matches on it cannot drift apart.
pub(crate) const NAME: &str = "Core\\Program";

/// `T` — the interface or class the enumeration is asked for, written at the
/// call site.
/// [`CoreTy::Written`] owns why a variable appearing in no parameter position
/// has to be supplied there.
const T: CoreTy = CoreTy::Written("T");

/// `I` — `implementingWith`'s interface or class, the same type argument `implementing`
/// calls `T`, renamed because that member's second variable is the shape.
const I: CoreTy = CoreTy::Written("I");

/// One row of `implementingWith<I, T>`'s answer: the instance and the one
/// attribute payload satisfying `T`, or `null`.
///
/// A shape in a return type, which `a_shape_is_only_ever_a_whole_parameter`
/// refuses for every other row because no helper can answer with one. This
/// member is expanded by `nvs_types::program`, which builds every row itself,
/// so the row spells the shape for the one reason the registry's return types
/// exist: the card, `nvs meta` and `nvs agent` print what the call answers.
const ROW: CoreTy = CoreTy::Shape(&[&[
    CoreField {
        name: "instance",
        ty: I,
        default: None,
    },
    CoreField {
        name: "attribute",
        ty: CoreTy::Nullable(&T),
        default: None,
    },
]]);

/// The registry row. See [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&PROGRAM_CARD),
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
            name: "implementingWith",
            names: &["member"],
            params: &[CoreTy::Text(Qual::Neutral)],
            defaults: &[Const::Str("")],
            return_ty: CoreTy::Array(&ROW),
            symbol: "nvs_core_program_implementing_with",
            doc: Some(&IMPLEMENTING_WITH_DOC),
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

/// `Core\Program`'s class card — `rule:core-api/reference-card`.
const PROGRAM_CARD: ClassDoc = ClassDoc {
    short: "Information about the whole program. `implementing` returns a new object of every \
            class that implements an interface or extends a class. `implementingWith` returns the \
            same objects, each with one attribute of its class. Novis finds these classes when \
            it compiles the program, so nothing is searched while it runs. `id` returns a text that identifies \
            this version of the program.",
};

/// `Core\Program::implementing`'s reference card — `rule:core-api/reference-card`.
const IMPLEMENTING_DOC: MethodDoc = MethodDoc {
    short: "Returns a new object of every class that is a `T`. `T` is an interface or a class, \
            and you write it between `<` and `>`: `Core\\Program::implementing<Module>()`. When \
            `T` is a class that is not abstract, `T` itself is in the list. Abstract classes are \
            not included. Each class needs a constructor without arguments, or the program does \
            not compile. Novis finds the classes when it compiles the program, so nothing is \
            searched while it runs.",
    params: &[],
    ret: "An `array<T>` with one new object per class, sorted by class name. Every call creates \
          new objects. The array is empty when no class is a `T`.",
    errors: &[],
};

/// `Core\Program::implementingWith`'s reference card — `rule:core-api/reference-card`.
const IMPLEMENTING_WITH_DOC: MethodDoc = MethodDoc {
    short: "Returns the same objects as `implementing<I>()`, each with one attribute of its class. \
            You write the interface or class `I` and the shape `T` of the attribute between `<` \
            and `>`: \
            `Core\\Program::implementingWith<Page, {path: string}>(\"render\")`. Novis reads the \
            attributes when it compiles the program, so nothing is searched while it runs.",
    params: &[ParamDoc {
        name: "member",
        desc: "The name of the method, property or constructor parameter that has the attribute. \
               An empty string reads the attributes of the class itself.",
        shape: &[],
    }],
    ret: "An array with one row per class, sorted by class name. Each row is \
          `{instance: I, attribute: ?T}`. `attribute` is `null` when the class has no attribute \
          of the shape `T`. Two matching attributes on one class do not compile.",
    errors: &[],
};

/// `Core\Program::id`'s reference card — `rule:core-api/reference-card`.
const ID_DOC: MethodDoc = MethodDoc {
    short: "Returns a text that identifies this version of the program. It is a `BLAKE3` hash of \
            all the compiled code and of the environment it was compiled for. The same code on \
            the same server gives the same value on every run.",
    params: &[],
    ret: "64 lowercase hexadecimal characters. The value changes when the code or the environment \
          changes. It is safe to print, because a hash does not show any source code.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: "The program was started without an identity. `nvs run` always gives a program \
               one, so this happens only when a program is started some other way.",
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_program_implementing" | "nvs_core_program_implementing_with" => {
            (expanded_at_compile_time as *const ()).cast()
        }
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
        b"nvs: a `Core\\Program` enumeration reached a runtime helper - `rule:programs/implementing` \
          expands `implementing` and `implementingWith` in `nvs check`, so this is a bug in \
          `nvs-ir`'s lowering rather than in the program\n",
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
    // covers: Core\Program::id
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
    // covers: Core\Program::id
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
