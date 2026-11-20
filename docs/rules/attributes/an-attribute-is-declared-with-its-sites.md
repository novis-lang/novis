```nvs
abstract class Storable
{
    /// Adds a database index over one or more columns of this table.
    repeatable attribute Index on class {
        /// The column names, separated by commas.
        columns: string,
        /// `true` means no two rows may have the same values in these columns.
        unique?: bool,
    }

    /// Sets how this property is stored.
    attribute Column on property, parameter {
        /// What happens to this row when the row it points to is deleted.
        onTargetDelete?: 'deleteThisObjectToo'|'setNull'|'refuse',
    }
}

/// Marks a method a job runner may call.
attribute Job on method { queue: string }
```

Only an **attribute declaration** may follow `#[`. It is written `[repeatable] attribute Name on <site>,
<site> { field, … }`, at namespace level or as a member of a class or interface body. An enum body
declares none, and one written there is `E0142`. `attribute`, `repeatable` and `on` are contextual words,
read as `type` is, so no identifier is taken from programs.

- **The body is a shape's field list**, `name: T` and `name?: T`, with `rule:types/shape-type`'s grammar
  and check. A field has no default. A doc comment above the declaration documents the attribute, and a
  doc comment above a field documents that key (`rule:types/shape-type`). `static` in a field names the
  class the attribute is used on, and `self` and `parent` are `E0303`
  (`rule:attributes/a-declaration-names-its-class-with-static`).
- **The declaration is a name of its own kind, not a type.** `#[`, a retrieval's type argument
  (`rule:attributes/a-retrieval-matches-the-declaration-it-names`), `Core\Router\Match::attribute<T>` and
  `Core\Program::implementingWith`'s `T` are the only places that name it. Written in a type position, it
  is the ordinary undefined-type `E0303`. In a body, it shares the member names of that body's aliases,
  constants, enums and enum cases, and a second member of one name is `E0304`.
- **The sites are fixed words:** `class`, `interface`, `enum`, `method` (a constructor included),
  `property`, `hook`, `constant`, `case` and `parameter`. The list after `on` has one or more of them and
  no wildcard. A missing list, an empty one or a word that is not a site is `E0141` at the word. A
  promoted constructor parameter is a property and a parameter, and an attribute fits there when its list
  has either word.
- **A program's declaration lists only the sites a retrieval reads:** `class`, `method`, `property` and
  `parameter`. Any other word in its list is `E1004` at that word, naming the four. `Core`'s declarations
  list the other five, such as `Core\Deprecated`'s. When a retrieval learns to read another site, this
  list grows by that site.

**Resolution.** `#[Name(…)]` resolves `Name` through the namespace and `use` scope, as every name does.
On a descendant of an owner, the owner's attributes are also in scope by their short names, and so are
the attributes of every owner that owner inherits from. The owner chain is the class graph's parents
and interfaces, walked once per use. `#[Owner::Name(…)]` resolves to the same declaration, and only the
owner that declares it may stand left of `::`: `Sub::Index`, where `Storable` declares `Index`, is
`E0405` naming `Storable`, as it is for an alias (`rule:types/class-scoped-alias`).

- A name nothing declares is `E0303`.
- A short name that both the owner chain and the namespace scope give is `E1007`, naming both
  declarations. The compiler never chooses one. The qualified form names one and is the fix.
- A name that resolves to a class, an interface, an enum or a `type` alias is `E1005`: only an attribute
  declaration may follow `#[`. When the name is a shape alias the program declares, the fix rewrites it
  to `attribute Name on <the site of this use> { … }` with the alias's fields.
- `#[{…}]`, a payload that names no declaration, is the parse error `E0140`, which says to declare an
  attribute.

**The checks at a use.** Each is its own diagnostic at the attribute, in this order, and once one of
them reports a use, the checks after it are skipped for that use. One mistake is one error.

1. **The site.** A use on a site the declaration does not list is `E0874`, naming this site and the
   allowed ones. `Core`'s compiler attributes are checked by the same code (`rule:attributes/attach-sites-and-forms`).
2. **The owner.** A use of an owned declaration on a declaration that is not one of the owner's
   descendants is `E1006`, naming the owner. A descendant is a class or interface that extends or
   implements the owner, directly or through its parents, and every member, hook and parameter declared
   in its body. The owner itself and its own members are not its descendants, so a use there is
   `E1006` whether it is written short or qualified. A namespace-level
   declaration has no owner, and this check passes.
3. **The count.** A second use of one declaration on one declaration needs `repeatable`
   (`rule:attributes/repeatable`). Without it, the second use is `E0763`, naming both.
4. **The payload.** The payload fits the declaration's fields, with the check
   `rule:attributes/attach-sites-and-forms` makes. A field that does not fit is `E0401`.

- **Memory:** nothing at run time and nothing per request. A payload is a compile-time constant, and a
  retrieval folds to its answer (`rule:attributes/retrieval-folds-while-checking`). While compiling, each
  use costs one site check, at most one walk up its class's parents, and one count per declaration.

**Declined:**
- **An attribute usable anywhere by its qualified name**, `#[Storable::Index]` on any class. It brings
  back an attribute that compiles where nothing reads it.
- **`on any`, or no `on` list meaning everywhere.** The same: an attribute that shows up everywhere says
  nothing about where it means something.
- **A meta-attribute on a `type` alias**, `#[Core\Attribute(on: 'class')] type Index = {…}`. It is one
  more name a developer must know, and a type alias carries no attributes.
- **Keeping the structural match beside the declaration.** Two attributes with the same fields would
  answer each other's retrievals again.
- **An attribute class, constructed when read.** It is class overhead for a bag of data.
- **The bare `#[{…}]` form.** Nobody can discover it, the editor can offer nothing for it, and no
  retrieval can name it.
- **A user declaration on a site no retrieval reads.** Its payload would compile where nothing can read
  it.
- **Picking the owner's declaration, or the namespace's, when both give a short name.** Either choice is
  silent, and adding a declaration in one scope would change what a use in the other means.
