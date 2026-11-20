```nvs
function Core\Attributes::get<T>(callable $target, string $member = ""): ?T;
function Core\Attributes::all<T>(callable $target, string $member = ""): array<T>;
```

`T` names an attribute declaration (`rule:attributes/an-attribute-is-declared-with-its-sites`), by its
short name or as `Owner::Name`, resolved as `#[` resolves it. A retrieval finds exactly the payloads
written under that declaration. It never matches by shape: two declarations with the same fields find
only their own payloads, and a payload written under one is never an answer for the other. The answer's
type is the declaration's shape. `Core`'s compiler attributes are declarations too, so
`get<Core\Command>` returns the `#[Core\Command]` payloads, typed by that declaration's fields.

A `T` that is not an attribute declaration is `E1008` at the type argument. That covers an inline
shape such as `get<{name: string}>`, a `type` alias, a class and an enum.

`$target` names a declaration through a first-class-callable reference, read syntactically and never
evaluated. A method is its own reference, `Foo::bar`. A class or interface has none of its own and
is named by its `constructor`'s, `Foo::constructor`, which every class definitely has. A parameter
is the owning method's reference plus its name as `$member`; a property is the owning class's
`constructor` reference plus its name. Those two overlap deliberately: for a promoted constructor
parameter they are the same declaration, so both rosters are consulted and their attributes joined.
A retrieval reads the one declaration the target resolves to. `Sub::constructor` with no member reads
the class `Sub` and none of its parents, and a method `Sub` overrides reads the override alone. A method
`Sub` inherits without overriding resolves to its parent's declaration, and that is what is read.

A written `$member` is checked against the target's real declarations and one naming neither roster is
`E0798`. The answer a misspelling would otherwise fold to is the answer a correct retrieval of an absent
attribute gives, and nothing downstream could tell them apart. A `$member` that is not a string literal
has no name to check and folds to an empty result.

`Core\Reflect\AttributeInfo::name()` returns the full name of the declaration a payload was written
under, such as `Shop\Storable::Index` for an owned one and `Shop\Job` for one at namespace level.

**Declined:**
- **The structural match**, every payload satisfying a shape `T` under width subtyping. Two attributes
  with the same fields answered each other's retrievals, and an inline shape named no attribute a reader
  could look up.
- **Keeping the structural match beside the declaration match.** Two ways to ask, with two answers for
  the same program.
- **Retrieval through `Core\Reflect`'s general walk.** Every attribute list is known while compiling,
  so a walk by name at run time throws that knowledge away and opens a second, broader introspection
  surface for a question the compiler answers narrowly.
- **A reference token for a property or a parameter, and `Foo::class` as the class target.** Every
  class already has a nameable `constructor` reference, so every target is one callable reference, and
  a member name string is what PHP's own reflection accepts for members that are not callable.
