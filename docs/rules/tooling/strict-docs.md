`nvs check` is silent about documentation. `nvs check --strict-docs` reports a **public** member with no
attached doc comment (`rule:tooling/doc-comment-attaches-to-the-next-declaration`); publishing a package
turns it on unconditionally. A private helper is never reported, and neither is an application, at any
setting, unless it asks.

**A member inherits a doc comment.** A public method, constant or property with no `///` of its own passes
when a member of the same kind and name has one on its parent class, on an interface it implements, or on
any class or interface those extend or implement in turn. A method matches a method, a constant a
constant, a static property a static property and an instance property an instance property. An override,
an implementation or a redeclaration means what its ancestor's declaration means, and `nvs doc` shows the
ancestor's comment under it, naming where it comes from. A member that overrides an undocumented one is
reported like any other. A promoted constructor parameter declares a property but carries no doc comment,
so it is neither reported nor inherited from. The ancestors are walked over the class graph the compiler
already built, and the member table records which names of each kind were declared with a doc comment, so
the check is a walk over names it already has.

Why this cannot become the failure mode it is modelled against: an editor that demands a docblock is
answered with a generated one, and a generated docblock is noise nobody reads and everybody deletes. Here
there is nothing to generate. With no `@param` and no `@return`
(`rule:tooling/doc-comment-tags-are-see-and-example`), a synthesized `///` would be empty, so no autofix is
possible and the only way to satisfy the check is to write a sentence. A diagnostic in every project was
rejected for the same reason: the pressure would be answered by `/// Charges the card.` above
`chargeTheCard()`, noise a human typed that will outlive the method's behaviour. Demanding the sentence
again above every implementation of an interface would be that same noise, which is why an override
inherits it.

Until a package manager exists, `--strict-docs` is opt-in only and nothing fires it automatically. If
publishing turns out to want more than "a public member has a comment" — a minimum length, a required first
sentence — that is a lint's design and belongs with the publisher.

**Memory:** nothing at run time. The member table holds one more set of names per member kind per class
(methods, constants, static and instance properties), and `--strict-docs` pays one walk up the class
graph per undocumented public member.

**Declined:**
- **An explicit marker such as `/// @inheritdoc`.** It is a third tag, which
  `rule:tooling/doc-comment-tags-are-see-and-example` refuses, and it is a line that says nothing, which
  is what this rule exists to prevent.
- **Inheriting only from interfaces.** Overriding a documented parent method has the same problem, and
  one rule for both is simpler.
- **Requiring a new sentence above a redeclared constant or property**, on the grounds that the
  redeclaration has its own value. The doc comment says what the member means, and a redeclaration keeps
  that meaning just as an overriding method keeps its contract; one rule for every member is simpler.
