`Core\Reflect` is read-only structural introspection, and it is a first-class feature rather than an
extension a deployment might not have compiled in. It reaches classes, interfaces, enums, methods,
properties, constants, parameters and attributes, from a value or from a class name, and it reports
which methods an interface declares as part of its contract versus as an internal helper. Traits are
absent because they do not exist. The descriptions are a family of classes, one per kind of
declaration: `Core\Reflect\ClassInfo`, `MethodInfo`, `PropertyInfo`, `ParameterInfo`,
`ConstantInfo`, `AttributeInfo` and `EnumInfo`.

Reflective access **enforces the same checks ordinary code would**: there is no
`setAccessible(true)`, so a private property is not readable through this door either, and a
reflective write runs the property observer an ordinary write would run. The refusal is
distinguishable from a misspelling, which is what makes the answer useful rather than merely safe.

`forClass` describes a class the program declares, an `abstract` class and an interface. It returns
`null` for a `Core` class, an anonymous object's `$shape{…}` class and every class the compiler made,
the names `construct` refuses as not declared (`rule:security/reflection-enforces-visibility`).
`forObject` describes any object, those included, so `name()` works on whatever a program holds.

What it costs is that a serializer or a container cannot reach state its author did not expose. That
is the trade: the alternative is a reflection API that makes every access modifier in the language
advisory.

`PropertyInfo::isLateInit(): bool` is true for a property declared `lateinit`, and
`PropertyInfo::isReadonly(): bool` for one declared `readonly`, an inherited property included. Each
reads its own per-slot bit on the class's descriptor. `isLateInit` is not read off a slot that starts
unset: a constructor may leave a slot unset while an observer runs, and that slot is not `lateinit`.

**Memory:** every class's and enum's description, and every row of it, is built once per process,
when the compiled unit is finished and before it is shared, as immortal objects every core reads.
`forClass`, `forObject` and `EnumInfo::of` return that object and allocate nothing, so
`forClass('A') === forClass('A')` is true, and so is `forObject` on two instances of one class. A
description is a pure function of the compiled unit, as a string literal is. That is why
`rule:security/no-cross-request-state` does not apply: nothing a request wrote is shared, nothing is
built during a request, and a request learns nothing from it, not even a first-call timing.

No word two cores can reach is ever written, so a description holds no array, since an array read
writes its borrow flag, and no address. Each slot is a scalar or an immortal string. A roster's rows
live beside the description, and the description holds its class's index in the unit's table and the
table's stamp. A member reads the rows through the calling request's own table, and a description
whose stamp is not that table's, such as a copy that outlived a reload, throws a `LogicError`, because
its index would name another class. A copy sent to a task keeps both numbers and reads the same rows.
A `Core` class's descriptor is in no unit's table, so `forObject` on a `Core` instance builds its
description per call, with stamp `0`.

Each roster member, `properties()`, `methods()`, `constants()`, `attributes()`, `parameters()` and
`cases()`, builds its array on its first call in a request and returns the same array after that, and
so is a `Core` instance's description. The kept array is freed when the request ends. A program that
writes to it separates by copy-on-write, so the kept array never changes. `readableProperties()` is
built per call, because its answer depends on the calling site.

What it spends: once per process for each loaded unit, one immortal object per class, property,
method, parameter, constant, attach site and enum, and one immortal string per name, type and folded
string, for every class, including one nothing reflects on. A class with two typed properties is about
600 bytes, and a whole unit of 31 classes and 44 enums about 45 KB, built in under half a millisecond
at load. Per request, one array and one map entry per roster the request reads, O(classes described),
never O(requests served). `readableProperties()` spends one array per call.

`ClassInfo::attributes()` lists only the attributes written on the described class's own
declaration and members. `ClassInfo::parent(): ?string` returns the full name of the class the
described class extends, written as `name()` writes a name, and `null` for a class that extends none
and for an interface: an interface's `extends` list names interfaces, and none of them is a parent
class. A class that extends a `Core` class gets that name, because it is true, and `forClass`
returns `null` for it, so a walk stops there with no special case. A program that wants the
ancestors' attributes walks the chain with `forClass($info->parent())` and reads each level's own
`attributes()`, in the order it chooses. The name is read off the class's descriptor when `parent()`
is called: one string per call, charged to the request that asked, and one string per class that
extends one, held once per process.

**Declined:**
- **An `attributes()` that includes the parents' attributes.** It reverses the own-only reading, an
  existing caller would get more rows than before, and the runtime would have to invent an order for
  an attribute written at two levels. The loop over `parent()` lets the program choose that order.
- **A `parent()` that returns a `ClassInfo`.** A name matches `name()`, and `forClass` turns it into
  a description when the caller wants one, so a second way to reach a description adds nothing.
- **A per-request cache of whole descriptions.** It builds each description once per request; the
  per-process build costs nothing on the request path.
- **Building a description on its first use, behind a lock.** It is work on the request path, and
  the first request after a reload is slower than the rest.
- **Lazy rosters through a descriptor pointer in a slot.** A reload leaves the pointer dangling; the
  stamp fails closed instead.
- **More modifiers on `PropertyInfo`: `static`, a default value, an observer.** Nobody asked, and each
  is a slot on every row.
