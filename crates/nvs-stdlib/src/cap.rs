//! `Core\Cap` — `rule:security/optional-capability-degrades`
//! 's one member, and the whole class: does the code running here hold a
//! capability, right now?
//!
//! It exists for one shape. § 6 splits what a package's manifest asks for into
//! `required` and `optional`, and only a `required` one refuses to build; a
//! package that declared `fs.write` optional compiles wherever it is not
//! granted, and needs a way to take the other branch:
//!
//! ```text
//! if (Core\Cap::has("fs.write")) { $this->persist($key, $value); }
//! ```
//!
//! # It reports, and nothing here widens anything
//!
//! `rule:security/no-runtime-grant` is the rule this class is written under: there is **no runtime
//! grant**, and adding one would spend the property static attribution is built
//! on. So this member reads the effective configuration and answers a `bool`,
//! and the door the guarded branch then reaches asks
//! `nvs_runtime::capability::require` again for itself. A program that lied to
//! itself about the answer gets a refusal at the door exactly as before —
//! which is why there is no `Core\Cap::drop` either: narrowing is
//! `Core\Config::set` and stays there (`rule:config/three-changeability-classes`).
//!
//! That is also why the row in `registry::CAPABILITIES` is `None`. Asking what
//! is granted performs no effect, resolves no name and opens no file, so ADR
//! 0118 § 1 has no door to put a check at — `Core\Cache::local`'s row one class
//! over is the same declaration for the same reason.
//!
//! # What "at this point in the request" currently means
//!
//! The spec's § 15 bullet says the **calling namespace**'s grants, narrowed by
//! anything the request or an enclosing isolate already dropped. The second
//! half is what this reads: [`nvs_runtime::capability::granted`] asks the
//! request's own snapshot, so a `Core\Config::set` that tightened a grant and
//! an isolate that spawned with less are both already in the answer.
//!
//! The first half is not narrowed yet, because there is nothing to narrow by:
//! `rule:security/grants-are-keyed-on-a-namespace`'s per-namespace `[grants]` table has no representation in
//! `nvs_config` — `Capabilities` is one table for the request — and § 4's
//! `E0604` is not on disk either. Until it is, this answers the request's whole
//! grant table, which is the **outer bound** of what any namespace inside it
//! can hold. A package therefore never sees `true` for something the deployment
//! did not grant; what it can see is `true` for something granted to the
//! application but not to the package, and the door refuses that call as it
//! always did. The compile-time half landing is where this member starts
//! answering the narrower question, and nothing about the shape here changes
//! when it does.
//!
//! # The argument is a roster name, checked while compiling
//!
//! A misspelled capability is a branch that silently never runs, so
//! `nvs_types::capability` refuses a *written* one that is not in `rule:security/capability-roster-is-closed`
//! 's table (`E0616`), reading [`is_capability`] rather than a copy of the
//! roster. A name that does not fold to a literal is left to run time and
//! answers `false`: it is not a capability, so nothing holds it, and that is
//! the one answer which cannot push a degrading branch onto the privileged
//! path.
//!
//! The parameter is [`Qual::Neutral`] and not [`Qual::Sink`]. Its content is
//! not an instruction anything executes — it selects a row in a report — and
//! the `bool` carries no byte of it, which is exactly what that mark says. Nor
//! is there anything to protect: the answer is drawn from the operator's own
//! configuration over a closed ten-name roster, so a caller that fed it a query
//! string learns nothing it could not learn by writing all ten names out.

use nvs_config::capability::{Cap, Scope};
use nvs_runtime::{Fault, Tag, Value};

use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc, Qual};

/// The class's own name, for the rosters in [`crate::registry`] that key on it.
pub(crate) const NAME: &str = r"Core\Cap";

/// The member's own name, so that [`is_query`] and the row cannot drift apart.
const HAS: &str = "has";

/// `Core\Cap`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Checks whether the running code has a capability, such as `fs.write`. Code can use it \
            to skip a feature that is not allowed. It never grants a capability.",
};

