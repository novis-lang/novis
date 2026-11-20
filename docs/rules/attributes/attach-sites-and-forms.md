An attribute attaches to a declaration: a class, an interface or an enum, a method, a property or one
of its hooks, a class constant, an enum case, and a method's parameter. It is written directly above
the declaration, or directly before the parameter or the enum case. Which of those sites one attribute
may use is its declaration's `on` list (`rule:attributes/an-attribute-is-declared-with-its-sites`), and
a use on any other site is **`E0874`** at the attribute, naming the site and the allowed ones.

```nvs
repeatable attribute Route on method { path: string, method: string }
attribute Column on property, parameter { type: string, length?: int }
attribute Cache on method { ttlSeconds: int }

class UserController {
    #[Column(type: "varchar", length: 255)]
    public string $name;

    #[Route(path: "/users/:id", method: "GET")]
    #[Cache(ttlSeconds: 60)]
    public function show(#[Column(type: "int")] int $id): Response { … }
}
```

There is one form, `#[Name(...)]`, and it takes named fields — there is no positional form. `Name`
resolves to an attribute declaration (`rule:attributes/an-attribute-is-declared-with-its-sites`), and
the payload object is checked against the declaration's fields by the width subtyping every
shape-typed position already uses. An array literal in a field is checked against that field's declared
array type, element by element and nested literals included, so `#[Tags(ids: [1, 2])]` fits
`{ids: array<int>}`. A string, `int`, `bool` or enum-case literal is read at the single-value type its
field names, as an assignment reads it (`rule:types/single-value-types`), so
`#[Link(onRemove: 'clear')]` fits `{onRemove: 'keep'|'clear'}`, in an array element and a nested shape
too. A retrieval returns the payload as it was accepted where it was written
(`rule:attributes/a-retrieval-matches-the-declaration-it-names`). Neither has the `int` to `float` row,
because the payload is stored as written. A field that does not fit is the ordinary mismatch `E0401`.

`Owner::Name` names an attribute declared in the body of the class or interface `Owner`, and the
owner's file is autoloaded like any other name the attribute writes. On the owner's descendants the
short `Name` resolves to it too. An attribute that takes no options is the same attribute with an empty
list, so `#[Core\Json\Derive()]` is `#[Core\Json\Derive]`.

- **Memory:** nothing per request. Resolving a name costs one lookup per use while compiling, and one
  walk up a class's parents for a short name on a descendant.

A malformed attribute is one error. The parser skips the rest of its `#[...]` group up to the
group's own `]`, so the declaration after it still parses and nothing else is reported for it.
"expected `]`" is reported only if nothing inside the group was reported yet, and the skip counts
brackets, parentheses and braces. Outside every bracket, a `;`, an unmatched closing bracket or a
keyword that begins a declaration also ends the skip without being consumed, so a group missing its
`]` loses only its own tokens. A `::` with no name after it is one error, and the checker says nothing
more about that attribute.

**`Core`'s compiler attributes are declarations too.** Each of them is the same declaration value a
program's `attribute` declaration builds: its name, its sites, `repeatable` and its fields with their
docs. The checker, the stub tree and the editor read that one value, so `E0874` is one check for every
attribute. `#[Core\Json\Derive]` and `#[Core\Db\Derive]` go on a class; their `Field`s on a property or
a promoted constructor parameter; `#[Core\Test]`, `#[Fixture]`, `#[TestWith]`, the three test hooks,
`#[Core\Command]`, `#[Core\Route]`, `#[Core\Access]` and `#[Core\Api]` on a method; and
`#[Core\Option]` and `#[Core\Query]` on a parameter. What an owning pass checks beyond the site (a
`static` method, a `#[Route]` beside `#[Api]`) stays with that pass. The rule that owns an attribute may
name its sites itself: `#[Core\Path]` is written before a `string` parameter
(`rule:programs/relative-paths-resolve-from-their-file`), and `#[Core\Deprecated]` above any of these
sites but a property hook and that hook's parameter
(`rule:attributes/a-deprecation-names-its-replacement-as-code`).

**Declined:**
- **A second table of sites for `Core`'s attributes**, beside the declarations. Two tables answer the
  same question, and the editor would read one while the checker read the other.
- **A lookup of its own in the attribute checker** for `Owner::Name`. It would be a second resolver for
  one syntax, and the owner chain the short name walks would be written twice.
- **The bare `#[{...}]` form and a shape-typed `type` alias as the name.** Neither says where the
  attribute may be used, so the editor cannot offer the attributes that fit
  (`rule:attributes/an-attribute-is-declared-with-its-sites`).
