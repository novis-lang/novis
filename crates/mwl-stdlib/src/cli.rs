//! `Core\Cli\Text` — the carrier of the terminal sink, and today the whole of
//! `Core\Cli` that exists.
//!
//! [ADR 0088](../../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
//! § 3's table pairs every context with a sink and every sink with a *carrier*:
//! `Core\Html\Markup` under an HTTP request, `Core\Cli\Text` everywhere else.
//! § 5 makes that carrier the return of `Core\Out::capture`, because bytes that
//! have already been through a sink cannot be handed back as a `string` without
//! the next `echo` escaping them a second time. So the carrier had to exist
//! before [`crate::out`] could, and this module is exactly that much of
//! `Core\Cli`.
//!
//! # What a `Text` is, and what it is not
//!
//! One slot, holding the bytes as they came out of the sink. **No member at
//! all**, which is deliberate rather than unfinished: everything a program does
//! with a `Text` today it does by producing one (`Core\Out::capture`) or by
//! writing one out (`echo`, whose row is `mwl_runtime::value_to_string`'s
//! carrier arm). `Text::plain`, `Text::styled`, `Text + Text`, `Cli\Style` and
//! `Cli\Color` are [ADR 0086](../../../../docs/adr/0086-core-cli-terminal-is-a-sink.md)
//! § 2's and land with the rest of `Core\Cli` at M8, together with the sink's
//! own substitution table — the one from § 1 that makes `Text::plain` a
//! constructor which *cannot* produce an injected escape sequence.
//!
//! That ordering is why nothing here substitutes anything. A `Text` this module
//! builds holds bytes the sink already wrote; applying § 1's table to them here
//! would be the second escape § 5 exists to prevent. The substitution belongs
//! at the sink, on the way in, and `echo` does not perform it yet — which is a
//! gap in the *sink*, not in the carrier, and is stated as gap 1 below.
//!
//! # Known gaps
//!
//! 1. **`echo` does not neutralize control bytes yet.** ADR 0086 § 1's table is
//!    unbuilt, so the terminal sink today writes what it is given. When it
//!    lands, nothing in this module changes: a captured `Text` will simply
//!    already hold the neutralized form.
//! 2. **`Core\Cli` itself does not exist** — no `write`, `isTty`, `width`,
//!    `colorDepth` or `displayWidth`, and no `Style`/`Color`/`Stream` beside
//!    this class. Spec § 13 lists them and `docs/plan/m8.md` owns when.

use crate::registry::CoreClass;

/// The class's fully-qualified name, as
/// [`CoreTy::Instance`](crate::registry::CoreTy::Instance) spells it.
///
/// Taken from `mwl_runtime::CARRIER_CLI_TEXT` rather than written again here:
/// the *sink* decides what its carrier is (ADR 0088 § 3), the sink lives in
/// `mwl-runtime`, and `mwl_runtime::value_to_string` renders whatever that
/// constant names. Two spellings could disagree and the render would silently
/// stop happening.
pub(crate) const NAME: &str = mwl_runtime::CARRIER_CLI_TEXT;

/// Spec § 13's `Core\Cli\Text`, as much of it as ADR 0088 § 5 needs — see the
/// module docs for why that is a slot and no members.
pub(crate) const TEXT: CoreClass = CoreClass {
    name: NAME,
    methods: &[],
    instance: &[],
    slots: &["text"],
    constants: &[],
};

/// A `Core\Cli\Text` carrying `text`, which must be a `Tag::Str` value the
/// caller is transferring — the one producer, called by [`crate::out`].
pub(crate) fn built(text: mwl_runtime::Value) -> mwl_runtime::Value {
    crate::instance::build(&TEXT, [text])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `mwl_runtime::CARRIER_TEXT_SLOT` is the index this class's registered
    /// layout gives `text`, and this class's name is one `mwl_runtime` renders
    /// — the two facts that make `echo` of a captured carrier work, and
    /// neither of them checkable from the crate that acts on them.
    #[test]
    fn the_carrier_slot_matches_the_registered_layout() {
        assert_eq!(TEXT.slot("text"), mwl_runtime::CARRIER_TEXT_SLOT);
        assert!(mwl_runtime::is_carrier(NAME));
    }

    /// The carrier holds exactly one slot: `mwl_runtime::value_to_string`
    /// renders slot 0 and nothing else, so a second one would be invisible to
    /// the only consumer there is.
    #[test]
    fn the_carrier_holds_one_slot() {
        assert_eq!(TEXT.slots.len(), 1);
    }
}
