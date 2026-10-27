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
//! is granted performs no effect, resolves no name and opens no file, so
//! `rule:security/capability-check-at-the-door` has no door to put a check at —
//! `Core\Cache::memory`'s row one class over is the same declaration for the
//! same reason.
//!
//! # What "at this point in the request" means
//!
//! The **calling namespace**'s grants, narrowed by anything the request or an
//! enclosing isolate already dropped. The namespace's half is known while
//! compiling: `nvs_types::capability::prepare_cap_has` hands the member its
//! `[grants]` line as the prepared word ([`PREPARED_GRANTS`] and the bits of
//! [`grants_mask`]), or every capability where no line matches. The request's
//! half is [`nvs_runtime::capability::granted`], which asks the request's own
//! snapshot, so a `Core\Config::set` that tightened a grant and an isolate that
//! spawned with less are both already in the answer. The answer is `true` only
//! where both say yes.
//!
//! A method reference to `has` is handed no word, so it answers `false`
//! wherever it was written. That fails closed, as `Core\Config::set`'s
//! reference does. **What it spends:** one bit test per call, and no memory.
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

/// The bit `crate::registry::PREPARED_MEMBERS`' argument 0 sets for `has`. The
/// bits below it are [`grants_mask`]'s, the capabilities of the call's
/// namespace. It lies above every other member's word, so a word meant for
/// another member never reads as this one.
pub const PREPARED_GRANTS: i64 = 1 << 32;

/// The capabilities `caps` as the bits of `has`'s prepared word: bit `i` is
/// `Cap::ALL[i]`. The roster has fewer than 32 names, which the assertion
/// below holds while compiling.
#[must_use]
pub fn grants_mask(caps: impl Iterator<Item = Cap>) -> u32 {
    caps.filter_map(|cap| Cap::ALL.iter().position(|each| *each == cap))
        .fold(0, |mask, at| mask | (1 << at))
}

const _: () = assert!(
    Cap::ALL.len() < 32,
    "the roster outgrew `has`'s prepared word"
);

/// Whether the prepared word `word` lists `cap`. The zero word, and any word
/// without [`PREPARED_GRANTS`], lists nothing.
fn word_lists(word: i64, cap: Cap) -> bool {
    word & PREPARED_GRANTS != 0
        && Cap::ALL
            .iter()
            .position(|each| *each == cap)
            .is_some_and(|at| word & (1 << at) != 0)
}

/// `Core\Cap`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "Checks whether the running code has a capability, such as `fs.write`. Code can use it \
            to skip a feature that is not allowed. It never grants a capability.",
};

/// `rule:security/optional-capability-degrades`'s one member, and there is deliberately no second.
///
/// The other class that needs no request, and beside `Core\Env` because
/// the two ask the same operator the same kind of question: what was this
/// process started with, and what was it allowed to do. `rule:security/optional-capability-degrades` is what
/// specifies it — the member exists so that a package which declared a
/// capability *optional* has a branch to take — and [`crate::cap`]'s module
/// doc owns why reporting a grant is not widening one.
// rank: 660
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
    short: "Checks whether the code running here has `$capability` at this point in the \
            request. The check starts from the capabilities in `nvs.toml`. Any capability that \
            the request or an enclosing isolate gave up is removed. This call grants nothing, \
            and the guarded code is still checked when it runs.",
    params: &[ParamDoc {
        name: "capability",
        desc: "A capability name as `nvs.toml` writes it, such as `fs.read` or `net.connect`. \
               A literal name that is not a capability does not compile. A computed name that is \
               not a capability returns `false`.",
        shape: &[],
    }],
    ret: "`true` when the capability is granted for at least something, and `false` otherwise. \
          A `true` does not mean every path or host is allowed. A grant limited to some paths \
          still throws an error for a path outside them.",
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

/// Every capability a call to `class::member` needs, and none for a member that
/// needs none — `rule:security/authority-is-the-enclosing-namespace`'s
/// question, asked of each `Core` call site while compiling.
///
/// Read from [`crate::registry::CAPABILITIES`], so the checker holds no copy of
/// a row, as [`is_capability`] holds none of the roster. A member whose door
/// asks by its argument, such as `Core\IO::open` by its mode, has a row for
/// each capability and needs all of them, because the checker sees the call and
/// not the argument. A member of a class with no row reaches no door, and needs
/// nothing either. The door still asks
/// the operator's grant when it runs (`rule:security/no-runtime-grant`), and
/// `every_door_names_the_capability_its_member_declares` holds each door to its
/// row, so a row that misreports fails a test rather than widening a namespace.
///
/// **What it costs:** one pass over the table per `Core` call site, at compile
/// time only. Nothing is allocated, and nothing is read on the request path.
pub fn needs<'a>(class: &'a str, member: &'a str) -> impl Iterator<Item = Cap> + 'a {
    crate::registry::CAPABILITIES
        .iter()
        .filter(move |(name, found, _)| *name == class && *found == member)
        .filter_map(|(_, _, cap)| *cap)
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
    ///
    /// Argument 0 is the prepared word: the capabilities of the call's
    /// namespace. A capability not in it answers `false` before the request's
    /// grant is read.
    fn nvs_core_cap_has(ctx, args: [2]) {
        let word = args[0].as_int().unwrap_or(0);
        let name = text(&args[1], HAS)?;
        let Some(cap) = Cap::parse(name) else {
            return Ok(Value::bool(false));
        };
        Ok(Value::bool(
            word_lists(word, cap)
                && nvs_runtime::capability::granted(ctx, cap, Scope::Unscoped),
        ))
    }
}
