- **An attribute's name is already load-bearing for two other passes before
  `rule:attributes/attach-sites-and-forms` says what it means.**
  `rule:core-classes/derive-attribute` matches `#[Json\Derive]`/`#[Json\Field]` nominally against a
  closed `Core`-owned roster, and `rule:programs/autoload` harvests every attribute name as a
  reference the autoloader places by prefix. A rule about what an attribute name may resolve to has
  to exempt the first and leave the second alone;
  `tests/conformance/lang/a-class-named-only-by-an-attribute-is-autoloaded.nvst` and the
  `json-derive-*` cases are what fail first. [until: gone tests/conformance/lang/a-class-named-only-by-an-attribute-is-autoloaded.nvst:#[]
