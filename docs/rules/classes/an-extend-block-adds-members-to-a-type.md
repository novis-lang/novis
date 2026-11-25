`extend <Type> as <Name> { … }` declares a block of members added to a class, an interface or an enum
that another package declares, and a file sees them only once it writes `use <Ns>\<Name>;`. `<Type>`
is written in full or through `use`, and may be a `Core` type or a type of another package. `<Name>`
is the block's own name in the file's namespace, as a class name is. `extend` is a contextual word: it
is read as one only at the start of a top-level statement, before a type name, so a method, a variable
or a property named `extend` stays an ordinary identifier.

**The declaration.** A block is a top-level declaration. Written inside a class body it is `E0233`, as
a nested class is (`rule:classes/a-type-is-declared-at-file-scope`). It may carry
`#[Core\Deprecated]` and no other attribute, and its members take attributes as class members do. A doc
comment and its tags work on the block and on each member. An autoloaded file declares one block and
nothing else, named as the autoload map finds it, so `App\Time\BusinessDays` is `BusinessDays.nvs`
under `App\Time`'s directory (`rule:programs/one-declaration-per-autoloaded-file`). A file reached by
`require` may declare blocks as it may declare anything. In user-facing text the feature is "an
`extend` block" and its members are "added" members. "Extension" already names a native `nvs ext`
component, and no text, rule title or code name calls this feature one.

**The target.** A target that names a `type` alias, an attribute or another block is **`E0336`**. A
target nothing declares is the ordinary undeclared-name error, with the `use` line that would resolve
it. A sealed `Core` class is a target, because a block gives it no layout of its own. A target declared in the block's own package is **`E0337`**,
and its help says to add the member to the type itself. For the application, that is any type the
application declares. Every type a project owns then has one place per member, and an enum of the
program's own takes methods and constants in its body (`rule:enums/an-enum-body-takes-methods-and-constants`).
`Packages::of` names both packages.

**Where an added member is visible.** In a file that writes `use <Ns>\<Name>;` for the block, an alias
included, and inside the block's own body. The block's own namespace is no
exception: names load lazily through the autoload map, and nothing but a `use` names a block. A call
that would resolve only through a block the file does not use is the ordinary unknown-member error
(`E0405` for `->`, `E0309` for `::`), and its help names the block and the `use` line when the
program's autoload map knows a block that adds the name.

**What a block may add.**

| Added | On a class | On an interface | On an enum |
|---|---|---|---|
| `public` or `private` instance method | yes | yes | yes, `$this` is the case |
| `public` or `private` static method | yes | yes | yes |
| constant | yes | yes | yes |
| nested `enum` | yes | yes | no |
| nested `attribute` | yes, used on the type's descendants | yes, used on its implementers | no |
| property with a `get` hook and no stored value | yes | yes | yes |
| stored property, with or without hooks | yes, when the class takes the added slot | no | no |

The stored property and its slot are `rule:classes/an-added-property-lives-in-one-hidden-slot`.

A nested enum or attribute belongs to the extended type, as one in the type's own body would: it is
`<Type>::<Name>` from a file that uses the block, and a bare `<Name>` inside the block. A file that
does not use the block cannot name it, and the error's help names the `use` line. An added attribute
is used on the type's descendants or implementers (`rule:attributes/an-attribute-is-declared-with-its-sites`),
so one added to a `Core` class, or to a `Core` interface only `Core` classes implement, could be used
nowhere and is **`E0338`**. Two blocks that add one enum name to a type, or a block whose enum name
the type already declares, are the duplicate-declaration error.

**What a block may not add.** Each is **`E0338`** at the member, and its help names what to write
instead: a constructor (a static factory), a `protected`, `abstract` or `final` member (a `public` or
`private` one), a `static` property (a static property of a class of your own), `readonly` on an added
property, a stored property that has no default, no `?` type and no `lateinit`, a new enum case, and
a member the table above leaves out, such as a `type` alias. An `implements` or `extends` clause on the block does not parse. In
a body, `parent` is `E0338`, and `static::` and `new static` are `E0338` whose help says to write
`self`. An added static method, or any method of a block on an enum, that uses `yield` is `E0338`. A member that
replaces or overrides anything is `rule:classes/an-added-member-never-changes-a-type`.

**Inside the body.** `$this` is a value of the extended type, and `self` names that type. A block's
body sees what code outside the type sees: the type's `public` members, the block's own members, and
the members of other blocks its file uses. `private` on an added member means visible inside that
block only. Visibility is checked with the block as the accessing place, never the type.

**Generic `Core` classes.** A generic `Core` class is extended with one new name for each of its
type parameters, in order, as `extend Core\ObjectMap<K, V> as Index { … }`. Inside the block each
name is a type variable, and `$this` is `Core\ObjectMap<K, V>`. A call substitutes the receiver's
type arguments, so `getOr(K $key, V $fallback): V` on a `Core\ObjectMap<Tag, int>` takes and returns
an `int`. The body is compiled once, with each name erased as `mixed` is, and the call converts at
the boundary as it does for any wider declared type. A list on a class that is not generic, a list
of the wrong length, no list on a generic class, and an entry that is not one bare name are
**`E0338`**, so a block of one instance, such as `Core\ObjectMap<Tag, int>`, does not compile. A
program's own classes take no type parameters, so this is `Core` only.

**Declined:**
- **Opening `Core` classes to `extends`.** `rule:core-api/core-classes-are-sealed` keeps its reasons.
- **Program-wide visibility, registered once.** Reading a file would not show where a method comes
  from, two dependencies adding one name would break an application that can fix neither, and every
  block change would recompile every file.
- **`extend` as a reserved word.** A reserved word breaks every identifier already spelled `extend`,
  and the contextual position is unambiguous.
- **Extending a scalar (`extend string`).** Novis has no methods on scalars. `extend Core\Str as
  Slugs` adds static functions where the string functions already are.
- **A block of one generic instance (`extend List<int>`).** Its members would exist on some values of
  a class and not others, and a receiver's type argument is erased where a call is resolved.
- **A block of a type the program owns.** It would give one type two places for each member.
