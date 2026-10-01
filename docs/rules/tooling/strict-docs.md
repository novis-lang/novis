`nvs check` is silent about documentation. `nvs check --strict-docs` reports a **public** member with no
attached doc comment (`rule:tooling/doc-comment-attaches-to-the-next-declaration`); publishing a package
turns it on unconditionally. A private helper is never reported, and neither is an application, at any
setting, unless it asks.

**A method inherits a doc comment.** A public method with no `///` of its own passes when it overrides or
implements a method of the same name that has one, on its parent class, on an interface it implements, or
on any class or interface those extend or implement in turn. The override says what its ancestor already
says, and `nvs doc` shows the ancestor's comment under it, naming where it comes from. A property and a
constant inherit nothing, and a method that overrides an undocumented one is reported like any other.

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
