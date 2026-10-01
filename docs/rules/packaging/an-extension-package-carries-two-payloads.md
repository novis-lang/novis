An extension may carry **two payloads in its one file**: the wasm component, and Novis source that
composes calls into it, in the `nvs.source` section
(`rule:packaging/an-nvsx-is-one-file-carrying-its-manifest`). Everything a program names is under one
namespace, the manifest registers **exactly one class** whose static methods are the component's closed
set of entry points, and every other class under that namespace is Novis source that calls them.
`Novis\Image` is the first of this shape — `Novis\Image\Codec` is the manifest's class, and the builder
above it is source (`rule:core-classes/image-pipeline`); `Novis\Spreadsheet\Engine` follows it
(`rule:core-classes/spreadsheet-has-no-io`).

Loading the extension makes its source resolvable by `autoload` with no line the program writes. The
source compiles like any other Novis file and holds the authority of its own namespace, which is no
grant unless the operator writes one. The one pin covers both payloads, so the source cannot be edited
under a pinned component, and `nvs ext inspect --source` prints it. A package may carry a `.nvsx` once
packages exist, and nothing here changes for that.

The split is the point rather than a packaging convenience. Building a plan is data manipulation and
costs nothing to do in Novis; the codecs belong inside the sandbox. A builder that lived in the guest
would spend a boundary crossing per method to append to an array
(`rule:packaging/the-boundary-is-the-cost`), and would hold plan state across calls in an instance
whose whole premise is that state does not outlive a request.

**Not on disk.** There is no source section and no loader to read one.
