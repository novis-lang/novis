PDF generation is the third built-in component (`rule:packaging/the-first-party-components-are-built-in`):
everything a program names is under `Novis\Pdf`, present in every binary, and run in the sandbox with
no grant. It is not `Core`: a layout engine, a font parser and image codecs unsandboxed in every request
would contradict the reason the image component is a component. It is not native: nothing in it holds
state or privilege past one request.

**The component performs no I/O of any kind.** Every asset a document uses — images, stylesheets beyond
the inline ones, fonts, and PDFs placed as pages — crosses the boundary as bytes in a caller-supplied
map, and a reference in the HTML, the CSS or an inline SVG resolves against that map and nothing else. A
`data:` URI decodes in place, because its bytes are already in the document. A `file:` or `http(s):`
URL, an absolute path, or a name the map lacks **throws** `LogicError` naming it, rather than rendering
a broken-image gap: a generated document must be deterministic, and refusing beats repairing. A program
that wants a remote image fetches it itself, under the outbound policy, and passes the bytes.

The class of exploit that defines HTML-to-PDF rendering — server-side request forgery and local file
read through an `<img src>` — is thereby absent by construction: there is nothing to trick into
fetching, because there is no fetching. The guest needs no clock either: a date in the document is text
the program wrote, and the file carries a creation time only when the program passes one
(`rule:core-classes/pdf-output-is-inert`). HTML, CSS, fonts, images and SVG are still hostile bytes. The
request's memory cap and deadline contain them, the HTML is held to the parser's element cap
(`rule:core-classes/html-parsing`), and the component adds **no limit of its own**: a bulk or slow
render belongs in `Core\Queue`.

The component's fixtures owe: a render with **no capability grants** that succeeds, so the no-I/O claim
is demonstrated rather than asserted; a `file:///…` reference, an `http:` URL and a name absent from
the asset map, each in HTML, CSS and SVG and each throwing, with no network or filesystem access
observable from the guest; and a `data:` image that renders.

- **Memory:** per rendering request, the HTML, the asset map, the layout tree and the output document,
  all inside the request's memory cap and freed when the request ends. A program that never renders
  pays nothing.

**Not shipped.** There is no PDF component in the tree.

**Declined:**
- **`Core`.** A layout engine, a font parser and image decoders would sit unsandboxed in every binary
  and every in-flight request.
- **Native.** Nothing here needs a connection lifetime or a runtime privilege, and the sandbox is the
  feature rather than a cost to route around.
- **Leaving the category to the third-party channel.** That produces incompatible libraries with
  disjoint CSS subsets, and the popular ones fetch whatever the HTML names, which is the category's
  vulnerability class. One first-party no-I/O way to do the job prevents both.
- **A fetch hook or an allow-list of hosts.** It brings back the fetching this rule removes, and a
  program already fetches under its own outbound policy.
- **A render limit of its own.** A render is a request's work, priced by the request's memory cap and
  deadline like any other; a second knob would be one more number to tune with no new bound behind it.
