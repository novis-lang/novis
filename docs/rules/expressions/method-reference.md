A method reference takes a reference to a declared method, and each receiver form has exactly one way
to write it. On a class it is the bare member: `Core\Str::length`, `Shop::show`, `self::helper`
(early-bound) and `static::helper` (late-bound), with no `(...)`. On an object it is
`$user->getName(...)`. There is no third form — no `::ref`, and no reuse of `::class`, which stays a
class-name-to-string operator, so one token never means two unrelated things.

**The bare form is unambiguous on a class because the casing rules already separate what `::` can
name.** A constant is `SCREAMING_SNAKE_CASE` (`E0113`), a method is `camelCase` (`E0111`), an enum case
or a nested type is `PascalCase`, and a static property carries `$`. So `Shop::show` can only name a
method. Should the parser be unable to tell a bare reference from a member access before the checker
runs, it produces one node and the checker decides by the casing and the class's members.
`Class::method(...)`, `self::method(...)` and `static::method(...)` do not compile, with a fix that
drops the `(...)`, so a reader meets one form per receiver.

**An object keeps `(...)`**, because a property and a method may share a name and `$obj->name` is the
property. `$obj->method` without `(...)` stays a property read.

**A reference that is created and dropped is a compile error.** `Cache::flush;` as a statement is the
one trap the bare form opens — a forgotten call — and it is refused with a fix to `Cache::flush()`. A
reference written where a `string` or another type that is not `callable` is expected is already a
type error, and its message adds the same hint.

Every form above resolves a method at the reference itself rather than at call time, and the checker
records that resolved target under a variant that cannot be mistaken for an ordinary call. Two shapes
are refused for having no method to resolve: `new C(...)`, because `new` names a class and a method
reference builds a callable carrying a callee rather than an allocation — write `fn (): C => new C(…)`,
which also says which arguments the construction takes; and `$m->method(...)` on a `mixed` receiver,
because a callable outlives the site and there is no class present to read a callee off.

A `callable` may carry its signature. The resolvability this rule is after comes from the reference at
the value's creation site rather than from the static type, so it holds whether or not the slot being
filled declares one.

**Declined:**
- **Keeping `Class::method(...)` beside the bare form.** Two ways to write one reference is surface
  with nothing behind it, and the bare form is the one Java and Kotlin readers already know.
- **Dropping `(...)` on an object too.** `$obj->name` is the property when a property and a method
  share the name, so the bare form would read differently depending on the class's other members.
