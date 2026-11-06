`Core\Xml` is one subsystem in two shapes — a tree that materialises the whole document and a
reader and writer that hold one window — and **no operation is available through both**, so a program
picks the shape that fits how much of the document it needs at once and pays for that shape only.

The two are not two spellings of one capability. The tree's defining property is that a program may
walk it in any order and any number of times; the stream's is that memory is O(one window) rather than
O(document). Neither shape can offer the other's property, so a program with a large export and a
program with a small configuration file are not being offered a preference — they are being offered the
only two answers that exist.

This is one of the two places in `Core` where two shapes of the same subsystem coexist, the other being
`Core\Json`'s whole document beside its element-at-a-time walk (`rule:core-classes/json-whole-and-stream`).
Each is a **named** exception to `rule:core-api/one-paradigm-per-operation` rather than a
quiet one, and the naming is what stops it spreading: a subsystem that wants two shapes has to earn its
own sentence. The disjointness is what makes it survivable — an operation reachable through both would
put a seam in the API where the memory story stops being a property of the door a program came in
through, and every member added afterwards would have to answer which side it belongs to.

Both shapes answer the same node family, the one `rule:core-classes/html-parsing` makes both parsers
produce, so a program that outgrows the tree does not learn a second vocabulary. The split is stated
once, in `crates/nvs-stdlib/src/xml.rs`'s module doc, and no member card re-argues it.

**Memory:** the tree is proportional to the document, attributed to the request that parsed it and
released with its arena; reading through the stream holds one window, and writing through it holds the
document being built and no tree of nodes at all.

**The stream reads one window of any source.** `Core\Xml::reader` takes a `string` or any forward
source (`Core\Bytes::Source|Core\Bytes::Chunks`, `rule:core-api/a-member-takes-the-source-the-program-holds`).
A `string` is walked where it lies. Any other source is read one chunk at a time into a window that
holds the node being read and at most one more chunk, so a file, an upload or a request body of any
size costs the walk the same. A node cut by a chunk's end is read again from its start once more text is
in, and the window then grows by at least what it holds, so a long node is scanned a constant number
of times per byte. **What it spends:** the largest node, about twice over, plus one chunk, per open walk.

Bytes are UTF-8, or UTF-16 with a byte order mark. A document read from bytes that declares any other
`encoding` throws a `ParseError` naming `Core\Encoding`, because a reader that ignored the declaration
would give the wrong characters and one that obeyed it would carry every table `Core\Encoding` already
has (`rule:core-api/no-encoding-argument`).

**Declined:**
- **A tree only.** It cannot serve a document larger than the request's memory ceiling, which is a real
  workload.
- **A stream only.** Every program that wants random access would write its own tree, once per program
  and unreviewed, and the node family would stop being shared with `Core\Html`.
