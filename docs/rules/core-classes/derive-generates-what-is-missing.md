The derive generates only what the class does not declare itself. A class writing its own encoder and
carrying the attribute gets the generated decoder and keeps its encoder — the common real case, since
a custom representation usually needs a mechanical inverse rather than a second bespoke one. A class
declaring **both** halves is a compile error: the attribute would generate nothing, and an attribute
with no effect is a mistake rather than a no-op.

`#[Db\Derive]` is one-directional. The row codec declares a read only; a write is an explicit
statement plus bound parameters, and generating an `INSERT` is the ORM already settled against. The
graph-copy operation gets no derive either: it handles every object with no per-class opt-in and is
not a declared wire contract at all. An anonymous shape encodes with no attribute, because a shape
literal has no declaration to carry one and no identity a property list could only guess at — its
encoding is structural, keyed on its field names alone. And there is **no validation**: a derived
codec checks types and presence, not that an email looks like one.

What it costs: a compile-time pass over the classes carrying the attribute, emitting straight-line
field-by-field code. **Nothing is stored per object and nothing is reflected at run time.** Footprint
is O(derived classes in compiled code), not O(objects) and not O(requests), and a program with no
derive attribute pays nothing at all, including no pass.
