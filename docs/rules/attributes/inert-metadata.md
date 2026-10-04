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
