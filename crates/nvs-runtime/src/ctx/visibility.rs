//! Whether a member is reachable from the body a call was written inside —
//! `rule:security/reflection-enforces-visibility`'s check, asked at run time of
//! a value whose class the checker never saw.
//!
//! The twin of `nvs_types::signatures::is_visible_from`, which answers the same
//! question wherever a receiver's class is written down. These two are asked
//! everywhere it is not: an erased call through a `mixed` receiver
//! ([`crate::call_erased_method_from`]) and every acting member of
//! `Core\Reflect\ClassInfo`. The pair has to give the answer that one does, and
//! what makes that possible is that the *level* travels rather than a
//! readable/not bit: `nvs_types::layout` reads the keyword and two bits per
//! member carry it to the descriptor, so a `protected` member is told from a
//! `private` one here.
//!
//! A property and a class constant are each asked for by name, and a method by
//! the row a caller already holds, which is the one asymmetry: a reflective
//! read is a member call and can afford a second name lookup, while an erased
//! dispatch is on the request path and has the row in hand by the time it
//! asks.
//!
//! **What it spends:** nothing per request and nothing per instance. A `public`
//! member is one array read. The walk is reached only by a non-`public` member
//! asked from a class that is not the subject's own, and is one lookup of the
//! site's name in the class table plus one scan of a flattened supertype set
//! the descriptor already holds.

use super::*;

impl Ctx {
    /// Whether property `name` of `subject` may be read or written from a body
    /// inside `site` — `None` for a frame inside no class, which is outside
    /// every class by construction.
    ///
    /// `false` for a property `subject` does not have, so a caller that owes a
    /// misspelling a refusal of its own asks that question first: answering a
    /// typo and a `private` read the same way is the confusion
    /// `Core\Reflect\ClassInfo::get`'s ordering exists to avoid.
    #[must_use]
    pub fn field_is_visible_from(
        &self,
        subject: &ClassDesc,
        name: &str,
        site: Option<&str>,
    ) -> bool {
        let Some(slot) = subject.field_slot(name, 0) else {
            return false;
        };
        subject.field_is_public(slot)
            || self.reaches(
                subject,
                site,
                subject.field_is_protected(slot),
                |desc| desc.field_slot(name, 0).is_some(),
                |desc| desc.field_slot(name, 0) == Some(slot),
            )
    }

    /// Whether `row` — a method table row of `subject`, which is what the
    /// caller holding one has already proved — may be called from a body inside
    /// `site`, on [`Self::field_is_visible_from`]'s terms exactly.
    #[must_use]
    pub fn method_is_visible_from(
        &self,
        subject: &ClassDesc,
        row: &crate::MethodRow,
        site: Option<&str>,
    ) -> bool {
        row.public
            || self.reaches(
                subject,
                site,
                row.protected,
                |desc| desc.method_row(&row.name).is_some(),
                |desc| {
                    desc.method_row(&row.name)
                        .is_some_and(|own| std::ptr::eq(own.code, row.code))
                },
            )
    }

    /// Whether the class constant `name` of `subject` may be read from a body
    /// inside `site`, on [`Self::field_is_visible_from`]'s terms exactly.
    ///
    /// The third door, and the one whose subject is a class rather than an
    /// instance: a constant claims no slot, so the roster is asked by name and
    /// `declares` is the same question one step over. `false` for a constant
    /// `subject` does not have, for the reason the property answer gives — a
    /// caller that owes a misspelling a refusal of its own asks that first.
    #[must_use]
    pub fn constant_is_visible_from(
        &self,
        subject: &ClassDesc,
        name: &str,
        site: Option<&str>,
    ) -> bool {
        let Some(constant) = subject.constant(name) else {
            return false;
        };
        constant.public
            || self.reaches(
                subject,
                site,
                constant.protected,
                |desc| desc.constant(name).is_some(),
                |desc| desc.constant(name).is_some(),
            )
    }

    /// The half of the three answers above that a `public` member never
    /// reaches: where a member of `subject` at this level is reachable from.
    /// `declares` says whether a class has a member of that name at all, and
    /// `same` whether it has that very member: the same slot for a property,
    /// the same code address for a method, and the same name for a constant.
    ///
    /// A `protected` member answers to every class the site *is* — a subclass
    /// inherits the member, so it reaches it — and to an ancestor of `subject`
    /// that declares it, which is the direction a subclass's own `$this->n`
    /// reads from: an ancestor above the declaration has no such member and is
    /// refused. A class outside the hierarchy is refused however it spells its
    /// own properties, which is why the relation is asked and not just
    /// `declares`.
    ///
    /// A `private` member answers to the class that declares it alone. The site
    /// is `subject` or an ancestor of it, the site has that very member, and no
    /// class the site also is has it, so the site did not inherit it. A
    /// subclass is refused its parent's private member on its own instance,
    /// and the parent reaches it on an instance of the subclass. A method a
    /// subclass declares again has its own address, so it is the subclass's.
    /// A `private` constant a subclass declares again over its parent's is
    /// refused to the subclass: a constant row names no declaring class, and
    /// refusing is the safe direction. **What it spends:** one class table
    /// lookup and one scan of the site's flattened supertypes per `private`
    /// access by name, and nothing on a descriptor.
    fn reaches(
        &self,
        subject: &ClassDesc,
        site: Option<&str>,
        protected: bool,
        declares: impl Fn(&ClassDesc) -> bool,
        same: impl Fn(&ClassDesc) -> bool,
    ) -> bool {
        let Some(site) = site else {
            return false;
        };
        if protected && site == subject.name() {
            return true;
        }
        if !protected && !subject.conforms_to_name(site) {
            return false;
        }
        let Some(desc) = self.class_desc(site) else {
            return false;
        };
        #[expect(
            unsafe_code,
            reason = "`class_desc` answers with a pointer into a class table this context shares \
                      ownership of, so the descriptor outlives this borrow, and nothing rewrites \
                      that table while a member reached through it is running"
        )]
        let desc = unsafe { &*desc };
        if !protected {
            return same(desc) && !desc.any_supertype(&same);
        }
        desc.conforms_to_name(subject.name()) || (subject.conforms_to_name(site) && declares(desc))
    }
}
