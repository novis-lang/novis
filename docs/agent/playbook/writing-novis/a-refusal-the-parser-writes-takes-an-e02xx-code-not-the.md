- **A refusal the parser writes takes an `E02xx` code, not the `E01xx` "next free parser code".**
  The bands are by kind, not by which crate reports them: `E01xx` is a malformed parse, `E02xx` is a
  rejected PHP construct, and `E_IMPORT_ALIAS_UNSUPPORTED` (`E0212`), `E_ENUM_MEMBER_UNSUPPORTED`
  (`E0220`) and `E_IMPORT_GROUP_UNSUPPORTED` (`E0238`) are all reported from `parser/decl.rs`. Find
  the sibling refusal's code first and take the number next to it, whatever band `brief.py`'s
  next-free list points at. [until: gone crates/nvs-diagnostics/src/lib.rs:E_IMPORT_ALIAS_UNSUPPORTED]
