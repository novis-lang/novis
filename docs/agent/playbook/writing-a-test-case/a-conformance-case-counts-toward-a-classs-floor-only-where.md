- **A conformance case counts toward a class's floor only where its `--FILE--` body writes the class
  name, and a nullable return does not carry the name in for it.** `corpus::holders` attributes every
  `->member(` in a case to the classes that case *mentions*, plus one class per member whose return type
  is a bare `CoreTy::Instance` — a `CoreTy::Nullable(&CoreTy::Instance(…))` is deliberately excluded,
  since a case holding one has narrowed it first. So a class reached only through a nullable answer needs
  its name written inside `--FILE--`, in a comment if nowhere else; the `--TEST--` title is not read at
  all.
  [until: gone crates/nvs-stdlib/tests/corpus/mod.rs:fn holders]
