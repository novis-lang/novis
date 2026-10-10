The writer never emits JavaScript, launch actions, embedded files, or any construct that makes a
reader fetch or execute on open. Links exist only where the source wrote one with an `http(s):` or
`mailto:` target. Rendering `tainted` input is safe by construction — the render is a transform, not
a sink — and a program embedding untrusted markup that wants its *tags* constrained sanitizes first (`rule:core-classes/html-sanitize`).

The writer also embeds **no timestamps and no generator entropy**, so two renders of equal input are
byte-equal. That turns a document test into a byte comparison and removes the class of flaky fixture
every PDF test suite otherwise grows.

The second, fidelity backend (`rule:core-classes/pdf-one-engine`) makes no exception to any of this:
it trades the no-I/O guarantee, not the inertness of what it writes.

The component's fixtures owe a scanner over the output of the whole fixture corpus that finds no
`/JavaScript`, `/Launch`, `/OpenAction` or `/EmbeddedFile` object, and the same input rendered twice
and compared byte for byte.

**Not shipped.** There is no PDF package in the tree.
