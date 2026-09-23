- **`Core\Attributes` retrieval reads any *recognized* attribute's payload, and `get<T>` answers
  only the first of two on one method.** `rule:attributes/structural-retrieval` is structural, so
  `Core\Attributes::get<{name: string, about: string}>(Deploy::deploy(...))` answers a
  `#[Core\Command]`; read an alias with `all<T>` and index it. A payload-less `#[Option]` is
  indistinguishable from no attribute. [until: reviewed 2026-09-06]
