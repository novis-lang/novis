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
(`tests/conformance/ext/an-extension-result-is-tainted-when-an-argument-was.nvst`).

**Declined:**
- **Stripping qualifiers at the boundary.** Nothing to design and nothing to freeze into the extension
  ABI, but unsound rather than merely limited: passing a value through any `.nvsx` would launder it.
