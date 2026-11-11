Three spellings accept a `class<T>` operand, and nothing else:

| site | resolves against | lowers to |
|---|---|---|
| `new $cls(...)` | `T`'s constructor | the dynamic-new instruction `new static` already uses |
| `$cls::f(...)` | `T`'s static or instance member roster | a virtual call |
| `$x is $cls` | nothing; the descriptor is the test | a descriptor-valued class test |

Every other operand type keeps `E0496`, with its help naming `as class<T>`. A bare `string` is
therefore still refused at all three sites — one refusal, with a fix the author can take. A
constant read, `$cls::CONST`, is not a site at all: a constant is inlined where it is read, so its
class side is a written name and any value there, a `class<T>` included, is `E0496`.

`new $cls(...)` pays the constructor lookup by name that `new static(...)` already pays, and nothing
more: the descriptor is in hand, so there is no resolution step. `new Dog()` gains no branch, no field
and no wider value.

A `class<T>` may hold an `abstract` class or an interface, and `new static(...)` may run on one
through a static call, so the dynamic `new` **refuses a class with no instances** where it allocates.
It throws a `LogicError` the program can catch, with the sentence `construct` gives such a class:
"`Animal` is `abstract` or an interface, so it has no instances, so it cannot be constructed". Nothing
is allocated and no constructor runs. The allocation reads the class's no-instances bit, the one
`ClassDesc::refusal` asks (`rule:classes/graph-copy`). **What it spends:** one byte load and one
branch per dynamic `new`; a `new` of a class written by name pays nothing, because the checker
already refused that class there. Declined: leaving it to the `FATAL` the first bodiless call raises.
A value of a type that has no instances is a broken promise of the type system, and a program cannot
catch a `FATAL`.

`new $cls(...)` types its arguments against **`T`'s** constructor, exactly as `new static(...)` types
them against the current class's; that is the only signature the site can see, and the value may be
any implementor of `T`. So the site is refused **at the `new`**, naming the subclass, when any
implementor of `T` declares a constructor incompatible with `T`'s (`E0794`). It is deliberately checked at the `new` rather than at the class declaration — a subclass never instantiated
through a class reference is nobody's problem.

`$x is $cls` is the dynamic class test, and it narrows its subject to `T` on the true edge
(`rule:types/narrowing`) — the value it tests holds `T` or an implementor, so the narrowing is what
the reference already promised. No second operator tests a class (`rule:types/one-type-test`).

`$obj->$name` is untouched by any of this: a class reference answers "which class", never "which
member" (`rule:types/property-key-access`).
