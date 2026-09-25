- **A `Core\Attributes` retrieval probe proves nothing about `$member` unless the target itself
  carries a matching attribute.** A computed `$member` was being ignored rather than folded to the
  empty result, and a fixture whose class carries no matching literal prints `null` under either
  behaviour, so the first probe read as agreement with `rule:attributes/structural-retrieval`.
  Attach a matching literal to the class *and* to the member before asserting anything about
  `$member`, the way
  `tests/conformance/lang/attributes-retrieval-answers-by-shape-in-declaration-order.nvst` does.
  [until: gone tests/conformance/lang/attributes-retrieval-answers-by-shape-in-declaration-order.nvst:Core\Attributes]
