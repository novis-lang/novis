A class takes part in a wire format only where it says so. JSON encoding and decoding go through one
explicit interface declaring both halves — an instance method that produces the document and a static that
reconstructs the class from one — or through a written `#[Json\Derive]` that generates both from the
class's declared properties. There is no magic hook.

**Structural** encoding of a class's public properties is refused. It makes the public shape an implicit
wire contract that a rename breaks with no diagnostic, and it needs an opt-out mechanism, which is a magic
hook under another name. A written attribute is not that: the participation is visible at the declaration,
and a `secret` property is refused there rather than silently omitted from the output. An anonymous object
(`rule:types/object-top`) is the one value that needs neither, because it has no declaration to carry
either — it encodes as an object keyed by its field names.

The decode half is part of the same contract, so a type that encodes also decodes, and no program
hand-writes the step that turns decoded data back into an object.
