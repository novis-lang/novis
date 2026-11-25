`EnumName::CaseName` **is** an integer constant, inlined at every use site the way a literal `5` is:
nothing is allocated, nothing is refcounted, there is no descriptor, and there is no per-isolate
storage slot for an enum at all (`rule:enums/representation`).

An enum body declares cases, methods and constants, and nothing that needs an object
(`rule:enums/an-enum-body-takes-methods-and-constants`). An `implements` clause is refused with
**`E0218`**. A property or a nested enum inside the body is refused with **`E0220`**, which names a
class for state. A case has no property of its own. A case is a key of an array typed
`array<E, T>`, and of no other array: a plain `array<T>` refuses it with **`E0434`**, whose help
names `array<E, T>` (`rule:types/arrays`).

An enum has no constructor. `new E()` and the constructor reference `E::constructor` are each
**`E0870`** at the enum's name, whose help says a case is written `E::Case`. A program's attribute
declaration may not list `enum` or `case`, so no program attribute is used on an enum or an enum case
(`rule:attributes/an-attribute-is-declared-with-its-sites`).

What a program writes for each job:

| Job | Novis |
|---|---|
| a case's integer | `E::A as int` (or `as uint`) |
| a case from an integer | `$n as E` / `$n as ?E` |
| a case's name | a constant `array<E, string>` table, or a `match` in a method of the enum, which must name every case (`rule:expressions/a-match-over-an-enum-covers-every-case`) |
| a value per case | an `array<E, T>`, as a constant (`rule:classes/a-constant-value-is-folded-when-the-program-compiles`) or built at run time |
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
- **A case as a key of a plain `array<T>`.** It would key by a number another enum shares, and give
  the key back as a `string`. `array<E, T>` keys by the cases of one enum and gives them back as cases.
- **Any key type but an enum.** `array<int, T>` would bring back the integer key domain
  `rule:types/arrays` removed.
- **The case name as the JSON key.** The array keeps no enum identity at run time.
- **A literal that must name every case.** The covering `match` is the total form, and a counting
  array starts empty.
- **An enum-keyed append.** There is no next case to append under.
