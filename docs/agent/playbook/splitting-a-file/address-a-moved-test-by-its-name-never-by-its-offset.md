- **Address a moved test by its name, never by its offset.** A test's item block starts at the doc
  comment above it, so an offset table addressed at `#[test]` lines files the first documented case
  of each group into the previous module, silently. A name table also refuses to run at all when a
  case is added or renamed, which is the failure you want. [until: reviewed 2026-09-06]
