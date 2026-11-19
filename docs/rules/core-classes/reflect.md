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

**Memory:** each `forClass` or `forObject` call builds one description and one row object per
declared property, method and constant, charged to the request that asked and freed with the
description. A program that describes the same class in a loop pays on every call.

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
- **A `parent()` that returns a `ClassInfo`.** A description builds one row object per property,
  method and constant, which a caller that only wants the name pays for nothing. A name matches
  `name()`, and `forClass` turns it into a description when the caller wants one.
