```nvs
/// The properties a search looks in.
attribute Search on class { fields: array<property<static>>, exact?: bool }

#[Search(fields: ['author', 'title'])]
class Post
{
    public string $author = '';
    public string $title = '';
}

var $search = Core\Attributes::get<Search>(Post::constructor);
```

**`static` in an attribute declaration's fields names the class the attribute is used on.** For an
attribute on a class, that is the class itself. For one on a property, a method or a parameter, it is
the class whose body declares that member. A declaration whose fields name `static` is lowered at each
use with that class, so one declaration checks every class it is used on against that class's own
properties. `property<static>` takes one name, and `array<property<static>>` a list of them.
`propertyPath<static>` follows the same rule, because it shares `property<T>`'s argument
(`rule:types/property-path`).

- **Only an attribute declaration's own body binds `static` this way.** A `type` alias keeps
  `rule:statements/static-is-a-member-modifier`'s reading: a top-level alias whose right-hand side names
  `static` is `E0303` where the alias is declared, also when an attribute declaration's field names that
  alias. No alias changes meaning with where it is used.
- **`self` and `parent` in an attribute declaration's fields are `E0303`.** `static` is the one way to
  name the class the attribute is on (`rule:statements/nothing-gets-a-second-name`).
- **A use with no class**, such as an attribute on a function, makes `static` in the declaration's
  fields `E0303` at that use's lowering.
- **A retrieval binds `static` to its target's class.** `Core\Attributes::get<Search>(Post::constructor)`
  and `::all` read the declaration's fields with `static` bound to the class the first argument names,
  never to the class the call is written in. A retrieval folds while compiling
  (`rule:attributes/retrieval-folds-while-checking`), so the target is always known.
  `Core\Program::implementingWith<I, T>` binds it once per class it joins. A retrieval whose target is
  not a class makes `static` in its type argument `E0303`.
- **A string literal in a payload at a `property<C>` field is read as a written name**, as an array
  element too, exactly as a literal at a key parameter is (`rule:types/property-key`). A public declared
  property of `C`, its ancestors' included, makes it a `property<C>`. Any other name, a `private` one
  and a `static` one included, is the unknown-member diagnostic at the literal. The payload keeps the
  name as written, which is a key's representation.
- **A payload is checked on the declaration it is written on, once.** A subclass inherits nothing to
  check again, because an attribute is read from the declaration it is written on alone.

- **Memory:** nothing at run time and nothing per request. The payload is the same strings it was. While
  compiling, a declaration whose fields name `static` is lowered once per use, which the interner makes
  one lookup for a second use on the same class.

**Declined:**
- **A marker attribute on a `string` field holding a comma-separated list of names.** It is a second
  mechanism beside `property<T>`, a small list syntax inside a string, and a value the program splits
  again at run time. A list of names the configuration declares is declined in
  `rule:types/single-value-types` for the same reason.
- **`static` bound where a top-level `type` alias is used.** Every alias that names `static` would then
  mean a different type at each use, and a reader of the alias could not tell which.
- **`self` as a second way to write the class.** In an owned declaration it would name the owner. An
  owned declaration is used only on the owner's descendants
  (`rule:attributes/an-attribute-is-declared-with-its-sites`), so `self` would never name the class the
  attribute is on.
