- **A `?T` *inside* an inline shape is refused at the declaration, and `E0756` names the whole shape
  rather than the arm that caused it.** `json_reachable` strips a field's `null` arm only at the top
  level — `codec_field` does it before pushing the site, and the `Ty::Shape` recursion asks the
  written member type — so `{note: ?string}` reads as having no JSON representation while
  `?{note: string}` is fine. Write `rule:types/shape-type`'s optional column instead,
  `{note?: string}`, which is the independent question and is reachable.
  [until: gone crates/nvs-types/src/derive.rs:Ty::Shape(fields) => fields]
