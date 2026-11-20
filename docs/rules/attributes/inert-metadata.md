`#[Name(field: value, ...)]` attaches an anonymous object to a declaration. `Name` is an attribute
declaration (`rule:attributes/an-attribute-is-declared-with-its-sites`), and the payload is the same
value an ordinary shape-typed binding takes rather than a new kind of value. **No class is declared,
instantiated or invoked for it**: an attribute is inert data from the moment it is parsed.

The whole payload resolves once, while compiling, into the compiled unit's constant pool — the storage
class an enum case's backing integer already uses (`rule:enums/closed-integer-type`). What it costs is
O(attachments written in the compiled code), held once per declaration, and never per request, per
object or per instance.

Retrieval is `Core\Attributes` and nothing else
(`rule:attributes/a-retrieval-matches-the-declaration-it-names`). There is no attribute table in the
compiled unit for `Core\Reflect` or anything else to walk.

An attribute is a bag of data, and the language already has a structurally checked bag of data: the
anonymous object and its shape type. The declaration is a shape's field list with a name, the sites it
may be used on and the docs of its keys. That is all a developer needs to find the right attribute, and
nothing more is declared.

**Declined:**
- **An attribute class per kind, constructed when read**, as PHP, Java and C# do. It is class overhead
  for what is a bag of data, and a declaration gives the same structural check of its payload with
  nothing ever instantiated.
- **A `type` alias as the attribute's name, or no name at all.** An alias names a type, not where it may
  be used, so the editor could not offer the attributes that fit, and a nameless payload could not be
  found or retrieved (`rule:attributes/an-attribute-is-declared-with-its-sites`).
- **Doc comments parsed by convention.** The compiler has no part in them: no structural check and no
  first-class retrieval, so every framework parses its own text.
- **A free-form string payload**, the way a Go struct tag is written. It loses the structural check
  entirely and leaves every validation to framework code at run time.
