Every member declaration in a class, interface or anonymous-class body writes exactly one of `public`,
`protected` or `private` — properties, class constants and methods alike, including a property carrying
only `readonly` or `lateinit`, and including an interface method, where `public` and `private` mean two
genuinely different constructs. An omission is a hard compile error whose fix hint offers `public`, so
accepting it on ported code preserves behaviour.

**There is no default, because there is nothing to default.** The question "what does an omission mean" has
no answer, and that is the point: nothing in the compiler, the reflection surface or a reader's head has to
hold one. The default an omission would need is a *security* default rather than a style one — an inferred
`public` is the mechanism by which an internal helper becomes public API because someone forgot a word, and
the author who forgot it is exactly the author who did not decide. There is no configuration, no
per-project override and no suppression annotation. A type is already mandatory in the same slots
(`rule:types/declaration`), and a member that must spell its type but may leave its visibility to a rule
would be an inconsistency rather than a smaller rule. Reflection exposes a member's declared visibility
(`rule:security/reflection-enforces-visibility`) and a debug dump annotates every property with it
(`rule:classes/no-magic-methods`); with a default, both would report a level nobody typed.

The cost is keystrokes: every member gains up to nine characters, and a ported PHP class needs one edit
per member that omitted the keyword. The language loses a rule in exchange: there is no default to
document and no table of which omissions are legal. The check is one pass over parsed declarations, and
nothing of it reaches the IR, so it costs nothing at run time.

An enum body has no member slot at all (`rule:enums/no-class-machinery`), so nothing here reaches one, and
a property hook has no visibility slot either. What each level *means* is checked at every access site
where it is written: a `private` member is reached from its own class's bodies, and a `protected` one also
from the classes that extend it. A class constant read, a property access and a method call are each a
diagnostic (`E0471`) outside that reach, and an access through an erased receiver reaches `public` members
only (`rule:types/erased-member-access`). The formatter never inserts the keyword, because a formatter
that changes meaning is not a formatter (`rule:tooling/fmt-never-inserts-visibility`).

**Declined:**
- **PHP's implicit `public`.** It makes the option that is safest to forget the widest one, and it is the
  only rule in this area a reader cannot see in the source.
- **An implicit `private`**, as in Rust and in C++ classes. It is safer, but a ported file would compile
  and quietly change meaning, which is worse than either PHP's rule or an error. Novis diverges from PHP
  loudly or not at all.
- **A warning, or a lint with a suppression annotation.** A construct the language does not allow is
  always an error, a warning is only for code that runs as written, and there is no suppression
  mechanism (`rule:core-api/casing-has-no-suppression`). A rule whose whole value is that the author decided
  is worth nothing if the author can skip it.
- **The keyword on properties only, with methods left PHP-shaped.** Half the rule costs a reader the
  whole rule, because they must still remember which half applies where.
- **Interface methods exempt**, on the argument that only `public` is legal there. That is not true
  here: `public` and `private` are two different constructs in an interface body
  (`rule:classes/interface-private-methods`).
- **A fourth keyword for package- or module-visible.** Novis has no module boundary below the class, and
  inventing one would be a language feature added as a syntax rule.
