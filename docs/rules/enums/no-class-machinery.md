`EnumName::CaseName` **is** an integer constant, inlined at every use site the way a literal `5` is:
nothing is allocated, nothing is refcounted, there is no descriptor, and there is no per-isolate
storage slot for an enum at all (`rule:enums/representation`).

An enum body therefore declares only cases and an optional backing type. An `implements` clause is
refused with **`E0218`**; a method, a property, a class constant or a trait use inside the body is
refused with **`E0220`**, which names the class such a member belongs on instead. A case has no
members of its own, and is not an array key — index with `$case as int`.

What stands in for each PHP spelling:

| PHP | Novis |
|---|---|
| `E::A->value` | `E::A as int` (or `as uint`) |
| `E::from($n)` / `E::tryFrom($n)` | `$n as E` / `$n as ?E` |
| `E::A->name` | a `match` of your own |
| `E::cases()` | a `static` method of your own returning `array<E>` |
| methods, constants, `implements` | a class that takes the enum |
| `BackedEnum`, `UnitEnum` | nothing — every enum is integer-backed |

Behaviour over an enum lives on some other class as a `static` method taking the enum, because there
is no standalone-function destination for it to go to.
