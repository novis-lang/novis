An extension may carry **two payloads in its one file**: the wasm component, and Novis source that
composes calls into it, in the `nvs.source` section
(`rule:packaging/an-nvsx-is-one-file-carrying-its-manifest`). Everything a program names is under one
namespace, the manifest registers **exactly one class** whose static methods are the component's closed
set of entry points, and every other class under that namespace is Novis source that calls them.
`Novis\Image` is the first of this shape — `Novis\Image\Codec` is the manifest's class, and the builder
above it is source (`rule:core-classes/image-pipeline`); `Novis\Spreadsheet\Engine` follows it
(`rule:core-classes/spreadsheet-has-no-io`).

Loading the extension makes its source resolvable by `autoload` with no line the program writes. The
source compiles like any other Novis file. **A third-party extension's source is a package**: its
namespace holds what a `[grants]` line names and nothing else. Where no line matches, it holds an
empty line, also where the application has no grant table at all, so the source can read no file and
reach no host until the operator writes one. The key is the namespace, so application code that
declares a namespace under the extension's is held to the same line. **A built-in component's source
keeps the application's authority**, because only Novis declares under `Novis\`. A manifest class
with one segment has no namespace to key on, so the loader refuses source beside it. The one pin
covers both payloads, so the source cannot be edited
under a pinned component, and `nvs ext inspect --source` prints it. A package carries a `.nvsx` as an
extension package (`rule:packaging/a-package-is-its-digest`), and its carried source must then be
under the package's namespace too.

The split is the point rather than a packaging convenience. Building a plan is data manipulation and
costs nothing to do in Novis; the codecs belong inside the sandbox. A builder that lived in the guest
would spend a boundary crossing per method to append to an array
(`rule:packaging/the-boundary-is-the-cost`), and would hold plan state across calls in an instance
whose whole premise is that state does not outlive a request.

**On disk.** `nvs_ext::source` reads the section, and the loader refuses a file whose
`namespace` is not under the class's own, and any file beside a class with one segment
(`crates/nvs-ext/tests/load.rs`). `nvs_types::capability::authority_of` gives a third-party
extension's namespace its implied empty line (`crates/nvs-cli/tests/package_grants.rs`). The graph walk resolves
a name under that namespace to the carried file with no `autoload` line
(`nvs_hir::ExtensionClass::source`), and `nvs run`, `nvs check` and `nvs test` fill that field
from the set's manifests
(`tests/conformance/ext/an-extension-source-class-resolves-with-no-autoload-line.nvst`). `nvs serve`
fills it from the live set's, which carry each extension's source files
(`crates/nvs-cli/tests/live_config.rs`).

**Declined:**
- **The source in a folder beside the `.nvsx`.** It would sit outside the pin, so it could change
  under a pinned component.
- **A minimal package system to carry the source.** It would design half of the package system
  ahead of the rest of it (`rule:packaging/a-package-is-its-digest`).
