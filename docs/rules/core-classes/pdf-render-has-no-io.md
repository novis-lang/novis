PDF generation is a first-party Tier 1 extension package, sandboxed, with everything a program names
under one namespace and one registered class. It is not `Core`: a layout engine, a font parser and
image codecs unsandboxed in every binary would contradict the reason the image component is an
extension at all. It is not native: nothing in it holds state or privilege.

**The component performs no I/O of any kind.** Every asset a document uses — images, stylesheets
beyond the inline ones, fonts — crosses the boundary as bytes in a caller-supplied map, and a
reference in the HTML resolves against that map and nothing else. A scheme, an absolute path, or a
name the map lacks **throws** rather than rendering a broken-image gap: a generated document must be
deterministic, and refusing beats repairing. A program that wants a remote image fetches it itself,
under the outbound policy, and passes the bytes.

The class of exploit that defines HTML-to-PDF rendering — server-side request forgery and local file
read through a `<img src>` — is thereby absent by construction: there is nothing to trick into
fetching, because there is no fetching. Fonts follow the same rule, with a small embedded default set
so a plain document renders out of the box, and the guest needs no clock.

**Not shipped.** There is no PDF package in the tree; M17 builds it.
