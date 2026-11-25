An added member adds a name and changes nothing a type already has: it never replaces, overrides or
hides a member, never satisfies an interface obligation, and only a statically typed call reaches it.
Every case where two members would meet is a compile error naming both places, so whether a call
reaches an added member or a declared one never depends on the receiver's static type.

**Collisions.** Each is **`E1011`**, naming both declarations:

- an added name equal to a member of the type or of an ancestor;
- two blocks in the program adding one name to one type, whether or not one file uses both;
- a class or interface in the program that extends or implements the type and declares the name;
- a later Novis release adding the name to a `Core` type, which turns the block's member into this
  error when the program is next compiled.

The descendant check runs over the whole compiled program, because the program is resolved before any
body is checked. It is never a per-file check: a per-file check would let a call on a subclass reach
the subclass's member in one file and the added one in another.

**Names the runtime calls by name.** An added method, instance or static, may not take the name of a
member of a compiler-declared interface (`Comparable`, `Stringable`, `Parses`, `Iterable`,
`Iterator`, their keyed pair, `PropertyObserver`, `PropertyGuard`, `LateInit` and `Revocable`). An
operator, a conversion to `string`, `foreach` and a route call these by name: `compareTo`,
`toString`, `parse`, `tryParse`, `format`, `iterate`, `advance`, `current`, `key` and the rest. That
is **`E1012`**, checked before the collisions above, and its help says a class of your own implements
the interface instead. The names are read from the interfaces' seeded members, so the list has one
home (`nvs_types::iter_lib`). An added method never satisfies
an interface obligation, so `$x is Comparable` returns what it returned before the block existed.

**Only a statically typed call.** A `mixed` or `object` receiver, a dynamic member name, `Core\Reflect`,
JSON encoding and decoding, `==`, `clone`, the graph copy to a task, the ordering operators and every
`Core` member never see an added member. `Reflect::methods()` and `Reflect::properties()` do not list
one, and `ClassInfo::call` of an added name throws as for an unknown member.

**Unions.** A union receiver finds an added member only when every type in the union gets it from the
same block, which happens through an interface they share. Otherwise the call is **`E1013`**, and its
help says to narrow the value with `is`.

**Restrictions.** A block's body sits, for `rule:packaging/a-restriction-limits-where-a-declaration-is-used`'s purposes, in the block's own
namespace and the block, never in the extended type, so a block cannot call a member restricted on its
type. A `[[restrict]]` entry names an added member by its block, as
`'App\Time\BusinessDays::isBusinessDay'`, and an entry on the extended type does not cover its added
members. A package may restrict only its own blocks, as it may restrict only its own code.

**Deprecation.** `#[Core\Deprecated]` on an added member or on a block warns at each use, through the
same resolved-use pass as every other deprecated member (`rule:attributes/a-deprecation-names-its-replacement-as-code`).

**The call.** An added method compiles to a function whose parameter 0 is the receiver, labelled
`<Block>::<name>`, and every call of it is one direct call. The collision rule makes the target unique,
so no call is virtual and none goes through `Core`'s dispatch. An added static method takes the
extended type's class descriptor in slot 0, as a static method does, and an added method on an enum
takes the case's integer. The checker records every added call as a resolved call, so the IR,
restriction and deprecation passes read it as they read any other.

**What it spends:** the same as calling a static method of a helper class. Nothing per call, and
nothing on any type no loaded block extends.

**Declined:**
- **Retroactive `implements` (`extend Date implements Sortable`).** It changes what `$x is Sortable`
  returns for a value that already exists, and dispatch through the interface would depend on which
  file asks.
- **An added member that overrides a declared one.** Which member runs would depend on whether the
  calling file uses the block.
- **Dispatching an added method through `mixed`.** A run-time lookup would need a table of every
  block on every object, and the answer would depend on which blocks happen to be loaded.
- **The last block loaded wins a collision.** The program's meaning would depend on load order.
