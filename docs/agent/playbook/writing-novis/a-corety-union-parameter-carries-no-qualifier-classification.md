- **A `CoreTy::Union` parameter carries no qualifier classification, so a text-like union is a
  `tainted`-refusing sink by accident.** `CoreTy::classification` answers `Some` only for
  `Text`/`Blob`/`SecretBlob`/`Entry`, and `admits_tainted_argument` maps `None` to `false` like a
  declared sink, so `Union(&[Text(Qual::Neutral), Blob(Qual::Neutral)])` passes every registry gate
  and refuses with `expected string|bytes, found tainted string`. A `Qual` lives on a leaf variant:
  write two classified parameters (`send`/`sendBytes`) rather than one union.
  [until: reviewed 2026-09-06]