/// `rule:security/optional-capability-degrades`'s one member, and there is deliberately no second.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[CoreMethod {
        name: HAS,
        names: &["capability"],
        params: &[CoreTy::Text(Qual::Neutral)],
        defaults: &[],
        return_ty: CoreTy::Bool,
        symbol: "nvs_core_cap_has",
        doc: Some(&HAS_DOC),
    }],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Cap::has`'s reference card — `rule:core-api/reference-card`.
const HAS_DOC: MethodDoc = MethodDoc {
    short: "Reports whether the code running here holds `$capability` at this point in the \
            request — the deployment's grant table, narrowed by anything the request or an \
            enclosing isolate already dropped. It grants nothing: the door the guarded branch \
            reaches asks again for itself.",
    params: &[ParamDoc {
        name: "capability",
        desc: "A capability name as `nvs.toml` grants it under — `fs.read`, `net.connect`. A \
               written name outside that roster is a compile error, and a computed one that is \
               not a capability answers `false`.",
        shape: &[],
    }],
    ret: "`true` where the capability is granted for something, `false` otherwise. A `true` is \
          not a promise about a particular path or host: a scoped grant still refuses an \
          argument outside it, at the door.",
    errors: &[],
};

/// Whether `class::member` is the capability query — the nominal test
/// `nvs_types::capability` makes against a resolved target, so that the
/// checker holds no copy of either spelling.
#[must_use]
pub fn is_query(class: &str, member: &str) -> bool {
    class == NAME && member == HAS
}

/// Whether `name` is one of `rule:security/capability-roster-is-closed`'s roster names.
///
/// The roster itself is `nvs_config::capability::Cap`, which is where a grant
/// line is read against it too — one table, so a name that configuration
/// accepts is exactly a name this member answers for.
#[must_use]
pub fn is_capability(name: &str) -> bool {
    Cap::parse(name).is_some()
}

/// Every roster name, comma-separated, for the help line of the refusal above.
///
/// Ordered as `Cap::ALL` is, which is § 8's table order rather than an
/// alphabetical one: an operator reading it is looking for the row they meant.
#[must_use]
pub fn roster() -> String {
    Cap::ALL
        .iter()
        .map(|cap| cap.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_cap_has" => (nvs_core_cap_has as *const ()).cast(),
        _ => return None,
    })
}

/// One `string` argument, or the engine fault a wrongly-tagged one is: the
/// signature is checked at compile time, so a bad tag here is a lowering bug
/// and not something a program can provoke.
fn text<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Cap::{member} expected {:?} for its capability, got tag {}",
            Tag::Str,
            value.tag_byte()
        ))
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Cap::has(string $capability): bool` — `rule:security/optional-capability-degrades`'s query.
    ///
    /// Asked [`Scope::Unscoped`], which is the honest scope for the question
    /// the caller asked: they named a capability and no argument to place
    /// inside it. `nvs_config::Capabilities::allows` answers `true` there for a
    /// scoped grant of any width, so `has("fs.write")` reports that the
    /// deployment granted writing *somewhere* — the roots it named are still
    /// compared at the door, and this member has no path to compare them
    /// against. The alternative, answering `false` for every list-shaped grant,
    /// would make the member useless to the one caller § 6 wrote it for.
    ///
    /// A name no capability has answers `false` rather than throwing. The
    /// written case is already `E0616` at compile time, so what reaches this
    /// arm is a computed name — and a program branching on one is asking
    /// whether it may proceed, where an exception is the wrong shape and `true`
    /// would be the wrong answer.
    fn nvs_core_cap_has(ctx, args: [1]) {
        let name = text(&args[0], HAS)?;
        let Some(cap) = Cap::parse(name) else {
            return Ok(Value::bool(false));
        };
        Ok(Value::bool(nvs_runtime::capability::granted(
            ctx,
            cap,
            Scope::Unscoped,
        )))
    }
}
