An attribute attaches to a declaration: a class, an interface or an enum, a method, a property or one
of its hooks, a class constant, an enum case, and a method's parameter. It is written directly above
the declaration, or directly before the parameter or the enum case.

```nvs
#[Route(path: "/users/:id", method: "GET")]
class UserController {
    #[Column(type: "varchar", length: 255)]
    public string $name;

    #[Cache(ttlSeconds: 60)]
    public function show(#[Autowire] UserRepository $repo): Response { … }
}
```

Two forms carry one payload between them, and both take named fields — there is no positional form.
The **bare** `#[{...}]` names no shape and is checked only as a well-formed anonymous object. The
**named** `#[Name(...)]` requires `Name` to resolve, through the ordinary namespace and `use` scope, to
a `type` alias whose right-hand side is a shape type; the payload object is then checked against it by the
width subtyping every shape-typed position already uses.

The name may be written `Owner::Name`, an alias declared as a member of an interface, class or enum
body (`rule:types/class-scoped-alias`), and it is then resolved exactly as the type `Owner::Name` is,
the owner's file being autoloaded like any other name the attribute writes. A name is therefore
written the same way in an attribute and in a type, and an alias that belongs to an interface needs
no second, file-scope declaration to be usable as an attribute
(`rule:statements/nothing-gets-a-second-name`). The checker builds the type atom and lowers it, so an
alias standing left of `::`, the owner's namespace and imports, and every diagnostic for a name that
resolves to nothing are the type position's own. Retrieval is structural and never reads the name
(`rule:attributes/structural-retrieval`), so `Page::Meta` and the shape it names find the same
payloads.

- **Memory:** nothing per request. The member form costs one type lowering per attribute while
  compiling, which the plain form already pays through the alias table.

Three answers, deliberately three. A name nothing declares is the ordinary undefined-name `E0303` —
an attribute name is not a second namespace, so it gets no refusal of its own; for `Owner::Name` it is
the answer the same type would get. A name resolving to something that is not a shape-typed alias, a
class, a `type Id = int;` or an enum case, is `E0726`. A payload that fails the shape is the ordinary
mismatch `E0401`.

A malformed attribute is one error. The parser skips the rest of its `#[...]` group up to the
group's own `]`, so the declaration after it still parses and nothing else is reported for it.
"expected `]`" is reported only if nothing inside the group was reported yet, and the skip counts
brackets, parentheses and braces. Outside every bracket, a `;`, an unmatched closing bracket or a
keyword that begins a declaration also ends the skip without being consumed, so a group missing its
`]` loses only its own tokens. A `::` with no name after it is one error, and that attribute is kept
as the bare form, so the checker says nothing more about it.

The one exemption is the closed, `Core`-owned roster of compiler-recognized attributes, matched by name
and naming no shape at all. Each is a plain name, so `Owner::Name` never matches one. Every userland
name is an alias or a mistake. A recognized attribute takes only the sites where it means something,
and the rule that owns it names them: `#[Core\Path]` is written before a `string` parameter
(`rule:programs/relative-paths-resolve-from-their-file`), and `#[Core\Deprecated]` above any of these
sites but a property hook and that hook's parameter
(`rule:attributes/a-deprecation-names-its-replacement-as-code`).

**Declined:**
- **A lookup of its own in the attribute checker** for `Owner::Name`, reading the member alias table
  directly. It would be a second resolver for one syntax, and the type position's handling of an alias
  left of `::` and its diagnostics would have to be copied into it.
- **Matching `Owner::Name` against the recognized roster.** No recognized attribute is a member of
  anything, so the only effect would be a second way to write a name the compiler acts on.
