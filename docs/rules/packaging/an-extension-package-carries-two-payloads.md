An extension package may carry **two payloads**: the `.nvsx` wasm component, and Novis source that
composes calls into it. Everything a program names is under one namespace, the component's manifest
registers **exactly one class** whose static methods are the component's closed set of entry points,
and every other class under that namespace is Novis source that calls them. `nvs/image` is the first
package of this shape — `Novis\Image\Codec` is the manifest's class, and the builder above it is source
(`rule:core-classes/image-pipeline`); `nvs/spreadsheet` follows it with `Novis\Spreadsheet\Engine`
(`rule:core-classes/spreadsheet-has-no-io`).

Loading is unchanged: the `.nvsx` alone is what an `[[extension]]` pin governs
(`rule:packaging/extension-loading-is-root-controlled`), and the source beside it is resolved exactly as
a source package's is.

The split is the point rather than a packaging convenience. Building a plan is data manipulation and
costs nothing to do in Novis; the codecs belong inside the sandbox. A builder that lived in the guest
would spend a boundary crossing per method to append to an array
(`rule:packaging/the-boundary-is-the-cost`), and would hold plan state across calls in an instance
whose whole premise is that state does not outlive a request.
