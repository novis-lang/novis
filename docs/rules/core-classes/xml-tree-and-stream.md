`Core\Xml` is one subsystem in two shapes — a tree that materialises the whole document and a
reader and writer that hold one window — and **no operation is available through both**, so a program
picks the shape that fits how much of the document it needs at once and pays for that shape only.

The two are not two spellings of one capability. The tree's defining property is that a program may
walk it in any order and any number of times; the stream's is that memory is O(one window) rather than
O(document). Neither shape can offer the other's property, so a program with a large export and a
program with a small configuration file are not being offered a preference — they are being offered the
only two answers that exist.

Spec § 17 names this outright as "the one place in this file where two shapes of the same subsystem
coexist", so it is the **named** exception to `rule:core-api/one-paradigm-per-operation` rather than a
quiet one, and the naming is what stops it spreading: a subsystem that wants two shapes has to earn its
own sentence. The disjointness is what makes it survivable — an operation reachable through both would
put a seam in the API where the memory story stops being a property of the door a program came in
through, and every member added afterwards would have to answer which side it belongs to.

Both shapes answer the same node family, the one `rule:core-classes/html-parsing` makes both parsers
produce, so a program that outgrows the tree does not learn a second vocabulary. The split is stated
once, in `crates/nvs-stdlib/src/xml.rs`'s module doc, and no member card re-argues it.

What it spends, per `rule:programs/memory-priority`: the tree is proportional to the document,
attributed to the request that parsed it and released with its arena; the stream holds one window.
