`EnumName::CaseName` **is** an integer constant, inlined at every use site the way a literal `5` is:
nothing is allocated, nothing is refcounted, there is no descriptor, and there is no per-isolate
storage slot for an enum at all (`rule:enums/representation`).

An enum body declares cases, methods and constants, and nothing that needs an object
(`rule:enums/an-enum-body-takes-methods-and-constants`). An `implements` clause is refused with
**`E0218`**. A property or a nested enum inside the body is refused with **`E0220`**, which names a
class for state. A case has no property of its own, and is not an array key — index with
`$case as int`.

An enum has no constructor. `new E()` and the constructor reference `E::constructor` are each
**`E0870`** at the enum's name, whose help says a case is written `E::Case`. A program's attribute
declaration may not list `enum` or `case`, so no program attribute is used on an enum or an enum case
(`rule:attributes/an-attribute-is-declared-with-its-sites`).

What a program writes for each job:

| Job | Novis |
|---|---|
| a case's integer | `E::A as int` (or `as uint`) |
| a case from an integer | `$n as E` / `$n as ?E` |
| a case's name | a `match` in a method of the enum |
| every case | a `static` method of the enum returning `array<E>` |
| behaviour and constants | methods and constants in the enum body |
| state, or an interface | a class that takes the enum |

A case that is a constant is free to inline. A case that is a heap reference is inlined only once the
optimiser has proved it immutable.

**Declined:**
- **`::from()` and `::tryFrom()`.** Building a case from its integer is the checked `as`, one
  conversion operator rather than a second API on every enum (`rule:types/conversion`).
- **`->value` and `->name`.** The integer is `as int`, and the name is a `match` or
  `rule:enums/reflection`.
- **A flags enum with `| & ^ ~` on the enum type itself.** Bitwise operators apply to the backing
  integer only (`rule:types/arithmetic`), so flags are combined after `as uint` and converted back
  with the checked `as`.
