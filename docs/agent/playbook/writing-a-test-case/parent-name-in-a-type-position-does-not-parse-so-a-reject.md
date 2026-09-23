- **`parent::Name` in a type position does not parse, so a `reject` case naming it pins a
  twenty-error cascade rather than the refusal it meant to.** The type grammar takes `Owner::Name`
  and a bare `Name`; `parent::` is neither, so the parser stops at the `::` and re-reads the rest of
  the method as class members, each with its own diagnostic. Assert what a subclass's *own* name
  gives — `Sub::Id` is `E0405` — and leave `parent::`/`self::`/`static::` to a case about the type
  grammar itself. [until: reviewed 2026-09-17]
