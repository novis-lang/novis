An extension call is an ordinary operation for the purposes of taint. If any argument is `tainted`,
every `string`/`bytes` in the result is `tainted`. This requires nothing in the manifest and nothing
new in the checker beyond treating an extension call like any other call.

Contagion is the default rather than the declared case because over-strict is the safe failure
direction: an extension that declares nothing gets the conservative answer, and a formatted tainted
number is still tainted. An extension cannot itself be a sink — it is a component with no ambient
authority, so everything it does to the world outside travels through a `Core` member, and that member
is classified (`rule:security/unclassified-parameter-refuses-tainted`).

The boundary has to take part because a first-party component crosses it too. A `tainted` string from
the request is routinely handed to the internationalization component and comes back formatted; if that
round trip cleaned it, the standard library would itself be the bypass.

**On disk.** A parameter with no declaration is `Qual::Contagious` in `nvs_types::ext_lib`
(`tests/conformance/ext/an-extension-result-is-tainted-when-an-argument-was.nvst`). The result
reaches into a shape's fields, which a `Core` row's does not: an extension's record is built by the
call, so every field of it is the call's output. Each method's `MethodSig::contagious_return` is
that tainted result, and a result with no `string` or `bytes` in it refuses a `tainted` argument
rather than dropping the bit (`crates/nvs-types/tests/extensions.rs`). A built-in component's
engine is no exception, so a component's own source types what an upload's records carry as
`tainted`: the cells `Novis\Spreadsheet\Reader` reads from a `tainted bytes` workbook are
(`tests/conformance/novis/spreadsheet-a-cell-read-from-an-upload-is-tainted.nvst`).

**Declined:**
- **Stripping qualifiers at the boundary.** Nothing to design and nothing to freeze into the extension
  ABI, but unsound rather than merely limited: passing a value through any `.nvsx` would launder it.
