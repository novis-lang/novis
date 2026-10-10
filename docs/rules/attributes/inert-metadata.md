`#[Name(field: value, ...)]`, or the bare `#[{field: value, ...}]`, attaches an anonymous object
to a declaration. It is the same value an ordinary shape-typed binding takes rather than a
new kind of value, and **no class is declared, instantiated or invoked for it**: an attribute is inert
data from the moment it is parsed.

The whole payload resolves once, while compiling, into the compiled unit's constant pool — the storage
class an enum case's backing integer already uses (`rule:enums/closed-integer-type`). What it costs is
O(attachments written in the compiled code), held once per declaration, and never per request, per
object or per instance.

Retrieval is `Core\Attributes` and nothing else (`rule:attributes/structural-retrieval`). There is no
attribute table in the compiled unit for `Core\Reflect` or anything else to walk.

An attribute is a bag of data, and the language already has a structurally checked bag of data: the
anonymous object and its shape type. Attaching that to a declaration needs no fourth mechanism. A
quick, one-off tag costs one `#[{...}]` and no declaration at all; a validated, reusable one costs one
`type` alias line.

**Declined:**
- **An attribute class per kind, constructed when read**, as PHP, Java and C# do. It is class overhead
  for what is a bag of data, and a `type` alias gives the same structural check with nothing ever
  instantiated.
- **Doc comments parsed by convention.** The compiler has no part in them: no structural check and no
  first-class retrieval, so every framework parses its own text.
- **A free-form string payload**, the way a Go struct tag is written. It loses the structural check
  entirely and leaves every validation to framework code at run time.
