The writer never emits JavaScript, launch actions, embedded files, or any construct that makes a
reader fetch or execute on open. Links exist only where the source wrote one with an `http(s):` or
`mailto:` target. Rendering `tainted` input is safe by construction — the render is a transform, not
a sink — and a program embedding untrusted markup that wants its *tags* constrained sanitizes first.

The writer also embeds **no timestamps and no generator entropy**, so two renders of equal input are
byte-equal. That turns a document test into a byte comparison and removes the class of flaky fixture
every PDF test suite otherwise grows.

The second, fidelity backend (`rule:core-classes/pdf-one-engine`) makes no exception to any of this:
it trades the no-I/O guarantee, not the inertness of what it writes.

**Not shipped.** There is no PDF package in the tree.
