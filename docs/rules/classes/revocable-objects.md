A class opts in with `implements Revocable`, and only a class with no parent class may write it. Every
subclass inherits it, and an abstract root class may implement it. A class with a parent that writes
it is `E_REVOCABLE_BELOW_ROOT`, whose help names the root class. An interface that extends it and an
enum that implements it are `E_REVOCABLE_NOT_A_CLASS`. Because only a root opts in, a variable typed
with a class that is not revocable never holds a revoked object, and that is what keeps every access
through such a type free of a check.

`Revocable` is a compiler-owned global interface, as `Stringable` and `Comparable` are. It brings two
`public` default methods, which a class cannot declare again (`E_REVOCABLE_METHOD_DECLARED`; the one
exception to `rule:classes/interface-default-methods`'s override):

- `revoke(): void` revokes the object. It cannot be undone, and a second call does nothing.
- `isRevoked(): bool` returns whether `revoke()` was called.

Both are intrinsics the runtime runs, not compiled bodies, and a call of either compiles no check.

**Using a revoked object throws `RevokedObjectError`**, a global subclass of `LogicError`, as
`RecursionError` is global, because the runtime throws it on a language operation. These throw: a
property read or write, a method call (a `Stringable` or `Comparable` call is one), `clone`,
`Core\Json::encode`, `serialize()`, a graph copy into a task or a cache, and a `Core\Reflect` read, call
or write. These do not: `isRevoked()`, a second `revoke()`, passing the object, storing it, `==`
(identity, `rule:expressions/equality-semantics`), the type test `is` (`rule:types/type-test`) and
`::class`. `Core\Debug` prints the class name and that the object is revoked, without its properties,
and does not throw, so a program can inspect one. The message names the class and the place of the
first `revoke()`: "This `Blog\Post` object was revoked in `src/Blog/Post.nvs` on line 42. A revoked
object cannot be used."

**Plain arrays, properties and variables keep the reference**, and a use of it throws as above. Only
`Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap` drop a revoked object
(`rule:core-api/collections-drop-revoked-objects`). The collections stay invariant:
`ObjectMap<string, Post>` is not an `ObjectMap<string, Storable>`, because both names would hold one
shared object. `ObjectMap<string, static>` in a parameter or return type is bound at the call, as
`array<static>` is (`rule:statements/static-is-a-member-modifier`).

A static property's default is a compile-time constant (`E0472`), so a class keeps its map in a
static property typed `?Core\ObjectMap<…>` and creates it on first use with `??=`.

`clone` of an object that is not revoked makes a copy that is not revoked and is in no collection.

**How it runs.** The revoked flag is the top bit of the object header's count word, so the header keeps
its size. Every read that compares or reports a count masks the bit, and no revocable object is
immortal. The inline release keeps its one unsigned compare, whose bound becomes the flag bit: a count
with the flag set takes the helper call, which masks. An object that is not revoked runs exactly the
instructions it ran before, and a revoked one pays one helper call per release.

The check is compiled only where the static type could be revocable: a revocable class, an interface,
`mixed`, `object`, a type parameter whose bound does not rule it out, and a union or nullable type
containing one of these. It is `CheckRevoked { object, site }` before a `FieldGet`, a `FieldSet`, a
`CallVirtual`, a direct call with a receiver and a `Clone`, and erased access through `mixed` is checked
inside the runtime's erased entries. It is one load and one predicted branch on the header line a
method call already reads, with a cold call to the thrower. `$this` is checked too, because a method
may call `revoke()` and then read a property.

**Memory.** Each revocable object carries one hidden 8-byte slot, the first of its root class, so it has
one offset in the whole tree. Each object in a collection costs one membership of about 16 to 24 bytes
per collection (`rule:core-api/collections-drop-revoked-objects`). All of it is on the request heap,
charged to the request and freed with it: O(in-flight), never O(requests served).

**Trade-offs.** Security: a use after delete goes from silent to a loud error. Latency: an access
through an interface, `mixed` or a revocable class pays one predicted branch, and every other access
pays nothing. Simplicity: one interface, two methods and one error for the user, and one flag bit and
one hidden slot in the runtime.

**Declined:**
- **Weak references.** They answer whether an object is still in memory, not whether it is still valid.
- **Automatic removal from plain arrays.** A value would change with no assignment, and finding the
  holders costs a back-pointer per array store or a heap walk per delete.
- **A `revoke` callable only from inside the class.** A public `delete()` already lets any holder end
  the row, so the restriction adds a rule and nothing else.
- **`implements Revocable` below the root.** Every access through a class type would need the check.
- **A `Disposable` or scope-exit hook.** `rule:classes/no-destructors` declines it.
- **An `ObjectArray` class.** The name promises `[]`, an array's `foreach` and `Core\Arr`, which it
  cannot have, and `ObjectMap<string, T>` already has the shape.
- **`ArrayAccess`-style hooks.** `rule:iteration/no-magic-collection-interfaces` declines them.
- **Lazy removal with a check on every read.** It makes `count()` O(n) after a revoke. Eager removal
  keeps reads on large collections fast.
