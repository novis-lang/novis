`EnumName::CaseName` **is** an integer constant, inlined at every use site the way a literal `5` is:
nothing is allocated, nothing is refcounted, there is no descriptor, and there is no per-isolate
storage slot for an enum at all (`rule:enums/representation`).

An enum body therefore declares only cases and an optional backing type. An `implements` clause is
refused with **`E0218`**; a method, a property, a class constant or a trait use inside the body is
refused with **`E0220`**, which names the class such a member belongs on instead. A case has no
members of its own, and is not an array key — index with `$case as int`.

An enum has no constructor. `new E()` and the constructor reference `E::constructor` are each
**`E0870`** at the enum's name, whose help says a case is written `E::Case`. A user attribute on an
enum or an enum case is `E0871` (`rule:attributes/attach-sites-and-forms`).

What a program writes for each job:

| Job | Novis |
|---|---|
| a case's integer | `E::A as int` (or `as uint`) |
| a case from an integer | `$n as E` / `$n as ?E` |
| a case's name | a `match` of your own |
| every case | a `static` method of your own returning `array<E>` |
| methods, constants, an interface | a class that takes the enum |

Behaviour over an enum lives on some other class as a `static` method taking the enum, because there
is no standalone-function destination for it to go to (`rule:classes/no-free-functions-or-constants`).

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
- **Static methods on a class that the checker treats as belonging to the enum.** It is dedicated
  syntax for what a `static` method taking the enum already does.
