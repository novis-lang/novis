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

`new $cls(...)` types its arguments against **`T`'s** constructor, exactly as `new static(...)` types
them against the current class's; that is the only signature the site can see, and the value may be
any implementor of `T`. So the site is refused **at the `new`**, naming the subclass, when any
implementor of `T` declares a constructor incompatible with `T`'s (`E0794`). This is stricter than
PHP and never *different* from PHP: every program it accepts, PHP runs the same way. It is
deliberately checked at the `new` rather than at the class declaration — a subclass never instantiated
through a class reference is nobody's problem.

`$x is $cls` is the dynamic class test, and it narrows its subject to `T` on the true edge
(`rule:types/narrowing`) — the value it tests holds `T` or an implementor, so the narrowing is what
the reference already promised. PHP spells this site with the operator Novis refuses
(`rule:php-migration/one-type-test`).

`$obj->$name` is untouched by any of this: a class reference answers "which class", never "which
member" (`rule:types/property-key-access`).
